use crate::crypto::Hash;
use crate::Result;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use flat_tree;
use merkle_tree_stream::Node as NodeTrait;
use std::cmp::Ordering;
use std::convert::AsRef;
use std::fmt::{self, Display};
use std::io::{Cursor, Seek, SeekFrom};

/// Nodes that are persisted to disk.
// TODO: replace `hash: Vec<u8>` with `hash: Hash`. This requires patching /
// rewriting the Blake2b crate to support `.from_bytes()` to serialize from
// disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
  pub(crate) index: usize,
  pub(crate) hash: Hash,
  pub(crate) length: usize,
  pub(crate) parent: usize,
  pub(crate) data: Option<Vec<u8>>,
}

impl Node {
  /// Create a new instance.
  // TODO: ensure sizes are correct.
  pub fn new(index: usize, hash: Hash, length: usize) -> Self {
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

    // TODO: subslice directly, move cursor forward.
    let capacity = 32;
    let hash = Hash::from_bytes(&buffer[..capacity]);

    let mut reader = Cursor::new(buffer);
    reader.seek(SeekFrom::Start(capacity as u64))?;
    // TODO: This will blow up on 32 bit systems, because usize can be 32 bits.
    // Note: we could stop using usize on any protocol specific parts of code?
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
    writer.extend_from_slice(&self.hash.as_bytes());
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
    &self.hash.as_bytes()
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
      self.index, self.hash, self.length
    )
  }
}

impl PartialOrd for Node {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(self.index.cmp(&other.index))
  }
}

impl Ord for Node {
  fn cmp(&self, other: &Self) -> Ordering {
    self.index.cmp(&other.index)
  }
}
