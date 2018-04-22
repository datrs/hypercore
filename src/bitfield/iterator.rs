//! Iterate over a bitfield.

use super::Bitfield;

/// Iterate over a bitfield.
#[derive(Debug)]
pub struct Iterator {
  start: usize,
  end: usize,
  index_end: usize,
  pos: usize,
  byte: usize,
  bitfield: Bitfield,
}

impl Iterator {
  /// Create a new instance.
  pub fn new(bitfield: &mut Bitfield) -> Self {
    Self {
      start: 0,
      end: 0,
      index_end: 0,
      pos: 0,
      byte: 0,
      bitfield,
    }
  }

  /// Grow the bitfield if needed.
  pub fn range(&mut self, start: usize, end: usize) {
    self.start = start;
    self.end = end;
    self.index_end = 2 * (end as f32 / 32).ceil();

    if self.end > self.bitfield.len() {
      self.bitfield.expand(self.end);
    }
  }

  pub fn seek(&mut self, offset: usize)  {
  offset += self.start;
  if offset < self.start {
    offset = self.start
    }

  if offset >= self.end {
    self.pos = -1
    return;
  }

  let o = offset & 7;

  self.pos = (offset - o) / 8;
  let left = self.bitfield.data.get_byte(self.pos);
  let right = if o == 0 {
    0
  } else {
    self.bitfield.masks.data_iterate_mask[o - 1]
  };

  self.byte = left | right;
}
