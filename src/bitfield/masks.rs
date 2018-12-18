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
      0b00_11_11_11, //  63
      0b11_00_11_11, // 207
      0b11_11_00_11, // 243
      0b11_11_11_00, // 252
    ];

    let index_iterate = vec![
      0b00_00_00_00, //   0
      0b11_00_00_00, // 192
      0b11_11_00_00, // 240
      0b11_11_11_00, // 252
    ];

    let data_iterate = vec![
      0b10_00_00_00, // 128
      0b11_00_00_00, // 192
      0b11_10_00_00, // 224
      0b11_11_00_00, // 240
      0b11_11_10_00, // 248
      0b11_11_11_00, // 252
      0b11_11_11_10, // 254
      0b11_11_11_11, // 255
    ];

    let data_update = vec![
      0b01_11_11_11, // 127
      0b10_11_11_11, // 191
      0b11_01_11_11, // 223
      0b11_10_11_11, // 239
      0b11_11_01_11, // 247
      0b11_11_10_11, // 251
      0b11_11_11_01, // 253
      0b11_11_11_10, // 254
    ];

    let mut map_parent_right = vec![0; 256];
    let mut map_parent_left = vec![0; 256];

    for i in 0..256 {
      let a = (i & (15 << 4)) >> 4;
      let b = i & 15;

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
    }

    let total_1_bits: Vec<_> =
      (0..256).map(|n| (n as u8).count_ones() as u8).collect();

    let mut next_data_0_bit: Vec<_> = (0..256)
      .map(|n| (!n as u8).leading_zeros() as i16)
      .collect();
    next_data_0_bit[255] = -1;

    let mut next_index_0_bit: Vec<_> =
      next_data_0_bit.iter().map(|n| n / 2).collect();
    next_index_0_bit[255] = -1;

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
