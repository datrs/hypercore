#![feature(test)]
extern crate test;

use anyhow::Error;
use random_access_memory::RandomAccessMemory;
use test::Bencher;

use hypercore::{Feed, Storage, Store};

fn create_feed(page_size: usize) -> Result<Feed<RandomAccessMemory>, Error> {
    let create = |_store: Store| Ok(RandomAccessMemory::new(page_size));
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
