#![deny(missing_docs)]
#![cfg_attr(test, deny(warnings))]
#![feature(external_doc)]
#![doc(include = "../README.md")]
// #![cfg_attr(test, feature(plugin))]
// #![cfg_attr(test, plugin(clippy))]

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate failure;

extern crate flat_tree as flat;
extern crate random_access_disk as rad;
extern crate random_access_memory as ram;
extern crate random_access_storage as ras;
extern crate sparse_bitfield;
extern crate tree_index;

pub mod bitfield;
pub mod crypto;
mod feed_builder;
pub mod storage;
pub mod feed;

pub use crypto::Keypair;
pub use feed_builder::FeedBuilder;
pub use storage::{Node, Storage, Store, NodeTrait};
pub use feed::Feed;

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
  pub use feed_builder::FeedBuilder;
  pub use storage::{Node, Storage, Store, NodeTrait};
  pub use feed::Feed;
}
