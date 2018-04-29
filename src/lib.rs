#![deny(missing_docs)]
#![cfg_attr(test, deny(warnings))]
#![feature(external_doc)]
#![doc(include = "../README.md")]
// #![cfg_attr(test, feature(plugin))]
// #![cfg_attr(test, plugin(clippy))]

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate failure;
extern crate random_access_disk as rad;
extern crate random_access_memory as ram;
extern crate random_access_storage as ras;

pub mod bitfield;
pub mod crypto;
pub mod storage;

use self::failure::Error;
use self::ras::SyncMethods;
use std::path::PathBuf;

use crypto::KeyPair;
pub use storage::{Storage, Store};

/// Append-only log structure.
pub struct Feed<T>
where
  T: SyncMethods,
{
  key_pair: KeyPair,
  storage: Storage<T>,
}

impl Feed<self::rad::SyncMethods> {
  /// Create a new instance that persists to disk at the location of `dir`.
  // TODO: Ensure that dir is always a directory.
  // NOTE: Should we `mkdirp` here?
  // NOTE: Should we call these `data.bitfield` / `data.tree`?
  pub fn new(dir: PathBuf) -> Result<Self, Error> {
    let key_pair = KeyPair::default(); // TODO: read key_pair;
    let storage = Storage::new(|storage: Store| {
      let name = match storage {
        Store::Tree => "tree",
        Store::Data => "data",
        Store::Bitfield => "bitfield",
        Store::Signatures => "signatures",
      };
      rad::Sync::new(dir.as_path().join(name))
    })?;

    Ok(Self {
      key_pair,
      storage,
    })
  }
}

impl<T> Feed<T>
where
  T: SyncMethods,
{
  /// Create a new instance with a custom storage backend.
  pub fn with_storage(storage: storage::Storage<T>) -> Result<Self, Error> {
    let key_pair = KeyPair::default(); // TODO: read key_pair;

    Ok(Self {
      key_pair,
      storage,
    })
  }

  /// Append data into the log.
  pub fn append(&self, _data: &[u8]) -> Result<(), Error> {
    unimplemented!();
  }

  /// Retrieve data from the log.
  pub fn get(&self, _index: usize) -> Option<&[u8]> {
    unimplemented!();
  }
}

/// Create a new instance with an in-memory storage backend.
///
/// ## Panics
/// Can panic if the constructing the in-memory store fails, which is highly
/// unlikely.
impl Default for Feed<self::ram::SyncMethods> {
  fn default() -> Self {
    let key_pair = KeyPair::default();
    let storage =
      storage::Storage::new(|_store: Store| ram::Sync::default()).unwrap();

    Self {
      key_pair,
      storage,
    }
  }
}
