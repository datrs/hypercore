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

mod iterator;
mod masks;

use self::masks::Masks;
use flat_tree::{self, Iterator as FlatIterator};
pub use sparse_bitfield::{Bitfield as SparseBitfield, Change};
use std::ops::Range;

/// Bitfield with `{data, tree, index} fields.`
#[derive(Debug)]
pub struct Bitfield {
  data: SparseBitfield,
  /// FIXME: SLEEP protocol tree field.
  pub tree: SparseBitfield,
  index: SparseBitfield,
  page_len: usize,
  length: usize,
  masks: Masks,
  iterator: FlatIterator,
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
      data: SparseBitfield::new(1024),
      tree: SparseBitfield::new(2048),
      index: SparseBitfield::new(256),
      page_len: 3328,
      length: 0,
      masks: Masks::new(),
      iterator: FlatIterator::new(0),
    }
  }

  /// Get the current length
  pub fn len(&self) -> usize {
    self.length
  }

  /// Returns `true` if the bitfield is empty
  pub fn is_empty(&self) -> bool {
    self.length == 0
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

    if self.data.set_byte(index, value).is_unchanged() {
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
    self.total_with_range(0..len)
  }

  /// Calculate the total of ... TODO(yw)
  pub fn total_with_start(&mut self, start: usize) -> u8 {
    let len = self.data.len();
    self.total_with_range(start..len)
  }

  /// Calculate the total of ... TODO(yw)
  pub fn total_with_range(&mut self, range: Range<usize>) -> u8 {
    let start = range.start;
    let end = range.end;

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

    let left_mask = 255 - self.masks.data_iterate[o];
    let right_mask = self.masks.data_iterate[e];

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

    let start = tree_index(index);

    let left = self.index.get_byte(start) & self.masks.index_update[o];
    let right = get_index_value(value) >> tree_index(o);
    let mut byte = left | right;
    let len = self.index.len();
    let max_len = self.page_len * 256;

    self.iterator.seek(start);

    while self.iterator.index() < max_len
      && self
        .index
        .set_byte(self.iterator.index(), byte)
        .is_changed()
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
    flat_tree::full_roots(tree_index(len), &mut roots);
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

        if set_byte_no_alloc(bf, ite.parent(), byte).is_unchanged() {
          break;
        }
      }
    }
  }

  /// Constructs an iterator from start to end
  pub fn iterator(&mut self) -> iterator::Iterator<'_> {
    let len = self.length;
    self.iterator_with_range(0, len)
  }

  /// Constructs an iterator from `start` to `end`
  pub fn iterator_with_range(
    &mut self,
    start: usize,
    end: usize,
  ) -> iterator::Iterator<'_> {
    let mut iter = iterator::Iterator::new(self);
    iter.range(start, end);
    iter.seek(0);

    iter
  }
}

// NOTE: can we move this into `sparse_bitfield`?
fn set_byte_no_alloc(
  bf: &mut SparseBitfield,
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

/// Convert the index to the index in the tree.
#[inline]
fn tree_index(index: usize) -> usize {
  2 * index
}
