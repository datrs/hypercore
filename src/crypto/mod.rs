//! Cryptographic functions.

extern crate ed25519_dalek;

mod hash;
mod key_pair;
mod merkle;
mod root;

pub use self::ed25519_dalek::{PublicKey, Signature};

pub use self::hash::Hash;
pub use self::key_pair::{generate as generate_keypair, sign, verify, Keypair};
pub use self::merkle::Merkle;
pub use self::root::Root;
