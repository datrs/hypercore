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
use self::storage::Node;
use std::path::PathBuf;

use crypto::{KeyPair, Merkle};
pub use storage::{Storage, Store};

/// Append-only log structure.
pub struct Feed<T>
where
  T: SyncMethods,
{
  merkle: Merkle,
  key_pair: KeyPair,
  storage: Storage<T>,
  byte_length: usize,
}

impl<T> Feed<T>
where
  T: SyncMethods,
{
  /// Create a new instance with a custom storage backend.
  pub fn with_storage(storage: storage::Storage<T>) -> Result<Self, Error> {
    let key_pair = KeyPair::default(); // TODO: read key_pair from disk;
    Ok(Self {
      merkle: Merkle::new(),
      byte_length: 0,
      key_pair,
      storage,
    })
  }

  /// Append data into the log.
  pub fn append(&mut self, data: &[u8]) -> Result<(), Error> {
    // let data = self.codec.encode(&data);
    let nodes = self.merkle.next(data);
    let mut offset = 0;

    // TODO: find a way to write this using `storage.put_data()`.
    // self
    //   .storage
    //   .data
    //   .write(self.byte_length + offset, data)?;
    offset += data.len();

    // FIXME: the conversion here is super gross.
    // FIXME: storage.put_node shouldn't require an index as the first arg.
    for node in nodes {
      let mut node = Node::new(
        node.position(),
        node.hash().to_vec(),
        node.len(),
      );
      self.storage.put_node(node.index(), &mut node)?;
    }

    self.byte_length += offset;
    unimplemented!();
  }

  /// Retrieve data from the log.
  pub fn get(&self, _index: usize) -> Option<&[u8]> {
    unimplemented!();
  }
}

impl Feed<self::rad::SyncMethods> {
  /// Create a new instance that persists to disk at the location of `dir`.
  // TODO: Ensure that dir is always a directory.
  // NOTE: Should we `mkdirp` here?
  // NOTE: Should we call these `data.bitfield` / `data.tree`?
  pub fn new(dir: PathBuf) -> Result<Self, Error> {
    let create = |storage: Store| {
      let name = match storage {
        Store::Tree => "tree",
        Store::Data => "data",
        Store::Bitfield => "bitfield",
        Store::Signatures => "signatures",
      };
      rad::Sync::new(dir.as_path().join(name))
    };

    Self::with_storage(Storage::new(create)?)
  }
}

/// Create a new instance with an in-memory storage backend.
///
/// ## Panics
/// Can panic if constructing the in-memory store fails, which is highly
/// unlikely.
impl Default for Feed<self::ram::SyncMethods> {
  fn default() -> Self {
    let create = |_store: Store| ram::Sync::default();
    let storage = storage::Storage::new(create).unwrap();
    Self::with_storage(storage).unwrap()
  }
}
