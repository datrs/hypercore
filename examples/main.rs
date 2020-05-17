use hypercore::Feed;

#[async_std::main]
async fn main() {
    let mut feed = Feed::default();

    feed.append(b"hello").await.unwrap();
    feed.append(b"world").await.unwrap();

    println!("{:?}", feed.get(0).await); // prints "hello"
    println!("{:?}", feed.get(1).await); // prints "world"
}
