extern crate byteorder;
extern crate failure;

use self::byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use failure::Error;
use std::io::Cursor;

/// Nodes that are persisted to disk.
// TODO: derive Ord, PartialOrd based on index.
#[derive(Debug)]
pub struct Node {
  index: usize,
  hash: Vec<u8>,
  length: usize,
}

impl Node {
  /// Create a new instance.
  // TODO: ensure sizes are correct.
  pub fn new(index: usize, hash: Vec<u8>, length: usize) -> Self {
    Self {
      index,
      hash,
      length,
    }
  }

  /// Convert a vector to a new instance.
  ///
  /// Requires the index at which the buffer was read to be passed.
  pub fn from_vec(index: usize, buffer: &[u8]) -> Result<Self, Error> {
    ensure!(buffer.len() == 40, "buffer should be 40 bytes");

    let mut reader = Cursor::new(buffer);

    // TODO: subslice directly, move cursor forward.
    let capacity = 32;
    let mut hash = Vec::with_capacity(capacity);
    for _ in 0..capacity {
      hash.push(reader.read_u8()?);
    }

    // FIXME: This will blow up on 32 bit systems, because usize can be 32 bits.
    let length = reader.read_u64::<BigEndian>()? as usize;
    Ok(Self {
      hash,
      length,
      index,
    })
  }

  /// Convert to a buffer that can be written to disk.
  pub fn to_vec(&mut self) -> Vec<u8> {
    let mut writer = Vec::with_capacity(40);
    writer.extend_from_slice(&self.hash);
    writer.write_u64::<BigEndian>(self.length as u64);
    writer
  }

  /// Get the current index.
  pub fn index(&self) -> usize {
    self.index
  }

  /// Get the current hash.
  pub fn hash(&self) -> &[u8] {
    &self.hash
  }

  /// The length of the data
  // TODO: check if we can compile this conditionally to return a u64 on 32 bit
  // systems. Would solve the downcasting problem.
  // TODO: should we expose a `.len_as_u64()` call?
  pub fn len(&self) -> usize {
    self.length
  }

  /// The length of the data
  pub fn is_empty(&self) -> bool {
    self.length == 0
  }
}
