/// Represents a slice to a known buffer. Useful for indicating changes that should be made to random
/// access storages. Data value of None, indicates that the should be truncated to the start
/// position.
#[derive(Debug)]
pub struct BufferSlice {
    pub(crate) index: u64,
    pub(crate) data: Option<Box<[u8]>>,
}
