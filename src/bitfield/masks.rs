/// Masks used to determine how to update bytes.
///
/// This piece of code is still a bit unclear; lots of magic numbers. It'd be
/// good to figure out what things mean.
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

impl Masks {
  pub fn new() -> Self {
    let index_update = vec![63, 207, 243, 252];
    let index_iterate = vec![0, 192, 240, 252];
    let data_iterate = vec![128, 192, 224, 240, 248, 252, 254, 255];
    let data_update = vec![127, 191, 223, 239, 247, 251, 253, 254];
    let mut map_parent_right = Vec::with_capacity(256);
    let mut map_parent_left = Vec::with_capacity(256);
    let mut next_data_0_bit = Vec::with_capacity(256);
    let mut next_index_0_bit = Vec::with_capacity(256);
    let mut total_1_bits = Vec::with_capacity(256);

    for i in 0..256 {
      let a = (i & (15 << 4)) >> 4;
      let b = i & 15;
      let nibble = vec![0, 1, 1, 2, 1, 2, 2, 3, 1, 2, 2, 3, 2, 3, 3, 4];

      let left = if a == 15 {
        3
      } else {
        if a == 0 {
          0
        } else {
          1
        }
      };

      let right = if b == 15 {
        3
      } else {
        if b == 0 {
          0
        } else {
          1
        }
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

      next_index_0_bit[i] = if i == 255 {
        -1
      } else {
        next_data_0_bit[i] / 2
      };

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
