use hypercore::PartialKeypair;

use anyhow::Error;
use ed25519_dalek::{PublicKey, SecretKey, PUBLIC_KEY_LENGTH, SECRET_KEY_LENGTH};
use futures::future::FutureExt;
#[cfg(not(feature = "v10"))]
use hypercore::{Feed, Storage, Store};
use random_access_memory as ram;
use sha2::{Digest, Sha256};

const TEST_PUBLIC_KEY_BYTES: [u8; PUBLIC_KEY_LENGTH] = [
    0x97, 0x60, 0x6c, 0xaa, 0xd2, 0xb0, 0x8c, 0x1d, 0x5f, 0xe1, 0x64, 0x2e, 0xee, 0xa5, 0x62, 0xcb,
    0x91, 0xd6, 0x55, 0xe2, 0x00, 0xc8, 0xd4, 0x3a, 0x32, 0x09, 0x1d, 0x06, 0x4a, 0x33, 0x1e, 0xe3,
];
// NB: In the javascript version this is 64 bytes, but that's because sodium appends the the public
// key after the secret key for some reason. Only the first 32 bytes are actually used in
// javascript side too for signing.
const TEST_SECRET_KEY_BYTES: [u8; SECRET_KEY_LENGTH] = [
    0x27, 0xe6, 0x74, 0x25, 0xc1, 0xff, 0xd1, 0xd9, 0xee, 0x62, 0x5c, 0x96, 0x2b, 0x57, 0x13, 0xc3,
    0x51, 0x0b, 0x71, 0x14, 0x15, 0xf3, 0x31, 0xf6, 0xfa, 0x9e, 0xf2, 0xbf, 0x23, 0x5f, 0x2f, 0xfe,
];

#[cfg(not(feature = "v10"))]
pub async fn create_feed(page_size: usize) -> Result<Feed<ram::RandomAccessMemory>, Error> {
    let create = |_store: Store| async move { Ok(ram::RandomAccessMemory::new(page_size)) }.boxed();
    let storage = Storage::new(create, false).await?;
    Feed::with_storage(storage).await
}

#[derive(PartialEq, Debug)]
pub struct HypercoreHash {
    pub bitfield: Option<String>,
    pub data: Option<String>,
    pub oplog: Option<String>,
    pub tree: Option<String>,
}

pub fn get_test_key_pair() -> PartialKeypair {
    let public = PublicKey::from_bytes(&TEST_PUBLIC_KEY_BYTES).unwrap();
    let secret = Some(SecretKey::from_bytes(&TEST_SECRET_KEY_BYTES).unwrap());
    PartialKeypair { public, secret }
}

pub fn create_hypercore_hash(dir: &str) -> HypercoreHash {
    let bitfield = hash_file(format!("{}/bitfield", dir));
    let data = hash_file(format!("{}/data", dir));
    let oplog = hash_file(format!("{}/oplog", dir));
    let tree = hash_file(format!("{}/tree", dir));
    HypercoreHash {
        bitfield,
        data,
        oplog,
        tree,
    }
}

pub fn hash_file(file: String) -> Option<String> {
    let path = std::path::Path::new(&file);
    if !path.exists() {
        None
    } else {
        let mut hasher = Sha256::new();
        let mut file = std::fs::File::open(path).unwrap();
        std::io::copy(&mut file, &mut hasher).unwrap();
        let hash_bytes = hasher.finalize();
        let hash = format!("{:X}", hash_bytes);
        // Empty file has this hash, don't make a difference between missing and empty file. Rust
        // is much easier and performant to write if the empty file is created.
        if hash == "E3B0C44298FC1C149AFBF4C8996FB92427AE41E4649B934CA495991B7852B855".to_string() {
            None
        } else {
            Some(format!("{:X}", hash_bytes))
        }
    }
}
