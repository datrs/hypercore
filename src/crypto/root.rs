//! Root node type. Functions as an intermediate type for hash methods that
//! operate on Root.
//!
//! ## Why?
//! Both `merkle-tree-stream` and `hypercore` have `Node` types. Even if in most
//! cases these types don't overlap, in a select few cases both need to be
//! passed to the same function. So in order to facilitate that, the `Root` type
//! is created. It's entirely borrowed, and allows passing either type down into
//! a function that accepts `Root`.

/// Root node found in flat-tree.
pub struct Root<'a> {
    index: &'a u64,
    length: &'a u64,
    hash: &'a [u8],
}

impl<'a> Root<'a> {
    /// Create a new instance.
    #[inline]
    pub fn new(index: &'a u64, length: &'a u64, hash: &'a [u8]) -> Self {
        Self {
            index,
            length,
            hash,
        }
    }

    /// Get the index at which this root was found inside a `flat-tree`.
    #[inline]
    pub fn index(&self) -> &u64 {
        &self.index
    }

    /// Get the lenght of the data.
    #[inline]
    pub fn len(&self) -> &u64 {
        &self.length
    }

    /// Check if the content is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        *self.length == 0
    }

    /// Get the hash.
    #[inline]
    pub fn hash(&self) -> &'a [u8] {
        &self.hash
    }
}
