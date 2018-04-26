//! Save data to a desired storage backend.

extern crate random_access_storage as ras;
extern crate sleep_parser;

use self::ras::SyncMethods;
use self::sleep_parser::{FileType, HashType, Header};
use super::crypto::{KeyPair, PublicKey, SecretKey};
use bitfield::Bitfield;

/// The types of stores that can be created.
#[derive(Debug)]
pub enum Store {
  /// Public key
  Key,
  /// Secret key
  SecretKey,
  /// Tree
  Tree,
  /// Data
  Data,
  /// Bitfield
  Bitfield,
  /// Signatures
  Signatures,
}

/// Save data to a desired storage backend.
// #[derive(Debug)]
pub struct Storage {
  public_key: PublicKey,
  secret_key: SecretKey,
  tree: ras::Sync<Box<SyncMethods>>,
  data: ras::Sync<Box<SyncMethods>>,
  bitfield: ras::Sync<Box<SyncMethods>>,
  signatures: ras::Sync<Box<SyncMethods>>,
  // cache_size
}

impl Storage {
  /// Create a new instance.
  // Named `.open()` in the JS version. Replaces the `.openKey()` method too by
  // requiring a key pair to be initialized before creating a new instance.
  pub fn new(
    key_pair: KeyPair,
    create: fn(Store) -> ras::Sync<Box<SyncMethods>>,
  ) -> Self {
    // let missing = 5;
    let instance = Self {
      public_key: key_pair.public_key,
      secret_key: key_pair.secret_key,
      tree: create(Store::Tree),
      data: create(Store::Data),
      bitfield: create(Store::Bitfield),
      signatures: create(Store::Signatures),
    };

    let header = Header::new(FileType::BitField, 3328, HashType::None);
    instance.bitfield.write(0, header.to_vec());

    instance
  }

  /// TODO(yw) docs
  pub fn put_data(&mut self) {
    unimplemented!();
  }

  /// TODO(yw) docs
  pub fn get_data(&mut self) {
    unimplemented!();
  }

  /// TODO(yw) docs
  pub fn next_signature(&mut self) {
    unimplemented!();
  }

  /// TODO(yw) docs
  pub fn get_signature(&mut self) {
    unimplemented!();
  }

  /// TODO(yw) docs
  pub fn put_signature(&mut self) {
    unimplemented!();
  }

  /// TODO(yw) docs
  pub fn data_offset(&mut self) {
    unimplemented!();
  }

  /// TODO(yw) docs
  pub fn get_node(&mut self) {
    unimplemented!();
  }

  /// TODO(yw) docs
  pub fn put_node(&mut self) {
    unimplemented!();
  }

  /// TODO(yw) docs
  pub fn put_bitfield(&mut self) {
    unimplemented!();
  }

  /// TODO(yw) docs
  pub fn open_key(&mut self) {
    unimplemented!();
  }
}

impl Drop for Storage {
  fn drop(&mut self) {
    unimplemented!();
  }
}
