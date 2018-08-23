extern crate failure;
extern crate hypercore;
extern crate random_access_memory as ram;
extern crate random_access_storage;

use self::failure::Error;
use self::random_access_storage::RandomAccessMethods;
use hypercore::{Feed, PublicKey, SecretKey, Storage, Store};
use std::fmt::Debug;

pub fn create_feed(
  page_size: usize,
) -> Result<Feed<ram::RandomAccessMemoryMethods>, Error> {
  let create = |_store: Store| ram::RandomAccessMemory::new(page_size);
  let storage = Storage::new(create)?;
  Ok(Feed::with_storage(storage)?)
}

pub fn copy_keys(
  feed: &Feed<impl RandomAccessMethods<Error = Error> + Debug>,
) -> (PublicKey, SecretKey) {
  match &feed.secret_key() {
    Some(secret) => {
      let secret = secret.to_bytes();
      let public = &feed.public_key().to_bytes();

      let public = PublicKey::from_bytes(public).unwrap();
      let secret = SecretKey::from_bytes(&secret).unwrap();

      return (public, secret);
    }
    _ => panic!("<tests/helpers>: Could not access secret key"),
  }
}
