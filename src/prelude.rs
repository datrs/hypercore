//! Convenience wrapper to import all of Hypercore's core.
//!
//! ```rust
//!
//! use hypercore::prelude::*;
//!
//! fn main () {
//!   let feed = Feed::default();
//! }
//! ```
pub use crate::feed::Feed;
// pub use feed_builder::FeedBuilder;
pub use crate::storage::{Node, NodeTrait, Storage, Store};
