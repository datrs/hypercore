/// Represents a slice to a known buffer. Useful for indicating changes that should be made to random
/// access storages. Data value of None, indicates that the should be truncated to the start
/// position.
#[derive(Debug)]
pub struct BufferSlice {
    pub(crate) index: u64,
    pub(crate) data: Option<Box<[u8]>>,
}

impl BufferSlice {
    pub fn get_data_mut(&self) -> Box<[u8]> {
        let data = self.data.as_ref().unwrap();
        let mut buffer = vec![0; data.len()];
        buffer.copy_from_slice(&data);
        buffer.into_boxed_slice()
    }
}

/// Represents an instruction to read a known buffer.
#[derive(Debug)]
pub struct BufferSliceInstruction {
    pub(crate) index: u64,
    pub(crate) len: u64,
}
