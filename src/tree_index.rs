extern crate flat_tree as flat;
extern crate sparse_bitfield as bitfield;

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

  pub fn proof(&self) {
    unimplemented!();
  }

  pub fn digest(&self) {
    unimplemented!();
  }

  pub fn blocks(&self) {
    unimplemented!();
  }

  pub fn roots(&self) {
    unimplemented!();
  }

  pub fn verified_by(&self) {
    unimplemented!();
  }

  pub fn get(&self) {
    unimplemented!();
  }

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
