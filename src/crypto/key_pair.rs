//! Generate an `Ed25519` keypair.

extern crate ed25519_dalek;
extern crate failure;
extern crate rand;
extern crate sha2;

pub use self::ed25519_dalek::{Keypair, PublicKey, Signature};

use self::failure::Error;
use self::rand::OsRng;
use self::sha2::Sha512;

/// Generate a new `Ed25519` key pair.
pub fn generate() -> Keypair {
  let mut cspring: OsRng = OsRng::new().unwrap();
  Keypair::generate::<Sha512>(&mut cspring)
}

/// Sign a byte slice using a keypair's private key.
pub fn sign(keypair: &Keypair, msg: &[u8]) -> Signature {
  keypair.sign::<Sha512>(msg)
}

/// Verify a signature on a message with a keypair's public key.
pub fn verify(
  public: &PublicKey,
  msg: &[u8],
  sig: &Signature,
) -> Result<(), Error> {
  ensure!(
    public.verify::<Sha512>(msg, sig),
    "Signature verification failed"
  );
  Ok(())
}

#[test]
fn can_verify_messages () {
  let keypair = generate();
  let from = b"hello";
  let sig = sign(&keypair, from);
  verify(&keypair.public, from, &sig).unwrap();
  verify(&keypair.public, b"oops", &sig).is_err();
}
