#![forbid(unsafe_code, bad_style, future_incompatible)]
#![forbid(rust_2018_idioms, rust_2018_compatibility)]
#![forbid(missing_debug_implementations)]
#![forbid(missing_docs)]
// FIXME: Off during v10 coding
// #![cfg_attr(test, deny(warnings))]

//! ## Introduction
//! Hypercore is a secure, distributed append-only log. Built for sharing
//! large datasets and streams of real time data as part of the [Dat] project.
//! This is a rust port of [the original node version][dat-node]
//! aiming for interoperability. The primary way to use this crate is through the [Feed] struct.
//!
//! ## Example
//! ```rust
//! #[cfg(not(feature = "v10"))]
//! # fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//! # async_std::task::block_on(async {
//! let mut feed = hypercore::open("./feed.db").await?;
//!
//! feed.append(b"hello").await?;
//! feed.append(b"world").await?;
//!
//! assert_eq!(feed.get(0).await?, Some(b"hello".to_vec()));
//! assert_eq!(feed.get(1).await?, Some(b"world".to_vec()));
//! # Ok(())
//! # })
//! # }
//! #[cfg(feature = "v10")]
//! # fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//! # async_std::task::block_on(async {
//! // unimplemented
//! Ok(())
//! # })
//! # }
//! ```
//!
//! [dat-node]: https://github.com/mafintosh/hypercore
//! [Dat]: https://github.com/datrs
//! [Feed]: crate::feed::Feed

#[cfg(not(feature = "v10"))]
pub mod bitfield;
pub mod compact_encoding;
pub mod prelude;

mod audit;

#[cfg(feature = "v10")]
mod bitfield_v10;
mod common;
#[cfg(feature = "v10")]
mod core;
mod crypto;
#[cfg(feature = "v10")]
mod data;
mod event;
#[cfg(not(feature = "v10"))]
mod feed;
#[cfg(not(feature = "v10"))]
mod feed_builder;
#[cfg(feature = "v10")]
mod oplog;
mod proof;
mod replicate;
#[cfg(not(feature = "v10"))]
mod storage;
#[cfg(feature = "v10")]
mod storage_v10;
#[cfg(feature = "v10")]
mod tree;

pub use crate::common::Node;
#[cfg(feature = "v10")]
pub use crate::common::Store;
#[cfg(feature = "v10")]
pub use crate::core::Hypercore;
pub use crate::crypto::{generate_keypair, sign, verify, Signature};
pub use crate::event::Event;
#[cfg(not(feature = "v10"))]
pub use crate::feed::Feed;
#[cfg(not(feature = "v10"))]
pub use crate::feed_builder::FeedBuilder;
pub use crate::proof::Proof;
pub use crate::replicate::Peer;
#[cfg(not(feature = "v10"))]
pub use crate::storage::{NodeTrait, PartialKeypair, Storage, Store};
#[cfg(feature = "v10")]
pub use crate::storage_v10::{PartialKeypair, Storage};
pub use ed25519_dalek::{PublicKey, SecretKey};

use std::path::Path;

/// Create a new Hypercore `Feed`.
#[cfg(not(feature = "v10"))]
pub async fn open<P: AsRef<Path>>(
    path: P,
) -> anyhow::Result<Feed<random_access_disk::RandomAccessDisk>> {
    Feed::open(path).await
}
