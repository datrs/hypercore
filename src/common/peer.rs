use crate::Node;

#[derive(Debug, Clone, PartialEq)]
/// Request of a DataBlock or DataHash from peer
pub struct RequestBlock {
    /// Hypercore index
    pub index: u64,
    /// TODO: document
    pub nodes: u64,
}

#[derive(Debug, Clone, PartialEq)]
/// Request of a DataSeek from peer
pub struct RequestSeek {
    /// TODO: document
    pub bytes: u64,
}

#[derive(Debug, Clone, PartialEq)]
/// Request of a DataUpgrade from peer
pub struct RequestUpgrade {
    /// Hypercore start index
    pub start: u64,
    /// Length of elements
    pub length: u64,
}

#[derive(Debug, Clone, PartialEq)]
/// Block of data to peer
pub struct DataBlock {
    /// Hypercore index
    pub index: u64,
    /// Data block value in bytes
    pub value: Vec<u8>,
    /// TODO: document
    pub nodes: Vec<Node>,
}

#[derive(Debug, Clone, PartialEq)]
/// Data hash to peer
pub struct DataHash {
    /// Hypercore index
    pub index: u64,
    /// TODO: document
    pub nodes: Vec<Node>,
}

#[derive(Debug, Clone, PartialEq)]
/// TODO: Document
pub struct DataSeek {
    /// TODO: Document
    pub bytes: u64,
    /// TODO: Document
    pub nodes: Vec<Node>,
}

#[derive(Debug, Clone, PartialEq)]
/// TODO: Document
pub struct DataUpgrade {
    /// TODO: Document
    pub start: u64,
    /// TODO: Document
    pub length: u64,
    /// TODO: Document
    pub nodes: Vec<Node>,
    /// TODO: Document
    pub additional_nodes: Vec<Node>,
    /// TODO: Document
    pub signature: Vec<u8>,
}
