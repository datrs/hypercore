//! Hypercore's main abstraction. Exposes an append-only, secure log structure.
pub(crate) mod inner;

use futures::future::Either;
use tracing::instrument;

#[cfg(feature = "cache")]
use crate::common::cache::CacheOptions;
use crate::{
    common::{BitfieldUpdate, HypercoreError, StoreInfo},
    core::inner::HypercoreInnerInner,
    crypto::PartialKeypair,
    storage::Storage,
};
use hypercore_schema::{Proof, RequestBlock, RequestSeek, RequestUpgrade};

pub(crate) use inner::HypercoreInner;
use inner::update_contiguous_length;

#[derive(Debug)]
pub(crate) struct HypercoreOptions {
    pub(crate) key_pair: Option<PartialKeypair>,
    pub(crate) open: bool,
    #[cfg(feature = "cache")]
    pub(crate) node_cache_options: Option<CacheOptions>,
}

impl HypercoreOptions {
    pub(crate) fn new() -> Self {
        Self {
            key_pair: None,
            open: false,
            #[cfg(feature = "cache")]
            node_cache_options: None,
        }
    }
}

macro_rules! ininner {
    ($self:expr) => {
        $self.inner.inner.lock().unwrap()
    };
}

/// Hypercore is an append-only log structure.
#[derive(Debug, Clone)]
pub struct Hypercore {
    pub(crate) inner: HypercoreInner,
}

/// Response from append, matches that of the Javascript result
#[derive(Debug, PartialEq)]
pub struct AppendOutcome {
    /// Length of the hypercore after append
    pub length: u64,
    /// Byte length of the hypercore after append
    pub byte_length: u64,
}

/// Info about the hypercore
#[derive(Debug, PartialEq)]
pub struct Info {
    /// Length of the hypercore
    pub length: u64,
    /// Byte length of the hypercore
    pub byte_length: u64,
    /// Continuous length of entries in the hypercore with data
    /// starting from index 0
    pub contiguous_length: u64,
    /// Fork index. 0 if hypercore not forked.
    pub fork: u64,
    /// True if hypercore is writeable, false if read-only
    pub writeable: bool,
}

impl Hypercore {
    /// Creates/opens new hypercore using given storage and options
    pub(crate) async fn new(
        storage: Storage,
        options: HypercoreOptions,
    ) -> Result<Hypercore, HypercoreError> {
        Ok(Hypercore {
            inner: HypercoreInner::new(storage, options).await?,
        })
    }

    /// Gets basic info about the Hypercore
    pub fn info(&self) -> Info {
        self.inner.info()
    }

    /// Appends a data slice to the hypercore.
    #[instrument(err, skip_all, fields(data_len = data.len()))]
    pub async fn append(&self, data: &[u8]) -> Result<AppendOutcome, HypercoreError> {
        self.append_batch(&[data]).await
    }

    /// Appends a given batch of data slices to the hypercore.
    #[instrument(err, skip_all, fields(batch_len = batch.as_ref().len()))]
    pub async fn append_batch<A: AsRef<[u8]>, B: AsRef<[A]>>(
        &self,
        batch: B,
    ) -> Result<AppendOutcome, HypercoreError> {
        let secret_key = match self.inner.key_pair().secret {
            Some(key) => key,
            None => return Err(HypercoreError::NotWritable),
        };

        if batch.as_ref().is_empty() {
            return Ok(self.inner.append_outcome());
        }
        // Create a changeset for the tree
        let mut changeset = ininner!(self).tree.changeset();
        let mut batch_length: usize = 0;
        for data in batch.as_ref().iter() {
            batch_length += changeset.append(data.as_ref());
        }
        changeset.hash_and_sign(&secret_key);

        // Write the received data to the block store
        let byte_length = ininner!(self).tree.byte_length;
        let info =
            ininner!(self)
                .block_store
                .append_batch(batch.as_ref(), batch_length, byte_length);
        { ininner!(self).storage.flush_info(info) }.await?;

        // Append the changeset to the Oplog
        let bitfield_update = BitfieldUpdate {
            drop: false,
            start: changeset.ancestors,
            length: changeset.batch_length,
        };
        let outcome = {
            let HypercoreInnerInner { oplog, header, .. } = &mut *ininner!(self);
            oplog.append_changeset(&changeset, Some(bitfield_update.clone()), false, header)?
        };
        {
            ininner!(self)
                .storage
                .flush_infos(Vec::from(outcome.infos_to_flush))
        }
        .await?;
        ininner!(self).header = outcome.header;

        // Write to bitfield
        ininner!(self).bitfield.update(&bitfield_update);

        // Contiguous length is known only now
        {
            let HypercoreInnerInner {
                bitfield, header, ..
            } = &mut *ininner!(self);
            update_contiguous_length(header, bitfield, &bitfield_update);
        }

        // Commit changeset to in-memory tree
        ininner!(self).tree.commit(changeset)?;

        // Now ready to flush
        if self.inner.should_flush_bitfield_and_tree_and_oplog() {
            self.inner.flush_bitfield_and_tree_and_oplog(false).await?;
        }

        #[cfg(feature = "replication")]
        {
            use tracing::trace;

            trace!(bitfield_update = ?bitfield_update, "Hppercore.append_batch emit DataUpgrade & Have");
            let _ = ininner!(self)
                .events
                .send(crate::replication::events::DataUpgrade {});
            let _ = ininner!(self)
                .events
                .send(crate::replication::events::Have::from(&bitfield_update));
        }

        Ok(self.inner.append_outcome())
    }

    #[cfg(feature = "replication")]
    /// Subscribe to core events relevant to replication
    pub fn event_subscribe(&self) -> async_broadcast::Receiver<crate::replication::events::Event> {
        self.inner.event_subscribe()
    }

    /// Check if core has the block at the given `index` locally
    #[instrument(ret, skip(self))]
    pub fn has(&self, index: u64) -> bool {
        self.inner.has(index)
    }

    /// Read value at given index, if any.
    #[instrument(err, skip(self))]
    pub async fn get(&self, index: u64) -> Result<Option<Vec<u8>>, HypercoreError> {
        Ok(self.inner.get(index).await?.map(|b| b.into_vec()))
    }

    /// Clear data for entries between start and end (exclusive) indexes.
    #[instrument(err, skip(self))]
    pub async fn clear(&mut self, start: u64, end: u64) -> Result<(), HypercoreError> {
        if start >= end {
            // NB: This is what javascript does, so we mimic that here
            return Ok(());
        }
        // Write to oplog
        let infos_to_flush = ininner!(self).oplog.clear(start, end)?;
        {
            ininner!(self)
                .storage
                .flush_infos(Vec::from(infos_to_flush))
        }
        .await?;

        // Set bitfield
        ininner!(self).bitfield.set_range(start, end - start, false);

        // Set contiguous length
        if start < ininner!(self).header.hints.contiguous_length {
            ininner!(self).header.hints.contiguous_length = start;
        }

        // Find the biggest hole that can be punched into the data
        let start = if let Some(index) = ininner!(self).bitfield.last_index_of(true, start) {
            index + 1
        } else {
            0
        };
        let end = if let Some(index) = ininner!(self).bitfield.index_of(true, end) {
            index
        } else {
            ininner!(self).tree.length
        };

        // Find byte offset for first value
        let mut infos: Vec<StoreInfo> = Vec::new();
        let clear_offset = match { ininner!(self).tree.byte_offset(start, None)? } {
            Either::Right(value) => value,
            Either::Left(instructions) => {
                let new_infos = {
                    ininner!(self)
                        .storage
                        .read_infos_to_vec(Vec::from(instructions))
                }
                .await?;
                infos.extend(new_infos);
                match ininner!(self).tree.byte_offset(start, Some(&infos))? {
                    Either::Right(value) => value,
                    Either::Left(_) => {
                        return Err(HypercoreError::InvalidOperation {
                            context: format!("Could not read offset for index {start} from tree"),
                        });
                    }
                }
            }
        };

        // Find byte range for last value
        let last_byte_range = self.inner.byte_range(end - 1, infos).await?;

        let clear_length = (last_byte_range.index + last_byte_range.length) - clear_offset;

        // Clear blocks
        let info_to_flush = ininner!(self).block_store.clear(clear_offset, clear_length);
        { ininner!(self).storage.flush_info(info_to_flush) }.await?;

        // Now ready to flush
        if self.inner.should_flush_bitfield_and_tree_and_oplog() {
            self.inner.flush_bitfield_and_tree_and_oplog(false).await?;
        }

        Ok(())
    }

    /// Access the key pair.
    pub fn key_pair(&self) -> PartialKeypair {
        self.inner.key_pair()
    }

    /// Create a proof for given request
    #[instrument(err, skip_all)]
    pub async fn create_proof(
        &self,
        block: Option<RequestBlock>,
        hash: Option<RequestBlock>,
        seek: Option<RequestSeek>,
        upgrade: Option<RequestUpgrade>,
    ) -> Result<Option<Proof>, HypercoreError> {
        self.inner.create_proof(block, hash, seek, upgrade).await
    }

    /// Verify and apply proof received from peer, returns true if changed, false if not
    /// possible to apply.
    #[instrument(skip_all)]
    pub async fn verify_and_apply_proof(&self, proof: Proof) -> Result<bool, HypercoreError> {
        self.inner.verify_and_apply_proof(proof).await
    }

    #[allow(dead_code)]
    async fn verify_and_apply_proof_old(&self, proof: &Proof) -> Result<bool, HypercoreError> {
        if proof.fork != ininner!(self).tree.fork {
            return Ok(false);
        }
        let changeset = self.inner.verify_proof(proof.clone()).await?;
        if !ininner!(self).tree.commitable(&changeset) {
            return Ok(false);
        }

        // In javascript there's _verifyExclusive and _verifyShared based on changeset.upgraded, but
        // here we do only one. _verifyShared groups together many subsequent changesets into a single
        // oplog push, and then flushes in the end only for the whole group.
        let bitfield_update: Option<BitfieldUpdate> = if let Some(block) = &proof.block.as_ref() {
            let byte_offset = match {
                ininner!(self)
                    .tree
                    .byte_offset_in_changeset(block.index, &changeset, None)?
            } {
                Either::Right(value) => value,
                Either::Left(instructions) => {
                    let infos = {
                        ininner!(self)
                            .storage
                            .read_infos_to_vec(Vec::from(instructions))
                    }
                    .await?;
                    match ininner!(self).tree.byte_offset_in_changeset(
                        block.index,
                        &changeset,
                        Some(&infos),
                    )? {
                        Either::Right(value) => value,
                        Either::Left(_) => {
                            return Err(HypercoreError::InvalidOperation {
                                context: format!(
                                    "Could not read offset for index {} from tree",
                                    block.index
                                ),
                            });
                        }
                    }
                }
            };

            // Write the value to the block store
            let info_to_flush = ininner!(self).block_store.put(&block.value, byte_offset);
            { ininner!(self).storage.flush_info(info_to_flush) }.await?;

            // Return a bitfield update for the given value
            Some(BitfieldUpdate {
                drop: false,
                start: block.index,
                length: 1,
            })
        } else {
            // Only from DataBlock can there be changes to the bitfield
            None
        };

        // Append the changeset to the Oplog
        let outcome = {
            let HypercoreInnerInner { oplog, header, .. } = &mut *ininner!(self);
            oplog.append_changeset(&changeset, bitfield_update.clone(), false, header)?
        };
        {
            ininner!(self)
                .storage
                .flush_infos(Vec::from(outcome.infos_to_flush))
        }
        .await?;
        ininner!(self).header = outcome.header;

        if let Some(bitfield_update) = &bitfield_update {
            // Write to bitfield
            ininner!(self).bitfield.update(bitfield_update);

            // Contiguous length is known only now
            {
                let HypercoreInnerInner {
                    bitfield, header, ..
                } = &mut *ininner!(self);
                update_contiguous_length(header, bitfield, bitfield_update);
            }
        }

        // Commit changeset to in-memory tree
        ininner!(self).tree.commit(changeset)?;

        // Now ready to flush
        if self.inner.should_flush_bitfield_and_tree_and_oplog() {
            self.inner.flush_bitfield_and_tree_and_oplog(false).await?;
        }

        #[cfg(feature = "replication")]
        {
            if proof.upgrade.is_some() {
                // Notify replicator if we receieved an upgrade
                let _ = ininner!(self)
                    .events
                    .send(crate::replication::events::DataUpgrade {});
            }

            // Notify replicator if we receieved a bitfield update
            if let Some(ref bitfield) = bitfield_update {
                let _ = ininner!(self)
                    .events
                    .send(crate::replication::events::Have::from(bitfield));
            }
        }
        Ok(true)
    }

    /// Used to fill the nodes field of a `RequestBlock` during
    /// synchronization.
    #[instrument(err, skip(self))]
    pub async fn missing_nodes(&self, index: u64) -> Result<u64, HypercoreError> {
        self.inner
            .missing_nodes_from_merkle_tree_index(index * 2)
            .await
    }

    /// Get missing nodes using a merkle tree index. Advanced variant of missing_nodes
    /// that allow for special cases of searching directly from the merkle tree.
    #[instrument(err, skip(self))]
    pub async fn missing_nodes_from_merkle_tree_index(
        &self,
        merkle_tree_index: u64,
    ) -> Result<u64, HypercoreError> {
        self.inner
            .missing_nodes_from_merkle_tree_index(merkle_tree_index)
            .await
    }

    /// Makes the hypercore read-only by deleting the secret key. Returns true if the
    /// hypercore was changed, false if the hypercore was already read-only. This is useful
    /// in scenarios where a hypercore should be made immutable after initial values have
    /// been stored.
    #[instrument(err, skip_all)]
    pub async fn make_read_only(&mut self) -> Result<bool, HypercoreError> {
        if ininner!(self).key_pair.secret.is_some() {
            ininner!(self).key_pair.secret = None;
            ininner!(self).header.key_pair.secret = None;
            // Need to flush clearing traces to make sure both oplog slots are cleared
            { ininner!(self).flush_bitfield_and_tree_and_oplog(true) }.await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::crypto::{PartialKeypair, generate_signing_key};

    #[tokio::test]
    async fn core_create_proof_block_only() -> Result<(), HypercoreError> {
        let hypercore = create_hypercore_with_data(10).await?;

        let proof = hypercore
            .create_proof(Some(RequestBlock { index: 4, nodes: 2 }), None, None, None)
            .await?
            .unwrap();
        let block = proof.block.unwrap();
        assert_eq!(proof.upgrade, None);
        assert_eq!(proof.seek, None);
        assert_eq!(block.index, 4);
        assert_eq!(block.nodes.len(), 2);
        assert_eq!(block.nodes[0].index, 10);
        assert_eq!(block.nodes[1].index, 13);
        Ok(())
    }

    #[tokio::test]
    async fn core_create_proof_block_and_upgrade() -> Result<(), HypercoreError> {
        let hypercore = create_hypercore_with_data(10).await?;
        let proof = hypercore
            .create_proof(
                Some(RequestBlock { index: 4, nodes: 0 }),
                None,
                None,
                Some(RequestUpgrade {
                    start: 0,
                    length: 10,
                }),
            )
            .await?
            .unwrap();
        let block = proof.block.unwrap();
        let upgrade = proof.upgrade.unwrap();
        assert_eq!(proof.seek, None);
        assert_eq!(block.index, 4);
        assert_eq!(block.nodes.len(), 3);
        assert_eq!(block.nodes[0].index, 10);
        assert_eq!(block.nodes[1].index, 13);
        assert_eq!(block.nodes[2].index, 3);
        assert_eq!(upgrade.start, 0);
        assert_eq!(upgrade.length, 10);
        assert_eq!(upgrade.nodes.len(), 1);
        assert_eq!(upgrade.nodes[0].index, 17);
        assert_eq!(upgrade.additional_nodes.len(), 0);
        Ok(())
    }

    #[tokio::test]
    async fn core_create_proof_block_and_upgrade_and_additional() -> Result<(), HypercoreError> {
        let hypercore = create_hypercore_with_data(10).await?;
        let proof = hypercore
            .create_proof(
                Some(RequestBlock { index: 4, nodes: 0 }),
                None,
                None,
                Some(RequestUpgrade {
                    start: 0,
                    length: 8,
                }),
            )
            .await?
            .unwrap();
        let block = proof.block.unwrap();
        let upgrade = proof.upgrade.unwrap();
        assert_eq!(proof.seek, None);
        assert_eq!(block.index, 4);
        assert_eq!(block.nodes.len(), 3);
        assert_eq!(block.nodes[0].index, 10);
        assert_eq!(block.nodes[1].index, 13);
        assert_eq!(block.nodes[2].index, 3);
        assert_eq!(upgrade.start, 0);
        assert_eq!(upgrade.length, 8);
        assert_eq!(upgrade.nodes.len(), 0);
        assert_eq!(upgrade.additional_nodes.len(), 1);
        assert_eq!(upgrade.additional_nodes[0].index, 17);
        Ok(())
    }

    #[tokio::test]
    async fn core_create_proof_block_and_upgrade_from_existing_state() -> Result<(), HypercoreError>
    {
        let hypercore = create_hypercore_with_data(10).await?;
        let proof = hypercore
            .create_proof(
                Some(RequestBlock { index: 1, nodes: 0 }),
                None,
                None,
                Some(RequestUpgrade {
                    start: 1,
                    length: 9,
                }),
            )
            .await?
            .unwrap();
        let block = proof.block.unwrap();
        let upgrade = proof.upgrade.unwrap();
        assert_eq!(proof.seek, None);
        assert_eq!(block.index, 1);
        assert_eq!(block.nodes.len(), 0);
        assert_eq!(upgrade.start, 1);
        assert_eq!(upgrade.length, 9);
        assert_eq!(upgrade.nodes.len(), 3);
        assert_eq!(upgrade.nodes[0].index, 5);
        assert_eq!(upgrade.nodes[1].index, 11);
        assert_eq!(upgrade.nodes[2].index, 17);
        assert_eq!(upgrade.additional_nodes.len(), 0);
        Ok(())
    }

    #[tokio::test]
    async fn core_create_proof_block_and_upgrade_from_existing_state_with_additional()
    -> Result<(), HypercoreError> {
        let hypercore = create_hypercore_with_data(10).await?;
        let proof = hypercore
            .create_proof(
                Some(RequestBlock { index: 1, nodes: 0 }),
                None,
                None,
                Some(RequestUpgrade {
                    start: 1,
                    length: 5,
                }),
            )
            .await?
            .unwrap();
        let block = proof.block.unwrap();
        let upgrade = proof.upgrade.unwrap();
        assert_eq!(proof.seek, None);
        assert_eq!(block.index, 1);
        assert_eq!(block.nodes.len(), 0);
        assert_eq!(upgrade.start, 1);
        assert_eq!(upgrade.length, 5);
        assert_eq!(upgrade.nodes.len(), 2);
        assert_eq!(upgrade.nodes[0].index, 5);
        assert_eq!(upgrade.nodes[1].index, 9);
        assert_eq!(upgrade.additional_nodes.len(), 2);
        assert_eq!(upgrade.additional_nodes[0].index, 13);
        assert_eq!(upgrade.additional_nodes[1].index, 17);
        Ok(())
    }

    #[tokio::test]
    async fn core_create_proof_block_and_seek_1_no_upgrade() -> Result<(), HypercoreError> {
        let hypercore = create_hypercore_with_data(10).await?;
        let proof = hypercore
            .create_proof(
                Some(RequestBlock { index: 4, nodes: 2 }),
                None,
                Some(RequestSeek { bytes: 8 }),
                None,
            )
            .await?
            .unwrap();
        let block = proof.block.unwrap();
        assert_eq!(proof.seek, None); // seek included in block
        assert_eq!(proof.upgrade, None);
        assert_eq!(block.index, 4);
        assert_eq!(block.nodes.len(), 2);
        assert_eq!(block.nodes[0].index, 10);
        assert_eq!(block.nodes[1].index, 13);
        Ok(())
    }

    #[tokio::test]
    async fn core_create_proof_block_and_seek_2_no_upgrade() -> Result<(), HypercoreError> {
        let hypercore = create_hypercore_with_data(10).await?;
        let proof = hypercore
            .create_proof(
                Some(RequestBlock { index: 4, nodes: 2 }),
                None,
                Some(RequestSeek { bytes: 10 }),
                None,
            )
            .await?
            .unwrap();
        let block = proof.block.unwrap();
        assert_eq!(proof.seek, None); // seek included in block
        assert_eq!(proof.upgrade, None);
        assert_eq!(block.index, 4);
        assert_eq!(block.nodes.len(), 2);
        assert_eq!(block.nodes[0].index, 10);
        assert_eq!(block.nodes[1].index, 13);
        Ok(())
    }

    #[tokio::test]
    async fn core_create_proof_block_and_seek_3_no_upgrade() -> Result<(), HypercoreError> {
        let hypercore = create_hypercore_with_data(10).await?;
        let proof = hypercore
            .create_proof(
                Some(RequestBlock { index: 4, nodes: 2 }),
                None,
                Some(RequestSeek { bytes: 13 }),
                None,
            )
            .await?
            .unwrap();
        let block = proof.block.unwrap();
        let seek = proof.seek.unwrap();
        assert_eq!(proof.upgrade, None);
        assert_eq!(block.index, 4);
        assert_eq!(block.nodes.len(), 1);
        assert_eq!(block.nodes[0].index, 10);
        assert_eq!(seek.nodes.len(), 2);
        assert_eq!(seek.nodes[0].index, 12);
        assert_eq!(seek.nodes[1].index, 14);
        Ok(())
    }

    #[tokio::test]
    async fn core_create_proof_block_and_seek_to_tree_no_upgrade() -> Result<(), HypercoreError> {
        let hypercore = create_hypercore_with_data(16).await?;
        let proof = hypercore
            .create_proof(
                Some(RequestBlock { index: 0, nodes: 4 }),
                None,
                Some(RequestSeek { bytes: 26 }),
                None,
            )
            .await?
            .unwrap();
        let block = proof.block.unwrap();
        let seek = proof.seek.unwrap();
        assert_eq!(proof.upgrade, None);
        assert_eq!(block.nodes.len(), 3);
        assert_eq!(block.nodes[0].index, 2);
        assert_eq!(block.nodes[1].index, 5);
        assert_eq!(block.nodes[2].index, 11);
        assert_eq!(seek.nodes.len(), 2);
        assert_eq!(seek.nodes[0].index, 19);
        assert_eq!(seek.nodes[1].index, 27);
        Ok(())
    }

    #[tokio::test]
    async fn core_create_proof_block_and_seek_with_upgrade() -> Result<(), HypercoreError> {
        let hypercore = create_hypercore_with_data(10).await?;
        let proof = hypercore
            .create_proof(
                Some(RequestBlock { index: 4, nodes: 2 }),
                None,
                Some(RequestSeek { bytes: 13 }),
                Some(RequestUpgrade {
                    start: 8,
                    length: 2,
                }),
            )
            .await?
            .unwrap();
        let block = proof.block.unwrap();
        let seek = proof.seek.unwrap();
        let upgrade = proof.upgrade.unwrap();
        assert_eq!(block.index, 4);
        assert_eq!(block.nodes.len(), 1);
        assert_eq!(block.nodes[0].index, 10);
        assert_eq!(seek.nodes.len(), 2);
        assert_eq!(seek.nodes[0].index, 12);
        assert_eq!(seek.nodes[1].index, 14);
        assert_eq!(upgrade.nodes.len(), 1);
        assert_eq!(upgrade.nodes[0].index, 17);
        assert_eq!(upgrade.additional_nodes.len(), 0);
        Ok(())
    }

    #[tokio::test]
    async fn core_create_proof_seek_with_upgrade() -> Result<(), HypercoreError> {
        let hypercore = create_hypercore_with_data(10).await?;
        let proof = hypercore
            .create_proof(
                None,
                None,
                Some(RequestSeek { bytes: 13 }),
                Some(RequestUpgrade {
                    start: 0,
                    length: 10,
                }),
            )
            .await?
            .unwrap();
        let seek = proof.seek.unwrap();
        let upgrade = proof.upgrade.unwrap();
        assert_eq!(proof.block, None);
        assert_eq!(seek.nodes.len(), 4);
        assert_eq!(seek.nodes[0].index, 12);
        assert_eq!(seek.nodes[1].index, 14);
        assert_eq!(seek.nodes[2].index, 9);
        assert_eq!(seek.nodes[3].index, 3);
        assert_eq!(upgrade.nodes.len(), 1);
        assert_eq!(upgrade.nodes[0].index, 17);
        assert_eq!(upgrade.additional_nodes.len(), 0);
        Ok(())
    }

    #[tokio::test]
    async fn core_verify_proof_invalid_signature() -> Result<(), HypercoreError> {
        let hypercore = create_hypercore_with_data(10).await?;
        // Invalid clone hypercore with a different public key
        let hypercore_clone = create_hypercore_with_data(0).await?;
        let proof = hypercore
            .create_proof(
                None,
                Some(RequestBlock { index: 6, nodes: 0 }),
                None,
                Some(RequestUpgrade {
                    start: 0,
                    length: 10,
                }),
            )
            .await?
            .unwrap();
        assert!(hypercore_clone.verify_and_apply_proof(proof).await.is_err());
        Ok(())
    }

    #[tokio::test]
    async fn core_verify_and_apply_proof() -> Result<(), HypercoreError> {
        let main = create_hypercore_with_data(10).await?;
        let clone = create_hypercore_with_data_and_key_pair(
            0,
            PartialKeypair {
                public: { ininner!(main).key_pair.public },
                secret: None,
            },
        )
        .await?;
        let index = 6;
        let nodes = clone.missing_nodes(index).await?;
        let proof = main
            .create_proof(
                None,
                Some(RequestBlock { index, nodes }),
                None,
                Some(RequestUpgrade {
                    start: 0,
                    length: 10,
                }),
            )
            .await?
            .unwrap();
        assert!(clone.verify_and_apply_proof(proof).await?);
        let main_info = main.info();
        let clone_info = clone.info();
        assert_eq!(main_info.byte_length, clone_info.byte_length);
        assert_eq!(main_info.length, clone_info.length);
        assert!(main.get(6).await?.is_some());
        assert!(clone.get(6).await?.is_none());

        // Fetch data for index 6 and verify it is found
        let index = 6;
        let nodes = clone.missing_nodes(index).await?;
        let proof = main
            .create_proof(Some(RequestBlock { index, nodes }), None, None, None)
            .await?
            .unwrap();
        assert!(clone.verify_and_apply_proof(proof).await?);
        Ok(())
    }

    pub(crate) async fn create_hypercore_with_data(
        length: u64,
    ) -> Result<Hypercore, HypercoreError> {
        let signing_key = generate_signing_key();
        create_hypercore_with_data_and_key_pair(
            length,
            PartialKeypair {
                public: signing_key.verifying_key(),
                secret: Some(signing_key),
            },
        )
        .await
    }

    pub(crate) async fn create_hypercore_with_data_and_key_pair(
        length: u64,
        key_pair: PartialKeypair,
    ) -> Result<Hypercore, HypercoreError> {
        let storage = Storage::new_memory().await?;
        let hypercore = Hypercore::new(
            storage,
            HypercoreOptions {
                key_pair: Some(key_pair),
                open: false,
                #[cfg(feature = "cache")]
                node_cache_options: None,
            },
        )
        .await?;
        for i in 0..length {
            hypercore.append(format!("#{}", i).as_bytes()).await?;
        }
        Ok(hypercore)
    }
}
