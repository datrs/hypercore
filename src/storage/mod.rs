//! Save data to a desired storage backend.

extern crate ed25519_dalek;
extern crate flat_tree as flat;
extern crate merkle_tree_stream as merkle_stream;
extern crate random_access_disk as rad;
extern crate random_access_memory as ram;
extern crate random_access_storage as ras;
extern crate sleep_parser;

mod node;
mod persist;

pub use self::merkle_stream::Node as NodeTrait;
pub use self::node::Node;
pub use self::persist::Persist;

use self::ed25519_dalek::Signature;
use self::ras::RandomAccessMethods;
use self::sleep_parser::*;
use std::borrow::Borrow;
use std::fmt::Debug;
use std::ops::Range;
use Result;

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
#[derive(Debug)]
pub struct Storage<T>
where
  T: RandomAccessMethods + Debug,
{
  tree: ras::RandomAccess<T>,
  data: ras::RandomAccess<T>,
  bitfield: ras::RandomAccess<T>,
  signatures: ras::RandomAccess<T>,
}

impl<T> Storage<T>
where
  T: RandomAccessMethods + Debug,
{
  /// Create a new instance. Takes a keypair and a callback to create new
  /// storage instances.
  // Named `.open()` in the JS version. Replaces the `.openKey()` method too by
  // requiring a key pair to be initialized before creating a new instance.
  pub fn new<Cb>(create: Cb) -> Result<Self>
  where
    Cb: Fn(Store) -> ras::RandomAccess<T>,
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

  /// Write data to the feed.
  #[inline]
  pub fn write_data(&mut self, offset: usize, data: &[u8]) -> Result<()> {
    self.data.write(offset, &data)
  }

  /// Write a byte vector to a data storage (random-access instance) at the
  /// position of `index`.
  ///
  /// NOTE: Meant to be called from the `.put()` feed method. Probably used to
  /// insert data as-is after receiving it from the network (need to confirm
  /// with mafintosh).
  /// TODO: Ensure the signature size is correct.
  /// NOTE: Should we create a `Data` entry type?
  pub fn put_data(
    &mut self,
    index: usize,
    data: &[u8],
    nodes: &[Node],
  ) -> Result<()> {
    if data.is_empty() {
      return Ok(());
    }

    let range = self.data_offset(index, nodes)?;

    ensure!(
      range.len() == data.len(),
      format!("length  `{:?} != {:?}`", range.len(), data.len())
    );

    self.data.write(range.start, data)
  }

  /// Get data from disk that the user has written to it. This is stored
  /// unencrypted, so there's no decryption needed.
  // FIXME: data_offset always reads out index 0, length 0
  #[inline]
  pub fn get_data(&mut self, index: usize) -> Result<Vec<u8>> {
    let cached_nodes = Vec::new(); // FIXME: reuse allocation.
    let range = self.data_offset(index, &cached_nodes)?;
    self.data.read(range.start, range.len())
  }

  /// Search the signature stores for a `Signature`, starting at `index`.
  pub fn next_signature(&mut self, index: usize) -> Result<Signature> {
    let bytes = self.signatures.read(HEADER_OFFSET + 64 * index, 64)?;
    if not_zeroes(&bytes) {
      Ok(Signature::from_bytes(&bytes)?)
    } else {
      Ok(self.next_signature(index + 1)?)
    }
  }

  /// Get a `Signature` from the store.
  #[inline]
  pub fn get_signature(&mut self, index: usize) -> Result<Signature> {
    let bytes = self.signatures.read(HEADER_OFFSET + 64 * index, 64)?;
    ensure!(not_zeroes(&bytes), "No signature found");
    Ok(Signature::from_bytes(&bytes)?)
  }

  /// Write a `Signature` to `self.Signatures`.
  /// TODO: Ensure the signature size is correct.
  /// NOTE: Should we create a `Signature` entry type?
  #[inline]
  pub fn put_signature(
    &mut self,
    index: usize,
    signature: impl Borrow<Signature>,
  ) -> Result<()> {
    let signature = signature.borrow();
    self
      .signatures
      .write(HEADER_OFFSET + 64 * index, &signature.to_bytes())
  }

  /// TODO(yw) docs
  /// Get the offset for the data, return `(offset, size)`.
  pub fn data_offset(
    &mut self,
    index: usize,
    cached_nodes: &[Node],
  ) -> Result<Range<usize>> {
    let mut roots = Vec::new(); // FIXME: reuse alloc
    flat::full_roots(2 * index, &mut roots);

    let mut offset = 0;
    let mut pending = roots.len();
    let block_index = 2 * index;

    if pending == 0 {
      let len = match find_node(&cached_nodes, block_index) {
        Some(node) => node.len(),
        None => (self.get_node(block_index)?).len(),
      };
      return Ok(offset..offset + len);
    }

    for root in roots {
      // FIXME: we're always having a cache miss here. Check cache first before
      // getting a node from the backend.
      //
      // ```rust
      // let node = match find_node(cached_nodes, root) {
      //   Some(node) => node,
      //   None => self.get_node(root),
      // };
      // ```
      let node = self.get_node(root)?;

      offset += node.len();
      pending -= 1;
      if pending > 0 {
        continue;
      }

      let len = match find_node(&cached_nodes, block_index) {
        Some(node) => node.len(),
        None => (self.get_node(block_index)?).len(),
      };

      return Ok(offset..offset + len);
    }

    panic!("Loop executed without finding max value");
  }

  /// Get a `Node` from the `tree` storage.
  #[inline]
  pub fn get_node(&mut self, index: usize) -> Result<Node> {
    let buf = self.tree.read(HEADER_OFFSET + 40 * index, 40)?;
    let node = Node::from_bytes(index, &buf)?;
    Ok(node)
  }

  /// TODO(yw) docs
  /// Write a `Node` to the `tree` storage.
  /// TODO: prevent extra allocs here. Implement a method on node that can reuse
  /// a buffer.
  #[inline]
  pub fn put_node(&mut self, node: &Node) -> Result<()> {
    let index = node.index();
    let buf = node.to_bytes()?;
    self.tree.write(HEADER_OFFSET + 40 * index, &buf)
  }

  /// Write data to the internal bitfield module.
  /// TODO: Ensure the chunk size is correct.
  /// NOTE: Should we create a bitfield entry type?
  #[inline]
  pub fn put_bitfield(&mut self, offset: usize, data: &[u8]) -> Result<()> {
    self.bitfield.write(HEADER_OFFSET + offset, data)
  }

  /// TODO(yw) docs
  pub fn open_key(&mut self) {
    unimplemented!();
  }
}

/// Get a node from a vector of nodes.
#[inline]
fn find_node(nodes: &[Node], index: usize) -> Option<&Node> {
  for node in nodes {
    if node.index() == index {
      return Some(node);
    }
  }
  None
}

/// Check if a byte slice is not completely zero-filled.
#[inline]
fn not_zeroes(bytes: &[u8]) -> bool {
  for byte in bytes {
    if *byte != 0 {
      return true;
    }
  }
  false
}

#[test]
fn should_detect_zeroes() {
  let nums = vec![0; 10];
  assert!(!not_zeroes(&nums));

  let nums = vec![1; 10];
  assert!(not_zeroes(&nums));
}
