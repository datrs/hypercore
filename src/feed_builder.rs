use Feed;

extern crate flat_tree as flat;
extern crate random_access_disk as rad;
extern crate random_access_memory as ram;
extern crate random_access_storage as ras;
extern crate sparse_bitfield;
extern crate tree_index;

use crypto::{Keypair, Merkle};
use failure::Error;
use ras::RandomAccessMethods;
use sparse_bitfield::Bitfield;
use std::fmt::Debug;
use storage::Storage;
use tree_index::TreeIndex;

/// Construct a new `Feed` instance.
// TODO: make this an actual builder pattern.
// https://deterministic.space/elegant-apis-in-rust.html#builder-pattern
#[derive(Debug)]
pub struct FeedBuilder<T>
where
  T: RandomAccessMethods + Debug,
{
  keypair: Keypair,
  storage: Storage<T>,
}

impl<T> FeedBuilder<T>
where
  T: RandomAccessMethods + Debug,
{
  /// Create a new instance.
  #[inline]
  pub fn new(keypair: Keypair, storage: Storage<T>) -> Self {
    Self { keypair, storage }
  }

  /// Finalize the builder.
  #[inline]
  pub fn build(self) -> Result<Feed<T>, Error> {
    Ok(Feed {
      merkle: Merkle::new(),
      byte_length: 0,
      length: 0,
      bitfield: Bitfield::default(),
      tree: TreeIndex::default(),
      keypair: self.keypair,
      storage: self.storage,
    })
  }
}
