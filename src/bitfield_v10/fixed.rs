const FIXED_BITFIELD_LENGTH: usize = 1024;
// u32 has 4 bytes and a byte has 8 bits
const FIXED_BITFIELD_BITS_PER_ELEM: u64 = 4 * 8;
use std::convert::TryInto;

/// Fixed size bitfield
/// see:
/// https://github.com/holepunchto/bits-to-bytes/blob/main/index.js
/// for implementations.
#[derive(Debug)]
pub struct FixedBitfield {
    pub(crate) parent_index: u64,
    bitfield: [u32; FIXED_BITFIELD_LENGTH],
}

impl FixedBitfield {
    pub fn new(parent_index: u64) -> Self {
        Self {
            parent_index,
            bitfield: [0; FIXED_BITFIELD_LENGTH],
        }
    }

    pub fn get(&self, index: u64) -> bool {
        let n = FIXED_BITFIELD_BITS_PER_ELEM;
        let offset = index & (n - 1);
        let i: usize = ((index - offset) / n)
            .try_into()
            .expect("Could not fit 64 bit integer to usize on this architecture");
        self.bitfield[i] & (1 << offset) != 0
    }

    pub fn set(&mut self, index: u64, value: bool) -> bool {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bitfield_fixed_get_and_set() {
        let mut bitfield = FixedBitfield::new(0);
        assert_eq!(bitfield.get(0), false);
        bitfield.set(0, true);
        assert_eq!(bitfield.get(0), true);

        assert_eq!(bitfield.get(31), false);
        bitfield.set(31, true);
        assert_eq!(bitfield.get(31), true);

        assert_eq!(bitfield.get(32), false);
        bitfield.set(32, true);
        assert_eq!(bitfield.get(32), true);

        assert_eq!(bitfield.get(32767), false);
        bitfield.set(32767, true);
        assert_eq!(bitfield.get(32767), true);
    }
}
