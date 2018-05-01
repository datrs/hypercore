//! Save data to a desired storage backend.

extern crate failure;
extern crate flat_tree as flat;
extern crate random_access_disk as rad;
extern crate random_access_memory as ram;
extern crate random_access_storage as ras;
extern crate sleep_parser;

mod data;
mod node;
mod persist;

pub use self::data::Data;
pub use self::node::Node;
pub use self::persist::Persist;

use self::failure::Error;
use self::ras::SyncMethods;
use self::sleep_parser::*;

const HEADER_OFFSET: usize = 32;

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
  tree: ras::Sync<T>,
  data: ras::Sync<T>,
  bitfield: ras::Sync<T>,
  signatures: ras::Sync<T>,
}

impl<T> Storage<T>
where
  T: SyncMethods,
{
  /// Create a new instance. Takes a keypair and a callback to create new
  /// storage instances.
  // Named `.open()` in the JS version. Replaces the `.openKey()` method too by
  // requiring a key pair to be initialized before creating a new instance.
  pub fn new<Cb>(create: Cb) -> Result<Self, Error>
  where
    Cb: Fn(Store) -> ras::Sync<T>,
  {
    let mut instance = Self {
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
  /// NOTE: Should we create a `Data` entry type?
  pub fn put_data(
    &mut self,
    index: usize,
    data: &[u8],
    nodes: &[Node],
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
      .write(HEADER_OFFSET + 64 * index, signature)
  }

  /// TODO(yw) docs
  /// Get the offset for the data, return `(offset, size)`.
  pub fn data_offset(
    &mut self,
    index: usize,
    cached_nodes: &[Node],
  ) -> Result<(usize, usize), Error> {
    let mut roots = Vec::new(); // FIXME: reuse alloc
    flat::full_roots(2 * index, &mut roots);
    let mut offset = 0;
    let mut pending = roots.len();
    let blk = 2 * index;

    if pending == 0 {
      pending = 1;
      // onnode(null, null)
      return Ok((0, 0)); // TODO: fixme
    }

    // for root in roots {
    //   match find_node(cached_nodes, root) {
    //     Some(node) => onnode,
    //   }
    // }
    unimplemented!();
  }

  /// Get a `Node` from the `tree` storage.
  pub fn get_node(&mut self, index: usize) -> Result<Node, Error> {
    let buf = self.tree.read(HEADER_OFFSET + 40 * index, 40)?;
    Node::from_vec(index, &buf)
  }

  /// TODO(yw) docs
  /// Write a `Node` to the `tree` storage.
  /// TODO: prevent extra allocs here. Implement a method on node that can reuse
  /// a buffer.
  pub fn put_node(&mut self, node: &mut Node) -> Result<(), Error> {
    let index = node.index();
    let buf = node.to_vec()?;
    self
      .tree
      .write(HEADER_OFFSET + 40 * index, &buf)
  }

  /// Write data to the internal bitfield module.
  /// TODO: Ensure the chunk size is correct.
  /// NOTE: Should we create a bitfield entry type?
  pub fn put_bitfield(
    &mut self,
    offset: usize,
    data: &[u8],
  ) -> Result<(), Error> {
    self
      .bitfield
      .write(HEADER_OFFSET + offset, data)
  }

  /// TODO(yw) docs
  pub fn open_key(&mut self) {
    unimplemented!();
  }
}

/// Get a node from a vector of nodes.
// TODO: define type of node
fn find_node(nodes: Vec<Node>, index: usize) -> Option<Node> {
  for node in nodes {
    if node.index() == index {
      return Some(node);
    }
  }
  None
}
