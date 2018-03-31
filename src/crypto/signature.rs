extern crate rust_sodium as sodium;

use self::sodium::crypto::sign::ed25519;
use self::sodium::crypto::sign::ed25519::{sign_detached, SecretKey};

use std::ops::{Deref, DerefMut};

/// Ed25519 Signature.
#[derive(Debug, PartialEq)]
pub struct Signature {
  signature: ed25519::Signature,
}

impl Signature {
  /// Create a new signature for a piece of data using a secret key.
  pub fn new(data: &[u8], secret_key: &SecretKey) -> Self {
    let signature = sign_detached(&data, &secret_key);
    Signature { signature }
  }
}

impl Deref for Signature {
  type Target = ed25519::Signature;
  fn deref(&self) -> &Self::Target {
    &self.signature
  }
}

impl DerefMut for Signature {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.signature
  }
}
