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

// fn set_and_get_tree() {
//   let b = Bitfield::new();
//   let tree = b.tree

//   assert_eq!(tree.get(0), false)
//   assert_eq!(tree.set(0, true), true)
//   assert_eq!(tree.set(0, true), false)
//   assert_eq!(tree.get(0), true)

//   assert_eq!(tree.get(1424244), false)
//   assert_eq!(tree.set(1424244, true), true)
//   assert_eq!(tree.set(1424244, true), false)
//   assert_eq!(tree.get(1424244), true)

//   assert_eq!(b.get(0), false)
//   assert_eq!(b.get(1424244), false)

// }

// fn set_and_get_index() {
//   let b = Bitfield::new();
//   let ite = b.iterator(0, 100000000)
//   let i = 0

//   assert_eq!(ite.next(), 0)

//   b.set(0, true)
//   assert_eq!(ite.seek(0).next(), 1)

//   b.set(479, true)
//   assert_eq!(ite.seek(478).next(), 478)
//   assert_eq!(ite.next(), 480)

//   b.set(1, true)
//   assert_eq!(ite.seek(0).next(), 2)

//   b.set(2, true)
//   assert_eq!(ite.seek(0).next(), 3)

//   b.set(3, true)
//   assert_eq!(ite.seek(0).next(), 4)

//   for (i = 0; i < b.length; i++) {
//     b.set(i, true)
//   }

//   assert_eq!(ite.seek(0).next(), b.length)

//   for (i = 0; i < b.length; i++) {
//     b.set(i, false)
//   }

//   assert_eq!(ite.seek(0).next(), 0)

// }

// fn set_and_get_index_random () {
//   let b = Bitfield::new();

//   for (let i = 0; i < 100; i++) {
//     t.ok(check(), 'index validates')
//     set(Math.round(Math.random() * 2000), Math.round(Math.random() * 8))
//   }

//   t.ok(check(), 'index validates')

//   function check () {
//     let all = []
//     let ite = b.iterator()
//     let i = 0

//     for (i = 0; i < b.length; i++) {
//       all[i] = true
//     }

//     i = ite.next()

//     while (i > -1) {
//       all[i] = false
//       i = ite.next()
//     }

//     for (i = 0; i < b.length; i++) {
//       if (b.get(i) !== all[i]) {
//         return false
//       }
//     }

//     return true
//   }

//   function set (i, n) {
//     while (n--) {
//       b.set(i++, true)
//     }
//   }
// }
