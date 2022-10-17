//! Compact encoding module. Rust implementation of https://github.com/compact-encoding/compact-encoding.

pub mod custom;
pub mod generic;
pub mod types;

pub use types::{CompactEncoding, State};
