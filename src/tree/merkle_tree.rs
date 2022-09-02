use anyhow::ensure;
use anyhow::Result;

use crate::compact_encoding::State;
use crate::{
    common::{BufferSlice, BufferSliceInstruction},
    Node,
};

/// Merkle tree.
/// See https://github.com/hypercore-protocol/hypercore/blob/master/lib/merkle-tree.js
#[derive(Debug)]
pub struct MerkleTree {
    roots: Vec<Node>,
}

const NODE_SIZE: u64 = 40;

impl MerkleTree {
    /// Gets instructions to slices that should be read from storage
    /// based on `HeaderTree` `length` field. Call this before
    /// calling `open`.
    pub fn get_slice_instructions_to_read(
        header_tree_length: &u64,
    ) -> Box<[BufferSliceInstruction]> {
        let roots = get_root_indices(header_tree_length);

        roots
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
    pub fn open(header_tree_length: &u64, slices: Box<[BufferSlice]>) -> Result<Self> {
        let roots = get_root_indices(header_tree_length);

        let mut nodes: Vec<Node> = Vec::with_capacity(slices.len());
        for i in 0..roots.len() {
            let index = roots[i];
            ensure!(
                index == slices[i].index / NODE_SIZE,
                "Given slices vector not in the correct order"
            );
            let data = slices[i].data.as_ref().unwrap();
            let node = node_from_bytes(&index, data);
            nodes.push(node);
        }

        Ok(Self { roots: nodes })
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
