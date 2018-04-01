lazy_static! {
  static ref LEAF_TYPE: &'static [u8] = b"0";
  static ref PARENT_TYPE: &'static [u8] = b"1";
  static ref ROOT_TYPE: &'static [u8] = b"2";
  static ref HYPERCORE: &'static [u8] = b"hypercore";
}

extern crate blake2_rfc as blake2;
extern crate byteorder;
extern crate rust_sodium as sodium;

pub use self::blake2::blake2b::Blake2bResult;
pub use self::sodium::crypto::sign::ed25519::{PublicKey, SecretKey, Signature};

use self::blake2::blake2b::Blake2b;
use self::byteorder::{BigEndian, WriteBytesExt};
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

/// Compute the hash of a leaf block using `BLAKE2b`.
pub fn hash_leaf(data: &[u8]) -> Blake2bResult {
  let mut writer = vec![];
  writer
    .write_u64::<BigEndian>(data.len() as u64)
    .unwrap();

  let mut hasher = Blake2b::new(32);
  hasher.update(*LEAF_TYPE);
  hasher.update(&writer);
  hasher.update(data);
  hasher.finalize()
}
