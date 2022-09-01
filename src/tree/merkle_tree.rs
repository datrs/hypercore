use anyhow::Result;

use crate::common::{BufferSlice, BufferSliceInstruction};

/// Merkle tree.
/// See https://github.com/hypercore-protocol/hypercore/blob/master/lib/merkle-tree.js
#[derive(Debug)]
pub struct MerkleTree {}

impl MerkleTree {
    /// Gets instructions to slices that should be read from storage
    /// based on `HeaderTree` `length` field. Call this before
    /// calling `open`.
    pub fn get_slice_instructions_to_read(
        header_tree_length: u64,
    ) -> Box<[BufferSliceInstruction]> {
        let mut roots = vec![];
        flat_tree::full_roots(header_tree_length * 2, &mut roots);

        roots
            .iter()
            .map(|&index| BufferSliceInstruction { index, len: 8 })
            .collect::<Vec<BufferSliceInstruction>>()
            .into_boxed_slice()
    }

    /// Opens MerkleTree, based on read byte slices. Call `get_slice_instructions_to_read`
    /// before calling this to find out which slices to read.
    pub fn open(header_tree_length: u64, slices: Box<[BufferSlice]>) -> Result<Self> {
        // read_tree_bytes(0, 0).await?;
        // read_tree_bytes(0, 0)?;
        Ok(Self {})
    }
}
