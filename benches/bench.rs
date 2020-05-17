#![feature(test)]
extern crate test;

use anyhow::Error;
use random_access_memory::RandomAccessMemory;
use test::Bencher;

use hypercore::{Feed, Storage};

async fn create_feed(page_size: usize) -> Result<Feed<RandomAccessMemory>, Error> {
    let storage =
        Storage::new(|_| Box::pin(async move { Ok(RandomAccessMemory::new(page_size)) })).await?;
    Feed::with_storage(storage).await
}

#[bench]
fn create(b: &mut Bencher) {
    b.iter(|| {
        async_std::task::block_on(async {
            create_feed(1024).await.unwrap();
        });
    });
}

#[bench]
fn write(b: &mut Bencher) {
    async_std::task::block_on(async {
        let mut feed = create_feed(1024).await.unwrap();
        let data = Vec::from("hello");
        b.iter(|| {
            async_std::task::block_on(async {
                feed.append(&data).await.unwrap();
            });
        });
    });
}

#[bench]
fn read(b: &mut Bencher) {
    async_std::task::block_on(async {
        let mut feed = create_feed(1024).await.unwrap();
        let data = Vec::from("hello");
        for _ in 0..1000 {
            feed.append(&data).await.unwrap();
        }

        let mut i = 0;
        b.iter(|| {
            async_std::task::block_on(async {
                feed.get(i).await.unwrap();
                i += 1;
            });
        });
    });
}
