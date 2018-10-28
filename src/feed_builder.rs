use ed25519_dalek::{PublicKey, SecretKey};

use crate::bitfield::Bitfield;
use crate::crypto::Merkle;
use crate::storage::Storage;
use failure::Error;
use random_access_storage::RandomAccess;
use std::fmt::Debug;
use tree_index::TreeIndex;

use crate::Feed;
use crate::Result;

/// Construct a new `Feed` instance.
// TODO: make this an actual builder pattern.
// https://deterministic.space/elegant-apis-in-rust.html#builder-pattern
#[derive(Debug)]
pub struct FeedBuilder<T>
where
  T: RandomAccess + Debug,
{
  storage: Storage<T>,
  public_key: PublicKey,
  secret_key: Option<SecretKey>,
}

impl<T> FeedBuilder<T>
where
  T: RandomAccess<Error = Error> + Debug,
{
  /// Create a new instance.
  #[inline]
  pub fn new(public_key: PublicKey, storage: Storage<T>) -> Self {
    Self {
      storage,
      public_key,
      secret_key: None,
    }
  }

  /// Set the secret key.
  pub fn secret_key(mut self, secret_key: SecretKey) -> Self {
    self.secret_key = Some(secret_key);
    self
  }

  /// Finalize the builder.
  #[inline]
  pub fn build(self) -> Result<Feed<T>> {
    Ok(Feed {
      merkle: Merkle::new(),
      byte_length: 0,
      length: 0,
      bitfield: Bitfield::default(),
      tree: TreeIndex::default(),
      public_key: self.public_key,
      secret_key: self.secret_key,
      storage: self.storage,
      peers: vec![],
    })
  }
}
