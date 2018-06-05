extern crate failure;
extern crate hypercore;
extern crate random_access_memory as ram;

use self::failure::Error;
use hypercore::{Feed, Storage, Store};

pub fn create_feed(
  page_size: usize,
) -> Result<Feed<ram::RandomAccessMemoryMethods>, Error> {
  let create = |_store: Store| ram::RandomAccessMemory::new(page_size);
  let storage = Storage::new(create)?;
  Ok(Feed::with_storage(storage)?)
}
