use crate::Node;

#[derive(Debug, Clone, PartialEq)]
/// Block of data
pub struct DataBlock {
    /// Hypercore index
    pub index: u64,
    /// Data block value in bytes
    pub value: Vec<u8>,
    /// TODO: document
    pub nodes: Vec<Node>,
}

#[derive(Debug, Clone, PartialEq)]
/// Data hash
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
