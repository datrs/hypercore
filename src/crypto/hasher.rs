use super::Hash;

/// Hash data using `BLAKE2`.
pub trait Hasher {
  /// Create a `BLAKE2` hash.
  fn hash(&self) -> Hash;
}
