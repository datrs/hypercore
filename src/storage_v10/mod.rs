//! Save data to a desired storage backend.

use anyhow::{anyhow, Result};
use ed25519_dalek::{PublicKey, SecretKey};
use futures::future::FutureExt;
#[cfg(not(target_arch = "wasm32"))]
use random_access_disk::RandomAccessDisk;
use random_access_memory::RandomAccessMemory;
use random_access_storage::RandomAccess;
use std::fmt::Debug;
use std::path::PathBuf;

use crate::common::BufferSlice;

/// Key pair where for read-only hypercores the secret key can also be missing.
#[derive(Debug)]
pub struct PartialKeypair {
    /// Public key
    pub public: PublicKey,
    /// Secret key. If None, the hypercore is read-only.
    pub secret: Option<SecretKey>,
}

impl Clone for PartialKeypair {
    fn clone(&self) -> Self {
        let secret: Option<SecretKey> = match &self.secret {
            Some(secret) => {
                let bytes = secret.to_bytes();
                Some(SecretKey::from_bytes(&bytes).unwrap())
            }
            None => None,
        };
        PartialKeypair {
            public: self.public.clone(),
            secret,
        }
    }
}

/// The types of stores that can be created.
#[derive(Debug)]
pub enum Store {
    /// Tree
    Tree,
    /// Data
    Data,
    /// Bitfield
    Bitfield,
    /// Oplog
    Oplog,
}

/// Save data to a desired storage backend.
#[derive(Debug)]
pub struct Storage<T>
where
    T: RandomAccess + Debug,
{
    tree: T,
    data: T,
    bitfield: T,
    oplog: T,
}

impl<T> Storage<T>
where
    T: RandomAccess<Error = Box<dyn std::error::Error + Send + Sync>> + Debug + Send,
{
    /// Create a new instance. Takes a callback to create new storage instances and overwrite flag.
    pub async fn open<Cb>(create: Cb, overwrite: bool) -> Result<Self>
    where
        Cb: Fn(Store) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T>> + Send>>,
    {
        if overwrite {
            unimplemented!("Clearing storage not implemented");
        }
        let tree = create(Store::Tree).await?;
        let data = create(Store::Data).await?;
        let bitfield = create(Store::Bitfield).await?;
        let oplog = create(Store::Oplog).await?;

        let instance = Self {
            tree,
            data,
            bitfield,
            oplog,
        };

        Ok(instance)
    }

    /// Read fully a store.
    pub async fn read_all(&mut self, store: Store) -> Result<Box<[u8]>> {
        let storage = self.get_random_access(store);
        let len = storage.len().await.map_err(|e| anyhow!(e))?;
        let buf = storage.read(0, len).await.map_err(|e| anyhow!(e))?;
        Ok(buf.into_boxed_slice())
    }

    /// Flushes slices. Either writes directly to a random access storage or truncates the storage
    /// to the length of given index.
    pub async fn flush_slices(&mut self, store: Store, slices: Vec<BufferSlice>) -> Result<()> {
        let storage = self.get_random_access(store);
        for slice in slices {
            match slice.data {
                Some(data) => {
                    storage
                        .write(slice.index, &data.to_vec())
                        .await
                        .map_err(|e| anyhow!(e))?;
                }
                None => {
                    storage
                        .truncate(slice.index)
                        .await
                        .map_err(|e| anyhow!(e))?;
                }
            }
        }
        Ok(())
    }

    fn get_random_access(&mut self, store: Store) -> &mut T {
        match store {
            Store::Tree => &mut self.tree,
            Store::Data => &mut self.data,
            Store::Bitfield => &mut self.bitfield,
            Store::Oplog => &mut self.oplog,
        }
    }
}

impl Storage<RandomAccessMemory> {
    /// New storage backed by a `RandomAccessMemory` instance.
    pub async fn new_memory() -> Result<Self> {
        let create = |_| async { Ok(RandomAccessMemory::default()) }.boxed();
        // No reason to overwrite, as this is a new memory segment
        Ok(Self::open(create, false).await?)
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Storage<RandomAccessDisk> {
    /// New storage backed by a `RandomAccessDisk` instance.
    pub async fn new_disk(dir: &PathBuf, overwrite: bool) -> Result<Self> {
        let storage = |store: Store| {
            let name = match store {
                Store::Tree => "tree",
                Store::Data => "data",
                Store::Bitfield => "bitfield",
                Store::Oplog => "oplog",
            };
            RandomAccessDisk::open(dir.as_path().join(name)).boxed()
        };
        Ok(Self::open(storage, overwrite).await?)
    }
}
