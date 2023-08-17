//! Cryptographic functions.

mod hash;
mod key_pair;

pub(crate) use self::hash::{signable_tree, Hash};
pub use self::key_pair::{generate as generate_signing_key, sign, verify, PartialKeypair};
