extern crate random_access_memory as ram;

mod common;

use common::create_feed;
use futures::stream::StreamExt;
use hypercore::{generate_keypair, Event, Feed, NodeTrait, PublicKey, SecretKey, Storage};
use hypercore::{storage_disk, storage_memory};
use std::env::temp_dir;
use std::fs;
use std::io::Write;

#[async_std::test]
async fn create_with_key() {
    let keypair = generate_keypair();
    let storage = storage_memory().await.unwrap();
    let _feed = Feed::builder(keypair.public, storage)
        .secret_key(keypair.secret)
        .build()
        .await
        .unwrap();
}

#[async_std::test]
async fn display() {
    let feed = create_feed(50).await.unwrap();
    let output = format!("{}", feed);
    assert_eq!(output.len(), 61);
}

#[async_std::test]
async fn task_send() {
    use async_std::sync::{Arc, Mutex};
    use async_std::task;
    let mut feed = create_feed(50).await.unwrap();
    feed.append(b"hello").await.unwrap();
    let feed_arc = Arc::new(Mutex::new(feed));
    let feed = feed_arc.clone();
    task::spawn(async move {
        feed.lock().await.append(b"world").await.unwrap();
    })
    .await;
    let feed = feed_arc.clone();
    let t1 = task::spawn(async move {
        let value = feed.lock().await.get(0).await.unwrap();
        assert_eq!(value, Some(b"hello".to_vec()));
    });
    let feed = feed_arc.clone();
    let t2 = task::spawn(async move {
        let value = feed.lock().await.get(1).await.unwrap();
        assert_eq!(value, Some(b"world".to_vec()));
    });
    futures::future::join_all(vec![t1, t2]).await;
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
    let storage = Storage::new(
        |_| Box::pin(async { Ok(ram::RandomAccessMemory::new(50)) }),
        false,
    )
    .await
    .unwrap();
    let mut evil_feed = Feed::builder(public, storage)
        .secret_key(secret)
        .build()
        .await
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
    let storage = Storage::new(
        |_| Box::pin(async { Ok(ram::RandomAccessMemory::new(50)) }),
        false,
    )
    .await
    .unwrap();
    let mut b = Feed::builder(public, storage)
        .secret_key(secret)
        .build()
        .await
        .unwrap();

    for _ in 0..10u8 {
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
/// Put data from one feed into another, while veryfing hashes.
/// I.e. manual replication between two feeds.
async fn put_with_data() {
    // Create a writable feed.
    let mut a = create_feed(50).await.unwrap();
    // Create a second feed with the first feed's key.
    let mut b = create_clone(&a).await.unwrap();

    // Append 4 blocks of data to the writable feed.
    a.append(b"hi").await.unwrap();
    a.append(b"ola").await.unwrap();
    a.append(b"ahoj").await.unwrap();
    a.append(b"salut").await.unwrap();

    for i in 0..4 {
        // Generate a proof for the index.
        // The `include_hash` argument has to be set to false.
        let a_proof = a.proof(i, false).await.unwrap();
        // Get the data for the index.
        let a_data = a.get(i).await.unwrap();

        // Put the data into the other hypercore.
        b.put(i, a_data.as_deref(), a_proof.clone()).await.unwrap();

        // Load the data we've put.
        let b_data = b.get(i).await.unwrap();

        // Assert the data was put correctly.
        assert!(a_data == b_data, "Data correct");
    }
}

#[async_std::test]
async fn create_with_storage() {
    let storage = storage_memory().await.unwrap();
    assert!(
        Feed::with_storage(storage).await.is_ok(),
        "Could not create a feed with a storage."
    );
}

#[async_std::test]
async fn create_with_stored_public_key() {
    let mut storage = storage_memory().await.unwrap();
    let keypair = generate_keypair();
    storage.write_public_key(&keypair.public).await.unwrap();
    assert!(
        Feed::with_storage(storage).await.is_ok(),
        "Could not create a feed with a stored public key."
    );
}

#[async_std::test]
async fn create_with_stored_keys() {
    let mut storage = storage_memory().await.unwrap();
    let keypair = generate_keypair();
    storage.write_public_key(&keypair.public).await.unwrap();
    storage.write_secret_key(&keypair.secret).await.unwrap();
    assert!(
        Feed::with_storage(storage).await.is_ok(),
        "Could not create a feed with a stored keypair."
    );
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
    let storage = storage_disk(&dir).await.unwrap();
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

#[async_std::test]
async fn try_open_missing_dir() {
    use rand::distributions::Alphanumeric;
    use rand::{thread_rng, Rng};

    let rand_string: String = thread_rng().sample_iter(&Alphanumeric).take(5).collect();
    let mut dir = std::env::temp_dir();
    let path = format!("hypercore_rs_test/nonexistent_paths_test/{}", rand_string);
    dir.push(path);

    if Feed::open(&dir).await.is_err() {
        panic!("Opening nonexistent dir at a path should succeed");
    }

    if let Ok(d) = std::fs::metadata(dir) {
        if !d.is_dir() {
            panic!("Opening nonexistent dir at a path must create dir");
        }
    } else {
        panic!("Opening nonexistent dir at a path must create dir");
    }
}

#[async_std::test]
async fn try_open_file_as_dir() {
    if Feed::open("Cargo.toml").await.is_ok() {
        panic!("Opening path that points to a file must result in error");
    }
}

#[async_std::test]
async fn events_append() {
    let mut feed = create_feed(50).await.unwrap();
    let event_task = collect_events(&mut feed, 3);
    feed.append(br#"one"#).await.unwrap();
    feed.append(br#"two"#).await.unwrap();
    feed.append(br#"three"#).await.unwrap();

    let event_list = event_task.await;
    let mut expected = vec![];
    for _i in 0..3 {
        expected.push(Event::Append);
    }
    assert_eq!(event_list, expected, "Correct events emitted")
}

#[async_std::test]
async fn events_download() {
    let mut a = create_feed(50).await.unwrap();
    // Create a second feed with the first feed's key.
    let mut b = create_clone(&a).await.unwrap();

    let event_task = collect_events(&mut b, 3);

    a.append(b"one").await.unwrap();
    a.append(b"two").await.unwrap();
    a.append(b"three").await.unwrap();

    for i in 0..3 {
        let a_proof = a.proof(i, false).await.unwrap();
        let a_data = a.get(i).await.unwrap();
        b.put(i, a_data.as_deref(), a_proof).await.unwrap();
    }

    let event_list = event_task.await;

    let mut expected = vec![];
    for i in 0..3 {
        expected.push(Event::Download(i));
    }
    assert_eq!(event_list, expected, "Correct events emitted")
}

async fn create_clone(feed: &Feed) -> Result<Feed, anyhow::Error> {
    let (public, secret) = copy_keys(&feed);
    let storage = storage_memory().await?;
    let clone = Feed::builder(public, storage)
        .secret_key(secret)
        .build()
        .await?;
    Ok(clone)
}

fn copy_keys(feed: &Feed) -> (PublicKey, SecretKey) {
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

fn collect_events(feed: &mut Feed, n: usize) -> async_std::task::JoinHandle<Vec<Event>> {
    let mut events = feed.subscribe();
    let event_task = async_std::task::spawn(async move {
        let mut event_list = vec![];
        while let Some(event) = events.next().await {
            event_list.push(event);
            if event_list.len() == n {
                return event_list;
            }
        }
        event_list
    });
    event_task
}
