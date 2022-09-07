pub(crate) const FIXED_BITFIELD_LENGTH: usize = 1024;
pub(crate) const FIXED_BITFIELD_BYTES_LENGTH: usize = FIXED_BITFIELD_LENGTH * 4;
// u32 has 4 bytes and a byte has 8 bits
const FIXED_BITFIELD_BITS_PER_ELEM: u32 = 4 * 8;
use std::convert::TryInto;

/// Fixed size bitfield
/// see:
/// https://github.com/holepunchto/bits-to-bytes/blob/main/index.js
/// for implementations.
#[derive(Debug)]
pub struct FixedBitfield {
    pub(crate) parent_index: u64,
    pub(crate) dirty: bool,
    bitfield: [u32; FIXED_BITFIELD_LENGTH],
}

impl FixedBitfield {
    pub fn new(parent_index: u64) -> Self {
        Self {
            parent_index,
            dirty: false,
            bitfield: [0; FIXED_BITFIELD_LENGTH],
        }
    }

    pub fn from_data(parent_index: u64, data_index: usize, data: &[u8]) -> Self {
        let mut bitfield = [0; FIXED_BITFIELD_LENGTH];
        if data.len() >= data_index + 4 {
            let mut i = data_index;
            let limit = std::cmp::min(data_index + FIXED_BITFIELD_BYTES_LENGTH, data.len()) - 4;
            while i <= limit {
                let value: u32 = ((data[i] as u32) << 0)
                    | ((data[i + 1] as u32) << 8)
                    | ((data[i + 2] as u32) << 16)
                    | ((data[i + 3] as u32) << 24);
                bitfield[i / 4] = value;
                i += 4;
            }
        }
        Self {
            parent_index,
            dirty: false,
            bitfield,
        }
    }

    pub fn to_bytes(&self) -> Box<[u8]> {
        let mut data: [u8; FIXED_BITFIELD_BYTES_LENGTH] = [0; FIXED_BITFIELD_BYTES_LENGTH];
        let mut i = 0;
        for elem in self.bitfield {
            let bytes = &elem.to_le_bytes();
            data[i] = bytes[0];
            data[i + 1] = bytes[1];
            data[i + 2] = bytes[2];
            data[i + 3] = bytes[3];
            i += 4;
        }
        data.into()
    }

    pub fn get(&self, index: u32) -> bool {
        let n = FIXED_BITFIELD_BITS_PER_ELEM;
        let offset = index & (n - 1);
        let i: usize = ((index - offset) / n)
            .try_into()
            .expect("Could not fit 64 bit integer to usize on this architecture");
        self.bitfield[i] & (1 << offset) != 0
    }

    pub fn set(&mut self, index: u32, value: bool) -> bool {
        let n = FIXED_BITFIELD_BITS_PER_ELEM;
        let offset = index & (n - 1);
        let i: usize = ((index - offset) / n)
            .try_into()
            .expect("Could not fit 64 bit integer to usize on this architecture");
        let mask = 1 << offset;

        if value {
            if (self.bitfield[i] & mask) != 0 {
                return false;
            }
        } else {
            if (self.bitfield[i] & mask) == 0 {
                return false;
            }
        }
        self.bitfield[i] ^= mask;
        true
    }

    pub fn set_range(&mut self, start: u32, length: u32, value: bool) -> bool {
        let end: u32 = start + length;
        let n = FIXED_BITFIELD_BITS_PER_ELEM;

        let mut remaining: i64 = end as i64 - start as i64;
        let mut offset = start & (n - 1);
        let mut i: usize = ((start - offset) / n).try_into().unwrap();

        let mut changed = false;

        while remaining > 0 {
            let base: u32 = 2;
            let power: u32 = std::cmp::min(remaining, (n - offset).into())
                .try_into()
                .unwrap();
            let mask_seed = if power == 32 {
                // Go directly to this maximum value as the below
                // calculation overflows as 1 is subtracted after
                // the power.
                u32::MAX
            } else {
                base.pow(power) - 1
            };
            let mask: u32 = mask_seed << offset;

            if value {
                if (self.bitfield[i] & mask) != mask {
                    self.bitfield[i] |= mask;
                    changed = true;
                }
            } else {
                if (self.bitfield[i] & mask) != 0 {
                    self.bitfield[i] &= !mask;
                    changed = true;
                }
            }

            remaining -= (n - offset) as i64;
            offset = 0;
            i += 1;
        }

        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_value_range(bitfield: &FixedBitfield, start: u32, length: u32, value: bool) {
        for i in start..start + length {
            assert_eq!(bitfield.get(i), value);
        }
    }

    #[test]
    fn bitfield_fixed_get_and_set() {
        let mut bitfield = FixedBitfield::new(0);
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
    }

    #[test]
    fn bitfield_fixed_set_range() {
        let mut bitfield = FixedBitfield::new(0);
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
    }
}
