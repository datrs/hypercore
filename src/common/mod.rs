mod error;
mod node;
mod peer;
mod store;

pub use self::error::HypercoreError;
pub use self::node::{Node, NodeByteRange};
pub use self::peer::{
    DataBlock, DataHash, DataSeek, DataUpgrade, Proof, RequestBlock, RequestSeek, RequestUpgrade,
    ValuelessProof,
};
pub use self::store::{Store, StoreInfo, StoreInfoInstruction, StoreInfoType};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BitfieldUpdate {
    pub(crate) drop: bool,
    pub(crate) start: u64,
    pub(crate) length: u64,
}
