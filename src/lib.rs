#![forbid(unsafe_code, bad_style, future_incompatible)]
#![forbid(rust_2018_idioms, rust_2018_compatibility)]
#![forbid(missing_debug_implementations)]
#![forbid(missing_docs)]
#![cfg_attr(test, deny(warnings))]

//! ## Example
//! ```rust
//! extern crate hypercore;
//!
//! use hypercore::Feed;
//! use std::path::PathBuf;
//!
//! let path = PathBuf::from("./my-first-dataset");
//! let mut feed = Feed::new(&path).unwrap();
//!
//! feed.append(b"hello").unwrap();
//! feed.append(b"world").unwrap();
//!
//! println!("{:?}", feed.get(0)); // prints "hello"
//! println!("{:?}", feed.get(1)); // prints "world"
//! ```

#[macro_use]
extern crate failure;

extern crate blake2_rfc;
extern crate byteorder;
extern crate ed25519_dalek;
extern crate flat_tree;
extern crate merkle_tree_stream;
extern crate pretty_hash;
extern crate rand;
extern crate random_access_disk;
extern crate random_access_memory;
extern crate random_access_storage;
extern crate sha2;
extern crate sleep_parser;
extern crate sparse_bitfield;
extern crate tree_index;

pub mod bitfield;
pub mod prelude;

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

use failure::Error;

/// A specialized `Result` type for Hypercore operations.
pub type Result<T> = std::result::Result<T, Error>;
