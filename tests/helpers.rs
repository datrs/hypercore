extern crate failure;
extern crate hypercore;
extern crate random_access_memory as ram;
extern crate random_access_storage;

use self::failure::Error;
use self::random_access_storage::RandomAccess;
use hypercore::{Feed, PublicKey, SecretKey, Storage, Store};
use std::fmt::Debug;

pub fn create_feed(
  page_size: usize,
) -> Result<Feed<ram::RandomAccessMemory>, Error> {
  let create = |_store: Store| Ok(ram::RandomAccessMemory::new(page_size));
  let storage = Storage::new(create)?;
  Ok(Feed::with_storage(storage)?)
}
