//! Cryptographic functions.

lazy_static! {
  static ref LEAF_TYPE: &'static [u8] = b"0";
  static ref PARENT_TYPE: &'static [u8] = b"1";
  static ref ROOT_TYPE: &'static [u8] = b"2";
  // static ref HYPERCORE: &'static [u8] = b"hypercore";
}

extern crate blake2_rfc as blake2;
extern crate byteorder;
extern crate merkle_tree_stream as merkle_stream;
extern crate rust_sodium as sodium;

mod hash;
mod hasher;
mod key_pair;
mod sign;

pub use self::blake2::blake2b::Blake2bResult;
pub use self::hash::Hash;
pub use self::hasher::Hasher;
pub use self::key_pair::KeyPair;
pub use self::sign::Sign;
pub use self::sodium::crypto::sign::ed25519::{PublicKey, SecretKey, Signature};

use self::blake2::blake2b::Blake2b;
use self::byteorder::{BigEndian, WriteBytesExt};
use self::merkle_stream::Node;
use self::sodium::crypto::sign::ed25519::{sign_detached, verify_detached};

/// Sign a piece of data using an `Ed25519` secret key.
pub fn sign(data: &[u8], secret_key: &SecretKey) -> Signature {
  sign_detached(data, secret_key)
}

/// Verify a piece of data's signature using an `Ed25519` public key.
pub fn verify(
  signature: &Signature,
  data: &[u8],
  public_key: &PublicKey,
) -> bool {
  verify_detached(signature, data, public_key)
}

/// Compute the hash of a leaf node using `BLAKE2b`.
pub fn hash_leaf(data: &Node) -> Blake2bResult {
  let mut size = vec![]; // FIXME: allocate once only.
  size
    .write_u64::<BigEndian>(data.len() as u64)
    .unwrap();

  let mut hasher = Blake2b::new(32);
  hasher.update(*LEAF_TYPE);
  hasher.update(&size);
  hasher.update(data.as_ref().unwrap());
  hasher.finalize()
}

/// Compute the hash of a parent node using `BLAKE2b`.
pub fn hash_parent(a: &Node, b: &Node) -> Blake2bResult {
  assert!(b.position() > a.position());

  let mut size = vec![]; // FIXME: allocate once only.
  size
    .write_u64::<BigEndian>((a.len() + b.len()) as u64)
    .unwrap();

  let mut hasher = Blake2b::new(32);
  hasher.update(*PARENT_TYPE);
  hasher.update(&size);
  hasher.update(a.hash());
  hasher.update(b.hash());
  hasher.finalize()
}

/// Hash a set of roots.
// Called `crypto.tree()` in the JS implementation.
pub fn hash_roots(roots: &[&Node]) -> Blake2bResult {
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
  hasher.finalize()
}
