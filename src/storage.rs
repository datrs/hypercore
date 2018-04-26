//! Save data to a desired storage backend.

extern crate failure;
extern crate random_access_storage as ras;
extern crate sleep_parser;

use self::failure::Error;
use self::ras::SyncMethods;
use self::sleep_parser::*;
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
pub struct Storage<T>
where
  T: SyncMethods,
{
  public_key: PublicKey,
  secret_key: SecretKey,
  tree: ras::Sync<T>,
  data: ras::Sync<T>,
  bitfield: ras::Sync<T>,
  signatures: ras::Sync<T>,
  // cache_size
}

impl<T> Storage<T>
where
  T: SyncMethods,
{
  /// Create a new instance. Takes a keypair and a callback to create new
  /// storage instances.
  // Named `.open()` in the JS version. Replaces the `.openKey()` method too by
  // requiring a key pair to be initialized before creating a new instance.
  pub fn new(
    key_pair: KeyPair,
    create: fn(Store) -> ras::Sync<T>,
  ) -> Result<Self, Error> {
    // let missing = 5;
    let mut instance = Self {
      public_key: key_pair.public_key,
      secret_key: key_pair.secret_key,
      tree: create(Store::Tree),
      data: create(Store::Data),
      bitfield: create(Store::Bitfield),
      signatures: create(Store::Signatures),
    };

    let header = create_bitfield();
    instance.bitfield.write(0, &header.to_vec())?;

    let header = create_signatures();
    instance.signatures.write(0, &header.to_vec())?;

    let header = create_tree();
    instance.tree.write(0, &header.to_vec())?;

    Ok(instance)
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

impl<T> Drop for Storage<T>
where
  T: SyncMethods,
{
  fn drop(&mut self) {
    unimplemented!();
  }
}
