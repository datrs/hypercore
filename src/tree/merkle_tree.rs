use anyhow::Result;
use anyhow::{anyhow, ensure};
use ed25519_dalek::Signature;
use futures::future::Either;

use crate::common::NodeByteRange;
use crate::compact_encoding::State;
use crate::oplog::HeaderTree;
use crate::Store;
use crate::{
    common::{StoreInfo, StoreInfoInstruction},
    Node,
};

use super::MerkleTreeChangeset;

/// Merkle tree.
/// See https://github.com/hypercore-protocol/hypercore/blob/master/lib/merkle-tree.js
#[derive(Debug)]
pub struct MerkleTree {
    pub(crate) roots: Vec<Node>,
    pub(crate) length: u64,
    pub(crate) byte_length: u64,
    pub(crate) fork: u64,
    pub(crate) signature: Option<Signature>,
    unflushed: intmap::IntMap<Node>,
    truncated: bool,
    truncate_to: u64,
}

const NODE_SIZE: u64 = 40;

impl MerkleTree {
    /// Gets instructions to slices that should be read from storage
    /// based on `HeaderTree` `length` field. Call this before
    /// calling `open`.
    pub fn get_info_instructions_to_read(header_tree: &HeaderTree) -> Box<[StoreInfoInstruction]> {
        let root_indices = get_root_indices(&header_tree.length);

        root_indices
            .iter()
            .map(|&index| {
                StoreInfoInstruction::new_content(Store::Tree, NODE_SIZE * index, NODE_SIZE)
            })
            .collect::<Vec<StoreInfoInstruction>>()
            .into_boxed_slice()
    }

    /// Opens MerkleTree, based on read infos. Call `get_info_instructions_to_read`
    /// before calling this to find out which infos to read. The given infos
    /// need to be in the same order as the instructions from `get_info_instructions_to_read`!
    pub fn open(header_tree: &HeaderTree, infos: Box<[StoreInfo]>) -> Result<Self> {
        let root_indices = get_root_indices(&header_tree.length);

        let mut roots: Vec<Node> = Vec::with_capacity(infos.len());
        let mut byte_length: u64 = 0;
        let mut length: u64 = 0;

        for i in 0..root_indices.len() {
            let index = root_indices[i];
            ensure!(
                index == index_from_info(&infos[i]),
                "Given slices vector not in the correct order"
            );
            let data = infos[i].data.as_ref().unwrap();
            let node = node_from_bytes(&index, data);
            byte_length += node.length;
            // This is totalSpan in Javascript
            length += 2 * ((node.index - length) + 1);

            roots.push(node);
        }
        if length > 0 {
            length = length / 2;
        }

        Ok(Self {
            roots,
            length,
            byte_length,
            fork: header_tree.fork,
            unflushed: intmap::IntMap::new(),
            truncated: false,
            truncate_to: 0,
            signature: None,
        })
    }

    /// Initialize a changeset for this tree.
    /// This is called batch() in Javascript, see:
    /// https://github.com/hypercore-protocol/hypercore/blob/master/lib/merkle-tree.js
    pub fn changeset(&self) -> MerkleTreeChangeset {
        MerkleTreeChangeset::new(self.length, self.byte_length, self.fork, self.roots.clone())
    }

    /// Commit a created changeset to the tree.
    pub fn commit(&mut self, changeset: MerkleTreeChangeset) -> Result<()> {
        if !self.commitable(&changeset) {
            return Err(anyhow!(
                "Tree was modified during changeset, refusing to commit"
            ));
        }

        if changeset.upgraded {
            self.commit_truncation(&changeset);

            self.roots = changeset.roots;
            self.length = changeset.length;
            self.byte_length = changeset.byte_length;
            self.fork = changeset.fork;
            self.signature = changeset
                .hash_and_signature
                .map(|hash_and_signature| hash_and_signature.1);
        }

        for node in changeset.nodes {
            self.unflushed.insert(node.index, node);
        }

        Ok(())
    }

    /// Flush committed made changes to the tree
    pub fn flush(&mut self) -> Box<[StoreInfo]> {
        let mut infos_to_flush: Vec<StoreInfo> = Vec::new();
        if self.truncated {
            infos_to_flush.extend(self.flush_truncation());
        }
        infos_to_flush.extend(self.flush_nodes());
        infos_to_flush.into_boxed_slice()
    }

    /// Get storage byte range of given hypercore index
    pub fn byte_range(
        &self,
        hypercore_index: u64,
        infos: Option<&[StoreInfo]>,
    ) -> Result<Either<Box<[StoreInfoInstruction]>, NodeByteRange>> {
        // Converts a hypercore index into a merkle tree index
        let index = 2 * hypercore_index;

        // Check bounds
        let head = 2 * self.length;
        let compare_index = if index & 1 == 0 {
            index
        } else {
            flat_tree::right_span(index)
        };
        if compare_index >= head {
            return Err(anyhow!(
                "Hypercore index {} is out of bounds",
                hypercore_index
            ));
        }

        // Get nodes out of incoming infos
        let nodes: Vec<Node> = match infos {
            Some(infos) => infos
                .iter()
                .map(|info| node_from_bytes(&index_from_info(&info), info.data.as_ref().unwrap()))
                .collect(),
            None => vec![],
        };

        // Start with getting the requested node, which will get the length
        // of the byte range
        let length_result = self.get_node(index, &nodes)?;

        // As for the offset, that might require a lot more nodes to combine into
        // an offset
        let offset_result = self.byte_offset(index, &nodes)?;

        // Construct response of either instructions (Left) or the result (Right)
        let mut instructions: Vec<StoreInfoInstruction> = Vec::new();
        let mut byte_range = NodeByteRange {
            index: 0,
            length: 0,
        };
        match length_result {
            Either::Left(instruction) => {
                if infos.is_some() {
                    return Err(anyhow!("Could not return size from fetched nodes"));
                }
                instructions.push(instruction);
            }
            Either::Right(node) => {
                byte_range.length = node.length;
            }
        }
        match offset_result {
            Either::Left(offset_instructions) => {
                if infos.is_some() {
                    return Err(anyhow!("Could not return offset from fetched nodes"));
                }
                instructions.extend(offset_instructions);
            }
            Either::Right(offset) => {
                byte_range.index = offset;
            }
        }

        if instructions.is_empty() {
            Ok(Either::Right(byte_range))
        } else {
            Ok(Either::Left(instructions.into_boxed_slice()))
        }
    }

    fn commitable(&self, changeset: &MerkleTreeChangeset) -> bool {
        let correct_length: bool = if changeset.upgraded {
            changeset.original_tree_length == self.length
        } else {
            changeset.original_tree_length <= self.length
        };
        changeset.original_tree_fork == self.fork && correct_length
    }

    fn commit_truncation(&mut self, changeset: &MerkleTreeChangeset) {
        if changeset.ancestors < changeset.original_tree_length {
            if changeset.ancestors > 0 {
                let head = 2 * changeset.ancestors;
                let mut iter = flat_tree::Iterator::new(head - 2);
                loop {
                    // TODO: we should implement a contains() method in the Iterator
                    // similar to the Javascript
                    // https://github.com/mafintosh/flat-tree/blob/master/index.js#L152
                    // then this would work:
                    // if iter.contains(head) && iter.index() < head {
                    let index = iter.index();
                    let factor = iter.factor();
                    let contains: bool = if head > index {
                        head < (index + factor / 2)
                    } else {
                        if head < index {
                            head > (index - factor / 2)
                        } else {
                            true
                        }
                    };
                    if contains && index < head {
                        self.unflushed.insert(index, Node::new_blank(index));
                    }

                    if iter.offset() == 0 {
                        break;
                    }
                    iter.parent();
                }
            }

            self.truncate_to = if self.truncated {
                std::cmp::min(self.truncate_to, changeset.ancestors)
            } else {
                changeset.ancestors
            };

            self.truncated = true;
            let mut unflushed_indices_to_delete: Vec<u64> = Vec::new();
            for node in self.unflushed.iter() {
                if *node.0 >= 2 * changeset.ancestors {
                    unflushed_indices_to_delete.push(*node.0);
                }
            }
            for index_to_delete in unflushed_indices_to_delete {
                self.unflushed.remove(index_to_delete);
            }
        }
    }

    pub fn flush_truncation(&mut self) -> Vec<StoreInfo> {
        let offset = if self.truncate_to == 0 {
            0
        } else {
            (self.truncate_to - 1) * 80 + 40
        };
        self.truncate_to = 0;
        self.truncated = false;
        vec![StoreInfo::new_truncate(Store::Tree, offset)]
    }

    pub fn flush_nodes(&mut self) -> Vec<StoreInfo> {
        let mut infos_to_flush: Vec<StoreInfo> = Vec::with_capacity(self.unflushed.len());
        for node in self.unflushed.values() {
            let (mut state, mut buffer) = State::new_with_size(40);
            state.encode_u64(node.length, &mut buffer);
            state.encode_fixed_32(&node.hash, &mut buffer);
            infos_to_flush.push(StoreInfo::new_content(
                Store::Tree,
                node.index * 40,
                &buffer,
            ));
        }
        infos_to_flush
    }

    fn byte_offset(
        &self,
        index: u64,
        nodes: &Vec<Node>,
    ) -> Result<Either<Vec<StoreInfoInstruction>, u64>> {
        let index = if (index & 1) == 1 {
            flat_tree::left_span(index)
        } else {
            index
        };
        let mut head: u64 = 0;
        let mut offset: u64 = 0;

        for root_node in &self.roots {
            head += 2 * ((root_node.index - head) + 1);

            if index >= head {
                offset += root_node.length;
                continue;
            }
            let mut iter = flat_tree::Iterator::new(root_node.index);

            let mut instructions: Vec<StoreInfoInstruction> = Vec::new();
            while iter.index() != index {
                if index < iter.index() {
                    iter.left_child();
                } else {
                    let left_child = iter.left_child();
                    let node_or_instruction = self.get_node(left_child, nodes)?;
                    match node_or_instruction {
                        Either::Left(instruction) => {
                            instructions.push(instruction);
                        }
                        Either::Right(node) => {
                            offset += node.length;
                        }
                    }
                    iter.sibling();
                }
            }
            return if instructions.is_empty() {
                Ok(Either::Right(offset))
            } else {
                Ok(Either::Left(instructions))
            };
        }
        Err(anyhow!(
            "Could not calculate byte offset for index {}",
            index
        ))
    }

    fn get_node(
        &self,
        index: u64,
        nodes: &Vec<Node>,
    ) -> Result<Either<StoreInfoInstruction, Node>> {
        // First check if unflushed already has the node
        if let Some(node) = self.unflushed.get(index) {
            if node.blank || (self.truncated && node.index >= 2 * self.truncate_to) {
                // The node is either blank or being deleted
                return Err(anyhow!("Could not load node: {}", index));
            }
            return Ok(Either::Right(node.clone()));
        }

        // Then check if it's already in the incoming nodes
        for node in nodes {
            if node.index == index {
                return Ok(Either::Right(node.clone()));
            }
        }

        // If not, retunr an instruction
        Ok(Either::Left(StoreInfoInstruction::new_content(
            Store::Tree,
            40 * index,
            40,
        )))
    }
}

fn get_root_indices(header_tree_length: &u64) -> Vec<u64> {
    let mut roots = vec![];
    flat_tree::full_roots(header_tree_length * 2, &mut roots);
    roots
}

fn index_from_info(info: &StoreInfo) -> u64 {
    info.index / NODE_SIZE
}

fn node_from_bytes(index: &u64, data: &[u8]) -> Node {
    let len_buf = &data[..8];
    let hash = &data[8..];
    let mut state = State::from_buffer(len_buf);
    let len = state.decode_u64(len_buf);
    Node::new(*index, hash.to_vec(), len)
}
