#![forbid(unsafe_code, bad_style, future_incompatible)]
#![forbid(rust_2018_idioms, rust_2018_compatibility)]
#![forbid(missing_debug_implementations)]
#![forbid(missing_docs)]
#![warn(unreachable_pub)]
#![cfg_attr(test, deny(warnings))]
#![doc(test(attr(deny(warnings))))]

//! ## Introduction
//!
//! Hypercore is a secure, distributed append-only log. Built for sharing
//! large datasets and streams of real time data as part of the [Dat] project.
//! This is a rust port of [the original Javascript version][holepunch-hypercore]
//! aiming for interoperability with LTS version. The primary way to use this
//! crate is through the [Hypercore] struct, which can be created using the
//! [HypercoreBuilder].
//!
//! This crate supports WASM with `cargo build --target=wasm32-unknown-unknown`.
//!
//! ## Features
//!
//! ### `sparse` (default)
//!
//! When using disk storage, clearing values may create sparse files. On by default.
//!
//! ### `async-std` (default)
//!
//! Use the async-std runtime, on by default. Either this or `tokio` is mandatory.
//!
//! ### `tokio`
//!
//! Use the tokio runtime. Either this or `async_std` is mandatory.
//!
//! ### `cache`
//!
//! Use a moka cache for merkle tree nodes to speed-up reading.
//!
//! ## Example
//! ```rust
//! # #[cfg(feature = "tokio")]
//! # tokio_test::block_on(async {
//! # example().await;
//! # });
//! # #[cfg(feature = "async-std")]
//! # async_std::task::block_on(async {
//! # example().await;
//! # });
//! # async fn example() {
//! use hypercore::{HypercoreBuilder, Storage};
//!
//! // Create an in-memory hypercore using a builder
//! let mut hypercore = HypercoreBuilder::new(Storage::new_memory().await.unwrap())
//!     .build()
//!     .await
//!     .unwrap();
//!
//! // Append entries to the log
//! hypercore.append(b"Hello, ").await.unwrap();
//! hypercore.append(b"world!").await.unwrap();
//!
//! // Read entries from the log
//! assert_eq!(hypercore.get(0).await.unwrap().unwrap(), b"Hello, ");
//! assert_eq!(hypercore.get(1).await.unwrap().unwrap(), b"world!");
//! # }
//! ```
//!
//! Find more examples in the [examples] folder.
//!
//! [Dat]: https://github.com/datrs
//! [holepunch-hypercore]: https://github.com/holepunchto/hypercore
//! [Hypercore]: crate::core::Hypercore
//! [HypercoreBuilder]: crate::builder::HypercoreBuilder
//! [examples]: https://github.com/datrs/hypercore/tree/master/examples

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
