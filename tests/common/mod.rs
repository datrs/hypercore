use hypercore;

use anyhow::Error;
use futures::future::FutureExt;
#[cfg(not(feature = "v10"))]
use hypercore::{Feed, Storage, Store};
use random_access_memory as ram;
use sha2::{Digest, Sha256};

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
