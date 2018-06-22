pub use blake2_rfc::blake2b::Blake2bResult;

use blake2_rfc::blake2b::Blake2b;
use byteorder::{BigEndian, WriteBytesExt};
// use ed25519_dalek::PublicKey;
use merkle_tree_stream::Node as NodeTrait;
use std::convert::AsRef;
use std::ops::{Deref, DerefMut};
use storage::Node;

// https://en.wikipedia.org/wiki/Merkle_tree#Second_preimage_attack
lazy_static! {
  static ref LEAF_TYPE: &'static [u8] = b"0";
  static ref PARENT_TYPE: &'static [u8] = b"1";
  static ref ROOT_TYPE: &'static [u8] = b"2";
  // static ref HYPERCORE: &'static [u8] = b"hypercore";
}

/// `BLAKE2b` hash.
#[derive(Debug, Clone, PartialEq)]
pub struct Hash {
  hash: Blake2bResult,
}

impl Hash {
  /// Hash a `Leaf` node.
  pub fn from_leaf(data: &[u8]) -> Self {
    let mut size = vec![]; // TODO: allocate once only.
    size.write_u64::<BigEndian>(data.len() as u64).unwrap();

    let mut hasher = Blake2b::new(32);
    hasher.update(*LEAF_TYPE);
    hasher.update(&size);
    hasher.update(data);

    Self {
      hash: hasher.finalize(),
    }
  }

  /// Hash two `Leaf` nodes hashes together to form a `Parent` hash.
  pub fn from_hashes(left: &[u8], right: &[u8]) -> Self {
    let mut size = vec![]; // TODO: allocate once only.
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

  // /// Hash a public key. Useful to find the key you're looking for on a public
  // /// network without leaking the key itself.
  // pub fn from_key(public_key: PublicKey) -> Self {
  //   let mut hasher = Blake2b::new(32);
  //   hasher.update(*HYPERCORE);
  //   hasher.update(public_key.as_bytes());
  //   Self {
  //     hash: hasher.finalize(),
  //   }
  // }

  /// Hash a vector of `Root` nodes.
  // Called `crypto.tree()` in the JS implementation.
  pub fn from_roots(roots: &[impl AsRef<Node>]) -> Self {
    let mut hasher = Blake2b::new(32);
    hasher.update(*ROOT_TYPE);

    for node in roots {
      let node = node.as_ref();
      let mut position = Vec::with_capacity(1); // TODO: allocate once only.
      position
        .write_u64::<BigEndian>((node.index()) as u64)
        .unwrap();
      let mut len = Vec::with_capacity(1); // TODO: allocate once only.
      len.write_u64::<BigEndian>((node.len()) as u64).unwrap();
      hasher.update(node.hash());
      hasher.update(&position);
      hasher.update(&len);
    }

    Self {
      hash: hasher.finalize(),
    }
  }

  /// Returns a byte slice of this `Hash`'s contents.
  pub fn as_bytes(&self) -> &[u8] {
    self.hash.as_bytes()
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
