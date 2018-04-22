//! Bitfield module. Exposes `{data, tree, index}` internally. Serializable to
//! disk.
//!
//! TODO(yw): Document the magic mask format. (Will help to look at binary
//! versions of the numbers).
//!
//! TODO(yw): Document the use cases for this module, especially when opposed to
//! `sparse_bitfield`.
//!
//! NOTE(yw): in the JavaScript version, this code uses a single pager under the
//! hood. Because of Rust's borrow rules, that would be tricky to pull off for
//! us. So instead we've chosen to create three separate instances, with three
//! separate pagers powering it.
//! This means that when serializing to disk, we need to weave the contents of
//! all three of the pagers into a single instance. And when restoring it from
//! disk, we must do so again.
//! We need to make sure the performance impact of this stays well within
//! bounds.

extern crate flat_tree as flat;
extern crate sparse_bitfield as bitfield;

mod masks;

pub use self::bitfield::Change;
use self::masks::Masks;

/// Bitfield with `{data, tree, index} fields.`
pub struct Bitfield {
  data: bitfield::Bitfield,
  tree: bitfield::Bitfield,
  index: bitfield::Bitfield,
  page_len: usize,
  length: usize,
  masks: Masks,
  // iterator: flat::Iterator,
}

impl Bitfield {
  /// Create a new instance.
  pub fn new() -> Self {
    Self {
      data: bitfield::Bitfield::new(1024),
      tree: bitfield::Bitfield::new(2048),
      index: bitfield::Bitfield::new(256),
      page_len: 3328,
      length: 0,
      masks: Masks::new(),
    }
  }

  /// Set a value at an index.
  pub fn set(&mut self, index: usize, value: Option<bool>) -> Change {
    let o = mask_8b(index);
    let index = (index - o) / 8;

    let value = match value {
      Some(value) => self.data.get_byte(index) | 128 >> o,
      None => self.data.get_byte(index) & self.masks.data_update[o],
    };

    if let Change::Unchanged = self.data.set_byte(index, value) {
      return Change::Unchanged;
    }

    self.length = self.data.len();
    self.set_index(index, value);
    Change::Changed
  }

  /// Get a value at a position in the bitfield.
  pub fn get(&mut self, index: usize) -> bool {
    self.data.get(index)
  }

  /// Calculate the total of ... TODO(yw)
  pub fn total(&mut self, start: usize, end: usize) -> u8 {
    if end < start {
      return 0;
    }

    // FIXME
    // if end > this.data.len() {
    //   this.expand(end);
    // }

    let o = mask_8b(start);
    let e = mask_8b(end);

    let pos = (start - o) / 8;
    let last = (end - e) / 8;

    let left_mask = if o == 0 {
      255
    } else {
      255 - self.masks.data_iterate[o - 1]
    };

    let right_mask = if o == 0 {
      0
    } else {
      self.masks.data_iterate[e - 1]
    };

    let byte = self.data.get_byte(pos);
    if pos == last {
      let index = (byte & left_mask & right_mask) as usize;
      return self.masks.total_1_bits[index];
    }
    let index = (byte & left_mask) as usize;
    let mut total = self.masks.total_1_bits[index];

    for i in pos + 1..last {
      let index = self.data.get_byte(i) as usize;
      total += self.masks.total_1_bits[index];
    }

    let index: usize = self.data.get_byte(last) as usize & right_mask as usize;
    total + self.masks.total_1_bits[index]
  }

  /// Create an iterator that iterates over the bitfield.
  // Wait with implementing the iterator until the very end.
  pub fn iterator(&mut self, start: usize, end: usize) {
    unimplemented!();
  }

  /// Set a value at index.
  ///
  ///```txt
  ///                    (a + b | c + d | e + f | g + h)
  /// -> (a | b | c | d)                                (e | f | g | h)
  ///```
  ///
  /// NOTE(yw): lots of magic values going on; I have no idea what we're doing
  /// here.
  fn set_index(&mut self, index: usize, value: u8) -> Change {
    let o = index & 3;
    index = (index - o) / 4;

    let bitfield = self.index;
    let ite = self.iterator;
    let masks = self.masks;
    let start = 2 * index;

    let left = bitfield.get_byte(start) & self.masks.index_update[o];
    let right = get_index_value(value) >> (2 * o);
    let mut byte = left | right;
    let len = bitfield.len();
    let max_len = self.page_len * 256;

    ite.seek(start);

    while ite.index < max_len
      && bitfield.set_byte(ite.index, byte) == Change::Changed
    {
      if ite.is_left() {
        let index: usize = bitfield.get_byte(ite.sibling()).into();
        byte =
          masks.map_parent_left[byte as usize] | masks.map_parent_right[index];
      } else {
        let index: usize = bitfield.get_byte(ite.sibling()).into();
        byte =
          masks.map_parent_right[index] | masks.map_parent_left[byte as usize];
      }
      ite.parent();
    }

    // FIXME
    // if len != bitfield.len() {
    //   self.expand(len);
    // }

    if ite.index == start {
      Change::Changed
    } else {
      Change::Unchanged
    }
  }
}

#[inline]
fn get_index_value(index: u8) -> u8 {
  match index {
    255 => 192,
    0 => 0,
    _ => 64,
  }
}

#[inline]
fn mask_8b(num: usize) -> usize {
  num & 7
}
