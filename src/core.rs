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
use anyhow::Result;
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

        let oplog_open_outcome = Oplog::open(key_pair.clone(), oplog_bytes)?;
        storage
            .flush_infos(&oplog_open_outcome.infos_to_flush)
            .await?;

        // Open/create tree
        let info_instructions =
            MerkleTree::get_info_instructions_to_read(&oplog_open_outcome.header.tree);
        let infos = storage.read_infos(&info_instructions).await?;
        let tree = MerkleTree::open(&oplog_open_outcome.header.tree, infos)?;

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
        let bitfield = Bitfield::open(info);

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

    /// Appends a given batch of data blobs to the hypercore.
    pub async fn append_batch(&mut self, batch: &[&[u8]]) -> Result<AppendOutcome> {
        let secret_key = match &self.key_pair.secret {
            Some(key) => key,
            None => anyhow::bail!("No secret key, cannot append."),
        };

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
        // TODO
        //     this.bitfield.setRange(batch.ancestors, batch.length - batch.ancestors, true)

        // TODO:  contiguous length
        if self.should_flush_bitfield_and_tree_and_oplog() {
            self.flush_bitfield_and_tree_and_oplog().await?;
        }

        Ok(AppendOutcome {
            length: 0,
            byte_length: 0,
        })
    }

    fn should_flush_bitfield_and_tree_and_oplog(&mut self) -> bool {
        if self.skip_flush_count == 0
            || self.oplog.entries_byte_length >= MAX_OPLOG_ENTRIES_BYTE_SIZE
        {
            self.skip_flush_count = 4;
            true
        } else {
            self.skip_flush_count -= 1;
            false
        }
    }

    async fn flush_bitfield_and_tree_and_oplog(&mut self) -> Result<()> {
        // TODO:
        // let infos = self.bitfield.flush();
        // self.storage.flush_infos(Store::Bitfield, &infos).await?;
        // let infos = self.tree.flush();
        // self.storage.flush_infos(Store::Tree, &infos).await?;
        let infos_to_flush = self.oplog.flush(&self.header);
        self.storage.flush_infos(&infos_to_flush).await?;
        Ok(())
    }
}
