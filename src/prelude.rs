//! Convenience wrapper to import all of Hypercore's core.
//!
//! ```rust
//! use hypercore::prelude::*;
//! #[cfg(not(feature = "v10"))]
//! let feed = Feed::default();
//! ```
#[cfg(not(feature = "v10"))]
pub use crate::feed::Feed;
// pub use feed_builder::FeedBuilder;
pub use crate::storage::{Node, NodeTrait, Storage, Store};
