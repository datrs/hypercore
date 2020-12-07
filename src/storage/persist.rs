use super::BoxStorage;
use anyhow::Result;
use random_access_storage::RandomAccess;
use std::fmt::Debug;

/// Persist data to a `Storage` instance.
pub trait Persist<T>
where
    T: RandomAccess + Debug,
{
    /// Create an instance from a byte vector.
    fn from_bytes(index: u64, buf: &[u8]) -> Self;

    /// Create a vector.
    fn to_vec(&self) -> Result<Vec<u8>>;

    /// Persist into a storage backend.
    fn store(&self, index: u64, store: BoxStorage) -> Result<()>;
}
