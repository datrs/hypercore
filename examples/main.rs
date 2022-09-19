#[cfg(feature = "v9")]
use hypercore::Feed;

#[async_std::main]
#[cfg(feature = "v9")]
async fn main() {
    let mut feed = Feed::open("feed.db").await.expect("Failed to create dir");

    feed.append(b"hello").await.unwrap();
    feed.append(b"world").await.unwrap();

    drop(feed);

    let mut feed = Feed::open("feed.db").await.expect("Failed to create dir");

    feed.append(b"welcome").await.unwrap();
    feed.append(b"back").await.unwrap();

    println!("{:?}", format_res(feed.get(0).await)); // prints "hello"
    println!("{:?}", format_res(feed.get(1).await)); // prints "world"
    println!("{:?}", format_res(feed.get(2).await)); // prints "welcome"
    println!("{:?}", format_res(feed.get(3).await)); // prints "back"
}

#[async_std::main]
#[cfg(feature = "v10")]
async fn main() {
    unimplemented!();
}

fn format_res(res: anyhow::Result<Option<Vec<u8>>>) -> String {
    match res {
        Ok(Some(bytes)) => String::from_utf8(bytes).expect("Shouldnt fail in example"),
        Ok(None) => "Got None in feed".to_string(),
        Err(e) => format!("Error getting value from feed, reason = {}", e),
    }
}
