//! Generate an `Ed25519` keypair.

extern crate ed25519_dalek;
extern crate rand;
extern crate sha2;

pub use self::ed25519_dalek::Keypair;
use self::rand::OsRng;
use self::sha2::Sha512;

/// Generate a new `Ed25519` key pair.
pub fn generate() -> Keypair {
  let mut cspring: OsRng = OsRng::new().unwrap();
  Keypair::generate::<Sha512>(&mut cspring)
}
