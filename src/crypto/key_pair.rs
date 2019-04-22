//! Generate an `Ed25519` keypair.

pub use ed25519_dalek::{Keypair, PublicKey, SecretKey, Signature};

use crate::Result;
use rand::rngs::OsRng;
use sha2::Sha512;

/// Generate a new `Ed25519` key pair.
pub fn generate() -> Keypair {
  let mut cspring: OsRng = OsRng::new().unwrap();
  Keypair::generate::<Sha512, _>(&mut cspring)
}

/// Sign a byte slice using a keypair's private key.
pub fn sign(
  public_key: &PublicKey,
  secret: &SecretKey,
  msg: &[u8],
) -> Signature {
  secret.expand::<Sha512>().sign::<Sha512>(msg, public_key)
}

/// Verify a signature on a message with a keypair's public key.
pub fn verify(
  public: &PublicKey,
  msg: &[u8],
  sig: Option<&Signature>,
) -> Result<()> {
  match sig {
    None => bail!("Signature verification failed"),
    Some(sig) => {
      ensure!(
        public.verify::<Sha512>(msg, sig).is_ok(),
        "Signature verification failed"
      );
      Ok(())
    }
  }
}

#[test]
fn can_verify_messages() {
  let keypair = generate();
  let from = b"hello";
  let sig = sign(&keypair.public, &keypair.secret, from);
  verify(&keypair.public, from, Some(&sig)).unwrap();
  verify(&keypair.public, b"oops", Some(&sig)).unwrap_err();
}
