extern crate blake2_rfc as blake2;
extern crate byteorder;
extern crate merkle_tree_stream as merkle_stream;

pub use self::blake2::blake2b::Blake2bResult;

use self::blake2::blake2b::Blake2b;
use self::byteorder::{BigEndian, WriteBytesExt};
use self::merkle_stream::Node;
use std::ops::{Deref, DerefMut};

lazy_static! {
  static ref LEAF_TYPE: &'static [u8] = b"0";
  static ref PARENT_TYPE: &'static [u8] = b"1";
  static ref ROOT_TYPE: &'static [u8] = b"2";
  // static ref HYPERCORE: &'static [u8] = b"hypercore";
}

/// `BLAKE2b` hash.
pub struct Hash {
  hash: Blake2bResult,
}

impl Hash {
  /// Hash a `Leaf` node.
  pub fn from_leaf(data: &[u8]) -> Self {
    let mut size = vec![]; // FIXME: allocate once only.
    size
      .write_u64::<BigEndian>(data.len() as u64)
      .unwrap();

    let mut hasher = Blake2b::new(32);
    hasher.update(*LEAF_TYPE);
    hasher.update(&size);
    hasher.update(data);

    Self {
      hash: hasher.finalize(),
    }
  }

  /// Hash two `Leaf` nodes hashes together to form a `Parent` hash.
  pub fn from_parent(left: &[u8], right: &[u8]) -> Self {
    let mut size = vec![]; // FIXME: allocate once only.
    size
      .write_u64::<BigEndian>((left.len() + right.len()) as u64)
      .unwrap();

    let mut hasher = Blake2b::new(32);
    hasher.update(*PARENT_TYPE);
    hasher.update(&size);
    hasher.update(left);
    hasher.update(right);

    Self {
      hash: hasher.finalize(),
    }
  }

  /// Hash a vector of `Root` nodes.
  // Called `crypto.tree()` in the JS implementation.
  pub fn from_roots(roots: &[&Node]) -> Self {
    let mut hasher = Blake2b::new(32);
    hasher.update(*ROOT_TYPE);

    for node in roots {
      let mut position = Vec::with_capacity(1); // FIXME: allocate once only.
      position
        .write_u64::<BigEndian>((node.position()) as u64)
        .unwrap();
      let mut len = Vec::with_capacity(1); // FIXME: allocate once only.
      len
        .write_u64::<BigEndian>((node.len()) as u64)
        .unwrap();
      hasher.update(node.hash());
      hasher.update(&position);
      hasher.update(&len);
    }

    Self {
      hash: hasher.finalize(),
    }
  }
}

impl Deref for Hash {
  type Target = Blake2bResult;

  fn deref(&self) -> &Self::Target {
    &self.hash
  }
}

impl DerefMut for Hash {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.hash
  }
}
