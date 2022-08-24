use crate::common::Node;
use crate::crypto::Hash;
use merkle_tree_stream::{HashMethods, MerkleTreeStream, NodeKind, PartialNode};
use std::sync::Arc;

#[derive(Debug)]
struct H;

impl HashMethods for H {
    type Node = Node;
    type Hash = Hash;

    fn leaf(&self, leaf: &PartialNode, _roots: &[Arc<Self::Node>]) -> Self::Hash {
        match leaf.data() {
            NodeKind::Leaf(data) => Hash::from_leaf(&data),
            NodeKind::Parent => unreachable!(),
        }
    }

    fn parent(&self, left: &Self::Node, right: &Self::Node) -> Self::Hash {
        Hash::from_hashes(left, right)
    }
}

/// Merkle Tree Stream
#[derive(Debug)]
pub struct Merkle {
    stream: MerkleTreeStream<H>,
    nodes: Vec<Arc<Node>>,
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

    pub fn from_nodes(nodes: Vec<Node>) -> Self {
        let nodes = nodes.into_iter().map(Arc::new).collect::<Vec<_>>();
        Self {
            stream: MerkleTreeStream::new(H, nodes.clone()),
            nodes,
        }
    }

    /// Access the next item.
    // TODO: remove extra conversion alloc.
    pub fn next(&mut self, data: &[u8]) {
        self.stream.next(&data, &mut self.nodes);
    }

    /// Get the roots vector.
    pub fn roots(&self) -> &Vec<Arc<Node>> {
        self.stream.roots()
    }

    /// Get the nodes from the struct.
    pub fn nodes(&self) -> &Vec<Arc<Node>> {
        &self.nodes
    }
}
