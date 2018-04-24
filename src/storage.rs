//! Save data to a desired storage backend.

// use bitfield::Bitfield;

/// Save data to a desired storage backend.
pub struct Storage {
  // key: Vec<u8>,
  // secret_key: Vec<u8>,
  // tree
  // bitfield
  // signatures
  // create (function)
}

impl Storage {
  /// Create a new instance.
  // Named `.open()` in the JS version.
  pub fn new(&mut self) {
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
