extern crate merkle_tree_stream;

use self::merkle_tree_stream::{HashMethods, MerkleTreeStream, Node,
                               NodeVector, PartialNode};
use super::Hash;
use std::rc::Rc;

#[derive(Debug)]
struct S;

impl HashMethods for S {
  // FIXME: remove double (triple?) allocation here.
  fn leaf(&self, leaf: &PartialNode, _roots: &[Rc<Node>]) -> Vec<u8> {
    let data = leaf.as_ref().unwrap();
    Hash::from_leaf(&data).as_bytes().to_vec()
  }

  fn parent(&self, a: &Node, b: &Node) -> Vec<u8> {
    let hash = Hash::from_parent(a.hash(), b.hash());
    hash.as_bytes().to_vec()
  }
}

/// Merkle Tree Stream
#[derive(Debug)]
pub struct Merkle {
  stream: MerkleTreeStream<S>,
  nodes: NodeVector,
}

impl Merkle {
  /// Create a new instance.
  // TODO: figure out the right allocation size for `roots` and `nodes`.
  pub fn new() -> Self {
    let roots = Vec::new();

    Self {
      nodes: Vec::new(),
      stream: MerkleTreeStream::new(S, roots),
    }
  }

  /// Access the next item.
  pub fn next(&mut self, data: &[u8]) -> &NodeVector {
    self.stream.next(&data, &mut self.nodes);
    self.nodes()
  }

  /// Get the nodes from the struct.
  #[inline]
  pub fn nodes(&self) -> &NodeVector {
    &self.nodes
  }
}
