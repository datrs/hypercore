use hypercore;

use anyhow::{Error, Result};
use futures::future::FutureExt;
#[cfg(not(feature = "v10"))]
use hypercore::Feed;
use hypercore::{Storage, Store};
use random_access_disk::RandomAccessDisk;
use random_access_memory as ram;
use sha2::{Digest, Sha256};
use std::path::PathBuf;

#[cfg(not(feature = "v10"))]
pub async fn create_feed(page_size: usize) -> Result<Feed<ram::RandomAccessMemory>, Error> {
    let create = |_store: Store| async move { Ok(ram::RandomAccessMemory::new(page_size)) }.boxed();
    let storage = Storage::new(create, false).await?;
    Feed::with_storage(storage).await
}

#[derive(PartialEq, Debug)]
pub struct HypercoreHash {
    pub bitfield: String,
    pub data: String,
    pub oplog: String,
    pub tree: String,
}

pub fn create_hypercore_hash(dir: String) -> Result<HypercoreHash, Error> {
    let bitfield = hash_file(format!("{}/bitfield", dir))?;
    let data = hash_file(format!("{}/data", dir))?;
    let oplog = hash_file(format!("{}/oplog", dir))?;
    let tree = hash_file(format!("{}/tree", dir))?;
    Ok(HypercoreHash {
        bitfield,
        data,
        oplog,
        tree,
    })
}

pub fn hash_file(file: String) -> Result<String, Error> {
    let mut hasher = Sha256::new();
    let mut file = std::fs::File::open(file)?;

    std::io::copy(&mut file, &mut hasher)?;
    let hash_bytes = hasher.finalize();
    Ok(format!("{:X}", hash_bytes))
}

pub async fn create_disk_storage(
    dir: &PathBuf,
    overwrite: bool,
) -> Result<Storage<RandomAccessDisk>> {
    let storage = |storage: Store| {
        let name = match storage {
            Store::Tree => "tree",
            Store::Data => "data",
            Store::Bitfield => "bitfield",
            #[cfg(not(feature = "v10"))]
            Store::Signatures => "signatures",
            #[cfg(not(feature = "v10"))]
            Store::Keypair => "key",
            #[cfg(feature = "v10")]
            Store::Oplog => "oplog",
        };
        RandomAccessDisk::open(dir.as_path().join(name)).boxed()
    };
    Ok(Storage::new(storage, overwrite).await?)
}
