lazy_static! {
  static ref LEAF_TYPE: &'static [u8] = b"0";
  static ref PARENT_TYPE: &'static [u8] = b"1";
  static ref ROOT_TYPE: &'static [u8] = b"2";
  static ref HYPERCORE: &'static [u8] = b"hypercore";
}

extern crate blake2_rfc as blake2;
extern crate byteorder;
extern crate merkle_tree_stream as merkle_stream;
extern crate rust_sodium as sodium;

pub use self::blake2::blake2b::Blake2bResult;
pub use self::sodium::crypto::sign::ed25519::{PublicKey, SecretKey, Signature};

use self::blake2::blake2b::Blake2b;
use self::byteorder::{BigEndian, WriteBytesExt};
use self::merkle_stream::Node;
use self::sodium::crypto::sign::ed25519::{sign_detached, verify_detached};

/// Generate an `Ed25519` keypair.
pub mod key_pair;

/// Sign an `Ed25519` key pair.
pub fn sign(data: &[u8], secret_key: &SecretKey) -> Signature {
  sign_detached(&data, &secret_key)
}

/// Verify an `Ed25519` key pair.
pub fn verify(
  signature: &Signature,
  data: &[u8],
  public_key: &PublicKey,
) -> bool {
  verify_detached(signature, &data, public_key)
}

/// Compute the hash of a leaf node using `BLAKE2b`.
/// TODO: accept `Node` types.
pub fn hash_leaf(data: &[u8]) -> Blake2bResult {
  let mut size = vec![]; // FIXME: allocate once only.
  size
    .write_u64::<BigEndian>(data.len() as u64)
    .unwrap();

  let mut hasher = Blake2b::new(32);
  hasher.update(*LEAF_TYPE);
  hasher.update(&size);
  hasher.update(data);
  hasher.finalize()
}

/// Compute the hash of a parent node using `BLAKE2b`.
pub fn hash_parent(a: &Node, b: &Node) -> Blake2bResult {
  assert!(b.index > a.index);

  let mut size = vec![]; // FIXME: allocate once only.
  size
    .write_u64::<BigEndian>((a.size + b.size) as u64)
    .unwrap();

  let mut hasher = Blake2b::new(32);
  hasher.update(*PARENT_TYPE);
  hasher.update(&size);
  hasher.update(&a.hash);
  hasher.update(&b.hash);
  hasher.finalize()
}
