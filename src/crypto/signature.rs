//! `Ed25519` Signature.

extern crate rust_sodium as sodium;

use self::sodium::crypto::sign::ed25519::{self, sign_detached,
                                          verify_detached, PublicKey,
                                          SecretKey};
use std::ops::{Deref, DerefMut};

/// Result type for the `verify` function.
#[derive(Debug, PartialEq)]
pub enum Verification {
  Verified,
  Unverified,
}

/// `Ed25519` Signature.
#[derive(Debug, PartialEq)]
pub struct Signature {
  index: usize,
  signature: ed25519::Signature,
}

impl Signature {
  /// Sign a piece of data using an `Ed25519` secret key.
  pub fn new(index: usize, data: &[u8], secret_key: &SecretKey) -> Self {
    Self {
      index,
      signature: sign_detached(data, secret_key),
    }
  }

  /// Verify a piece of data's signature using an `Ed25519` public key.
  pub fn verify(&self, data: &[u8], public_key: &PublicKey) -> Verification {
    let res = verify_detached(&self.signature, data, public_key);
    if res {
      Verification::Verified
    } else {
      Verification::Unverified
    }
  }

  // /// Convert the signature to a byte vector. Useful when persisting to disk.
  // pub fn to_bytes(&self) -> Vec<u8> {
  // }
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
