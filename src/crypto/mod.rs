// lazy_static! {
//   static ref LEAF_TYPE: &'static [u8] = b"0";
//   static ref PARENT_TYPE: &'static [u8] = b"1";
//   static ref ROOT_TYPE: &'static [u8] = b"2";
//   static ref HYPERCORE: &'static [u8] = b"hypercore";
// }

extern crate rust_sodium as sodium;
pub use self::sodium::crypto::sign::ed25519::{PublicKey, SecretKey, Signature};
use self::sodium::crypto::sign::ed25519::{sign_detached, verify_detached};

/// Generate an Ed25519 keypair.
pub mod key_pair;

/// Create an Ed25519 signature for data.
pub mod signature;

/// Sign an Ed25199 key pair.
pub fn sign(data: &[u8], secret_key: &SecretKey) -> Signature {
  sign_detached(&data, &secret_key)
}

/// Verify an Ed25199 key pair.
pub fn verify(
  signature: &Signature,
  data: &[u8],
  public_key: &PublicKey,
) -> bool {
  verify_detached(signature, &data, public_key)
}
