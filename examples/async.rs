use async_std::task;
#[cfg(feature = "v9")]
use hypercore::Feed;
use random_access_storage::RandomAccess;
use std::fmt::Debug;

#[cfg(feature = "v9")]
async fn append<T>(feed: &mut Feed<T>, content: &[u8])
where
    T: RandomAccess<Error = Box<dyn std::error::Error + Send + Sync>> + Debug + Send,
{
    feed.append(content).await.unwrap();
}

#[cfg(feature = "v9")]
async fn print<T>(feed: &mut Feed<T>)
where
    T: RandomAccess<Error = Box<dyn std::error::Error + Send + Sync>> + Debug + Send,
{
    println!("{:?}", feed.get(0).await);
    println!("{:?}", feed.get(1).await);
}

#[cfg(feature = "v9")]
fn main() {
    task::block_on(task::spawn(async {
        let mut feed = Feed::default();

        append(&mut feed, b"hello").await;
        append(&mut feed, b"world").await;
        print(&mut feed).await;
    }));
}

#[cfg(feature = "v10")]
fn main() {
    unimplemented!()
}
