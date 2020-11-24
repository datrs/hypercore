use async_std::task;
use hypercore::Feed;

async fn append(feed: &mut Feed, content: &[u8]) {
    feed.append(content).await.unwrap();
}

async fn print(feed: &mut Feed) {
    println!("{:?}", feed.get(0).await);
    println!("{:?}", feed.get(1).await);
}

fn main() {
    task::block_on(task::spawn(async {
        let mut feed = Feed::open_in_memory().await.unwrap();
        append(&mut feed, b"hello").await;
        append(&mut feed, b"world").await;
        print(&mut feed).await;
    }));
}
