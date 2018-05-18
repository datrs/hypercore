extern crate failure;
extern crate hypercore;
extern crate random_access_memory as ram;

use failure::Error;
use hypercore::{Feed, Storage, Store};

fn create_feed(page_size: usize) -> Result<Feed<ram::SyncMethods>, Error> {
  let create = |_store: Store| ram::Sync::new(page_size);
  let storage = Storage::new(create)?;
  Ok(Feed::with_storage(storage)?)
}

#[test]
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

// #[test]
// fn verify() {
//   let mut feed = create_feed(50).unwrap();

//   let storage = Storage::new(|_store: Store| ram::Sync::new(50)).unwrap();
//   let mut evil_feed = Feed::with_storage(storage).unwrap(); // FIXME: pass key from feed

//   feed.append(b"test").unwrap();
//   evil_feed.append(b"t0st").unwrap();

//   let sig = feed.signature(0).unwrap();
//   feed.verify(0, &sig).unwrap();

//   let res = evil_feed.verify(0, &sig);
//   assert!(res.is_err());
// }
