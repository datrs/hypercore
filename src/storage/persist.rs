use super::Storage;
use ras::RandomAccessMethods;
use std::fmt::Debug;
use Result;

/// Persist data to a `Storage` instance.
pub trait Persist<T>
where
  T: RandomAccessMethods + Debug,
{
  /// Create an instance from a byte vector.
  fn from_bytes(index: usize, buf: &[u8]) -> Self;

  /// Create a vector.
  fn to_vec(&self) -> Result<Vec<u8>>;

  /// Persist into a storage backend.
  fn store(&self, index: usize, store: Storage<T>) -> Result<()>;
}
