#![doc(include = "./tree_index/README.md")]

extern crate flat_tree as flat;
extern crate sparse_bitfield as bitfield;

mod proof;

pub use self::bitfield::{Bitfield, Change};
pub use self::proof::Proof;

/// Index a tree structure or something.
pub struct TreeIndex {
  bitfield: Bitfield,
}

impl TreeIndex {
  /// Create a new TreeIndex by passing it a sparse_bitfield instance.
  pub fn new(bitfield: Bitfield) -> Self {
    TreeIndex { bitfield }
  }

  /// Get a bit from the bitfield.
  pub fn get(&mut self, index: usize) -> bool {
    self.bitfield.get(index)
  }

  /// Set an index on the tree to `true`, and also all of the parents to the
  /// index. Walks the tree upward.
  ///
  /// Returns a "Change" member to indicate if the underlying value was changed.
  ///
  /// NOTE: we can probably change the bitfield.set syntax to return false to
  /// simplify this code a little.
  pub fn set(&mut self, index: usize) -> Change {
    if let Change::Unchanged = self.bitfield.set(index, true) {
      return Change::Unchanged;
    }

    let mut index = index;
    while self.bitfield.get(flat::sibling(index)) {
      index = flat::parent(index);
      if let Change::Unchanged = self.bitfield.set(index, true) {
        break;
      }
    }
    Change::Changed
  }

  /// Prove... something?
  ///
  /// TODO: Ask mafintosh what a good description for this would be.
  pub fn proof(&self) -> Proof {
    let _nodes: Vec<usize> = Vec::new();
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

// /// Shift a number to the right.
// #[inline]
// fn shift_right(n: usize) -> usize {
//   (n - (n & 1)) / 2
// }

// /// Do stuff with full roots.
// fn add_full_roots() {
//   unimplemented!();
// }

#[test]
fn can_create_new() {
  extern crate flat_tree as flat;
  extern crate sparse_bitfield as bitfield;

  pub use self::bitfield::Bitfield;

  let bitfield = Bitfield::new(1024);
  let _tree = TreeIndex::new(bitfield);
}

#[test]
fn can_set() {
  extern crate flat_tree as flat;
  extern crate sparse_bitfield as bitfield;

  pub use self::bitfield::Bitfield;

  let bitfield = Bitfield::new(1024);
  let mut tree = TreeIndex::new(bitfield);
  assert_eq!(tree.set(1), Change::Changed);
  assert_eq!(tree.set(1), Change::Unchanged);
  assert_eq!(tree.set(0), Change::Changed);
  assert_eq!(tree.set(0), Change::Unchanged);
}

#[test]
fn can_get() {
  extern crate flat_tree as flat;
  extern crate sparse_bitfield as bitfield;

  pub use self::bitfield::Bitfield;

  let bitfield = Bitfield::new(1024);
  let mut tree = TreeIndex::new(bitfield);
  tree.set(0);
  assert_eq!(tree.get(0), true);
  assert_eq!(tree.get(1), false);
}
