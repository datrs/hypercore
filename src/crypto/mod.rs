//! Cryptographic functions.

mod hash;
mod key_pair;
mod merkle;

pub use self::hash::Hash;
pub use self::key_pair::{
    generate as generate_keypair, sign, verify, PublicKey, SecretKey, Signature,
};
pub use self::merkle::Merkle;
