//! Hypercore-specific compact encodings
pub use compact_encoding::{CompactEncoding, State};
use std::ops::{Deref, DerefMut};

use crate::{
    DataBlock, DataHash, DataSeek, DataUpgrade, Node, RequestBlock, RequestSeek, RequestUpgrade,
};

#[derive(Debug, Clone)]
/// Wrapper struct for compact_encoding::State
pub struct HypercoreState(pub State);

impl HypercoreState {
    /// Passthrought to compact_encoding
    pub fn new() -> HypercoreState {
        HypercoreState(State::new())
    }

    /// Passthrought to compact_encoding
    pub fn new_with_size(size: usize) -> (HypercoreState, Box<[u8]>) {
        let (state, buffer) = State::new_with_size(size);
        (HypercoreState(state), buffer)
    }

    /// Passthrought to compact_encoding
    pub fn new_with_start_and_end(start: usize, end: usize) -> HypercoreState {
        HypercoreState(State::new_with_start_and_end(start, end))
    }

    /// Passthrought to compact_encoding
    pub fn from_buffer(buffer: &[u8]) -> HypercoreState {
        HypercoreState(State::from_buffer(buffer))
    }
}

impl Deref for HypercoreState {
    type Target = State;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for HypercoreState {
    fn deref_mut(&mut self) -> &mut State {
        &mut self.0
    }
}

impl CompactEncoding<Node> for HypercoreState {
    fn preencode(&mut self, value: &Node) {
        self.0.preencode(&value.index);
        self.0.preencode(&value.length);
        self.0.preencode_fixed_32();
    }

    fn encode(&mut self, value: &Node, buffer: &mut [u8]) {
        self.0.encode(&value.index, buffer);
        self.0.encode(&value.length, buffer);
        self.0.encode_fixed_32(&value.hash, buffer);
    }

    fn decode(&mut self, buffer: &[u8]) -> Node {
        let index: u64 = self.0.decode(buffer);
        let length: u64 = self.0.decode(buffer);
        let hash: Box<[u8]> = self.0.decode_fixed_32(buffer);
        Node::new(index, hash.to_vec(), length)
    }
}

impl CompactEncoding<Vec<Node>> for HypercoreState {
    fn preencode(&mut self, value: &Vec<Node>) {
        let len = value.len();
        self.0.preencode(&len);
        for val in value.into_iter() {
            self.preencode(val);
        }
    }

    fn encode(&mut self, value: &Vec<Node>, buffer: &mut [u8]) {
        let len = value.len();
        self.0.encode(&len, buffer);
        for val in value {
            self.encode(val, buffer);
        }
    }

    fn decode(&mut self, buffer: &[u8]) -> Vec<Node> {
        let len: usize = self.0.decode(buffer);
        let mut value = Vec::with_capacity(len);
        for _ in 0..len {
            value.push(self.decode(buffer));
        }
        value
    }
}

impl CompactEncoding<RequestBlock> for HypercoreState {
    fn preencode(&mut self, value: &RequestBlock) {
        self.0.preencode(&value.index);
        self.0.preencode(&value.nodes);
    }

    fn encode(&mut self, value: &RequestBlock, buffer: &mut [u8]) {
        self.0.encode(&value.index, buffer);
        self.0.encode(&value.nodes, buffer);
    }

    fn decode(&mut self, buffer: &[u8]) -> RequestBlock {
        let index: u64 = self.0.decode(buffer);
        let nodes: u64 = self.0.decode(buffer);
        RequestBlock { index, nodes }
    }
}

impl CompactEncoding<RequestSeek> for HypercoreState {
    fn preencode(&mut self, value: &RequestSeek) {
        self.0.preencode(&value.bytes);
    }

    fn encode(&mut self, value: &RequestSeek, buffer: &mut [u8]) {
        self.0.encode(&value.bytes, buffer);
    }

    fn decode(&mut self, buffer: &[u8]) -> RequestSeek {
        let bytes: u64 = self.0.decode(buffer);
        RequestSeek { bytes }
    }
}

impl CompactEncoding<RequestUpgrade> for HypercoreState {
    fn preencode(&mut self, value: &RequestUpgrade) {
        self.0.preencode(&value.start);
        self.0.preencode(&value.length);
    }

    fn encode(&mut self, value: &RequestUpgrade, buffer: &mut [u8]) {
        self.0.encode(&value.start, buffer);
        self.0.encode(&value.length, buffer);
    }

    fn decode(&mut self, buffer: &[u8]) -> RequestUpgrade {
        let start: u64 = self.0.decode(buffer);
        let length: u64 = self.0.decode(buffer);
        RequestUpgrade { start, length }
    }
}

impl CompactEncoding<DataBlock> for HypercoreState {
    fn preencode(&mut self, value: &DataBlock) {
        self.0.preencode(&value.index);
        self.0.preencode(&value.value);
        self.preencode(&value.nodes);
    }

    fn encode(&mut self, value: &DataBlock, buffer: &mut [u8]) {
        self.0.encode(&value.index, buffer);
        self.0.encode(&value.value, buffer);
        self.encode(&value.nodes, buffer);
    }

    fn decode(&mut self, buffer: &[u8]) -> DataBlock {
        let index: u64 = self.0.decode(buffer);
        let value: Vec<u8> = self.0.decode(buffer);
        let nodes: Vec<Node> = self.decode(buffer);
        DataBlock {
            index,
            value,
            nodes,
        }
    }
}

impl CompactEncoding<DataHash> for HypercoreState {
    fn preencode(&mut self, value: &DataHash) {
        self.0.preencode(&value.index);
        self.preencode(&value.nodes);
    }

    fn encode(&mut self, value: &DataHash, buffer: &mut [u8]) {
        self.0.encode(&value.index, buffer);
        self.encode(&value.nodes, buffer);
    }

    fn decode(&mut self, buffer: &[u8]) -> DataHash {
        let index: u64 = self.0.decode(buffer);
        let nodes: Vec<Node> = self.decode(buffer);
        DataHash { index, nodes }
    }
}

impl CompactEncoding<DataSeek> for HypercoreState {
    fn preencode(&mut self, value: &DataSeek) {
        self.0.preencode(&value.bytes);
        self.preencode(&value.nodes);
    }

    fn encode(&mut self, value: &DataSeek, buffer: &mut [u8]) {
        self.0.encode(&value.bytes, buffer);
        self.encode(&value.nodes, buffer);
    }

    fn decode(&mut self, buffer: &[u8]) -> DataSeek {
        let bytes: u64 = self.0.decode(buffer);
        let nodes: Vec<Node> = self.decode(buffer);
        DataSeek { bytes, nodes }
    }
}

impl CompactEncoding<DataUpgrade> for HypercoreState {
    fn preencode(&mut self, value: &DataUpgrade) {
        self.0.preencode(&value.start);
        self.0.preencode(&value.length);
        self.preencode(&value.nodes);
        self.preencode(&value.additional_nodes);
        self.0.preencode(&value.signature);
    }

    fn encode(&mut self, value: &DataUpgrade, buffer: &mut [u8]) {
        self.0.encode(&value.start, buffer);
        self.0.encode(&value.length, buffer);
        self.encode(&value.nodes, buffer);
        self.encode(&value.additional_nodes, buffer);
        self.0.encode(&value.signature, buffer);
    }

    fn decode(&mut self, buffer: &[u8]) -> DataUpgrade {
        let start: u64 = self.0.decode(buffer);
        let length: u64 = self.0.decode(buffer);
        let nodes: Vec<Node> = self.decode(buffer);
        let additional_nodes: Vec<Node> = self.decode(buffer);
        let signature: Vec<u8> = self.0.decode(buffer);
        DataUpgrade {
            start,
            length,
            nodes,
            additional_nodes,
            signature,
        }
    }
}
