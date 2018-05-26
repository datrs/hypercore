//! Masks used to determine how to update bytes.
//!
//! This piece of code is still a bit unclear; lots of magic numbers. It'd be
//! good to figure out what things mean.

#[derive(Debug)]
pub(super) struct Masks {
  pub index_update: Vec<u8>,
  pub index_iterate: Vec<u8>,
  pub data_iterate: Vec<u8>,
  pub data_update: Vec<u8>,
  pub map_parent_right: Vec<u8>,
  pub map_parent_left: Vec<u8>,
  pub next_data_0_bit: Vec<i16>,
  pub next_index_0_bit: Vec<i16>,
  pub total_1_bits: Vec<u8>,
}

// Masks are underscored at every 8 bytes.
impl Masks {
  pub fn new() -> Self {
    let index_update = vec![
      0b00111111, //  63
      0b11001111, // 207
      0b11110011, // 243
      0b11111100, // 252
    ];

    let index_iterate = vec![
      0b00000000, //   0
      0b11000000, // 192
      0b11110000, // 240
      0b11111100, // 252
    ];

    let data_iterate = vec![
      0b10000000, // 128
      0b11000000, // 192
      0b11100000, // 224
      0b11110000, // 240
      0b11111000, // 248
      0b11111100, // 252
      0b11111110, // 254
      0b11111111, // 255
    ];

    let data_update = vec![
      0b01111111, // 127
      0b10111111, // 191
      0b11011111, // 223
      0b11101111, // 223
      0b11110111, // 247
      0b11111011, // 251
      0b11111101, // 253
      0b11111110, // 254
    ];

    let mut map_parent_right = vec![0; 256];
    let mut map_parent_left = vec![0; 256];
    let mut next_data_0_bit = vec![0; 256];
    let mut next_index_0_bit = vec![0; 256];
    let mut total_1_bits = vec![0; 256];

    for i in 0..256 {
      let a = (i & (15 << 4)) >> 4;
      let b = i & 15;
      // Lookup table for how many `1`s exist in a number between 0 and 16 in
      // binary notation. It's called a "nibble" because it's half an octet.
      let nibble = vec![0, 1, 1, 2, 1, 2, 2, 3, 1, 2, 2, 3, 2, 3, 3, 4];

      let left = if a == 15 {
        3
      } else if a == 0 {
        0
      } else {
        1
      };

      let right = if b == 15 {
        3
      } else if b == 0 {
        0
      } else {
        1
      };

      map_parent_right[i] = left | right;
      map_parent_left[i] = map_parent_right[i] << 4;

      next_data_0_bit[i] = if i == 255 {
        -1
      } else {
        // The casting here between numbers is safe because we only operate
        // between 0..255. Floats are needed for log / ceil operations though.
        8 - ((256f32 - i as f32).log2() / (2f32).log2()).ceil() as i16
      };

      next_index_0_bit[i] = if i == 255 { -1 } else { next_data_0_bit[i] / 2 };

      total_1_bits[i] = nibble[i >> 4] + nibble[i & 0x0F];
    }

    Self {
      index_update,
      index_iterate,
      data_iterate,
      data_update,
      map_parent_right,
      map_parent_left,
      next_data_0_bit,
      next_index_0_bit,
      total_1_bits,
    }
  }
}

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn total_1_bits() {
    let masks = Masks::new();
    let total_1_bits: Vec<_> = (0..256)
      .map(|n| (n as u8).count_ones() as u8)
      .collect();
    assert_eq!(masks.total_1_bits, total_1_bits);
  }

  #[test]
  fn next_data_0_bit() {
    let masks = Masks::new();
    let mut next_data_0_bit: Vec<_> = (0..256)
      .map(|n| (!n as u8).leading_zeros() as i16)
      .collect();
    next_data_0_bit[255] = -1;
    assert_eq!(masks.next_data_0_bit, next_data_0_bit);
  }

  #[test]
  fn next_index_0_bit() {
    let masks = Masks::new();
    let mut next_index_0_bit: Vec<_> = (0..256)
      .map(|n| (!n as u8).leading_zeros() as i16 / 2)
      .collect();
    next_index_0_bit[255] = -1;
    assert_eq!(masks.next_index_0_bit, next_index_0_bit);
  }
}
