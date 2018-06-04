extern crate hypercore;

mod helpers;

use helpers::create_feed;

#[test]
fn regression_01() {
  let mut feed = create_feed(50).unwrap();
  feed.get(0).unwrap();
}
