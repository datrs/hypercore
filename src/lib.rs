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
mod common;
mod core;
mod crypto;
mod data;
mod oplog;
mod storage;
mod tree;

pub use crate::common::{
    DataBlock, DataHash, DataSeek, DataUpgrade, HypercoreError, Node, Proof, RequestBlock,
    RequestSeek, RequestUpgrade, Store,
};
pub use crate::core::Hypercore;
pub use crate::crypto::{generate_keypair, sign, verify, Signature};
pub use crate::storage::{PartialKeypair, Storage};
pub use ed25519_dalek::{
    ExpandedSecretKey, Keypair, PublicKey, SecretKey, EXPANDED_SECRET_KEY_LENGTH, KEYPAIR_LENGTH,
    PUBLIC_KEY_LENGTH, SECRET_KEY_LENGTH,
};
