#![cfg_attr(nightly, deny(missing_docs))]
#![cfg_attr(nightly, feature(external_doc))]
#![cfg_attr(nightly, doc(include = "../README.md"))]
#![cfg_attr(test, deny(warnings))]

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
mod crypto;
mod feed;
mod storage;
mod proof;

mod feed_builder;

pub use proof::Proof;
pub use crypto::{Keypair, Signature};
pub use feed::Feed;
pub use feed_builder::FeedBuilder;
pub use storage::{Node, NodeTrait, Storage, Store};

use failure::Error;

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

/// Custom result shorthand for Hypercore.
pub type Result<T> = std::result::Result<T, Error>;
