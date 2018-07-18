extern crate hypercore;

use hypercore::bitfield::{Bitfield, Change::*};

#[test]
fn set_and_get() {
  let mut b = Bitfield::new();

  assert_eq!(b.get(0), false);
  assert_eq!(b.set(0, true), Changed);
  assert_eq!(b.set(0, true), Unchanged);
  assert_eq!(b.get(0), true);

  assert_eq!(b.get(1424244), false);
  assert_eq!(b.set(1424244, true), Changed);
  assert_eq!(b.set(1424244, true), Unchanged);
  assert_eq!(b.get(1424244), true);
}

#[test]
fn get_total_positive_bits() {
  let mut b = Bitfield::new();

  assert_eq!(b.set(1, true), Changed);
  assert_eq!(b.set(2, true), Changed);
  assert_eq!(b.set(4, true), Changed);
  assert_eq!(b.set(5, true), Changed);
  assert_eq!(b.set(39, true), Changed);

  assert_eq!(b.total_with_range(0..4), 2);
  assert_eq!(b.total_with_range(3..4), 0);
  assert_eq!(b.total_with_range(3..5), 1);
  assert_eq!(b.total_with_range(3..40), 3);
  assert_eq!(b.total(), 5);
  assert_eq!(b.total_with_start(7), 1);
}
