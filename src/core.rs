//! Hypercore's main abstraction. Exposes an append-only, secure log structure.

use crate::{
    bitfield_v10::Bitfield,
    common::{Store, StoreInfoInstruction},
    crypto::generate_keypair,
    data::BlockStore,
    oplog::{Header, Oplog, MAX_OPLOG_ENTRIES_BYTE_SIZE},
    storage_v10::{PartialKeypair, Storage},
    tree::MerkleTree,
};
use anyhow::{anyhow, Result};
use ed25519_dalek::Signature;
use futures::future::Either;
use random_access_storage::RandomAccess;
use std::fmt::Debug;

/// Hypercore is an append-only log structure.
#[derive(Debug)]
pub struct Hypercore<T>
where
    T: RandomAccess<Error = Box<dyn std::error::Error + Send + Sync>> + Debug,
{
    pub(crate) key_pair: PartialKeypair,
    pub(crate) storage: Storage<T>,
    pub(crate) oplog: Oplog,
    pub(crate) tree: MerkleTree,
    pub(crate) block_store: BlockStore,
    pub(crate) bitfield: Bitfield,
    skip_flush_count: u8, // autoFlush in Javascript
    header: Header,
}

/// Response from append, matches that of the Javascript result
#[derive(Debug)]
pub struct AppendOutcome {
    pub length: u64,
    pub byte_length: u64,
}

impl<T> Hypercore<T>
where
    T: RandomAccess<Error = Box<dyn std::error::Error + Send + Sync>> + Debug + Send,
{
    /// Creates new hypercore using given storage with random key pair
    pub async fn new(storage: Storage<T>) -> Result<Hypercore<T>> {
        let key_pair = generate_keypair();
        Hypercore::new_with_key_pair(
            storage,
            PartialKeypair {
                public: key_pair.public,
                secret: Some(key_pair.secret),
            },
        )
        .await
    }

    /// Creates new hypercore with given storage and (partial) key pair
    pub async fn new_with_key_pair(
        mut storage: Storage<T>,
        key_pair: PartialKeypair,
    ) -> Result<Hypercore<T>> {
        // Open/create oplog
        let oplog_bytes = storage
            .read_info(StoreInfoInstruction::new_all_content(Store::Oplog))
            .await?
            .data
            .expect("Did not receive data");

        let mut oplog_open_outcome = Oplog::open(key_pair.clone(), oplog_bytes)?;
        storage
            .flush_infos(&oplog_open_outcome.infos_to_flush)
            .await?;

        // Open/create tree
        let info_instructions =
            MerkleTree::get_info_instructions_to_read(&oplog_open_outcome.header.tree);
        let infos = storage.read_infos(&info_instructions).await?;
        let mut tree = MerkleTree::open(&oplog_open_outcome.header.tree, infos)?;

        // Create block store instance
        let block_store = BlockStore::default();

        // Open bitfield
        let bitfield_store_length = storage
            .read_info(StoreInfoInstruction::new_size(Store::Bitfield, 0))
            .await?
            .length
            .expect("Did not get store length with size instruction");
        let info_instruction = Bitfield::get_info_instruction_to_read(bitfield_store_length);
        let info = storage.read_info(info_instruction).await?;
        let mut bitfield = Bitfield::open(info);

        // Process entries stored only to the oplog and not yet flushed into bitfield or tree
        if let Some(entries) = oplog_open_outcome.entries {
            for entry in entries.iter() {
                for node in &entry.tree_nodes {
                    tree.add_node(node.clone());
                }

                if let Some(entry_bitfield) = &entry.bitfield {
                    bitfield.set_range(
                        entry_bitfield.start,
                        entry_bitfield.length,
                        !entry_bitfield.drop,
                    );
                    update_contiguous_length(
                        &mut oplog_open_outcome.header,
                        &bitfield,
                        entry_bitfield.drop,
                        entry_bitfield.start,
                        entry_bitfield.length,
                    );
                }
                if let Some(tree_upgrade) = &entry.tree_upgrade {
                    // TODO: Generalize Either response stack
                    let mut changeset =
                        match tree.truncate(tree_upgrade.length, tree_upgrade.fork, None)? {
                            Either::Right(changeset) => changeset,
                            Either::Left(instructions) => {
                                let infos = storage.read_infos(&instructions).await?;
                                match tree.truncate(
                                    tree_upgrade.length,
                                    tree_upgrade.fork,
                                    Some(&infos),
                                )? {
                                    Either::Right(changeset) => changeset,
                                    Either::Left(_) => {
                                        return Err(anyhow!("Could not truncate"));
                                    }
                                }
                            }
                        };
                    changeset.ancestors = tree_upgrade.ancestors;
                    changeset.signature = Some(Signature::from_bytes(&tree_upgrade.signature)?);

                    // TODO: Skip reorg hints for now, seems to only have to do with replication
                    // addReorgHint(header.hints.reorgs, tree, batch)

                    // Commit changeset to in-memory tree
                    tree.commit(changeset)?;
                }
            }
        }

        Ok(Hypercore {
            key_pair,
            storage,
            oplog: oplog_open_outcome.oplog,
            tree,
            block_store,
            bitfield,
            skip_flush_count: 0,
            header: oplog_open_outcome.header,
        })
    }

    /// Appends a data slice to the hypercore.
    pub async fn append(&mut self, data: &[u8]) -> Result<AppendOutcome> {
        self.append_batch(&[data]).await
    }

    /// Appends a given batch of data slices to the hypercore.
    pub async fn append_batch(&mut self, batch: &[&[u8]]) -> Result<AppendOutcome> {
        let secret_key = match &self.key_pair.secret {
            Some(key) => key,
            None => anyhow::bail!("No secret key, cannot append."),
        };

        if !batch.is_empty() {
            // Create a changeset for the tree
            let mut changeset = self.tree.changeset();
            let mut batch_length: usize = 0;
            for data in batch.iter() {
                batch_length += changeset.append(data);
            }
            changeset.hash_and_sign(&self.key_pair.public, &secret_key);

            // Write the received data to the block store
            let info = self
                .block_store
                .append_batch(batch, batch_length, self.tree.byte_length);
            self.storage.flush_info(info).await?;

            // Append the changeset to the Oplog
            let outcome = self.oplog.append_changeset(&changeset, false, &self.header);
            self.storage.flush_infos(&outcome.infos_to_flush).await?;
            self.header = outcome.header;

            // Write to bitfield
            self.bitfield.set_range(
                changeset.ancestors,
                changeset.length - changeset.ancestors,
                true,
            );

            // Contiguous length is known only now
            update_contiguous_length(
                &mut self.header,
                &self.bitfield,
                false,
                changeset.ancestors,
                changeset.batch_length,
            );

            // Commit changeset to in-memory tree
            self.tree.commit(changeset)?;

            // Now ready to flush
            if self.should_flush_bitfield_and_tree_and_oplog() {
                self.flush_bitfield_and_tree_and_oplog().await?;
            }
        }

        // Return the new value
        Ok(AppendOutcome {
            length: self.tree.length,
            byte_length: self.tree.byte_length,
        })
    }

    /// Read value at given index, if any.
    pub async fn get(&mut self, index: u64) -> Result<Option<Vec<u8>>> {
        if !self.bitfield.get(index) {
            return Ok(None);
        }

        // TODO: Generalize Either response stack
        let byte_range = match self.tree.byte_range(index, None)? {
            Either::Right(byte_range) => byte_range,
            Either::Left(instructions) => {
                let infos = self.storage.read_infos(&instructions).await?;
                match self.tree.byte_range(index, Some(&infos))? {
                    Either::Right(byte_range) => byte_range,
                    Either::Left(_) => {
                        return Err(anyhow!("Could not read byte range"));
                    }
                }
            }
        };

        // TODO: Generalize Either response stack
        let data = match self.block_store.read(&byte_range, None) {
            Either::Right(data) => data,
            Either::Left(instruction) => {
                let info = self.storage.read_info(instruction).await?;
                match self.block_store.read(&byte_range, Some(info)) {
                    Either::Right(data) => data,
                    Either::Left(_) => {
                        return Err(anyhow!("Could not read block storage range"));
                    }
                }
            }
        };

        Ok(Some(data.to_vec()))
    }

    fn should_flush_bitfield_and_tree_and_oplog(&mut self) -> bool {
        if self.skip_flush_count == 0
            || self.oplog.entries_byte_length >= MAX_OPLOG_ENTRIES_BYTE_SIZE
        {
            self.skip_flush_count = 3;
            true
        } else {
            self.skip_flush_count -= 1;
            false
        }
    }

    async fn flush_bitfield_and_tree_and_oplog(&mut self) -> Result<()> {
        let infos = self.bitfield.flush();
        self.storage.flush_infos(&infos).await?;
        let infos = self.tree.flush();
        self.storage.flush_infos(&infos).await?;
        let infos_to_flush = self.oplog.flush(&self.header);
        self.storage.flush_infos(&infos_to_flush).await?;
        Ok(())
    }
}

fn update_contiguous_length(
    header: &mut Header,
    bitfield: &Bitfield,
    bitfield_drop: bool,
    bitfield_start: u64,
    bitfield_length: u64,
) {
    let end = bitfield_start + bitfield_length;
    let mut c = header.contiguous_length;
    if bitfield_drop {
        if c <= end && c > bitfield_start {
            c = bitfield_start;
        }
    } else {
        if c <= end && c >= bitfield_start {
            c = end;
            while bitfield.get(c) {
                c += 1;
            }
        }
    }

    if c != header.contiguous_length {
        header.contiguous_length = c;
    }
}
