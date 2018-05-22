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

extern crate flat_tree as flat;
extern crate random_access_disk as rad;
extern crate random_access_memory as ram;
extern crate random_access_storage as ras;
extern crate sparse_bitfield;
extern crate tree_index;

pub mod bitfield;
pub mod crypto;
mod feed_builder;
pub mod storage;

pub use crypto::Keypair;
pub use feed_builder::FeedBuilder;
pub use storage::{Node, Storage, Store, NodeTrait};

use crypto::{generate_keypair, sign, Hash, Merkle, Signature};
use failure::Error;
use ras::SyncMethods;
use sparse_bitfield::Bitfield;
use std::fmt::Debug;
use std::path::PathBuf;
use std::rc::Rc;
use tree_index::TreeIndex;

/// Append-only log structure.
pub struct Feed<T>
where
  T: SyncMethods + Debug,
{
  /// Merkle tree instance.
  merkle: Merkle,
  /// Ed25519 key pair.
  keypair: Keypair,
  /// Struct that saves data to a `random-access-storage` backend.
  storage: Storage<T>,
  /// Total length of data stored.
  byte_length: usize,
  /// TODO: description. Length of... roots?
  length: usize,
  /// Bitfield to keep track of which data we own.
  bitfield: Bitfield,
  tree: TreeIndex,
}

impl<T> Feed<T>
where
  T: SyncMethods + Debug,
{
  /// Create a new instance with a custom storage backend.
  pub fn with_storage(storage: storage::Storage<T>) -> Result<Self, Error> {
    let keypair = generate_keypair(); // TODO: read keypair from disk;
    Ok(FeedBuilder::new(keypair, storage).build()?)
  }

  /// Get the amount of entries in the feed.
  pub fn len(&self) -> usize {
    self.length
  }

  /// Get the total amount of bytes stored in the feed.
  pub fn byte_len(&self) -> usize {
    self.byte_length
  }

  /// Append data into the log.
  pub fn append(&mut self, data: &[u8]) -> Result<(), Error> {
    self.merkle.next(data);
    let mut offset = 0;

    self.storage.write_data(self.byte_length + offset, &data)?;
    offset += data.len();

    let hash = Hash::from_roots(self.merkle.roots());
    let index = self.length;
    let signature = sign(&self.keypair, hash.as_bytes());
    self.storage.put_signature(index, signature)?;

    for node in self.merkle.nodes() {
      self.storage.put_node(node)?;
    }

    self.byte_length += offset;

    self.bitfield.set(self.length, true);
    self.tree.set(2 * self.length);
    self.length += 1;

    Ok(())
  }

  /// Retrieve data from the log.
  pub fn get(&mut self, index: usize) -> Result<Option<Vec<u8>>, Error> {
    if !self.bitfield.get(index) {
      // NOTE: Do (network) lookup here once we have network code.
      return Ok(None);
    }
    Ok(Some(self.storage.get_data(index)?))
  }

  /// Get a signature from the store.
  pub fn signature(&mut self, index: usize) -> Result<Signature, Error> {
    ensure!(
      index <= self.length,
      format!("No signature found for {}", index)
    );
    Ok(self.storage.next_signature(index)?)
  }

  /// Verify the entire feed. Checks a signature against the signature of all
  /// root nodes combined.
  pub fn verify(
    &mut self,
    index: usize,
    signature: &Signature,
  ) -> Result<(), Error> {
    let roots = self.root_hashes(index)?;
    let roots = roots.into_iter().map(|i| Rc::new(i)).collect();

    let message = Hash::from_roots(&roots);
    let message = message.as_bytes();

    Ok(crypto::verify(&self.keypair.public, message, signature)?)
  }

  /// Get all the roots in the feed.
  // In the JavaScript implemenentation this calls to `._getRootsToVerify()`
  // internally. In Rust it seems better to just inline the code.
  pub fn root_hashes(&mut self, index: usize) -> Result<Vec<Node>, Error> {
    ensure!(
      index <= self.length,
      format!("Root index bounds exceeded {} > {}", index, self.length)
    );
    let roots_index = index * 2 + 2;
    let mut indexes = vec![];
    flat::full_roots(roots_index, &mut indexes);

    let mut roots = Vec::with_capacity(indexes.len());
    for index in indexes {
      let node = self.storage.get_node(index)?;
      roots.push(node);
    }

    Ok(roots)
  }

  /// Access the keypair.
  pub fn keypair(&self) -> &Keypair {
    &self.keypair
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

    let storage = Storage::new(create)?;
    let keypair = generate_keypair(); // TODO: read keypair from disk;
    Ok(FeedBuilder::new(keypair, storage).build()?)
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
