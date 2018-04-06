/// A merkle proof for an index.
///
/// Merkle trees are proven by checking the parent hashes.
pub struct Proof {
  /// Index of the node this was verified by.
  pub verified_by: usize,

  /// Merkle proof for the index you pass, written in `flat-tree` notation.
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
