//! Convenience wrapper to import all of Hypercore's core.
//!
//! ```rust
//! use hypercore::prelude::*;
//! #[cfg(feature = "v9")]
//! let feed = Feed::default();
//! ```
#[cfg(feature = "v9")]
pub use crate::feed::Feed;
// pub use feed_builder::FeedBuilder;
pub use crate::common::Node;
#[cfg(feature = "v10")]
pub use crate::common::Store;
#[cfg(feature = "v10")]
pub use crate::core::Hypercore;
#[cfg(feature = "v9")]
pub use crate::storage::{NodeTrait, Storage, Store};
#[cfg(feature = "v10")]
pub use crate::storage_v10::{PartialKeypair, Storage};
