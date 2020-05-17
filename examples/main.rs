use hypercore::Feed;

fn main() {
    let mut feed = Feed::default();

    feed.append(b"hello").unwrap();
    feed.append(b"world").unwrap();

    println!("{:?}", feed.get(0)); // prints "hello"
    println!("{:?}", feed.get(1)); // prints "world"
}
