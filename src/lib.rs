#![cfg_attr(nightly, deny(missing_docs))]
#![cfg_attr(nightly, feature(external_doc))]
#![cfg_attr(nightly, doc(include = "../README.md"))]
#![cfg_attr(test, deny(warnings))]

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
mod feed;
mod feed_builder;
mod proof;
mod replicate;
mod storage;

pub use crypto::{Keypair, Signature};
pub use feed::Feed;
pub use feed_builder::FeedBuilder;
pub use proof::Proof;
pub use replicate::Peer;
pub use storage::{Node, NodeTrait, Storage, Store};

use failure::Error;

/// A specialized `Result` type for Hypercore operations.
pub type Result<T> = std::result::Result<T, Error>;
