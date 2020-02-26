extern crate random_access_memory as ram;

mod common;

use common::create_feed;
use hypercore::{generate_keypair, Feed, NodeTrait, PublicKey, SecretKey, Storage};
use random_access_storage::RandomAccess;
use std::env::temp_dir;
use std::fmt::Debug;
use std::fs;
use std::io::Write;

#[async_std::test]
async fn create_with_key() {
    let keypair = generate_keypair();
    let storage = Storage::new_memory().await.unwrap();
    let _feed = Feed::builder(keypair.public, storage)
        .secret_key(keypair.secret)
        .build()
        .unwrap();
}

#[async_std::test]
async fn display() {
    let feed = create_feed(50).await.unwrap();
    let output = format!("{}", feed);
    assert_eq!(output.len(), 61);
}

#[async_std::test]
/// Verify `.append()` and `.get()` work.
async fn set_get() {
    let mut feed = create_feed(50).await.unwrap();
    feed.append(b"hello").await.unwrap();
    feed.append(b"world").await.unwrap();

    assert_eq!(feed.get(0).await.unwrap(), Some(b"hello".to_vec()));
    assert_eq!(feed.get(1).await.unwrap(), Some(b"world".to_vec()));
}

#[async_std::test]
async fn append() {
    let mut feed = create_feed(50).await.unwrap();
    feed.append(br#"{"hello":"world"}"#).await.unwrap();
    feed.append(br#"{"hello":"mundo"}"#).await.unwrap();
    feed.append(br#"{"hello":"welt"}"#).await.unwrap();

    assert_eq!(feed.len(), 3);
    assert_eq!(feed.byte_len(), 50);

    assert_eq!(
        feed.get(0).await.unwrap(),
        Some(br#"{"hello":"world"}"#.to_vec())
    );
    assert_eq!(
        feed.get(1).await.unwrap(),
        Some(br#"{"hello":"mundo"}"#.to_vec())
    );
    assert_eq!(
        feed.get(2).await.unwrap(),
        Some(br#"{"hello":"welt"}"#.to_vec())
    );
}

#[async_std::test]
/// Verify the `.root_hashes()` method returns the right nodes.
async fn root_hashes() {
    // If no roots exist we should get an error.
    let mut feed = create_feed(50).await.unwrap();
    let res = feed.root_hashes(0).await;
    assert!(res.is_err());

    // If 1 entry exists, [0] should be the root.
    feed.append(b"data").await.unwrap();
    let roots = feed.root_hashes(0).await.unwrap();
    assert_eq!(roots.len(), 1);
    assert_eq!(roots[0].index(), 0);

    // If we query out of bounds, we should get an error.
    let res = feed.root_hashes(6).await;
    assert!(res.is_err());

    // If 3 entries exist, [2,4] should be the roots.
    feed.append(b"data").await.unwrap();
    feed.append(b"data").await.unwrap();
    let roots = feed.root_hashes(2).await.unwrap();
    assert_eq!(roots.len(), 2);
    assert_eq!(roots[0].index(), 1);
    assert_eq!(roots[1].index(), 4);
}

#[async_std::test]
async fn verify() {
    let mut feed = create_feed(50).await.unwrap();
    let (public, secret) = copy_keys(&feed);
    let feed_bytes = secret.to_bytes().to_vec();
    let storage = Storage::new(|_| Box::pin(async { Ok(ram::RandomAccessMemory::new(50)) }))
        .await
        .unwrap();
    let mut evil_feed = Feed::builder(public, storage)
        .secret_key(secret)
        .build()
        .unwrap();

    let evil_bytes = match &feed.secret_key() {
        Some(key) => key.to_bytes(),
        None => panic!("no secret key found"),
    };

    // Verify the keys are the same.
    assert_eq!(&feed_bytes, &evil_bytes.to_vec());

    // Verify that the signature on a single feed is correct.
    feed.append(b"test").await.unwrap();
    let sig = feed.signature(0).await.unwrap();
    feed.verify(0, &sig).await.unwrap();

    // Verify that the signature between two different feeds is different.
    evil_feed.append(b"t0st").await.unwrap();
    let res = evil_feed.verify(0, &sig).await;
    assert!(res.is_err());
}

#[async_std::test]
async fn put() {
    let mut a = create_feed(50).await.unwrap();
    let (public, secret) = copy_keys(&a);
    let storage = Storage::new(|_| Box::pin(async { Ok(ram::RandomAccessMemory::new(50)) }))
        .await
        .unwrap();
    let mut b = Feed::builder(public, storage)
        .secret_key(secret)
        .build()
        .unwrap();

    for _ in 0..10 {
        a.append(b"foo").await.unwrap();
    }

    let proof = a.proof(0, true).await.unwrap();
    b.put(0, None, proof).await.expect("no error");
    let proof = a
        .proof_with_digest(4, b.digest(4), true)
        .await
        .expect(".proof() index 4, digest 4");
    b.put(4, None, proof).await.unwrap();
}

#[async_std::test]
async fn create_with_storage() {
    let storage = Storage::new_memory().await.unwrap();
    assert!(
        Feed::with_storage(storage).await.is_ok(),
        "Could not create a feed with a storage."
    );
}

#[async_std::test]
async fn create_with_stored_public_key() {
    let mut storage = Storage::new_memory().await.unwrap();
    let keypair = generate_keypair();
    storage.write_public_key(&keypair.public).await.unwrap();
    assert!(
        Feed::with_storage(storage).await.is_ok(),
        "Could not create a feed with a stored public key."
    );
}

#[async_std::test]
async fn create_with_stored_keys() {
    let mut storage = Storage::new_memory().await.unwrap();
    let keypair = generate_keypair();
    storage.write_public_key(&keypair.public).await.unwrap();
    storage.write_secret_key(&keypair.secret).await.unwrap();
    assert!(
        Feed::with_storage(storage).await.is_ok(),
        "Could not create a feed with a stored keypair."
    );
}

fn copy_keys(
    feed: &Feed<impl RandomAccess<Error = Box<dyn std::error::Error + Send + Sync>> + Debug + Send>,
) -> (PublicKey, SecretKey) {
    match &feed.secret_key() {
        Some(secret) => {
            let secret = secret.to_bytes();
            let public = &feed.public_key().to_bytes();

            let public = PublicKey::from_bytes(public).unwrap();
            let secret = SecretKey::from_bytes(&secret).unwrap();

            (public, secret)
        }
        _ => panic!("<tests/common>: Could not access secret key"),
    }
}

#[async_std::test]
async fn audit() {
    let mut feed = create_feed(50).await.unwrap();
    feed.append(b"hello").await.unwrap();
    feed.append(b"world").await.unwrap();
    match feed.audit().await {
        Ok(audit_report) => {
            assert_eq!(audit_report.valid_blocks, 2);
            assert_eq!(audit_report.invalid_blocks, 0);
        }
        Err(e) => {
            panic!(e);
        }
    }
}

#[async_std::test]
async fn audit_bad_data() {
    let mut dir = temp_dir();
    dir.push("audit_bad_data");
    let storage = Storage::new_disk(&dir).await.unwrap();
    let mut feed = Feed::with_storage(storage).await.unwrap();
    feed.append(b"hello").await.unwrap();
    feed.append(b"world").await.unwrap();
    let datapath = dir.join("data");
    let mut hypercore_data = fs::OpenOptions::new()
        .write(true)
        .open(datapath)
        .expect("Unable to open the hypercore's data file!");
    hypercore_data
        .write_all(b"yello")
        .expect("Unable to corrupt the hypercore data file!");

    match feed.audit().await {
        Ok(audit_report) => {
            assert_eq!(audit_report.valid_blocks, 1);
            assert_eq!(audit_report.invalid_blocks, 1);
            // Ensure that audit has cleared up the invalid block
            match feed.audit().await {
                Ok(audit_report) => {
                    assert_eq!(
                        audit_report.valid_blocks, 1,
                        "Audit did not clean up the invalid block!"
                    );
                    assert_eq!(
                        audit_report.invalid_blocks, 0,
                        "Audit did not clean up the invalid block!"
                    );
                    fs::remove_dir_all(dir)
                        .expect("Should be able to remove our temporary directory");
                }
                Err(e) => {
                    fs::remove_dir_all(dir)
                        .expect("Should be able to remove our temporary directory");
                    panic!(e);
                }
            }
        }
        Err(e) => {
            fs::remove_dir_all(dir).expect("Should be able to remove our temporary directory");
            panic!(e);
        }
    }
}
