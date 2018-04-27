//! Save data to a desired storage backend.

extern crate failure;
extern crate flat_tree as flat;
extern crate random_access_disk as rad;
extern crate random_access_memory as ram;
extern crate random_access_storage as ras;
extern crate sleep_parser;

use self::failure::Error;
use self::ras::SyncMethods;
use self::sleep_parser::*;
use super::crypto::{KeyPair, PublicKey, SecretKey};
use bitfield::Bitfield;
use std::path::PathBuf;

/// The types of stores that can be created.
#[derive(Debug)]
pub enum Store {
  /// Tree
  Tree,
  /// Data
  Data,
  /// Bitfield
  Bitfield,
  /// Signatures
  Signatures,
}

/// Save data to a desired storage backend.
// #[derive(Debug)]
pub struct Storage<T>
where
  T: SyncMethods,
{
  public_key: PublicKey,
  secret_key: SecretKey,
  tree: ras::Sync<T>,
  data: ras::Sync<T>,
  bitfield: ras::Sync<T>,
  signatures: ras::Sync<T>,
  // cache_size
}

impl<T> Storage<T>
where
  T: SyncMethods,
{
  /// Create a new instance. Takes a keypair and a callback to create new
  /// storage instances.
  // Named `.open()` in the JS version. Replaces the `.openKey()` method too by
  // requiring a key pair to be initialized before creating a new instance.
  pub fn with_storage<Cb>(key_pair: KeyPair, create: Cb) -> Result<Self, Error>
  where
    Cb: Fn(Store) -> ras::Sync<T>,
  {
    let mut instance = Self {
      public_key: key_pair.public_key,
      secret_key: key_pair.secret_key,
      tree: create(Store::Tree),
      data: create(Store::Data),
      bitfield: create(Store::Bitfield),
      signatures: create(Store::Signatures),
    };

    let header = create_bitfield();
    instance.bitfield.write(0, &header.to_vec())?;

    let header = create_signatures();
    instance.signatures.write(0, &header.to_vec())?;

    let header = create_tree();
    instance.tree.write(0, &header.to_vec())?;

    Ok(instance)
  }

  /// Write `Data` to `self.Data`.
  /// TODO: Ensure the signature size is correct.
  /// NOTE: Should we create a `Signature` entry type?
  pub fn put_data(
    &mut self,
    index: usize,
    data: &[u8],
    nodes: &[u8],
  ) -> Result<(), Error> {
    if data.is_empty() {
      return Ok(());
    }

    let (offset, size) = self.data_offset(index, nodes)?;
    ensure!(size == data.len(), "Unexpected size data");
    self.data.write(offset, data)
  }

  /// TODO(yw) docs
  pub fn get_data(&mut self) {
    unimplemented!();
  }

  /// TODO(yw) docs
  pub fn next_signature(&mut self) {
    unimplemented!();
  }

  /// TODO(yw) docs
  pub fn get_signature(&mut self) {
    unimplemented!();
  }

  /// Write a `Signature` to `self.Signatures`.
  /// TODO: Ensure the signature size is correct.
  /// NOTE: Should we create a `Signature` entry type?
  pub fn put_signature(
    &mut self,
    index: usize,
    signature: &[u8],
  ) -> Result<(), Error> {
    self
      .signatures
      .write(32 + 64 * index, signature)
  }

  /// TODO(yw) docs
  /// Get the offset for the data, return `(offset, size)`.
  pub fn data_offset(
    &mut self,
    index: usize,
    cached_nodes: &[u8],
  ) -> Result<(usize, usize), Error> {
    let mut roots = Vec::new();
    flat::full_roots(2 * index, &mut roots);
    let mut offset = 0;
    let pending = roots.len();
    let blk = 2 * index;

    if pending == 0 {
      pending = 1;
      // onnode(null, null)
      return Ok(());
    }

    // for root in roots {
    //   match find_node(cached_nodes, root) {
    //     Some(node) => onnode,
    //   }
    // }
    unimplemented!();
  }

  /// TODO(yw) docs
  pub fn get_node(&mut self) {
    unimplemented!();
  }

  /// TODO(yw) docs
  pub fn put_node(&mut self) {
    unimplemented!();
  }

  /// Write data to the internal bitfield module.
  /// TODO: Ensure the chunk size is correct.
  /// NOTE: Should we create a bitfield entry type?
  pub fn put_bitfield(
    &mut self,
    offset: usize,
    data: &[u8],
  ) -> Result<(), Error> {
    self.bitfield.write(32 + offset, data)
  }

  /// TODO(yw) docs
  pub fn open_key(&mut self) {
    unimplemented!();
  }
}

impl Storage<self::rad::SyncMethods> {
  /// Create a new instance that persists to disk at the location of `dir`.
  // TODO: Ensure that dir is always a directory.
  // NOTE: Should we `mkdirp` here?
  // NOTE: Should we call these `data.bitfield` / `data.tree`?
  pub fn new(key_pair: KeyPair, dir: PathBuf) -> Result<Self, Error> {
    Self::with_storage(key_pair, |storage: Store| {
      let name = match storage {
        Store::Tree => "tree",
        Store::Data => "data",
        Store::Bitfield => "bitfield",
        Store::Signatures => "signatures",
      };
      rad::Sync::new(dir.as_path().join(name))
    })
  }
}

impl Default for Storage<self::ram::SyncMethods> {
  /// Create a new instance with a memory backend and an ephemeral key pair.
  fn default() -> Self {
    let key_pair = KeyPair::default();
    Self::with_storage(key_pair, |_store: Store| ram::Sync::default()).unwrap()
  }
}

// /// Get a node from a vector of nodes.
// // TODO: define type of node
// fn find_node(nodes: Vec<Node>, index: usize) -> Option<Node> {
//   for node in nodes {
//     if node.index == index {
//       return Some(node);
//     }
//   }
//   None
// }
