//! Based on https://github.com/mafintosh/hypercore/blob/cf08d8c907e302cf4b699738f229b050eba41b59/test/compat.js

use ed25519_dalek;

use tempfile;

use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use data_encoding::HEXLOWER;
use ed25519_dalek::Keypair;
use hypercore::Feed;
use hypercore::{Storage, Store};
use random_access_disk::RandomAccessDisk;
use remove_dir_all::remove_dir_all;

#[async_std::test]
async fn deterministic_data_and_tree() {
    let expected_tree = hex_bytes(concat!(
        "0502570200002807424c414b4532620000000000000000000000000000000000ab27d45f509274",
        "ce0d08f4f09ba2d0e0d8df61a0c2a78932e81b5ef26ef398df0000000000000001064321a8413b",
        "e8c604599689e2c7a59367b031b598bceeeb16556a8f3252e0de000000000000000294c1705400",
        "5942a002c7c39fbb9c6183518691fb401436f1a2f329b380230af800000000000000018dfe81d5",
        "76464773f848b9aba1c886fde57a49c283ab57f4a297d976d986651e00000000000000041d2fad",
        "c9ce604c7e592949edc964e45aaa10990d7ee53328439ef9b2cf8aa6ff00000000000000013a8d",
        "cc74e80b8314e8e13e1e462358cf58cf5fc4413a9b18a891ffacc551c395000000000000000228",
        "28647a654a712738e35f49d1c05c676010be0b33882affc1d1e7e9fee59d400000000000000001",
        "000000000000000000000000000000000000000000000000000000000000000000000000000000",
        "00baac70b6d38243efa028ee977c462e4bec73d21d09ceb8cc16f4d4b1ee228a45000000000000",
        "0001d1b021632c7fab84544053379112ca7b165bb21283821816c5b6c89ff7f78e2d0000000000",
        "000002d2ab421cece792033058787a5ba72f3a701fddc25540d5924e9819d7c12e02f200000000",
        "00000001"
    ));

    for _ in 0..5 {
        let (dir, storage) = mk_storage().await;
        let mut feed = Feed::with_storage(storage).await.unwrap();

        let data = b"abcdef";
        for &b in data {
            feed.append(&[b]).await.unwrap();
        }
        assert_eq!(read_bytes(&dir, Store::Data), data);
        assert_eq!(read_bytes(&dir, Store::Tree), expected_tree);

        remove_dir_all(dir).unwrap()
    }
}

#[test]
#[ignore]
fn deterministic_data_and_tree_after_replication() {
    // Port from mafintosh/hypercore when the necessary features are implemented
    unimplemented!();
}

#[async_std::test]
#[ignore]
async fn deterministic_signatures() {
    let key = hex_bytes("9718a1ff1c4ca79feac551c0c7212a65e4091278ec886b88be01ee4039682238");
    let keypair_bytes = hex_bytes(concat!(
        "53729c0311846cca9cc0eded07aaf9e6689705b6a0b1bb8c3a2a839b72fda383",
        "9718a1ff1c4ca79feac551c0c7212a65e4091278ec886b88be01ee4039682238"
    ));

    let expected_signatures = hex_bytes(concat!(
        "050257010000400745643235353139000000000000000000000000000000000084684e8dd76c339",
        "d6f5754e813204906ee818e6c6cdc6a816a2ac785a3e0d926ac08641a904013194fe6121847b7da",
        "d4e361965d47715428eb0a0ededbdd5909d037ff3c3614fa0100ed9264a712d3b77cbe7a4f6eadd",
        "8f342809be99dfb9154a19e278d7a5de7d2b4d890f7701a38b006469f6bab1aff66ac6125d48baf",
        "dc0711057675ed57d445ce7ed4613881be37ebc56bb40556b822e431bb4dc3517421f9a5e3ed124",
        "eb5c4db8367386d9ce12b2408613b9fec2837022772a635ffd807",
    ));

    for _ in 0..5 {
        let (dir, storage) = mk_storage().await;
        let keypair = mk_keypair(&keypair_bytes, &key);
        let mut feed = Feed::builder(keypair.public, storage)
            .secret_key(keypair.secret)
            .build()
            .unwrap();

        let data = b"abc";
        for &b in data {
            feed.append(&[b]).await.unwrap();
        }

        assert_eq!(read_bytes(&dir, Store::Data), data);
        assert_eq!(read_bytes(&dir, Store::Signatures), expected_signatures);

        remove_dir_all(dir).unwrap()
    }
}

#[test]
#[ignore]
fn deterministic_signatures_after_replication() {
    // Port from mafintosh/hypercore when the necessary features are implemented
    unimplemented!();
}

fn hex_bytes(hex: &str) -> Vec<u8> {
    HEXLOWER.decode(hex.as_bytes()).unwrap()
}

fn storage_path<P: AsRef<Path>>(dir: P, s: Store) -> PathBuf {
    let filename = match s {
        Store::Tree => "tree",
        Store::Data => "data",
        Store::Bitfield => "bitfield",
        Store::Signatures => "signatures",
        Store::Keypair => "key",
    };
    dir.as_ref().join(filename)
}

async fn mk_storage() -> (PathBuf, Storage<RandomAccessDisk>) {
    let temp_dir = tempfile::tempdir().unwrap();
    let dir = temp_dir.into_path();
    let storage = Storage::new(|s| {
        let dir = dir.clone();
        Box::pin(async move { RandomAccessDisk::open(storage_path(dir, s)).await })
    })
    .await
    .unwrap();
    (dir, storage)
}

fn read_bytes<P: AsRef<Path>>(dir: P, s: Store) -> Vec<u8> {
    let mut f = File::open(storage_path(dir, s)).unwrap();
    let mut bytes = Vec::new();
    f.read_to_end(&mut bytes).unwrap();
    bytes
}

fn mk_keypair(keypair_bytes: &[u8], public_key: &[u8]) -> Keypair {
    let keypair = Keypair::from_bytes(&keypair_bytes).unwrap();
    assert_eq!(
        keypair.secret.as_bytes().as_ref(),
        &keypair_bytes[..ed25519_dalek::SECRET_KEY_LENGTH]
    );
    assert_eq!(keypair.public.as_bytes().as_ref(), public_key);
    keypair
}
