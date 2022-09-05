use anyhow::ensure;
use anyhow::Result;

use crate::compact_encoding::State;
use crate::oplog::HeaderTree;
use crate::{
    common::{BufferSlice, BufferSliceInstruction},
    Node,
};

use super::MerkleTreeChangeset;

/// Merkle tree.
/// See https://github.com/hypercore-protocol/hypercore/blob/master/lib/merkle-tree.js
#[derive(Debug)]
pub struct MerkleTree {
    roots: Vec<Node>,
    length: u64,
    byte_length: u64,
    fork: u64,
}

const NODE_SIZE: u64 = 40;

impl MerkleTree {
    /// Gets instructions to slices that should be read from storage
    /// based on `HeaderTree` `length` field. Call this before
    /// calling `open`.
    pub fn get_slice_instructions_to_read(
        header_tree: &HeaderTree,
    ) -> Box<[BufferSliceInstruction]> {
        let root_indices = get_root_indices(&header_tree.length);

        root_indices
            .iter()
            .map(|&index| BufferSliceInstruction {
                index: NODE_SIZE * index,
                len: NODE_SIZE,
            })
            .collect::<Vec<BufferSliceInstruction>>()
            .into_boxed_slice()
    }

    /// Opens MerkleTree, based on read byte slices. Call `get_slice_instructions_to_read`
    /// before calling this to find out which slices to read. The given slices
    /// need to be in the same order as the instructions from `get_slice_instructions_to_read`!
    pub fn open(header_tree: &HeaderTree, slices: Box<[BufferSlice]>) -> Result<Self> {
        let root_indices = get_root_indices(&header_tree.length);

        let mut roots: Vec<Node> = Vec::with_capacity(slices.len());
        let mut byte_length: u64 = 0;
        let mut length: u64 = 0;
        for i in 0..root_indices.len() {
            let index = root_indices[i];
            ensure!(
                index == slices[i].index / NODE_SIZE,
                "Given slices vector not in the correct order"
            );
            let data = slices[i].data.as_ref().unwrap();
            let node = node_from_bytes(&index, data);
            byte_length += node.length;
            // This is totalSpan in Javascript
            length += 2 * ((node.index - length) + 1);

            roots.push(node);
        }

        Ok(Self {
            roots,
            length,
            byte_length,
            fork: header_tree.fork,
        })
    }

    /// Initialize a changeset for this tree.
    /// This is called batch() in Javascript, see:
    /// https://github.com/hypercore-protocol/hypercore/blob/master/lib/merkle-tree.js
    pub fn changeset(&self) -> MerkleTreeChangeset {
        MerkleTreeChangeset::new(self.length, self.byte_length, self.fork, self.roots.clone())
    }
}

fn get_root_indices(header_tree_length: &u64) -> Vec<u64> {
    let mut roots = vec![];
    flat_tree::full_roots(header_tree_length * 2, &mut roots);
    roots
}

fn node_from_bytes(index: &u64, data: &[u8]) -> Node {
    let len_buf = &data[..8];
    let hash = &data[8..];
    let mut state = State::from_buffer(len_buf);
    let len = state.decode_u64(len_buf);
    Node::new(*index, hash.to_vec(), len)
}
