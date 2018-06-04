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
extern crate sparse_bitfield;

mod masks;

use self::masks::Masks;
pub use self::sparse_bitfield::Change;

/// Bitfield with `{data, tree, index} fields.`
// #[derive(Debug)]
pub struct Bitfield {
  data: sparse_bitfield::Bitfield,
  /// FIXME: SLEEP protocol tree field.
  pub tree: sparse_bitfield::Bitfield,
  index: sparse_bitfield::Bitfield,
  page_len: usize,
  length: usize,
  masks: Masks,
  iterator: flat::Iterator,
}

impl Default for Bitfield {
  fn default() -> Self {
    Bitfield::new()
  }
}

impl Bitfield {
  /// Create a new instance.
  pub fn new() -> Self {
    Self {
      data: sparse_bitfield::Bitfield::new(1024),
      tree: sparse_bitfield::Bitfield::new(2048),
      index: sparse_bitfield::Bitfield::new(256),
      page_len: 3328,
      length: 0,
      masks: Masks::new(),
      iterator: flat::Iterator::new(0),
    }
  }

  /// Set a value at an index.
  pub fn set(&mut self, index: usize, value: bool) -> Change {
    let o = mask_8b(index);
    let index = (index - o) / 8;

    let value = if value {
      self.data.get_byte(index) | 128 >> o
    } else {
      self.data.get_byte(index) & self.masks.data_update[o]
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

  /// Calculate the total for the whole data.
  pub fn total(&mut self) -> u8 {
    let len = self.data.len();
    self.total_with_range(0, len)
  }

  /// Calculate the total from the start value.
  pub fn total_with_start(&mut self, start: usize) -> u8 {
    let len = self.data.len();
    self.total_with_range(start, len)
  }

  /// Calculate the total up until the end value.
  pub fn total_with_end(&mut self, end: usize) -> u8 {
    self.total_with_range(0, end)
  }

  /// Calculate the total of ... TODO(yw)
  pub fn total_with_range(&mut self, start: usize, end: usize) -> u8 {
    if end < start {
      return 0;
    }

    if end > self.data.len() {
      self.expand(end);
    }

    let o = mask_8b(start);
    let e = mask_8b(end);

    let pos = (start - o) / 8;
    let last = (end - e) / 8;

    let left_mask = if o == 0 {
      255
    } else {
      255 - self.masks.data_iterate[o - 1]
    };

    let right_mask = if e == 0 {
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
  pub fn iterator(&mut self, _start: usize, _end: usize) {
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
  fn set_index(&mut self, mut index: usize, value: u8) -> Change {
    let o = index & 3;
    index = (index - o) / 4;

    let start = 2 * index;

    let left = self.index.get_byte(start) & self.masks.index_update[o];
    let right = get_index_value(value) >> (2 * o);
    let mut byte = left | right;
    let len = self.index.len();
    let max_len = self.page_len * 256;

    self.iterator.seek(start);

    while self.iterator.index() < max_len
      && self.index.set_byte(self.iterator.index(), byte) == Change::Changed
    {
      if self.iterator.is_left() {
        let index: usize = self.index.get_byte(self.iterator.sibling()).into();
        byte = self.masks.map_parent_left[byte as usize]
          | self.masks.map_parent_right[index];
      } else {
        let index: usize = self
          .index
          .get_byte(self.iterator.sibling()) // FIXME: out of bounds read
          .into();
        byte = self.masks.map_parent_right[byte as usize]
          | self.masks.map_parent_left[index];
      }
      self.iterator.parent();
    }

    if len != self.index.len() {
      self.expand(len);
    }

    if self.iterator.index() == start {
      Change::Unchanged
    } else {
      Change::Changed
    }
  }

  fn expand(&mut self, len: usize) {
    let mut roots = vec![]; // FIXME: alloc.
    flat::full_roots(2 * len, &mut roots);
    let bf = &mut self.index;
    let ite = &mut self.iterator;
    let masks = &self.masks;
    let mut byte;

    for root in roots {
      ite.seek(root);
      byte = bf.get_byte(ite.index());

      loop {
        if ite.is_left() {
          let index = bf.get_byte(ite.sibling()) as usize;
          byte = masks.map_parent_left[byte as usize]
            | masks.map_parent_right[index];
        } else {
          let index = bf.get_byte(ite.sibling()) as usize;
          byte = masks.map_parent_right[byte as usize]
            | masks.map_parent_left[index];
        }

        if let Change::Unchanged = set_byte_no_alloc(bf, ite.parent(), byte) {
          break;
        }
      }
    }
  }
}

// NOTE: can we move this into `sparse_bitfield`?
fn set_byte_no_alloc(
  bf: &mut sparse_bitfield::Bitfield,
  index: usize,
  byte: u8,
) -> Change {
  if 8 * index >= bf.len() {
    return Change::Unchanged;
  }
  bf.set_byte(index, byte)
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
