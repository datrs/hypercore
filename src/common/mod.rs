#[cfg(feature = "cache")]
pub(crate) mod cache;
mod error;
mod node;
mod peer;
mod store;

pub use self::error::HypercoreError;
pub use self::node::Node;
pub(crate) use self::node::NodeByteRange;
pub(crate) use self::peer::ValuelessProof;
pub use self::peer::{
    DataBlock, DataHash, DataSeek, DataUpgrade, Proof, RequestBlock, RequestSeek, RequestUpgrade,
};
pub use self::store::Store;
pub(crate) use self::store::{StoreInfo, StoreInfoInstruction, StoreInfoType};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BitfieldUpdate {
    pub(crate) drop: bool,
    pub(crate) start: u64,
    pub(crate) length: u64,
}
