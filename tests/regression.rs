mod common;

use common::create_feed;

// Postmortem: errors were happening correctly, but the error check in
// `.signature()` was off. Instead of checking for a range (`<`), we were
// checking inclusively `<=`. All we had to do was fix the check, and we all
// good.
#[async_std::test]
async fn regression_01() {
    let mut feed = create_feed(50).await.unwrap();
    assert_eq!(feed.len(), 0);
    feed.signature(0).await.unwrap_err();

    let data = b"some_data";
    feed.append(data).await.unwrap();
    feed.signature(0).await.unwrap();
}
