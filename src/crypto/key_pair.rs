extern crate rust_sodium as sodium;

pub use self::sodium::crypto::sign::ed25519::{PublicKey, SecretKey, Seed};
use self::sodium::crypto::sign::ed25519::{gen_keypair, keypair_from_seed};

/// Ed25519 key pair.
#[derive(Debug, PartialEq)]
pub struct KeyPair {
  /// The public key.
  pub public_key: PublicKey,
  /// The secret key.
  pub secret_key: SecretKey,
}

impl KeyPair {
  /// Create a new ed25519 key pair instance from a Seed.
  pub fn with_seed(seed: &Seed) -> Self {
    let (public_key, secret_key) = keypair_from_seed(&seed);
    KeyPair {
      public_key,
      secret_key,
    }
  }
}

impl Default for KeyPair {
  fn default() -> Self {
    let (public_key, secret_key) = gen_keypair();
    KeyPair {
      public_key,
      secret_key,
    }
  }
}
