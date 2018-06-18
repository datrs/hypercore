//! Cryptographic functions.

mod hash;
mod key_pair;
mod merkle;

pub use ed25519_dalek::{PublicKey, Signature};

pub use self::hash::Hash;
pub use self::key_pair::{generate as generate_keypair, sign, verify, Keypair};
pub use self::merkle::Merkle;
