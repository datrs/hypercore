//! Hypercore's main abstraction. Exposes an append-only, secure log structure.

pub use crate::storage_v10::{PartialKeypair, Storage, Store};

use crate::{crypto::generate_keypair, data::BlockStore, oplog::Oplog, sign, tree::MerkleTree};
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
    //     /// Bitfield to keep track of which data we own.
    //     pub(crate) bitfield: Bitfield,
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
        let oplog_bytes = storage.read_all(Store::Oplog).await?;
        let oplog_open_outcome = Oplog::open(key_pair.clone(), oplog_bytes)?;
        storage
            .flush_slices(Store::Oplog, oplog_open_outcome.slices_to_flush)
            .await?;

        // Open/create tree
        let slice_instructions =
            MerkleTree::get_slice_instructions_to_read(&oplog_open_outcome.header.tree);
        let slices = storage.read_slices(Store::Tree, slice_instructions).await?;
        let tree = MerkleTree::open(&oplog_open_outcome.header.tree, slices)?;

        // Create block store instance
        let block_store = BlockStore::default();

        Ok(Hypercore {
            key_pair,
            storage,
            oplog: oplog_open_outcome.oplog,
            tree,
            block_store,
        })
    }

    /// Appends a given batch of data blobs to the hypercore.
    pub async fn append_batch(&mut self, batch: Vec<&[u8]>) -> Result<AppendOutcome> {
        let secret_key = match &self.key_pair.secret {
            Some(key) => key,
            None => anyhow::bail!("No secret key, cannot append."),
        };

        let mut changeset = self.tree.changeset();
        let mut batch_length: usize = 0;
        for data in &batch {
            batch_length += changeset.append(data);
        }
        let hash = changeset.hash_and_sign(&self.key_pair.public, &secret_key);
        let slice = self
            .block_store
            .append_batch(batch, batch_length, self.tree.byte_length);
        self.storage.flush_slice(Store::Data, slice).await?;

        Ok(AppendOutcome {
            length: 0,
            byte_length: 0,
        })
    }
}
