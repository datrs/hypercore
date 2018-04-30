//! Cryptographic functions.

// TODO: wrap these types too.
extern crate rust_sodium as sodium;
pub use self::sodium::crypto::sign::ed25519::{PublicKey, SecretKey};

mod hash;
mod hasher;
mod key_pair;
mod merkle;
mod sign;
mod signature;

pub use self::hash::Hash;
pub use self::hasher::Hasher;
pub use self::key_pair::KeyPair;
pub use self::merkle::Merkle;
pub use self::sign::Sign;
pub use self::signature::Signature;
