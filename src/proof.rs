use crate::Node;
use crate::Signature;

/// A merkle proof for an index, created by the `.proof()` method.
#[derive(Debug, PartialEq, Clone)]
pub struct Proof {
    /// The index to which this proof corresponds.
    pub index: u64,
    /// Nodes that verify the index you passed.
    pub nodes: Vec<Node>,
    /// An `ed25519` signature, guaranteeing the integrity of the nodes.
    pub signature: Option<Signature>,
}

impl Proof {
    /// Access the `index` field from the proof.
    pub fn index(&self) -> u64 {
        self.index
    }

    /// Access the `nodes` field from the proof.
    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    /// Access the `signature` field from the proof.
    pub fn signature(&self) -> Option<&Signature> {
        self.signature.as_ref()
    }
}
