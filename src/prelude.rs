/// Convenience wrapper to import all of Hypercore's core.
///
/// ```rust
/// extern crate hypercore;
///
/// use hypercore::prelude::*;
///
/// fn main () {
///   let feed = Feed::default();
/// }
/// ```
pub mod prelude {
  pub use crypto::Keypair;
  pub use feed::Feed;
  pub use feed_builder::FeedBuilder;
  pub use storage::{Node, NodeTrait, Storage, Store};
}
