use ed25519_dalek::PublicKey;
use hypercore::{generate_keypair, sign, verify, Signature, Storage};

#[test]
fn should_write_and_read_keypair() {
    let keypair = generate_keypair();
    let msg = b"hello";
    // prepare a signature
    let sig: Signature = sign(&keypair.public, &keypair.secret, msg);

    let mut storage = Storage::new_memory().unwrap();
    assert!(
        storage.write_secret_key(&keypair.secret).is_ok(),
        "Can not store secret key."
    );
    assert!(
        storage.write_public_key(&keypair.public).is_ok(),
        "Can not store public key."
    );

    let read = storage.read_public_key();
    assert!(read.is_ok(), "Can not read public key");
    let public_key: PublicKey = read.unwrap();
    assert!(verify(&public_key, msg, Some(&sig)).is_ok());
}

#[test]
fn should_read_partial_keypair() {
    let keypair = generate_keypair();
    let mut storage = Storage::new_memory().unwrap();
    assert!(
        storage.write_public_key(&keypair.public).is_ok(),
        "Can not store public key."
    );

    let partial = storage.read_partial_keypair().unwrap();
    assert!(partial.secret.is_none(), "A secret key is present");
}

#[test]
fn should_read_no_keypair() {
    let mut storage = Storage::new_memory().unwrap();
    let partial = storage.read_partial_keypair();
    assert!(partial.is_none(), "A key is present");
}

#[test]
fn should_read_empty_public_key() {
    let mut storage = Storage::new_memory().unwrap();
    assert!(storage.read_public_key().is_err());
}
