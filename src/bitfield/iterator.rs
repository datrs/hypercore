//! Iterate over a bitfield.

use super::Bitfield;

/// Iterate over a bitfield.
#[derive(Debug)]
pub struct Iterator<'a> {
  start: usize,
  end: usize,
  index_end: usize,
  pos: Option<usize>,
  byte: u8,
  bitfield: &'a mut Bitfield,
}

impl<'a> Iterator<'a> {
  /// Create a new instance.
  pub fn new(bitfield: &'a mut Bitfield) -> Self {
    Self {
      start: 0,
      end: 0,
      index_end: 0,
      pos: Some(0),
      byte: 0,
      bitfield,
    }
  }

  /// Grow the bitfield if needed.
  pub fn range(&mut self, start: usize, end: usize) {
    self.start = start;
    self.end = end;
    self.index_end = 2 * ((end + 31) / 32);

    if self.end > self.bitfield.length {
      self.bitfield.expand(self.end);
    }
  }

  /// Seek to `offset`
  pub fn seek(&mut self, mut offset: usize) -> &mut Self {
    offset += self.start;
    // FIXME This is fishy. Offset and start is unsigned, so `offset < self.start` can only
    //  be true when the previous addition overflows. The overflow would cause a panic, so,
    //  either the addition should be a wrapping_add, or rather, the original offset should
    //  be checked to ensure it is less than `self.end - self.start`.
    if offset < self.start {
      offset = self.start;
    }

    if offset >= self.end {
      self.pos = None;
      return self;
    }

    let o = offset % 8;

    let pos = offset / 8;
    self.pos = Some(pos);
    let left = self.bitfield.data.get_byte(pos);
    let right = if o == 0 {
      0
    } else {
      self.bitfield.masks.data_iterate[o - 1]
    };

    self.byte = left | right;

    self
  }

  pub fn next(&mut self) -> Option<usize> {
    let mut pos = if let Some(p) = self.pos {
      p
    } else {
      return None;
    };

    let mut free = self.bitfield.masks.next_data_0_bit[self.byte as usize];

    while free == -1 {
      pos += 1;
      self.byte = self.bitfield.data.get_byte(pos);
      free = self.bitfield.masks.next_data_0_bit[self.byte as usize];

      if free == -1 {
        pos = if let Some(p) = self.skip_ahead(pos) {
          p
        } else {
          return None;
        };

        self.byte = self.bitfield.data.get_byte(pos);
        free = self.bitfield.masks.next_data_0_bit[self.byte as usize];
      }
    }
    self.pos = Some(pos);

    self.byte |= self.bitfield.masks.data_iterate[free as usize];

    let n = 8 * pos + free as usize;
    if n < self.end {
      Some(n)
    } else {
      None
    }
  }

  pub fn skip_ahead(&mut self, start: usize) -> Option<usize> {
    let bitfield_index = &self.bitfield.index;
    let tree_end = self.index_end;
    let iter = &mut self.bitfield.iterator;
    let o = start & 3;

    iter.seek(2 * (start / 4));

    let mut tree_byte = bitfield_index.get_byte(iter.index())
      | self.bitfield.masks.index_iterate[o];

    while self.bitfield.masks.next_index_0_bit[tree_byte as usize] == -1 {
      if iter.is_left() {
        iter.next();
      } else {
        iter.next();
        iter.parent();
      }

      if right_span(iter) >= tree_end {
        while right_span(iter) >= tree_end && is_parent(iter) {
          iter.left_child();
        }
        if right_span(iter) >= tree_end {
          return None;
        }
      }

      tree_byte = bitfield_index.get_byte(iter.index());
    }

    while iter.factor() > 2 {
      if self.bitfield.masks.next_index_0_bit[tree_byte as usize] < 2 {
        iter.left_child();
      } else {
        iter.right_child();
      }

      tree_byte = bitfield_index.get_byte(iter.index());
    }

    let mut free = self.bitfield.masks.next_index_0_bit[tree_byte as usize];
    if free == -1 {
      free = 4;
    }

    let next = iter.index() * 2 + free as usize;

    if next <= start {
      Some(start + 1)
    } else {
      Some(next)
    }
  }
}

fn right_span(iter: &flat_tree::Iterator) -> usize {
  iter.index() + iter.factor() / 2 - 1
}

fn is_parent(iter: &flat_tree::Iterator) -> bool {
  iter.index() & 1 == 1
}
