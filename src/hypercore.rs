//! Hypercore's main abstraction. Exposes an append-only, secure log structure.

pub use crate::storage::{PartialKeypair, Storage};

use crate::{
    crypto::{generate_keypair, PublicKey, SecretKey},
    oplog::Oplog,
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
    //     /// Merkle tree instance.
    //     pub(crate) tree: Merkle,
    //     /// Bitfield to keep track of which data we own.
    //     pub(crate) bitfield: Bitfield,
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
        let oplog_bytes = storage.read_oplog().await?;
        let oplog = Oplog::open(key_pair.clone(), oplog_bytes);

        Ok(Hypercore {
            key_pair,
            storage,
            oplog,
        })
    }
}
