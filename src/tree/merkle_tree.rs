use anyhow::Result;
use anyhow::{anyhow, ensure};
use ed25519_dalek::Signature;
use futures::future::Either;
use intmap::IntMap;

use crate::common::NodeByteRange;
use crate::common::Proof;
use crate::compact_encoding::State;
use crate::oplog::HeaderTree;
use crate::{
    common::{StoreInfo, StoreInfoInstruction},
    Node,
};
use crate::{
    DataBlock, DataHash, DataSeek, DataUpgrade, RequestBlock, RequestSeek, RequestUpgrade, Store,
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
    unflushed: IntMap<Node>,
    truncated: bool,
    truncate_to: u64,
}

const NODE_SIZE: u64 = 40;

impl MerkleTree {
    /// Opens MerkleTree, based on read infos.
    pub fn open(
        header_tree: &HeaderTree,
        infos: Option<&[StoreInfo]>,
    ) -> Result<Either<Box<[StoreInfoInstruction]>, Self>> {
        match infos {
            None => {
                let root_indices = get_root_indices(&header_tree.length);

                Ok(Either::Left(
                    root_indices
                        .iter()
                        .map(|&index| {
                            StoreInfoInstruction::new_content(
                                Store::Tree,
                                NODE_SIZE * index,
                                NODE_SIZE,
                            )
                        })
                        .collect::<Vec<StoreInfoInstruction>>()
                        .into_boxed_slice(),
                ))
            }
            Some(infos) => {
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

                Ok(Either::Right(Self {
                    roots,
                    length,
                    byte_length,
                    fork: header_tree.fork,
                    unflushed: IntMap::new(),
                    truncated: false,
                    truncate_to: 0,
                    signature: None,
                }))
            }
        }
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
            self.signature = changeset.signature;
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
        let index = self.validate_hypercore_index(hypercore_index)?;
        // Get nodes out of incoming infos
        let nodes: IntMap<Option<Node>> = infos_to_nodes(infos);

        // Start with getting the requested node, which will get the length
        // of the byte range
        let length_result = self.required_node(index, &nodes)?;

        // As for the offset, that might require fetching a lot more nodes whose
        // lengths to sum
        let offset_result = self.byte_offset_from_nodes(index, &nodes)?;

        // Construct response of either instructions (Left) or the result (Right)
        let mut instructions: Vec<StoreInfoInstruction> = Vec::new();
        let mut byte_range = NodeByteRange {
            index: 0,
            length: 0,
        };
        match length_result {
            Either::Left(instruction) => {
                instructions.push(instruction);
            }
            Either::Right(node) => {
                byte_range.length = node.length;
            }
        }
        match offset_result {
            Either::Left(offset_instructions) => {
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

    /// Get the byte offset given hypercore index
    pub fn byte_offset(
        &self,
        hypercore_index: u64,
        infos: Option<&[StoreInfo]>,
    ) -> Result<Either<Box<[StoreInfoInstruction]>, u64>> {
        let index = self.validate_hypercore_index(hypercore_index)?;
        // Get nodes out of incoming infos
        let nodes: IntMap<Option<Node>> = infos_to_nodes(infos);
        // Get offset
        let offset_result = self.byte_offset_from_nodes(index, &nodes)?;
        match offset_result {
            Either::Left(offset_instructions) => {
                Ok(Either::Left(offset_instructions.into_boxed_slice()))
            }
            Either::Right(offset) => Ok(Either::Right(offset)),
        }
    }

    pub fn add_node(&mut self, node: Node) {
        self.unflushed.insert(node.index, node);
    }

    pub fn truncate(
        &mut self,
        length: u64,
        fork: u64,
        infos: Option<&[StoreInfo]>,
    ) -> Result<Either<Box<[StoreInfoInstruction]>, MerkleTreeChangeset>> {
        let head = length * 2;
        let mut full_roots = vec![];
        flat_tree::full_roots(head, &mut full_roots);
        let nodes: IntMap<Option<Node>> = infos_to_nodes(infos);
        let mut changeset = self.changeset();

        let mut instructions: Vec<StoreInfoInstruction> = Vec::new();
        for i in 0..full_roots.len() {
            let root = full_roots[i];
            if i < changeset.roots.len() && changeset.roots[i].index == root {
                continue;
            }
            while changeset.roots.len() > i {
                changeset.roots.pop();
            }

            let node_or_instruction = self.required_node(root, &nodes)?;
            match node_or_instruction {
                Either::Left(instruction) => {
                    instructions.push(instruction);
                }
                Either::Right(node) => {
                    changeset.roots.push(node);
                }
            }
        }

        if instructions.is_empty() {
            while changeset.roots.len() > full_roots.len() {
                changeset.roots.pop();
            }
            changeset.fork = fork;
            changeset.length = length;
            changeset.ancestors = length;
            changeset.byte_length = changeset
                .roots
                .iter()
                .fold(0, |acc, node| acc + node.length);
            changeset.upgraded = true;
            Ok(Either::Right(changeset))
        } else {
            Ok(Either::Left(instructions.into_boxed_slice()))
        }
    }

    /// Creates proof from a requests.
    /// TODO: This is now just a clone of javascript's
    /// https://github.com/hypercore-protocol/hypercore/blob/7e30a0fe353c70ada105840ec1ead6627ff521e7/lib/merkle-tree.js#L604
    /// The implementation should be rewritten to make it clearer.
    pub fn create_proof(
        &self,
        block: Option<&RequestBlock>,
        hash: Option<&RequestBlock>,
        seek: Option<&RequestSeek>,
        upgrade: Option<&RequestUpgrade>,
        infos: Option<&[StoreInfo]>,
    ) -> Result<Either<Box<[StoreInfoInstruction]>, Proof>> {
        let nodes: IntMap<Option<Node>> = infos_to_nodes(infos);
        let mut instructions: Vec<StoreInfoInstruction> = Vec::new();
        let fork = self.fork;
        let signature = self.signature;
        let head = 2 * self.length;
        let (from, to) = if let Some(upgrade) = upgrade.as_ref() {
            let from = upgrade.start * 2;
            (from, from + upgrade.length * 2)
        } else {
            (0, head)
        };
        let indexed = normalize_indexed(block, hash);

        if from >= to || to > head {
            return Err(anyhow!("Invalid upgrade"));
        }

        let mut sub_tree = head;
        let mut p = LocalProof {
            indexed: None,
            seek: None,
            nodes: None,
            upgrade: None,
            additional_upgrade: None,
        };
        let mut untrusted_sub_tree = false;
        if let Some(indexed) = indexed.as_ref() {
            if seek.is_some() && upgrade.is_some() && indexed.index >= from {
                return Err(anyhow!(
                    "Cannot both do a seek and block/hash request when upgrading"
                ));
            }

            if let Some(upgrade) = upgrade.as_ref() {
                untrusted_sub_tree = indexed.last_index < upgrade.start;
            } else {
                untrusted_sub_tree = true;
            }

            if untrusted_sub_tree {
                sub_tree = nodes_to_root(indexed.index, indexed.nodes, to)?;
                let seek_root = if let Some(seek) = seek.as_ref() {
                    let index_or_instructions =
                        self.seek_untrusted_tree(sub_tree, seek.bytes, &nodes)?;
                    match index_or_instructions {
                        Either::Left(new_instructions) => {
                            instructions.extend(new_instructions);
                            return Ok(Either::Left(instructions.into_boxed_slice()));
                        }
                        Either::Right(index) => index,
                    }
                } else {
                    head
                };
                if let Either::Left(new_instructions) = self.block_and_seek_proof(
                    Some(indexed),
                    seek.is_some(),
                    seek_root,
                    sub_tree,
                    &mut p,
                    &nodes,
                )? {
                    instructions.extend(new_instructions);
                }
            } else if upgrade.is_some() {
                sub_tree = indexed.index;
            }
        }
        if !untrusted_sub_tree {
            if let Some(seek) = seek.as_ref() {
                // TODO: This also most likely now doesn't work correctly
                let index_or_instructions = self.seek_from_head(to, seek.bytes, &nodes)?;
                sub_tree = match index_or_instructions {
                    Either::Left(new_instructions) => {
                        instructions.extend(new_instructions);
                        return Ok(Either::Left(instructions.into_boxed_slice()));
                    }
                    Either::Right(index) => index,
                };
            }
        }

        if upgrade.is_some() {
            if let Either::Left(new_instructions) = self.upgrade_proof(
                indexed.as_ref(),
                seek.is_some(),
                from,
                to,
                sub_tree,
                &mut p,
                &nodes,
            )? {
                instructions.extend(new_instructions);
            }

            if head > to {
                if let Either::Left(new_instructions) =
                    self.additional_upgrade_proof(to, head, &mut p, &nodes)?
                {
                    instructions.extend(new_instructions);
                }
            }
        }

        if instructions.is_empty() {
            let (data_block, data_hash): (Option<DataBlock>, Option<DataHash>) =
                if let Some(block) = block.as_ref() {
                    //
                    (
                        Some(DataBlock {
                            index: block.index,
                            value: vec![],           // TODO: this needs to come in
                            nodes: p.nodes.unwrap(), // TODO: unwrap
                        }),
                        None,
                    )
                } else if let Some(hash) = hash.as_ref() {
                    //
                    (
                        None,
                        Some(DataHash {
                            index: hash.index,
                            nodes: p.nodes.unwrap(), // TODO: unwrap
                        }),
                    )
                } else {
                    (None, None)
                };

            let data_seek: Option<DataSeek> = if let Some(seek) = seek.as_ref() {
                if let Some(p_seek) = p.seek {
                    Some(DataSeek {
                        bytes: seek.bytes,
                        nodes: p_seek,
                    })
                } else {
                    None
                }
            } else {
                None
            };

            let data_upgrade: Option<DataUpgrade> = if let Some(upgrade) = upgrade.as_ref() {
                Some(DataUpgrade {
                    start: upgrade.start,
                    length: upgrade.length,
                    nodes: p.upgrade.unwrap(), // TODO: unwrap
                    additional_nodes: if let Some(additional_upgrade) = p.additional_upgrade {
                        additional_upgrade
                    } else {
                        vec![]
                    },
                    signature: signature.unwrap().to_bytes().to_vec(), // TODO: unwrap
                })
            } else {
                None
            };

            Ok(Either::Right(Proof {
                fork,
                block: data_block,
                hash: data_hash,
                seek: data_seek,
                upgrade: data_upgrade,
            }))
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
                    let index = iter.index();
                    if iter.contains(head) && index < head {
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

    /// Validates given hypercore index and returns tree index
    fn validate_hypercore_index(&self, hypercore_index: u64) -> Result<u64> {
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
        Ok(index)
    }

    fn byte_offset_from_nodes(
        &self,
        index: u64,
        nodes: &IntMap<Option<Node>>,
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
                    let node_or_instruction = self.required_node(left_child, nodes)?;
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

    fn required_node(
        &self,
        index: u64,
        nodes: &IntMap<Option<Node>>,
    ) -> Result<Either<StoreInfoInstruction, Node>> {
        match self.node(index, nodes, false)? {
            Either::Left(value) => Ok(Either::Left(value)),
            Either::Right(node) => {
                if let Some(node) = node {
                    Ok(Either::Right(node))
                } else {
                    Err(anyhow!("Node at {} was required", index))
                }
            }
        }
    }

    fn optional_node(
        &self,
        index: u64,
        nodes: &IntMap<Option<Node>>,
    ) -> Result<Either<StoreInfoInstruction, Option<Node>>> {
        self.node(index, nodes, true)
    }

    fn node(
        &self,
        index: u64,
        nodes: &IntMap<Option<Node>>,
        allow_miss: bool,
    ) -> Result<Either<StoreInfoInstruction, Option<Node>>> {
        // First check if unflushed already has the node
        if let Some(node) = self.unflushed.get(index) {
            if node.blank || (self.truncated && node.index >= 2 * self.truncate_to) {
                // The node is either blank or being deleted
                return if allow_miss {
                    Ok(Either::Right(None))
                } else {
                    Err(anyhow!("Could not load node: {}", index))
                };
            }
            return Ok(Either::Right(Some(node.clone())));
        }

        // Then check if it's already in the incoming nodes
        let result = nodes.get(index);
        if let Some(node_maybe) = result {
            if let Some(node) = node_maybe {
                return Ok(Either::Right(Some(node.clone())));
            } else if allow_miss {
                return Ok(Either::Right(None));
            } else {
                return Err(anyhow!("Could not load node: {}", index));
            }
        }

        // If not, retunr an instruction
        let offset = 40 * index;
        let length = 40;
        let info = if allow_miss {
            StoreInfoInstruction::new_content_allow_miss(Store::Tree, offset, length)
        } else {
            StoreInfoInstruction::new_content(Store::Tree, offset, length)
        };
        Ok(Either::Left(info))
    }

    fn seek_from_head(
        &self,
        head: u64,
        bytes: u64,
        nodes: &IntMap<Option<Node>>,
    ) -> Result<Either<Vec<StoreInfoInstruction>, u64>> {
        let mut instructions: Vec<StoreInfoInstruction> = Vec::new();
        let mut roots = vec![];
        flat_tree::full_roots(head, &mut roots);
        let mut bytes = bytes;

        for i in 0..roots.len() {
            let root = roots[i];
            let node_or_instruction = self.required_node(root, nodes)?;
            match node_or_instruction {
                Either::Left(instruction) => {
                    instructions.push(instruction);
                }
                Either::Right(node) => {
                    if bytes == node.length {
                        return Ok(Either::Right(root));
                    }
                    if bytes > node.length {
                        bytes -= node.length;
                        continue;
                    }
                    let instructions_or_result = self.seek_trusted_tree(root, bytes, nodes)?;
                    return match instructions_or_result {
                        Either::Left(new_instructions) => {
                            instructions.extend(new_instructions);
                            Ok(Either::Left(instructions))
                        }
                        Either::Right(index) => Ok(Either::Right(index)),
                    };
                }
            }
        }

        if instructions.is_empty() {
            Ok(Either::Right(head))
        } else {
            Ok(Either::Left(instructions))
        }
    }

    /// Trust that bytes are within the root tree and find the block at bytes.
    fn seek_trusted_tree(
        &self,
        root: u64,
        bytes: u64,
        nodes: &IntMap<Option<Node>>,
    ) -> Result<Either<Vec<StoreInfoInstruction>, u64>> {
        if bytes == 0 {
            return Ok(Either::Right(root));
        }
        let mut iter = flat_tree::Iterator::new(root);
        let mut instructions: Vec<StoreInfoInstruction> = Vec::new();
        let mut bytes = bytes;
        while iter.index() & 1 != 0 {
            let node_or_instruction = self.optional_node(iter.left_child(), nodes)?;
            match node_or_instruction {
                Either::Left(instruction) => {
                    instructions.push(instruction);
                    // Need to break immediately because it is unknown
                    // if this node is the one that will match. This means
                    // this function needs to be called in a loop where incoming
                    // nodes increase with each call.
                    break;
                }
                Either::Right(node) => {
                    if let Some(node) = node {
                        if node.length == bytes {
                            return Ok(Either::Right(iter.index()));
                        }
                        if node.length > bytes {
                            continue;
                        }
                        bytes -= node.length;
                        iter.sibling();
                    } else {
                        iter.parent();
                        return Ok(Either::Right(iter.index()));
                    }
                }
            }
        }
        if instructions.is_empty() {
            Ok(Either::Right(iter.index()))
        } else {
            Ok(Either::Left(instructions))
        }
    }

    /// Try to find the block at bytes without trusting that it *is* within the root passed.
    fn seek_untrusted_tree(
        &self,
        root: u64,
        bytes: u64,
        nodes: &IntMap<Option<Node>>,
    ) -> Result<Either<Vec<StoreInfoInstruction>, u64>> {
        let mut instructions: Vec<StoreInfoInstruction> = Vec::new();
        let offset_or_instructions = self.byte_offset_from_nodes(root, nodes)?;
        let mut bytes = bytes;
        match offset_or_instructions {
            Either::Left(new_instructions) => {
                instructions.extend(new_instructions);
            }
            Either::Right(offset) => {
                if offset > bytes {
                    return Err(anyhow!("Invalid seek"));
                }
                if offset == bytes {
                    return Ok(Either::Right(root));
                }
                bytes -= offset;
                let node_or_instruction = self.required_node(root, nodes)?;
                match node_or_instruction {
                    Either::Left(instruction) => {
                        instructions.push(instruction);
                    }
                    Either::Right(node) => {
                        if node.length <= bytes {
                            return Err(anyhow!("Invalid seek"));
                        }
                    }
                }
            }
        }
        let instructions_or_result = self.seek_trusted_tree(root, bytes, nodes)?;
        match instructions_or_result {
            Either::Left(new_instructions) => {
                instructions.extend(new_instructions);
                Ok(Either::Left(instructions))
            }
            Either::Right(index) => Ok(Either::Right(index)),
        }
    }

    fn block_and_seek_proof(
        &self,
        indexed: Option<&NormalizedIndexed>,
        is_seek: bool,
        seek_root: u64,
        root: u64,
        p: &mut LocalProof,
        nodes: &IntMap<Option<Node>>,
    ) -> Result<Either<Vec<StoreInfoInstruction>, ()>> {
        if let Some(indexed) = indexed {
            let mut iter = flat_tree::Iterator::new(indexed.index);
            let mut instructions: Vec<StoreInfoInstruction> = Vec::new();
            let mut p_nodes: Vec<Node> = Vec::new();

            if !indexed.value {
                let node_or_instruction = self.required_node(iter.index(), nodes)?;
                match node_or_instruction {
                    Either::Left(instruction) => {
                        instructions.push(instruction);
                    }
                    Either::Right(node) => {
                        p_nodes.push(node);
                    }
                }
            }
            while iter.index() != root {
                iter.sibling();
                if is_seek && iter.contains(seek_root) && iter.index() != seek_root {
                    let success_or_instruction =
                        self.seek_proof(seek_root, iter.index(), p, nodes)?;
                    match success_or_instruction {
                        Either::Left(new_instructions) => {
                            instructions.extend(new_instructions);
                        }
                        _ => (),
                    }
                } else {
                    let node_or_instruction = self.required_node(iter.index(), nodes)?;
                    match node_or_instruction {
                        Either::Left(instruction) => {
                            instructions.push(instruction);
                        }
                        Either::Right(node) => {
                            p_nodes.push(node);
                        }
                    }
                }

                iter.parent();
            }
            p.nodes = Some(p_nodes);
            if instructions.is_empty() {
                Ok(Either::Right(()))
            } else {
                Ok(Either::Left(instructions))
            }
        } else {
            self.seek_proof(seek_root, root, p, nodes)
        }
    }

    fn seek_proof(
        &self,
        seek_root: u64,
        root: u64,
        p: &mut LocalProof,
        nodes: &IntMap<Option<Node>>,
    ) -> Result<Either<Vec<StoreInfoInstruction>, ()>> {
        let mut iter = flat_tree::Iterator::new(seek_root);
        let mut instructions: Vec<StoreInfoInstruction> = Vec::new();
        let mut seek_nodes: Vec<Node> = Vec::new();
        let node_or_instruction = self.required_node(iter.index(), nodes)?;
        match node_or_instruction {
            Either::Left(instruction) => {
                instructions.push(instruction);
            }
            Either::Right(node) => {
                seek_nodes.push(node);
            }
        }

        while iter.index() != root {
            iter.sibling();
            let node_or_instruction = self.required_node(iter.index(), nodes)?;
            match node_or_instruction {
                Either::Left(instruction) => {
                    instructions.push(instruction);
                }
                Either::Right(node) => {
                    seek_nodes.push(node);
                }
            }
            iter.parent();
        }
        p.seek = Some(seek_nodes);
        if instructions.is_empty() {
            Ok(Either::Right(()))
        } else {
            Ok(Either::Left(instructions))
        }
    }

    fn upgrade_proof(
        &self,
        indexed: Option<&NormalizedIndexed>,
        is_seek: bool,
        from: u64,
        to: u64,
        sub_tree: u64,
        p: &mut LocalProof,
        nodes: &IntMap<Option<Node>>,
    ) -> Result<Either<Vec<StoreInfoInstruction>, ()>> {
        let mut instructions: Vec<StoreInfoInstruction> = Vec::new();
        let mut upgrade: Vec<Node> = Vec::new();
        let mut has_upgrade = false;

        if from == 0 {
            has_upgrade = true;
        }

        let mut iter = flat_tree::Iterator::new(0);
        let mut has_full_root = iter.full_root(to);
        while has_full_root {
            // check if they already have the node
            if iter.index() + iter.factor() / 2 < from {
                iter.next_tree();
                has_full_root = iter.full_root(to);
                continue;
            }

            // connect existing tree
            if !has_upgrade && iter.contains(from - 2) {
                has_upgrade = true;
                let root = iter.index();
                let target = from - 2;

                iter.seek(target);

                while iter.index() != root {
                    iter.sibling();
                    if iter.index() > target {
                        if p.nodes.is_none() && p.seek.is_none() && iter.contains(sub_tree) {
                            let success_or_instructions = self.block_and_seek_proof(
                                indexed,
                                is_seek,
                                sub_tree,
                                iter.index(),
                                p,
                                nodes,
                            )?;
                            if let Either::Left(new_instructions) = success_or_instructions {
                                instructions.extend(new_instructions);
                            }
                        } else {
                            let node_or_instruction = self.required_node(iter.index(), nodes)?;
                            match node_or_instruction {
                                Either::Left(instruction) => {
                                    instructions.push(instruction);
                                }
                                Either::Right(node) => upgrade.push(node),
                            }
                        }
                    }
                    iter.parent();
                }

                iter.next_tree();
                has_full_root = iter.full_root(to);
                continue;
            }

            if !has_upgrade {
                has_upgrade = true;
            }

            // if the subtree included is a child of this tree, include that one
            // instead of a dup node
            if p.nodes.is_none() && p.seek.is_none() && iter.contains(sub_tree) {
                let success_or_instructions =
                    self.block_and_seek_proof(indexed, is_seek, sub_tree, iter.index(), p, nodes)?;
                if let Either::Left(new_instructions) = success_or_instructions {
                    instructions.extend(new_instructions);
                }
                iter.next_tree();
                has_full_root = iter.full_root(to);
                continue;
            }

            // add root (can be optimised since the root might be in tree.roots)
            let node_or_instruction = self.required_node(iter.index(), nodes)?;
            match node_or_instruction {
                Either::Left(instruction) => {
                    instructions.push(instruction);
                }
                Either::Right(node) => upgrade.push(node),
            }

            iter.next_tree();
            has_full_root = iter.full_root(to);
        }

        if has_upgrade {
            p.upgrade = Some(upgrade);
        }

        if instructions.is_empty() {
            Ok(Either::Right(()))
        } else {
            Ok(Either::Left(instructions))
        }
    }

    fn additional_upgrade_proof(
        &self,
        from: u64,
        to: u64,
        p: &mut LocalProof,
        nodes: &IntMap<Option<Node>>,
    ) -> Result<Either<Vec<StoreInfoInstruction>, ()>> {
        let mut instructions: Vec<StoreInfoInstruction> = Vec::new();
        let mut additional_upgrade: Vec<Node> = Vec::new();
        let mut has_additional_upgrade = false;

        if from == 0 {
            has_additional_upgrade = true;
        }

        let mut iter = flat_tree::Iterator::new(0);
        let mut has_full_root = iter.full_root(to);
        while has_full_root {
            // check if they already have the node
            if iter.index() + iter.factor() / 2 < from {
                iter.next_tree();
                has_full_root = iter.full_root(to);
                continue;
            }

            // connect existing tree
            if !has_additional_upgrade && iter.contains(from - 2) {
                has_additional_upgrade = true;
                let root = iter.index();
                let target = from - 2;

                iter.seek(target);

                while iter.index() != root {
                    iter.sibling();
                    if iter.index() > target {
                        let node_or_instruction = self.required_node(iter.index(), nodes)?;
                        match node_or_instruction {
                            Either::Left(instruction) => {
                                instructions.push(instruction);
                            }
                            Either::Right(node) => additional_upgrade.push(node),
                        }
                    }
                    iter.parent();
                }

                iter.next_tree();
                has_full_root = iter.full_root(to);
                continue;
            }

            if !has_additional_upgrade {
                has_additional_upgrade = true;
            }

            // add root (can be optimised since the root is in tree.roots)
            let node_or_instruction = self.required_node(iter.index(), nodes)?;
            match node_or_instruction {
                Either::Left(instruction) => {
                    instructions.push(instruction);
                }
                Either::Right(node) => additional_upgrade.push(node),
            }

            iter.next_tree();
            has_full_root = iter.full_root(to);
        }

        if has_additional_upgrade {
            p.additional_upgrade = Some(additional_upgrade);
        }

        if instructions.is_empty() {
            Ok(Either::Right(()))
        } else {
            Ok(Either::Left(instructions))
        }
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

fn infos_to_nodes(infos: Option<&[StoreInfo]>) -> IntMap<Option<Node>> {
    match infos {
        Some(infos) => {
            let mut nodes: IntMap<Option<Node>> = IntMap::with_capacity(infos.len());
            for info in infos {
                let index = index_from_info(&info);
                if !info.miss {
                    let node = node_from_bytes(&index, info.data.as_ref().unwrap());
                    nodes.insert(index, Some(node));
                } else {
                    nodes.insert(index, None);
                }
            }
            nodes
        }
        None => IntMap::new(),
    }
}

#[derive(Debug, Copy, Clone)]
struct NormalizedIndexed {
    pub value: bool,
    pub index: u64,
    pub nodes: u64,
    pub last_index: u64,
}

fn normalize_indexed(
    block: Option<&RequestBlock>,
    hash: Option<&RequestBlock>,
) -> Option<NormalizedIndexed> {
    if let Some(block) = block {
        Some(NormalizedIndexed {
            value: true,
            index: block.index * 2,
            nodes: block.nodes,
            last_index: block.index,
        })
    } else if let Some(hash) = hash {
        Some(NormalizedIndexed {
            value: false,
            index: hash.index,
            nodes: hash.nodes,
            last_index: flat_tree::right_span(hash.index) / 2,
        })
    } else {
        None
    }
}

/// Struct to use for local building of proof
#[derive(Debug, Clone)]
struct LocalProof {
    pub indexed: Option<NormalizedIndexed>,
    pub seek: Option<Vec<Node>>,
    pub nodes: Option<Vec<Node>>,
    pub upgrade: Option<Vec<Node>>,
    pub additional_upgrade: Option<Vec<Node>>,
}

fn nodes_to_root(index: u64, nodes: u64, head: u64) -> Result<u64> {
    let mut iter = flat_tree::Iterator::new(index);
    for _ in 0..nodes {
        iter.parent();
        if iter.contains(head) {
            return Err(anyhow!("Nodes is out of bounds"));
        }
    }
    Ok(iter.index())
}
