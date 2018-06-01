#![feature(test)]

extern crate failure;
extern crate hypercore;
extern crate random_access_memory as ram;

extern crate test;

use self::test::Bencher;
use failure::Error;
use hypercore::{Feed, Storage, Store};

fn create_feed(
  page_size: usize,
) -> Result<Feed<ram::RandomAccessMethods>, Error> {
  let create = |_store: Store| ram::Sync::new(page_size);
  let storage = Storage::new(create)?;
  Ok(Feed::with_storage(storage)?)
}

#[bench]
fn create(b: &mut Bencher) {
  b.iter(|| {
    create_feed(1024).unwrap();
  });
}

#[bench]
fn write(b: &mut Bencher) {
  let mut feed = create_feed(1024).unwrap();
  let data = Vec::from("hello");
  b.iter(|| {
    feed.append(&data).unwrap();
  });
}

#[bench]
fn read(b: &mut Bencher) {
  let mut feed = create_feed(1024).unwrap();
  let data = Vec::from("hello");
  for _ in 0..1000 {
    feed.append(&data).unwrap();
  }

  let mut i = 0;
  b.iter(|| {
    feed.get(i).unwrap();
    i += 1;
  });
}
