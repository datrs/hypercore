#![feature(test)]

extern crate failure;
extern crate hypercore;
extern crate random_access_memory as ram;

extern crate test;

use self::test::Bencher;
use failure::Error;
use hypercore::{Feed, Storage, Store};

fn create_feed(page_size: usize) -> Result<Feed<ram::SyncMethods>, Error> {
  let create = |_store: Store| ram::Sync::new(page_size);
  let storage = Storage::new(create)?;
  Ok(Feed::with_storage(storage)?)
}

#[bench]
fn create(b: &mut Bencher) {
  b.iter(|| {
    create_feed(50).unwrap();
  });
}

#[bench]
fn write(b: &mut Bencher) {
  let mut feed = create_feed(50).unwrap();
  b.iter(|| {
    feed.append(b"hello").unwrap();
  });
}

#[bench]
fn read(b: &mut Bencher) {
  let mut feed = create_feed(50).unwrap();
  for _ in 0..1000 {
    feed.append(b"hello").unwrap();
  }

  let mut i = 0;
  b.iter(|| {
    feed.get(i).unwrap();
    i += 1;
  });
}
