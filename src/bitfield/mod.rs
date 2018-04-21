extern crate flat_tree as flat;
extern crate sparse_bitfield as bitfield;

mod masks;

use self::masks::Masks;
pub use bitfield::Change;

pub struct Bitfield {
  data: bitfield::Bitfield,
  tree: bitfield::Bitfield,
  index: bitfield::Bitfield,
  length: usize,
  masks: Masks,
  // iterator: flat::Iterator,
}

impl Bitfield {
  pub fn new() -> Self {
    Self {
      data: bitfield::Bitfield::new(1024),
      tree: bitfield::Bitfield::new(2048),
      index: bitfield::Bitfield::new(256),
      length: 0,
      masks: Masks::new(),
    }
  }

  pub fn set(&mut self, index: usize, value: Option<bool>) -> Change {
    let o = mask_8b(index);
    let index = (index - o) / 8;

    let value = match value {
      Some(value) => self.data.get_byte(index) | 128 >> o,
      None => self.data.get_byte(index) & self.masks.data_update,
    };
    unimplemented!();
  }

  pub fn get(&mut self) {
    unimplemented!();
  }

  pub fn total(&mut self) {
    unimplemented!();
  }

  // Wait with implementing the iterator until the very end.
  pub fn iterator(&mut self, start: usize, end: usize) {
    unimplemented!();
  }

  fn set_index(&mut self) {
    unimplemented!();
  }
}

#[inline]
fn mask_8b(num: usize) -> usize {
  num & 7
}
