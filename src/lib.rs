#![forbid(unsafe_code, bad_style, future_incompatible)]
#![forbid(rust_2018_idioms, rust_2018_compatibility)]
#![forbid(missing_debug_implementations)]
#![forbid(missing_docs)]
#![warn(unreachable_pub)]
#![cfg_attr(test, deny(warnings))]
#![doc(test(attr(deny(warnings))))]

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
//! // unimplemented
//! Ok(())
//! # })
//! # }
//! ```
//!
//! [dat-node]: https://github.com/mafintosh/hypercore
//! [Dat]: https://github.com/datrs
//! [Feed]: crate::feed::Feed

pub mod encoding;
pub mod prelude;

mod bitfield;
mod builder;
mod common;
mod core;
mod crypto;
mod data;
mod oplog;
mod storage;
mod tree;

#[cfg(feature = "cache")]
pub use crate::builder::CacheOptionsBuilder;
pub use crate::builder::HypercoreBuilder;
pub use crate::common::{
    DataBlock, DataHash, DataSeek, DataUpgrade, HypercoreError, Node, Proof, RequestBlock,
    RequestSeek, RequestUpgrade, Store,
};
pub use crate::core::{AppendOutcome, Hypercore, Info};
pub use crate::crypto::{generate_signing_key, sign, verify, PartialKeypair};
pub use crate::storage::Storage;
pub use ed25519_dalek::{
    SecretKey, Signature, SigningKey, VerifyingKey, KEYPAIR_LENGTH, PUBLIC_KEY_LENGTH,
    SECRET_KEY_LENGTH,
};
