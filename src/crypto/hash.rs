pub use blake2_rfc::blake2b::Blake2bResult;

use blake2_rfc::blake2b::Blake2b;
use byteorder::{BigEndian, WriteBytesExt};
// use ed25519_dalek::PublicKey;
use merkle_tree_stream::Node as NodeTrait;
use std::convert::AsRef;
use std::mem;
use std::ops::{Deref, DerefMut};
use storage::Node;

// https://en.wikipedia.org/wiki/Merkle_tree#Second_preimage_attack
const LEAF_TYPE: [u8; 1] = [0; 1];
const PARENT_TYPE: [u8; 1] = [1; 1];
const ROOT_TYPE: [u8; 1] = [2; 1];
//const HYPERCORE: [u8; 9] = *b"hypercore";

/// `BLAKE2b` hash.
#[derive(Debug, Clone, PartialEq)]
pub struct Hash {
  hash: Blake2bResult,
}

impl Hash {
  /// Hash a `Leaf` node.
  pub fn from_leaf(data: &[u8]) -> Self {
    let size = u64_as_be(data.len() as u64);

    let mut hasher = Blake2b::new(32);
    hasher.update(&LEAF_TYPE);
    hasher.update(&size);
    hasher.update(data);

    Self {
      hash: hasher.finalize(),
    }
  }

  /// Hash two `Leaf` nodes hashes together to form a `Parent` hash.
  pub fn from_hashes(left: &Node, right: &Node) -> Self {
    let (node1, node2) = if left.index <= right.index {
      (left, right)
    } else {
      (right, left)
    };

    let size = u64_as_be((node1.length + node2.length) as u64);

    let mut hasher = Blake2b::new(32);
    hasher.update(&PARENT_TYPE);
    hasher.update(&size);
    hasher.update(node1.hash());
    hasher.update(node2.hash());

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
    hasher.update(&ROOT_TYPE);

    for node in roots {
      let node = node.as_ref();
      hasher.update(node.hash());
      hasher.update(&u64_as_be((node.index()) as u64));
      hasher.update(&u64_as_be((node.len()) as u64));
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

fn u64_as_be(n: u64) -> [u8; 8] {
  let mut size = [0u8; mem::size_of::<u64>()];
  size.as_mut().write_u64::<BigEndian>(n).unwrap();
  size
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

#[cfg(test)]
mod tests {
  use super::*;

  extern crate data_encoding;
  use self::data_encoding::HEXLOWER;

  fn hex_bytes(hex: &str) -> Vec<u8> {
    HEXLOWER.decode(hex.as_bytes()).unwrap()
  }

  fn check_hash(hash: Hash, hex: &str) {
    assert_eq!(hash.as_bytes(), &hex_bytes(hex)[..]);
  }

  #[test]
  fn leaf_hash() {
    check_hash(
      Hash::from_leaf(&[]),
      "5187b7a8021bf4f2c004ea3a54cfece1754f11c7624d2363c7f4cf4fddd1441e",
    );
    check_hash(
      Hash::from_leaf(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]),
      "e1001bb0bb9322b6b202b2f737dc12181b11727168d33ca48ffe361c66cd1abe",
    );
  }

  #[test]
  fn parent_hash() {
    let d1: &[u8] = &[0, 1, 2, 3, 4];
    let d2: &[u8] = &[42, 43, 44, 45, 46, 47, 48];
    let node1 = Node::new(0, Hash::from_leaf(d1).as_bytes().to_vec(), d1.len());
    let node2 = Node::new(1, Hash::from_leaf(d2).as_bytes().to_vec(), d2.len());
    check_hash(
      Hash::from_hashes(&node1, &node2),
      "6fac58578fa385f25a54c0637adaca71fdfddcea885d561f33d80c4487149a14",
    );
    check_hash(
      Hash::from_hashes(&node2, &node1),
      "6fac58578fa385f25a54c0637adaca71fdfddcea885d561f33d80c4487149a14",
    );
  }

  #[test]
  fn root_hash() {
    let d1: &[u8] = &[0, 1, 2, 3, 4];
    let d2: &[u8] = &[42, 43, 44, 45, 46, 47, 48];
    let node1 = Node::new(0, Hash::from_leaf(d1).as_bytes().to_vec(), d1.len());
    let node2 = Node::new(1, Hash::from_leaf(d2).as_bytes().to_vec(), d2.len());
    check_hash(
      Hash::from_roots(&[&node1, &node2]),
      "2d117e0bb15c6e5236b6ce764649baed1c41890da901a015341503146cc20bcd",
    );
    check_hash(
      Hash::from_roots(&[&node2, &node1]),
      "9826c8c2d28fc309cce73a4b6208e83e5e4b0433d2369bfbf8858272153849f1",
    );
  }
}
