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
#[cfg(not(feature = "v10"))]
pub use crate::common::Node;
#[cfg(feature = "v10")]
pub use crate::core::Hypercore;
#[cfg(not(feature = "v10"))]
pub use crate::storage::{NodeTrait, Storage, Store};
#[cfg(feature = "v10")]
pub use crate::storage_v10::{PartialKeypair, Storage, Store};
