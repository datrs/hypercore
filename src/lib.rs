#![deny(missing_docs)]
#![cfg_attr(test, deny(warnings))]
#![feature(external_doc)]
#![doc(include = "../README.md")]
#![cfg_attr(test, feature(plugin))]
#![cfg_attr(test, plugin(clippy))]

extern crate failure;

use failure::Error;
use std::path::PathBuf;

/// Append-only log structure.
pub struct Log {}

impl Log {
  /// Create a new instance.
  pub fn new(_path: PathBuf) -> Self {
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
