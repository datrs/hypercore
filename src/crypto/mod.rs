//! Cryptographic functions.

extern crate ed25519_dalek;

mod hash;
mod hasher;
mod key_pair;
mod merkle;

pub use self::ed25519_dalek::Signature;

pub use self::hash::Hash;
pub use self::hasher::Hasher;
pub use self::key_pair::{generate as generate_keypair, Keypair};
pub use self::merkle::Merkle;
