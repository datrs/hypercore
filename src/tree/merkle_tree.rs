use anyhow::Result;
use anyhow::{anyhow, ensure};
use ed25519_dalek::Signature;

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

    /// Opens MerkleTree, based on read byte slices. Call `get_info_instructions_to_read`
    /// before calling this to find out which slices to read. The given slices
    /// need to be in the same order as the instructions from `get_info_instructions_to_read`!
    pub fn open(header_tree: &HeaderTree, slices: Box<[StoreInfo]>) -> Result<Self> {
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

    pub fn flush(&mut self) -> Box<[StoreInfo]> {
        let mut infos_to_flush: Vec<StoreInfo> = Vec::new();
        if self.truncated {
            infos_to_flush.extend(self.flush_truncation());
        }
        infos_to_flush.extend(self.flush_nodes());
        infos_to_flush.into_boxed_slice()
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
