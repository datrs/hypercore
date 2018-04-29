extern crate failure;

use self::failure::Error;
use super::{PublicKey, SecretKey, Signature};

/// Sign and verify data using `Ed25519` key pairs.
pub trait Sign {
  /// Sign a piece of data using the secret key from an `Ed25519` key pair.
  fn sign(&mut self, secret_key: &SecretKey) -> Signature;

  /// Verify a piece of data's signature using an `Ed25519` key pair's public
  /// key.
  fn verify(
    &mut self,
    public_key: &PublicKey,
    signature: &Signature,
  ) -> Result<(), Error>;
}
