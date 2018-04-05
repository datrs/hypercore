extern crate failure;

// use super::crypto::*;

use self::failure::Error;
use std::path::PathBuf;

/// Append-only log structure.
pub struct Feed {
  /// Location on disk of where to persist the archive.
  pub path: PathBuf,
}

impl Feed {
  /// Create a new instance with an on-disk storage backend.
  pub fn new(_path: PathBuf) -> Self {
    unimplemented!();
  }

  /// Create a new instance with a custom storage backend.
  pub fn with_storage() -> Self {
    unimplemented!();
  }

  /// Append data into the log.
  pub fn append(&self, _data: &[u8]) -> Result<(), Error> {
    unimplemented!();
  }

  /// Retrieve data from the log.
  pub fn get(&self, _index: usize) -> Option<&[u8]> {
    unimplemented!();
  }
}

/// Create a new instance with an in-memory storage backend.
impl Default for Feed {
  fn default() -> Self {
    unimplemented!();
  }
}
