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

    /// Read the full oplog bytes.
    pub async fn read_oplog(&mut self) -> Result<Box<[u8]>> {
        let len = self.oplog.len().await.map_err(|e| anyhow!(e))?;
        let buf = self.oplog.read(0, len).await.map_err(|e| anyhow!(e))?;
        Ok(buf.into_boxed_slice())
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
        let storage = |storage: Store| {
            let name = match storage {
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
