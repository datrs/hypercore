extern crate failure;
extern crate hypercore;
extern crate random_access_memory as ram;

mod helpers;

use helpers::create_feed;
use hypercore::{FeedBuilder, Keypair, NodeTrait, Storage};

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
  let f_bytes = &feed.keypair().to_bytes();
  let keypair = Keypair::from_bytes(f_bytes).unwrap();

  let storage = Storage::new(|_| ram::RandomAccessMemory::new(50)).unwrap();
  let mut evil_feed = FeedBuilder::new(keypair, storage).build().unwrap();
  let ef_bytes = &feed.keypair().to_bytes();

  // Verify the keys are the same.
  assert_eq!(&f_bytes.to_vec(), &ef_bytes.to_vec());

  // Verify that the signature on a single feed is correct.
  feed.append(b"test").unwrap();
  let sig = feed.signature(0).unwrap();
  feed.verify(0, &sig).unwrap();

  // Verify that the signature between two different feeds is different.
  evil_feed.append(b"t0st").unwrap();
  let res = evil_feed.verify(0, &sig);
  assert!(res.is_err());
}
