extern crate hypercore;

mod helpers;

use helpers::create_feed;

#[test]
fn regression_01() {
  let mut feed = create_feed(50).unwrap();
  let sig = feed.signature(0).unwrap();
  feed.verify(0, &sig).unwrap();
}
