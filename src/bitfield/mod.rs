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
use std::convert::TryInto;
use std::ops::Range;

/// Bitfield with `{data, tree, index} fields.`
#[derive(Debug)]
pub struct Bitfield {
    data: SparseBitfield,
    index: SparseBitfield,
    page_len: u64,
    length: u64,
    masks: Masks,
    iterator: FlatIterator,
}

impl Bitfield {
    /// Create a new instance.
    pub fn new() -> (Self, SparseBitfield) {
        let s = Self {
            data: SparseBitfield::new(1024),
            index: SparseBitfield::new(256),
            page_len: 3328,
            length: 0,
            masks: Masks::new(),
            iterator: FlatIterator::new(0),
        };
        (s, SparseBitfield::new(2048))
    }

    /// Create new instance from byteslice
    pub fn from_slice(slice: &[u8]) -> (Self, SparseBitfield) {
        // khodzha:
        // slice is packed as data|tree|index|data|tree|index|...
        // so for each 1024 + 2048 + 256 bytes
        // we extract first 1024 bytes to data
        // then next 2048 bytes to tree
        // then next 256 bytes to index
        let mut data = SparseBitfield::new(1024);
        let mut tree = SparseBitfield::new(2048);
        let mut index = SparseBitfield::new(256);
        slice
            .chunks_exact(1024 + 2048 + 256)
            .enumerate()
            .for_each(|(page_idx, chunk)| {
                chunk.iter().enumerate().for_each(|(idx, byte)| {
                    if idx < 1024 {
                        data.set_byte(page_idx * 1024 + idx, *byte);
                    } else if idx < 1024 + 2048 {
                        tree.set_byte(page_idx * 1024 + (idx - 1024), *byte);
                    } else {
                        index.set_byte(page_idx * 1024 + (idx - 1024 - 2048), *byte);
                    }
                });
            });
        let length = data
            .len()
            .try_into()
            .expect("Failed to convert len:usize to length:u64");
        let s = Self {
            data,
            index,
            length,
            page_len: 3328,
            masks: Masks::new(),
            iterator: FlatIterator::new(0),
        };

        (s, tree)
    }

    /// Convert to vec
    pub fn to_bytes(&self, tree: &tree_index::TreeIndex) -> std::io::Result<Vec<u8>> {
        let tree = tree.as_bitfield();
        let data_bytes = self.data.to_bytes()?;
        let tree_bytes = tree.to_bytes()?;
        let index_bytes = self.index.to_bytes()?;

        let max_pages_len = std::cmp::max(
            std::cmp::max(self.data.page_len(), tree.page_len()),
            self.index.page_len(),
        );

        let data_ps = self.data.page_size();
        let tree_ps = tree.page_size();
        let index_ps = self.index.page_size();

        let total_ps = data_ps + tree_ps + index_ps;

        let mut vec = Vec::with_capacity(max_pages_len * total_ps);

        for i in 0..max_pages_len {
            extend_buf_from_slice(&mut vec, &data_bytes, i, data_ps);
            extend_buf_from_slice(&mut vec, &tree_bytes, i, tree_ps);
            extend_buf_from_slice(&mut vec, &index_bytes, i, index_ps);
        }

        Ok(vec)
    }

    /// Get the current length
    pub fn len(&self) -> u64 {
        self.length
    }

    /// Returns `true` if the bitfield is empty
    pub fn is_empty(&self) -> bool {
        self.length == 0
    }

    /// Set a value at an index.
    pub fn set(&mut self, index: u64, value: bool) -> Change {
        let o = mask_8b(index);
        let index = (index - o) / 8;

        let value = if value {
            self.data.get_byte(index as usize) | 128 >> o
        } else {
            self.data.get_byte(index as usize) & self.masks.data_update[o as usize]
        };

        if self.data.set_byte(index as usize, value).is_unchanged() {
            return Change::Unchanged;
        }

        self.length = self.data.len() as u64;
        self.set_index(index, value);
        Change::Changed
    }

    /// Get a value at a position in the bitfield.
    pub fn get(&mut self, index: u64) -> bool {
        self.data.get(index as usize)
    }

    /// Calculate the total for the whole data.
    pub fn total(&mut self) -> u8 {
        let len = self.data.len() as u64;
        self.total_with_range(0..len)
    }

    /// Calculate the total of ... TODO(yw)
    pub fn total_with_start(&mut self, start: u64) -> u8 {
        let len = self.data.len() as u64;
        self.total_with_range(start..len)
    }

    /// Calculate the total of ... TODO(yw)
    pub fn total_with_range(&mut self, range: Range<u64>) -> u8 {
        let start = range.start;
        let end = range.end;

        if end < start {
            return 0;
        }

        if end > self.data.len() as u64 {
            self.expand(end);
        }

        let o = mask_8b(start);
        let e = mask_8b(end);

        let pos = (start - o) / 8;
        let last = (end - e) / 8;

        let left_mask = 255 - self.masks.data_iterate[o as usize];
        let right_mask = self.masks.data_iterate[e as usize];

        let byte = self.data.get_byte(pos as usize);
        if pos == last {
            let index = (byte & left_mask & right_mask) as u64;
            return self.masks.total_1_bits[index as usize];
        }
        let index = (byte & left_mask) as u64;
        let mut total = self.masks.total_1_bits[index as usize];

        for i in pos + 1..last {
            let index = self.data.get_byte(i as usize) as u64;
            total += self.masks.total_1_bits[index as usize];
        }

        let index: u64 = self.data.get_byte(last as usize) as u64 & right_mask as u64;
        total + self.masks.total_1_bits[index as usize]
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
    fn set_index(&mut self, mut index: u64, value: u8) -> Change {
        let o = index & 3;
        index = (index - o) / 4;

        let start = tree_index(index);

        let left = self.index.get_byte(start as usize) & self.masks.index_update[o as usize];
        let right = get_index_value(value) >> tree_index(o);
        let mut byte = left | right;
        let len = self.index.len();
        let max_len = self.data.page_len() * 256;

        self.iterator.seek(start);

        while self.iterator.index() < max_len as u64
            && self
                .index
                .set_byte(self.iterator.index() as usize, byte)
                .is_changed()
        {
            if self.iterator.is_left() {
                let index: u64 = self.index.get_byte(self.iterator.sibling() as usize).into();
                byte = self.masks.map_parent_left[byte as usize]
                    | self.masks.map_parent_right[index as usize];
            } else {
                let index: u64 = self
                    .index
                    .get_byte(self.iterator.sibling() as usize) // FIXME: out of bounds read
                    .into();
                byte = self.masks.map_parent_right[byte as usize]
                    | self.masks.map_parent_left[index as usize];
            }
            self.iterator.parent();
        }

        if len != self.index.len() {
            self.expand(len as u64);
        }

        if self.iterator.index() == start {
            Change::Unchanged
        } else {
            Change::Changed
        }
    }

    fn expand(&mut self, len: u64) {
        let mut roots = vec![]; // FIXME: alloc.
        flat_tree::full_roots(tree_index(len), &mut roots);
        let bf = &mut self.index;
        let ite = &mut self.iterator;
        let masks = &self.masks;
        let mut byte;

        for root in roots {
            ite.seek(root);
            byte = bf.get_byte(ite.index() as usize);

            loop {
                if ite.is_left() {
                    let index = bf.get_byte(ite.sibling() as usize) as u64;
                    byte = masks.map_parent_left[byte as usize]
                        | masks.map_parent_right[index as usize];
                } else {
                    let index = bf.get_byte(ite.sibling() as usize) as u64;
                    byte = masks.map_parent_right[byte as usize]
                        | masks.map_parent_left[index as usize];
                }

                if set_byte_no_alloc(bf, ite.parent(), byte).is_unchanged() {
                    break;
                }
            }
        }
    }

    // TODO: use the index to speed this up *a lot*
    /// https://github.com/mafintosh/hypercore/blob/06f3a1f573cb74ee8cfab2742455318fbf7cc3a2/lib/bitfield.js#L111-L126
    pub fn compress(&self, start: usize, length: usize) -> std::io::Result<Vec<u8>> {
        // On Node versions this fields might not be present on the want/request message
        // When both start and length are not present (!0 in node is false), return all data bytes encoded
        if start == 0 && length == 0 {
            return Ok(bitfield_rle::encode(&self.data.to_bytes()?));
        }

        use std::io::{Cursor, Write};
        let mut buf = Cursor::new(Vec::with_capacity(length));

        let page_size = self.data.page_size() as f64;
        let mut p = start as f64 / page_size / 8.0;
        let end = p + length as f64 / page_size / 8.0;
        let offset = p * page_size;

        while p < end {
            let index = p as usize;
            let page = self.data.pages.get(index);
            if let Some(page) = page {
                if page.len() != 0 {
                    buf.set_position((p * page_size - offset) as u64);
                    buf.write_all(&page)?;
                }
            }
            p += 1.0;
        }

        Ok(bitfield_rle::encode(&buf.into_inner()))
    }

    /// Constructs an iterator from start to end
    pub fn iterator(&mut self) -> iterator::Iterator<'_> {
        let len = self.length;
        self.iterator_with_range(0, len)
    }

    /// Constructs an iterator from `start` to `end`
    pub fn iterator_with_range(&mut self, start: u64, end: u64) -> iterator::Iterator<'_> {
        let mut iter = iterator::Iterator::new(self);
        iter.range(start, end);
        iter.seek(0);

        iter
    }
}

// NOTE: can we move this into `sparse_bitfield`?
fn set_byte_no_alloc(bf: &mut SparseBitfield, index: u64, byte: u8) -> Change {
    if 8 * index >= bf.len() as u64 {
        return Change::Unchanged;
    }
    bf.set_byte(index as usize, byte)
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
fn mask_8b(num: u64) -> u64 {
    num & 7
}

/// Convert the index to the index in the tree.
#[inline]
fn tree_index(index: u64) -> u64 {
    2 * index
}

// copies slice to buf or fills buf with len-of-slice zeros
fn extend_buf_from_slice(buf: &mut Vec<u8>, bytes: &[u8], i: usize, pagesize: usize) {
    if i * pagesize >= bytes.len() {
        for _ in 0..pagesize {
            buf.push(0);
        }
    } else {
        let range = (i * pagesize)..((i + 1) * pagesize);
        buf.extend_from_slice(&bytes[range]);
    }
}
