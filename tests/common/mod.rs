use hypercore;

use anyhow::Error;
use futures::future::FutureExt;
use hypercore::{Feed, Storage, Store};
use random_access_memory as ram;

pub async fn create_feed(page_size: usize) -> Result<Feed, Error> {
    let create = |_store: Store| async move { Ok(ram::RandomAccessMemory::new(page_size)) }.boxed();
    let storage = Storage::new(create, false).await?;
    Feed::with_storage(storage).await
}
