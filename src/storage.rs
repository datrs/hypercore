//! Save data to a desired storage backend.

use bitfield::Bitfield;

/// The types of stores that can be created.
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
#[derive(Debug)]
pub struct Storage {
  // key: Vec<u8>,
  // secret_key: Vec<u8>,
  // tree
  // bitfield
  // signatures
  // create (function)
  // cache_size
}

impl Storage {
  /// Create a new instance.
  // Named `.open()` in the JS version.
  pub fn new(create: fn(Store) -> Bitfield) -> Self {
    // let missing = 5;

    let key = create(Store::Key);
    let secret_key = create(Store::SecretKey);
    let tree = create(Store::Tree);
    let data = create(Store::Data);
    let bitfield = create(Store::Bitfield);
    let signatures = create(Store::Signatures);

    unimplemented!();
  }

  /// Create a new instance but only for keys.
  // Named `.open_key()` in the JS version.
  pub fn new_with_key(&mut self) {
    unimplemented!();
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
