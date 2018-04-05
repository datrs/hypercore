/// A link to which node checks another node.
pub struct Proof {
  /// Index of the node this was verified by.
  pub verified_by: usize,

  /// Nodes that are verified.
  pub nodes: Vec<usize>,
}

impl Proof {
  /// Create a new [`Proof`] instance.
  pub fn new(verified_by: usize, nodes: Vec<usize>) -> Self {
    Proof {
      nodes,
      verified_by,
    }
  }
}
