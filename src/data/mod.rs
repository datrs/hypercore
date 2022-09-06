use crate::common::BufferSlice;

/// Block store
#[derive(Debug, Default)]
pub struct BlockStore {}

impl BlockStore {
    pub fn append_batch(
        &self,
        batch: &[&[u8]],
        batch_length: usize,
        byte_length: u64,
    ) -> BufferSlice {
        let mut buffer: Vec<u8> = Vec::with_capacity(batch_length);
        for data in batch.iter() {
            buffer.extend_from_slice(data);
        }
        BufferSlice {
            index: byte_length,
            data: Some(buffer.into_boxed_slice()),
        }
    }
}
