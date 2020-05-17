mod common;

use common::create_feed;

// Postmortem: errors were happening correctly, but the error check in
// `.signature()` was off. Instead of checking for a range (`<`), we were
// checking inclusively `<=`. All we had to do was fix the check, and we all
// good.
#[test]
fn regression_01() {
    let mut feed = create_feed(50).unwrap();
    assert_eq!(feed.len(), 0);
    feed.signature(0).unwrap_err();

    let data = b"some_data";
    feed.append(data).unwrap();
    feed.signature(0).unwrap();
}
