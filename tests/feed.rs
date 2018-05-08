extern crate hypercore;

use hypercore::Feed;
use std::path::PathBuf;

#[test]
fn set_get() {
  let path = PathBuf::from("./my-first-dataset");
  let mut feed = Feed::new(path).unwrap();

  feed.append(b"hello").unwrap();
  feed.append(b"world").unwrap();

  assert_eq!(feed.get(0).unwrap(), Some(b"hello".to_vec()));
  assert_eq!(feed.get(1).unwrap(), Some(b"world".to_vec()));
}
