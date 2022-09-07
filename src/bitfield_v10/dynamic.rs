use super::fixed::FixedBitfield;

const DYNAMIC_BITFIELD_PAGE_SIZE: usize = 32768;

/// Dynamic sized bitfield, uses a collection of `FixeddBitfield` elements.
#[derive(Debug)]
pub struct DynamicBitfield {
    pages: intmap::IntMap<FixedBitfield>,
}

impl DynamicBitfield {
    pub fn new() -> Self {
        Self {
            pages: intmap::IntMap::new(),
        }
    }
}
