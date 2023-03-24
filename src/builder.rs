use std::fmt::Debug;

use random_access_storage::RandomAccess;

use crate::{Hypercore, HypercoreError, PartialKeypair, Storage};

/// Options for a Hypercore instance.
#[derive(Debug)]
pub struct Options<T>
where
    T: RandomAccess + Debug + Send,
{
    /// Existing key pair to use
    pub key_pair: Option<PartialKeypair>,
    /// Storage
    pub storage: Option<Storage<T>>,
}

impl<T> Options<T>
where
    T: RandomAccess + Debug + Send,
{
    /// Create with default options.
    pub fn new(storage: Storage<T>) -> Self {
        Self {
            storage: Some(storage),
            key_pair: None,
        }
    }
}

/// Build a Hypercore instance with options.
#[derive(Debug)]
pub struct Builder<T>(Options<T>)
where
    T: RandomAccess + Debug + Send;

impl<T> Builder<T>
where
    T: RandomAccess + Debug + Send,
{
    /// Create a hypercore builder with a given storage
    pub fn new(storage: Storage<T>) -> Self {
        Self(Options::new(storage))
    }

    /// Set key pair.
    pub fn set_key_pair(mut self, key_pair: PartialKeypair) -> Self {
        self.0.key_pair = Some(key_pair);
        self
    }

    /// Build a new Hypercore.
    pub async fn build_new(mut self) -> Result<Hypercore<T>, HypercoreError> {
        if let Some(storage) = self.0.storage.take() {
            if let Some(key_pair) = self.0.key_pair.take() {
                Hypercore::new_with_key_pair(storage, key_pair).await
            } else {
                Hypercore::new(storage).await
            }
        } else {
            return Err(HypercoreError::BadArgument {
                context: "Storage must be provided when building a new hypercore".to_string(),
            });
        }
    }

    /// Build an existing Hypercore.
    pub async fn build_existing(mut self) -> Result<Hypercore<T>, HypercoreError> {
        if self.0.key_pair.is_some() {
            return Err(HypercoreError::BadArgument {
                context: "Key pair can not be used when building an existing hypercore".to_string(),
            });
        }
        if let Some(storage) = self.0.storage.take() {
            Hypercore::open(storage).await
        } else {
            return Err(HypercoreError::BadArgument {
                context: "Storage must be provided when building an existing hypercore".to_string(),
            });
        }
    }
}
