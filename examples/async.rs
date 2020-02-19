use async_std::task;
use failure::Error;
use hypercore::Feed;
use random_access_storage::RandomAccess;
use std::fmt::Debug;

async fn append<T>(feed: &mut Feed<T>, content: &[u8])
where
    T: RandomAccess<Error = Error> + Debug,
{
    feed.append(content).unwrap();
}

async fn print<T>(feed: &mut Feed<T>)
where
    T: RandomAccess<Error = Error> + Debug,
{
    println!("{:?}", feed.get(0));
    println!("{:?}", feed.get(1));
}

fn main() {
    task::block_on(task::spawn(async {
        let mut feed = Feed::default();

        append(&mut feed, b"hello").await;
        append(&mut feed, b"world").await;
        print(&mut feed).await;
    }));
}
