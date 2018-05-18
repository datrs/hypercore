extern crate merkle_tree_stream;

use self::merkle_tree_stream::{
  HashMethods, MerkleTreeStream, Node as NodeTrait, PartialNode,
};
use super::Hash;
use std::rc::Rc;
use storage::Node;

#[derive(Debug)]
struct S;

impl HashMethods<Node> for S {
  // FIXME: remove double (triple?) allocation here.
  fn leaf(&self, leaf: &PartialNode, _roots: &[Rc<Node>]) -> Vec<u8> {
    let data = leaf.as_ref().unwrap();
    Hash::from_leaf(&data).as_bytes().to_vec()
  }

  fn parent(&self, a: &Node, b: &Node) -> Vec<u8> {
    let hash = Hash::from_hashes(a.hash(), b.hash());
    hash.as_bytes().to_vec()
  }

  fn node(&self, partial: &PartialNode, hash: Vec<u8>) -> Node {
    unimplemented!();
  }
}

/// Merkle Tree Stream
#[derive(Debug)]
pub struct Merkle {
  stream: MerkleTreeStream<S, Node>,
  nodes: Vec<Rc<Node>>,
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
  // TODO: remove extra conversion alloc.

  // NOTE: Convert from the Merkle nodes into our own node type. Ideally we
  // could pass our own node type to the Merkle module.
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
