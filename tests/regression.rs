extern crate hypercore;

mod helpers;

use helpers::create_feed;

#[test]
fn regression_01() {
  let mut feed = create_feed(50).unwrap();
  assert_eq!(feed.len(), 0);
  feed.signature(0).is_err();

  let data = b"some_data";
  feed.append(data).unwrap();
  feed.signature(0).unwrap();
}
