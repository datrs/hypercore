use crate::common::{NodeByteRange, Store, StoreInfo, StoreInfoInstruction};
use futures::future::Either;

/// Block store
#[derive(Debug, Default)]
pub(crate) struct BlockStore {}

impl BlockStore {
    pub(crate) fn append_batch(
        &self,
        batch: &[&[u8]],
        batch_length: usize,
        byte_length: u64,
    ) -> StoreInfo {
        let mut buffer: Vec<u8> = Vec::with_capacity(batch_length);
        for data in batch.iter() {
            buffer.extend_from_slice(data);
        }
        StoreInfo::new_content(Store::Data, byte_length, &buffer)
    }

    pub(crate) fn put(&self, value: &[u8], offset: u64) -> StoreInfo {
        StoreInfo::new_content(Store::Data, offset, value)
    }

    pub(crate) fn read(
        &self,
        byte_range: &NodeByteRange,
        info: Option<StoreInfo>,
    ) -> Either<StoreInfoInstruction, Box<[u8]>> {
        if let Some(info) = info {
            Either::Right(info.data.unwrap())
        } else {
            Either::Left(StoreInfoInstruction::new_content(
                Store::Data,
                byte_range.index,
                byte_range.length,
            ))
        }
    }

    /// Clears a segment, returns infos to write to storage.
    pub(crate) fn clear(&mut self, start: u64, length: u64) -> StoreInfo {
        StoreInfo::new_delete(Store::Data, start, length)
    }
}
