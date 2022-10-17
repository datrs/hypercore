//! Hypercore-specific compact encodings
use super::{CompactEncoding, State};
use crate::{
    DataBlock, DataHash, DataSeek, DataUpgrade, Node, RequestBlock, RequestSeek, RequestUpgrade,
};

impl CompactEncoding<Node> for State {
    fn preencode(&mut self, value: &Node) {
        self.preencode(&value.index);
        self.preencode(&value.length);
        self.preencode_fixed_32();
    }

    fn encode(&mut self, value: &Node, buffer: &mut [u8]) {
        self.encode(&value.index, buffer);
        self.encode(&value.length, buffer);
        self.encode_fixed_32(&value.hash, buffer);
    }

    fn decode(&mut self, buffer: &[u8]) -> Node {
        let index: u64 = self.decode(buffer);
        let length: u64 = self.decode(buffer);
        let hash: Box<[u8]> = self.decode_fixed_32(buffer);
        Node::new(index, hash.to_vec(), length)
    }
}

impl CompactEncoding<Vec<Node>> for State {
    fn preencode(&mut self, value: &Vec<Node>) {
        let len = value.len();
        self.preencode(&len);
        for val in value.into_iter() {
            self.preencode(val);
        }
    }

    fn encode(&mut self, value: &Vec<Node>, buffer: &mut [u8]) {
        let len = value.len();
        self.encode(&len, buffer);
        for val in value {
            self.encode(val, buffer);
        }
    }

    fn decode(&mut self, buffer: &[u8]) -> Vec<Node> {
        let len: usize = self.decode(buffer);
        let mut value = Vec::with_capacity(len);
        for _ in 0..len {
            value.push(self.decode(buffer));
        }
        value
    }
}

impl CompactEncoding<RequestBlock> for State {
    fn preencode(&mut self, value: &RequestBlock) {
        self.preencode(&value.index);
        self.preencode(&value.nodes);
    }

    fn encode(&mut self, value: &RequestBlock, buffer: &mut [u8]) {
        self.encode(&value.index, buffer);
        self.encode(&value.nodes, buffer);
    }

    fn decode(&mut self, buffer: &[u8]) -> RequestBlock {
        let index: u64 = self.decode(buffer);
        let nodes: u64 = self.decode(buffer);
        RequestBlock { index, nodes }
    }
}

impl CompactEncoding<RequestSeek> for State {
    fn preencode(&mut self, value: &RequestSeek) {
        self.preencode(&value.bytes);
    }

    fn encode(&mut self, value: &RequestSeek, buffer: &mut [u8]) {
        self.encode(&value.bytes, buffer);
    }

    fn decode(&mut self, buffer: &[u8]) -> RequestSeek {
        let bytes: u64 = self.decode(buffer);
        RequestSeek { bytes }
    }
}

impl CompactEncoding<RequestUpgrade> for State {
    fn preencode(&mut self, value: &RequestUpgrade) {
        self.preencode(&value.start);
        self.preencode(&value.length);
    }

    fn encode(&mut self, value: &RequestUpgrade, buffer: &mut [u8]) {
        self.encode(&value.start, buffer);
        self.encode(&value.length, buffer);
    }

    fn decode(&mut self, buffer: &[u8]) -> RequestUpgrade {
        let start: u64 = self.decode(buffer);
        let length: u64 = self.decode(buffer);
        RequestUpgrade { start, length }
    }
}

impl CompactEncoding<DataBlock> for State {
    fn preencode(&mut self, value: &DataBlock) {
        self.preencode(&value.index);
        self.preencode(&value.value);
        self.preencode(&value.nodes);
    }

    fn encode(&mut self, value: &DataBlock, buffer: &mut [u8]) {
        self.encode(&value.index, buffer);
        self.encode(&value.value, buffer);
        self.encode(&value.nodes, buffer);
    }

    fn decode(&mut self, buffer: &[u8]) -> DataBlock {
        let index: u64 = self.decode(buffer);
        let value: Vec<u8> = self.decode(buffer);
        let nodes: Vec<Node> = self.decode(buffer);
        DataBlock {
            index,
            value,
            nodes,
        }
    }
}

impl CompactEncoding<DataHash> for State {
    fn preencode(&mut self, value: &DataHash) {
        self.preencode(&value.index);
        self.preencode(&value.nodes);
    }

    fn encode(&mut self, value: &DataHash, buffer: &mut [u8]) {
        self.encode(&value.index, buffer);
        self.encode(&value.nodes, buffer);
    }

    fn decode(&mut self, buffer: &[u8]) -> DataHash {
        let index: u64 = self.decode(buffer);
        let nodes: Vec<Node> = self.decode(buffer);
        DataHash { index, nodes }
    }
}

impl CompactEncoding<DataSeek> for State {
    fn preencode(&mut self, value: &DataSeek) {
        self.preencode(&value.bytes);
        self.preencode(&value.nodes);
    }

    fn encode(&mut self, value: &DataSeek, buffer: &mut [u8]) {
        self.encode(&value.bytes, buffer);
        self.encode(&value.nodes, buffer);
    }

    fn decode(&mut self, buffer: &[u8]) -> DataSeek {
        let bytes: u64 = self.decode(buffer);
        let nodes: Vec<Node> = self.decode(buffer);
        DataSeek { bytes, nodes }
    }
}

impl CompactEncoding<DataUpgrade> for State {
    fn preencode(&mut self, value: &DataUpgrade) {
        self.preencode(&value.start);
        self.preencode(&value.length);
        self.preencode(&value.nodes);
        self.preencode(&value.additional_nodes);
        self.preencode(&value.signature);
    }

    fn encode(&mut self, value: &DataUpgrade, buffer: &mut [u8]) {
        self.encode(&value.start, buffer);
        self.encode(&value.length, buffer);
        self.encode(&value.nodes, buffer);
        self.encode(&value.additional_nodes, buffer);
        self.encode(&value.signature, buffer);
    }

    fn decode(&mut self, buffer: &[u8]) -> DataUpgrade {
        let start: u64 = self.decode(buffer);
        let length: u64 = self.decode(buffer);
        let nodes: Vec<Node> = self.decode(buffer);
        let additional_nodes: Vec<Node> = self.decode(buffer);
        let signature: Vec<u8> = self.decode(buffer);
        DataUpgrade {
            start,
            length,
            nodes,
            additional_nodes,
            signature,
        }
    }
}
