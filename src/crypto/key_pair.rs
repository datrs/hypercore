//! Generate an `Ed25519` keypair.

pub use ed25519_dalek::{ExpandedSecretKey, Keypair, PublicKey, SecretKey, Signature, Verifier};

use anyhow::{bail, ensure, Result};
use rand::rngs::{OsRng, StdRng};
use rand::SeedableRng;

/// Generate a new `Ed25519` key pair.
pub fn generate() -> Keypair {
    let mut rng = StdRng::from_rng(OsRng::default()).unwrap();
    Keypair::generate(&mut rng)
}

/// Sign a byte slice using a keypair's private key.
pub fn sign(public_key: &PublicKey, secret: &SecretKey, msg: &[u8]) -> Signature {
    ExpandedSecretKey::from(secret).sign(msg, public_key)
}

/// Verify a signature on a message with a keypair's public key.
pub fn verify(public: &PublicKey, msg: &[u8], sig: Option<&Signature>) -> Result<()> {
    match sig {
        None => bail!("Signature verification failed"),
        Some(sig) => {
            ensure!(
                public.verify(msg, sig).is_ok(),
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
