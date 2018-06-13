extern crate merkle_tree_stream;

use self::merkle_tree_stream::{
  HashMethods, MerkleTreeStream, Node as NodeTrait, PartialNode,
};
use super::Hash;
use std::rc::Rc;
use storage::Node;

#[derive(Debug)]
struct H;

impl HashMethods for H {
  type Node = Node;
  type Hash = Hash;

  fn leaf(&self, leaf: &PartialNode, _roots: &[Rc<Self::Node>]) -> Self::Hash {
    let data = leaf.as_ref().unwrap(); // TODO: remove the need for unwrap here.
    Hash::from_leaf(&data)
  }

  fn parent(&self, left: &Self::Node, right: &Self::Node) -> Self::Hash {
    Hash::from_hashes(left.hash(), right.hash())
  }

  fn node(&self, partial: &PartialNode, hash: Self::Hash) -> Self::Node {
    let data = match partial.data() {
      Some(data) => Some(data.clone()),
      None => None,
    };

    Node {
      index: partial.index(),
      parent: partial.parent,
      length: partial.len(),
      hash: hash.as_bytes().into(),
      data,
    }
  }
}

/// Merkle Tree Stream
#[derive(Debug)]
pub struct Merkle {
  stream: MerkleTreeStream<H>,
  nodes: Vec<Rc<Node>>,
}

impl Default for Merkle {
  fn default() -> Self {
    Merkle::new()
  }
}

impl Merkle {
  /// Create a new instance.
  // TODO: figure out the right allocation size for `roots` and `nodes`.
  pub fn new() -> Self {
    Self {
      nodes: vec![],
      stream: MerkleTreeStream::new(H, vec![]),
    }
  }

  /// Access the next item.
  // TODO: remove extra conversion alloc.
  pub fn next(&mut self, data: &[u8]) {
    self.stream.next(&data, &mut self.nodes);
  }

  /// Get the roots vector.
  pub fn roots(&self) -> &Vec<Rc<Node>> {
    self.stream.roots()
  }

  /// Get the nodes from the struct.
  pub fn nodes(&self) -> &Vec<Rc<Node>> {
    &self.nodes
  }
}
