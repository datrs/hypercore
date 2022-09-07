use super::fixed::FixedBitfield;
use crate::{
    common::{StoreInfo, StoreInfoInstruction},
    Store,
};
use std::{cell::RefCell, convert::TryInto};

const DYNAMIC_BITFIELD_PAGE_SIZE: usize = 32768;

/// Dynamic sized bitfield, uses a map of `FixedBitfield` elements.
/// See:
/// https://github.com/hypercore-protocol/hypercore/blob/master/lib/bitfield.js
/// for reference.
#[derive(Debug)]
pub struct DynamicBitfield {
    pages: intmap::IntMap<RefCell<FixedBitfield>>,
    unflushed: Vec<u64>,
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
            unflushed: vec![],
        }
    }

    pub fn get(&self, index: u64) -> bool {
        let j = index & (DYNAMIC_BITFIELD_PAGE_SIZE as u64 - 1);
        let i = (index - j) / DYNAMIC_BITFIELD_PAGE_SIZE as u64;

        if !self.pages.contains_key(i) {
            false
        } else {
            let p = self.pages.get(i).unwrap().borrow();
            p.get(j.try_into().expect("Index should have fit into u32"))
        }
    }

    pub fn set(&mut self, index: u64, value: bool) -> bool {
        let j = index & (DYNAMIC_BITFIELD_PAGE_SIZE as u64 - 1);
        let i = (index - j) / DYNAMIC_BITFIELD_PAGE_SIZE as u64;

        if !self.pages.contains_key(i) {
            self.pages.insert(i, RefCell::new(FixedBitfield::new(i)));
        }

        let mut p = self.pages.get_mut(i).unwrap().borrow_mut();
        let changed: bool = p.set(j.try_into().expect("Index should have fit into u32"), value);

        if changed && !p.dirty {
            p.dirty = true;
            self.unflushed.push(i);
        }
        changed
    }

    pub fn set_range(&mut self, start: u64, length: u64, value: bool) {
        let mut j = start & (DYNAMIC_BITFIELD_PAGE_SIZE as u64 - 1);
        let mut i = (start - j) / (DYNAMIC_BITFIELD_PAGE_SIZE as u64);
        let mut length = length;

        while length > 0 {
            if !self.pages.contains_key(i) {
                self.pages.insert(i, RefCell::new(FixedBitfield::new(i)));
            }
            let mut p = self.pages.get_mut(i).unwrap().borrow_mut();

            let end = std::cmp::min(j + length, DYNAMIC_BITFIELD_PAGE_SIZE as u64);

            let range_start: u32 = j
                .try_into()
                .expect("Range start should have fit into a u32");
            let range_end: u32 = (end - j)
                .try_into()
                .expect("Range end should have fit into a u32");

            let changed = p.set_range(range_start, range_end, value);
            if changed && !p.dirty {
                p.dirty = true;
                self.unflushed.push(i);
            }

            j = 0;
            i += 1;
            length -= range_end as u64;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_value_range(bitfield: &DynamicBitfield, start: u64, length: u64, value: bool) {
        for i in start..start + length {
            assert_eq!(bitfield.get(i), value);
        }
    }

    #[test]
    fn bitfield_dynamic_get_and_set() {
        let mut bitfield = DynamicBitfield::open(StoreInfo::new_content(Store::Bitfield, 0, &[]));
        assert_value_range(&bitfield, 0, 9, false);
        bitfield.set(0, true);
        assert_eq!(bitfield.get(0), true);

        assert_value_range(&bitfield, 1, 63, false);
        bitfield.set(31, true);
        assert_eq!(bitfield.get(31), true);

        assert_value_range(&bitfield, 32, 32, false);
        assert_eq!(bitfield.get(32), false);
        bitfield.set(32, true);
        assert_eq!(bitfield.get(32), true);
        assert_value_range(&bitfield, 33, 31, false);

        assert_value_range(&bitfield, 32760, 8, false);
        assert_eq!(bitfield.get(32767), false);
        bitfield.set(32767, true);
        assert_eq!(bitfield.get(32767), true);
        assert_value_range(&bitfield, 32760, 7, false);

        // Now for over one fixed bitfield values
        bitfield.set(32768, true);
        assert_value_range(&bitfield, 32767, 2, true);
        assert_value_range(&bitfield, 32769, 9, false);

        bitfield.set(10000000, true);
        assert_eq!(bitfield.get(10000000), true);
        assert_value_range(&bitfield, 9999990, 10, false);
        assert_value_range(&bitfield, 10000001, 9, false);
    }

    #[test]
    fn bitfield_dynamic_set_range() {
        let mut bitfield = DynamicBitfield::open(StoreInfo::new_content(Store::Bitfield, 0, &[]));
        bitfield.set_range(0, 2, true);
        assert_value_range(&bitfield, 0, 2, true);
        assert_value_range(&bitfield, 3, 61, false);

        bitfield.set_range(2, 3, true);
        assert_value_range(&bitfield, 0, 5, true);
        assert_value_range(&bitfield, 5, 59, false);

        bitfield.set_range(1, 3, false);
        assert_eq!(bitfield.get(0), true);
        assert_value_range(&bitfield, 1, 3, false);
        assert_value_range(&bitfield, 4, 1, true);
        assert_value_range(&bitfield, 5, 59, false);

        bitfield.set_range(30, 30070, true);
        assert_value_range(&bitfield, 5, 25, false);
        assert_value_range(&bitfield, 30, 100, true);
        assert_value_range(&bitfield, 30050, 50, true);
        assert_value_range(&bitfield, 31000, 50, false);

        bitfield.set_range(32750, 18, true);
        assert_value_range(&bitfield, 32750, 18, true);

        bitfield.set_range(32765, 3, false);
        assert_value_range(&bitfield, 32750, 15, true);
        assert_value_range(&bitfield, 32765, 3, false);

        // Now for over one fixed bitfield values
        bitfield.set_range(32765, 15, true);
        assert_value_range(&bitfield, 32765, 15, true);
        assert_value_range(&bitfield, 32780, 9, false);
        bitfield.set_range(32766, 3, false);
        assert_value_range(&bitfield, 32766, 3, false);

        bitfield.set_range(10000000, 50, true);
        assert_value_range(&bitfield, 9999990, 9, false);
        assert_value_range(&bitfield, 10000050, 9, false);

        bitfield.set_range(10000010, 10, false);
        assert_value_range(&bitfield, 10000000, 10, true);
        assert_value_range(&bitfield, 10000010, 10, false);
        assert_value_range(&bitfield, 10000020, 30, true);
        assert_value_range(&bitfield, 10000050, 9, false);
    }
}
