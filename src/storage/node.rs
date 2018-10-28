use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use crate::Result;
use flat_tree;
use merkle_tree_stream::Node as NodeTrait;
use pretty_hash::fmt as pretty_fmt;
use std::convert::AsRef;
use std::fmt::{self, Display};
use std::io::Cursor;

/// Nodes that are persisted to disk.
// TODO: derive Ord, PartialOrd based on index.
// TODO: replace `hash: Vec<u8>` with `hash: Hash`. This requires patching /
// rewriting the Blake2b crate to support `.from_bytes()` to serialize from
// disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
  pub(crate) index: usize,
  pub(crate) hash: Vec<u8>,
  pub(crate) length: usize,
  pub(crate) parent: usize,
  pub(crate) data: Option<Vec<u8>>,
}

impl Node {
  /// Create a new instance.
  // TODO: ensure sizes are correct.
  pub fn new(index: usize, hash: Vec<u8>, length: usize) -> Self {
    Self {
      index,
      hash,
      length,
      parent: flat_tree::parent(index),
      data: Some(Vec::with_capacity(0)),
    }
  }

  /// Convert a vector to a new instance.
  ///
  /// Requires the index at which the buffer was read to be passed.
  pub fn from_bytes(index: usize, buffer: &[u8]) -> Result<Self> {
    ensure!(buffer.len() == 40, "buffer should be 40 bytes");

    let parent = flat_tree::parent(index);
    let mut reader = Cursor::new(buffer);

    // TODO: subslice directly, move cursor forward.
    let capacity = 32;
    let mut hash = Vec::with_capacity(capacity);
    for _ in 0..capacity {
      hash.push(reader.read_u8()?);
    }

    // TODO: This will blow up on 32 bit systems, because usize can be 32 bits.
    let length = reader.read_u64::<BigEndian>()? as usize;
    Ok(Self {
      hash,
      length,
      index,
      parent,
      data: Some(Vec::with_capacity(0)),
    })
  }

  /// Convert to a buffer that can be written to disk.
  pub fn to_bytes(&self) -> Result<Vec<u8>> {
    let mut writer = Vec::with_capacity(40);
    writer.extend_from_slice(&self.hash);
    writer.write_u64::<BigEndian>(self.length as u64)?;
    Ok(writer)
  }
}

impl NodeTrait for Node {
  #[inline]
  fn index(&self) -> usize {
    self.index
  }

  #[inline]
  fn hash(&self) -> &[u8] {
    &self.hash
  }

  #[inline]
  fn len(&self) -> usize {
    self.length
  }

  #[inline]
  fn is_empty(&self) -> bool {
    self.length == 0
  }

  #[inline]
  fn parent(&self) -> usize {
    self.parent
  }
}

impl AsRef<Node> for Node {
  #[inline]
  fn as_ref(&self) -> &Self {
    self
  }
}

impl Display for Node {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
      f,
      "Node {{ index: {}, hash: {}, length: {} }}",
      self.index,
      pretty_fmt(&self.hash).unwrap(),
      self.length
    )
  }
}
