//! Cryptographic functions.

mod hash;
mod key_pair;

pub use self::hash::signable_tree;
pub use self::hash::Hash;
pub use self::key_pair::{
    generate as generate_keypair, sign, verify, PublicKey, SecretKey, Signature,
};
