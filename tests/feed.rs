extern crate failure;
extern crate hypercore;
extern crate random_access_memory as ram;

mod helpers;

use helpers::{copy_keys, create_feed};
use hypercore::{generate_keypair, Feed, NodeTrait, Storage};

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
