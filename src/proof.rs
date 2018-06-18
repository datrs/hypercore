use Node;
use Signature;

/// A merkle proof for an index, created by the `.proof()` method.
#[derive(Debug, PartialEq, Clone)]
pub struct Proof {
  /// The index to which this proof corresponds.
  pub index: usize,
  /// Nodes that verify the index you passed.
  pub nodes: Vec<Node>,
  /// An `ed25519` signature, guaranteeing the integrity of the nodes.
  pub signature: Signature,
}
