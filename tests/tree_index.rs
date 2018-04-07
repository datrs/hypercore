extern crate flat_tree as flat;
extern crate hypercore;
extern crate sparse_bitfield as bitfield;

use bitfield::Bitfield;
use hypercore::tree_index::{Change, TreeIndex};

#[test]
fn can_create_new() {
  let bitfield = Bitfield::new(1024);
  let _tree = TreeIndex::new(bitfield);
}

#[test]
fn can_set() {
  let bitfield = Bitfield::new(1024);
  let mut tree = TreeIndex::new(bitfield);
  assert_eq!(tree.set(1), Change::Changed);
  assert_eq!(tree.set(1), Change::Unchanged);
  assert_eq!(tree.set(0), Change::Changed);
  assert_eq!(tree.set(0), Change::Unchanged);
}

#[test]
fn can_get() {
  let bitfield = Bitfield::new(1024);
  let mut tree = TreeIndex::new(bitfield);
  tree.set(0);
  assert_eq!(tree.get(0), true);
  assert_eq!(tree.get(1), false);
}

mod blocks {
  use super::*;
  #[test]
  fn can_index_blocks() {
    let bitfield = Bitfield::new(1024);
    let mut tree = TreeIndex::new(bitfield);

    tree.set(0);
    assert_eq!(tree.blocks(), 1);
    tree.set(3);
    assert_eq!(tree.blocks(), 4);
  }
}

mod proof {
  use super::*;
  #[test]
  fn returns_none_for_out_of_bounds() {
    let bitfield = Bitfield::new(1024);
    let mut tree = TreeIndex::new(bitfield);
    assert!(tree.proof(0).is_none());
  }
}
