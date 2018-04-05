//! Stateful tree index. Or well, stateful flat-tree. It's what happens when you
//! combine a flat-tree with a sparse-bitfield - which ends up being pretty
//! cool!

extern crate flat_tree as flat;
extern crate sparse_bitfield as bitfield;

mod proof;

pub use self::bitfield::Bitfield;

/// Index a tree structure or something.
pub struct TreeIndex {
  bitfield: Bitfield,
}

impl TreeIndex {
  /// Create a new TreeIndex by passing it a sparse_bitfield instance.
  pub fn new(bitfield: Bitfield) -> Self {
    TreeIndex { bitfield }
  }

  /// Prove... something?
  ///
  /// TODO: Ask mafintosh what a good description for this would be.
  pub fn proof(&self) -> proof::Proof {
    unimplemented!();
  }

  /// Create a digest for data at index.
  pub fn digest(&self) {
    unimplemented!();
  }

  /// Get the amount of... blocks?
  pub fn blocks(&self) {
    unimplemented!();
  }

  /// Get all root nodes.
  pub fn roots(&self) {
    unimplemented!();
  }

  /// Find the node that verified the node that's passed.
  pub fn verified_by(&self) {
    unimplemented!();
  }

  /// Set a bit on the bitfield.
  ///
  /// NOTE: maybe we should turn this into the Deref trait / accessors? Keep the
  /// API the same as with bitfield.
  pub fn get(&self) {
    unimplemented!();
  }

  /// Set a bit on the bitfield.
  pub fn set(&self) {
    unimplemented!();
  }
}

/// Create a TreeIndex with an empty sparse_bitfield instance with a page size
/// of `1024`.
impl Default for TreeIndex {
  fn default() -> Self {
    TreeIndex {
      bitfield: Bitfield::new(1024),
    }
  }
}

/// Shift a number to the right.
#[inline]
fn shift_right(n: usize) -> usize {
  (n - (n & 1)) / 2
}

/// Do stuff with full roots.
fn add_full_roots() {
  unimplemented!();
}
