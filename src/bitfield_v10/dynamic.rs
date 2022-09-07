use crate::{
    common::{StoreInfo, StoreInfoInstruction},
    Store,
};

use super::fixed::FixedBitfield;

const DYNAMIC_BITFIELD_PAGE_SIZE: usize = 32768;

/// Dynamic sized bitfield, uses a map of `FixedBitfield` elements.
/// See:
/// https://github.com/hypercore-protocol/hypercore/blob/master/lib/bitfield.js
/// for reference.
#[derive(Debug)]
pub struct DynamicBitfield {
    pages: intmap::IntMap<FixedBitfield>,
}

impl DynamicBitfield {
    /// Gets info instruction to read based on the bitfield store length
    pub fn get_info_instruction_to_read(bitfield_store_length: u64) -> StoreInfoInstruction {
        // Read only multiples of 4 bytes. Javascript:
        //    const size = st.size - (st.size & 3)
        let length = bitfield_store_length - (bitfield_store_length & 3);
        StoreInfoInstruction::new_content(Store::Bitfield, 0, length)
    }

    pub fn open(info: StoreInfo) -> Self {
        Self {
            pages: intmap::IntMap::new(),
        }
    }
}
