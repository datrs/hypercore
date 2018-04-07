#![doc(include = "./tree_index/README.md")]

// https://github.com/mafintosh/hypercore/blob/master/lib/tree-index.js

extern crate flat_tree as flat;
extern crate sparse_bitfield as bitfield;

mod proof;

pub use self::bitfield::{Bitfield, Change};
pub use self::proof::Proof;

/// Returned by `.verified_by()`.
pub struct Verification {
  /// Node that verifies the index.
  pub node: usize,
  /// The highest Node found.
  pub top: usize,
}

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

  /// Determine which Nodes prove the correctness for the Node at `index`.
  // - opts.hash: always push index to nodes.
  // - nodes: proven nodes.
  // - opts.digest: not sure what digest does.
  pub fn proof(&mut self, index: usize) -> Option<Proof> {
    let _nodes: Vec<usize> = Vec::new();
    if !self.get(index) {
      return None;
    }
    None
    // unimplemented!();
  }

  /// Create a digest for data at index.
  pub fn digest(&self) {
    unimplemented!();
  }

  /// Get the position of the highest entry in the tree. Aka max.
  ///
  /// NOTE: should we rename this to `.len()` ?
  pub fn blocks(&mut self) -> usize {
    let mut top = 0;
    let mut next = 0;
    let max = self.bitfield.len();

    while flat::right_span(next) < max {
      next = flat::parent(next);
      if self.get(next) {
        top = next;
      }
    }

    if self.get(top) {
      self.verified_by(top).node / 2
    } else {
      0
    }
  }

  /// Get all root nodes.
  ///
  /// TODO: don't make this allocate, but fill a vector instead.
  pub fn roots(&mut self) -> Vec<usize> {
    flat::full_roots(2 * self.blocks())
  }

  /// Find the node that verified the node that's passed.
  ///
  /// This is different from the Javascript implementation in that it doesn't
  /// push the `top` value into an array, but returns it instead through the
  /// `Verification` type.
  pub fn verified_by(&mut self, index: usize) -> Verification {
    let has_index = self.get(index);
    if !has_index {
      return Verification {
        node: 0,
        top: 0,
      };
    }

    // Find root of current tree.
    let mut depth = flat::depth(index);
    let mut top = index;
    let mut parent = flat::parent_with_depth(top, depth);
    depth += 1;
    while self.get(parent) && self.get(flat::sibling(top)) {
      top = parent;
      parent = flat::parent_with_depth(top, depth);
      depth += 1;
    }

    // Expand right down.
    //
    // NOTE: this is probably a candidate to move to `flat-tree`.
    depth -= 1;
    while depth != 0 {
      top = flat::left_child_with_depth(
        flat::index(depth, flat::offset_with_depth(top, depth) + 1),
        depth,
      ).unwrap();
      depth -= 1;

      while !self.get(top) && depth > 0 {
        top = flat::left_child_with_depth(top, depth).unwrap();
        depth -= 1;
      }
    }

    let node = if self.get(top) {
      top + 2
    } else {
      top
    };

    Verification { node, top }
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
