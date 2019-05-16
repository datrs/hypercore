use crate::crypto::Hash;
use crate::storage::Node;
use merkle_tree_stream::{
  HashMethods, MerkleTreeStream, NodeKind, PartialNode,
};
use std::rc::Rc;

#[derive(Debug)]
struct Hasher;

impl HashMethods for Hasher {
  type Node = Node;
  type Hash = Hash;

  fn leaf(&self, leaf: &PartialNode, _roots: &[Rc<Self::Node>]) -> Self::Hash {
    match leaf.data() {
      NodeKind::Leaf(data) => Hash::from_leaf(&data),
      NodeKind::Parent => unreachable!(),
    }
  }

  fn parent(&self, left: &Self::Node, right: &Self::Node) -> Self::Hash {
    Hash::from_hashes(left, right)
  }

  fn node(&self, partial: &PartialNode, hash: Self::Hash) -> Self::Node {
    let data = match partial.data() {
      NodeKind::Leaf(data) => Some(data.clone()),
      NodeKind::Parent => None,
    };

    Node {
      index: partial.index(),
      parent: partial.parent,
      length: partial.len(),
      hash: hash,
      data,
    }
  }
}

/// Merkle Tree Stream
#[derive(Debug)]
pub struct Merkle {
  stream: MerkleTreeStream<Hasher>,
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
      stream: MerkleTreeStream::new(Hasher, vec![]),
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
