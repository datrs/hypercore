#![forbid(unsafe_code, bad_style, future_incompatible)]
#![forbid(rust_2018_idioms, rust_2018_compatibility)]
#![forbid(missing_debug_implementations)]
#![forbid(missing_docs)]
#![cfg_attr(test, deny(warnings))]

//! ## Introduction
//! Hypercore is a secure, distributed append-only log. Built for sharing
//! large datasets and streams of real time data as part of the [Dat] project.
//! This is a rust port of [the original node version][dat-node]
//! aiming for interoperability. The primary way to use this crate is through the [Feed] struct.
//!
//! ## Example
//! ```rust
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
//! ```
//!
//! [dat-node]: https://github.com/mafintosh/hypercore
//! [Dat]: https://github.com/datrs
//! [Feed]: crate::feed::Feed

pub mod bitfield;
pub mod prelude;

mod audit;
mod crypto;
mod event;
mod feed;
mod feed_builder;
mod proof;
mod replicate;
mod storage;

pub use crate::crypto::{generate_keypair, sign, verify, Signature};
pub use crate::event::Event;
pub use crate::feed::Feed;
pub use crate::feed_builder::FeedBuilder;
pub use crate::proof::Proof;
pub use crate::replicate::Peer;
pub use crate::storage::{Node, NodeTrait, Storage, Store};
pub use ed25519_dalek::{PublicKey, SecretKey};

use std::path::Path;

/// Create a new Hypercore `Feed`.
pub async fn open<P: AsRef<Path>>(path: P) -> anyhow::Result<Feed> {
    Feed::open_from_disk(path).await
}
