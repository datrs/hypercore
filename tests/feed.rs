extern crate failure;
extern crate hypercore;
extern crate random_access_memory as ram;
extern crate random_access_storage;

mod common;

use self::failure::Error;
use self::random_access_storage::RandomAccess;
use common::create_feed;
use hypercore::{generate_keypair, Feed, NodeTrait, Proof, PublicKey, SecretKey, Storage};
use std::env::temp_dir;
use std::fmt::Debug;
use std::fs;
use std::io::Write;

#[test]
fn create_with_key() {
    let keypair = generate_keypair();
    let storage = Storage::new_memory().unwrap();
    let _feed = Feed::builder(keypair.public, storage)
        .secret_key(keypair.secret)
        .build()
        .unwrap();
}

#[test]
fn display() {
    let feed = create_feed(50).unwrap();
    let output = format!("{}", feed);
    assert_eq!(output.len(), 61);
}

#[test]
/// Verify `.append()` and `.get()` work.
fn set_get() {
    let mut feed = create_feed(50).unwrap();
    feed.append(b"hello").unwrap();
    feed.append(b"world").unwrap();

    assert_eq!(feed.get(0).unwrap(), Some(b"hello".to_vec()));
    assert_eq!(feed.get(1).unwrap(), Some(b"world".to_vec()));
}

#[test]
fn append() {
    let mut feed = create_feed(50).unwrap();
    feed.append(br#"{"hello":"world"}"#).unwrap();
    feed.append(br#"{"hello":"mundo"}"#).unwrap();
    feed.append(br#"{"hello":"welt"}"#).unwrap();

    assert_eq!(feed.len(), 3);
    assert_eq!(feed.byte_len(), 50);

    assert_eq!(feed.get(0).unwrap(), Some(br#"{"hello":"world"}"#.to_vec()));
    assert_eq!(feed.get(1).unwrap(), Some(br#"{"hello":"mundo"}"#.to_vec()));
    assert_eq!(feed.get(2).unwrap(), Some(br#"{"hello":"welt"}"#.to_vec()));
}

#[test]
/// Verify the `.root_hashes()` method returns the right nodes.
fn root_hashes() {
    // If no roots exist we should get an error.
    let mut feed = create_feed(50).unwrap();
    let res = feed.root_hashes(0);
    assert!(res.is_err());

    // If 1 entry exists, [0] should be the root.
    feed.append(b"data").unwrap();
    let roots = feed.root_hashes(0).unwrap();
    assert_eq!(roots.len(), 1);
    assert_eq!(roots[0].index(), 0);

    // If we query out of bounds, we should get an error.
    let res = feed.root_hashes(6);
    assert!(res.is_err());

    // If 3 entries exist, [2,4] should be the roots.
    feed.append(b"data").unwrap();
    feed.append(b"data").unwrap();
    let roots = feed.root_hashes(2).unwrap();
    assert_eq!(roots.len(), 2);
    assert_eq!(roots[0].index(), 1);
    assert_eq!(roots[1].index(), 4);
}

#[test]
fn verify() {
    let mut feed = create_feed(50).unwrap();
    let (public, secret) = copy_keys(&feed);
    let feed_bytes = secret.to_bytes().to_vec();
    let storage = Storage::new(|_| Ok(ram::RandomAccessMemory::new(50))).unwrap();
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
    feed.append(b"test").unwrap();
    let sig = feed.signature(0).unwrap();
    feed.verify(0, &sig).unwrap();

    // Verify that the signature between two different feeds is different.
    evil_feed.append(b"t0st").unwrap();
    let res = evil_feed.verify(0, &sig);
    assert!(res.is_err());
}

#[test]
fn put() {
    let mut a = create_feed(50).unwrap();
    let (public, secret) = copy_keys(&a);
    let storage = Storage::new(|_| Ok(ram::RandomAccessMemory::new(50))).unwrap();
    let mut b = Feed::builder(public, storage)
        .secret_key(secret)
        .build()
        .unwrap();

    for _ in 0..10 {
        a.append(b"foo").unwrap();
    }

    let proof = a.proof(0, true).unwrap();
    b.put(0, None, proof).expect("no error");
    let proof = a
        .proof_with_digest(4, b.digest(4), true)
        .expect(".proof() index 4, digest 4");
    b.put(4, None, proof).unwrap();
}

#[test]
/// Put data from one feed into another, while veryfing hashes.
/// I.e. manual replication between two feeds.
fn put_with_data() {
    // Create a writable feed.
    let mut a = create_feed(50).unwrap();

    // Create a second feed with the first feed's key.
    let (public, secret) = copy_keys(&a);
    let storage = Storage::new_memory().unwrap();
    let mut b = Feed::builder(public, storage)
        .secret_key(secret)
        .build()
        .unwrap();

    // Append 4 blocks of data to the writable feed.
    a.append(b"hi").unwrap();
    a.append(b"ola").unwrap();
    a.append(b"ahoj").unwrap();
    a.append(b"salut").unwrap();

    for i in 0..4 {
        // Generate a proof for the index.
        // The `include_hash` argument has to be set to false.
        let a_proof = a.proof(i, false).unwrap();
        // Get the data for the index.
        let a_data = a.get(i).unwrap();

        // Put the data into the other hypercore.
        b.put(i, a_data.as_deref(), a_proof.clone()).unwrap();

        // Load the data we've put.
        let b_data = b.get(i).unwrap();

        // Debug output.
        // eprintln!("A: idx {} data {:?}", i, &a_data);
        // eprintln!("Proof: {:#?}", fmt_proof(&a_proof));
        // eprintln!("B: idx {} {:?}", i, &b_data);

        // Assert the data was put correctly.
        assert!(a_data == b_data, "Data correct");
    }
}

/// Helper function to format proofs in a readable debug format.
#[allow(dead_code)]
fn fmt_proof(proof: &Proof) -> Vec<String> {
    proof
        .nodes
        .iter()
        .map(|n| {
            format!(
                "idx {} len {} parent {} hash {:?}..",
                n.index(),
                n.len(),
                n.parent(),
                &n.hash()[0..5]
            )
        })
        .collect::<Vec<String>>()
}

#[test]
fn create_with_storage() {
    let storage = Storage::new_memory().unwrap();
    assert!(
        Feed::with_storage(storage).is_ok(),
        "Could not create a feed with a storage."
    );
}

#[test]
fn create_with_stored_public_key() {
    let mut storage = Storage::new_memory().unwrap();
    let keypair = generate_keypair();
    storage.write_public_key(&keypair.public).unwrap();
    assert!(
        Feed::with_storage(storage).is_ok(),
        "Could not create a feed with a stored public key."
    );
}

#[test]
fn create_with_stored_keys() {
    let mut storage = Storage::new_memory().unwrap();
    let keypair = generate_keypair();
    storage.write_public_key(&keypair.public).unwrap();
    storage.write_secret_key(&keypair.secret).unwrap();
    assert!(
        Feed::with_storage(storage).is_ok(),
        "Could not create a feed with a stored keypair."
    );
}

fn copy_keys(feed: &Feed<impl RandomAccess<Error = Error> + Debug>) -> (PublicKey, SecretKey) {
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

#[test]
fn audit() {
    let mut feed = create_feed(50).unwrap();
    feed.append(b"hello").unwrap();
    feed.append(b"world").unwrap();
    match feed.audit() {
        Ok(audit_report) => {
            assert_eq!(audit_report.valid_blocks, 2);
            assert_eq!(audit_report.invalid_blocks, 0);
        }
        Err(e) => {
            panic!(e);
        }
    }
}

#[test]
fn audit_bad_data() {
    let mut dir = temp_dir();
    dir.push("audit_bad_data");
    let storage = Storage::new_disk(&dir).unwrap();
    let mut feed = Feed::with_storage(storage).unwrap();
    feed.append(b"hello").unwrap();
    feed.append(b"world").unwrap();
    let datapath = dir.join("data");
    let mut hypercore_data = fs::OpenOptions::new()
        .write(true)
        .open(datapath)
        .expect("Unable to open the hypercore's data file!");
    hypercore_data
        .write_all(b"yello")
        .expect("Unable to corrupt the hypercore data file!");

    match feed.audit() {
        Ok(audit_report) => {
            assert_eq!(audit_report.valid_blocks, 1);
            assert_eq!(audit_report.invalid_blocks, 1);
            // Ensure that audit has cleared up the invalid block
            match feed.audit() {
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
