use hypercore::compact_encoding::{CompactEncoding, State};

// The max value for 1 byte length is 252
const MAX_ONE_BYTE_UINT: u8 = 252;

// The min value for 2 byte length is 253
const MIN_TWO_BYTE_UINT: u8 = 253;

#[test]
fn cenc_basic() {
    let str_value_1 = "foo";
    let str_value_2 = (0..MAX_ONE_BYTE_UINT).map(|_| "X").collect::<String>();
    let u32_value_3: u32 = u32::MAX;
    let u32_value_4: u32 = 0xF0E1D2C3;

    let mut enc_state = State::new();
    enc_state.preencode_str(&str_value_1);
    enc_state.preencode(&str_value_2);
    enc_state.preencode(&u32_value_3);
    enc_state.preencode(&u32_value_4);
    let mut buffer = enc_state.create_buffer();
    // Strings: 1 byte for length, 3/252 bytes for content
    // u32: 1 byte for u32 signifier, 4 bytes for data
    assert_eq!(buffer.len(), 1 + 3 + 1 + 252 + 1 + 4 + 1 + 4);
    enc_state.encode_str(&str_value_1, &mut buffer);
    enc_state.encode(&str_value_2, &mut buffer);
    enc_state.encode(&u32_value_3, &mut buffer);
    enc_state.encode(&u32_value_4, &mut buffer);
    let mut dec_state = State::from_buffer(&buffer);
    let str_value_1_ret: String = dec_state.decode(&buffer);
    assert_eq!(str_value_1, str_value_1_ret);
    let str_value_2_ret: String = dec_state.decode(&buffer);
    assert_eq!(str_value_2, str_value_2_ret);
    let u32_value_3_ret: u32 = dec_state.decode(&buffer);
    assert_eq!(u32_value_3, u32_value_3_ret);
    let u32_value_4_ret: u32 = dec_state.decode(&buffer);
    assert_eq!(u32_value_4, u32_value_4_ret);
}

#[test]
fn cenc_string_long() {
    let str_value = (0..MIN_TWO_BYTE_UINT).map(|_| "X").collect::<String>();
    assert_eq!(str_value.len(), 253);
    let mut enc_state = State::new();
    enc_state.preencode(&str_value);
    let mut buffer = enc_state.create_buffer();
    // 1 byte for u16 signifier, 2 bytes for length, 256 bytes for data
    assert_eq!(buffer.len(), 1 + 2 + 253);
    enc_state.encode(&str_value, &mut buffer);
    let mut dec_state = State::from_buffer(&buffer);
    let str_value_ret: String = dec_state.decode(&buffer);
    assert_eq!(str_value, str_value_ret);
}

#[test]
fn cenc_u32_as_u16() {
    let u32_value: u32 = u16::MAX.into();
    let mut enc_state = State::new();
    enc_state.preencode(&u32_value);
    let mut buffer = enc_state.create_buffer();
    // 1 byte for u16 signifier, 2 bytes for length
    assert_eq!(buffer.len(), 1 + 2);
    enc_state.encode(&u32_value, &mut buffer);
    let mut dec_state = State::from_buffer(&buffer);
    let u32_value_ret: u32 = dec_state.decode(&buffer);
    assert_eq!(u32_value, u32_value_ret);
}

#[test]
fn cenc_u32_as_u8() {
    let u32_value: u32 = MAX_ONE_BYTE_UINT.into();
    let mut enc_state = State::new();
    enc_state.preencode(&u32_value);
    let mut buffer = enc_state.create_buffer();
    // 1 byte for data
    assert_eq!(buffer.len(), 1);
    enc_state.encode(&u32_value, &mut buffer);
    let mut dec_state = State::from_buffer(&buffer);
    let u32_value_ret: u32 = dec_state.decode(&buffer);
    assert_eq!(u32_value, u32_value_ret);
}

#[test]
fn cenc_buffer() {
    let buf_value = vec![0xFF, 0x00].into_boxed_slice();
    let mut enc_state = State::new();
    enc_state.preencode(&buf_value);
    let mut buffer = enc_state.create_buffer();
    // 1 byte for length, 2 bytes for data
    assert_eq!(buffer.len(), 3);
    enc_state.encode(&buf_value, &mut buffer);
    let mut dec_state = State::from_buffer(&buffer);
    let buf_value_ret: Box<[u8]> = dec_state.decode(&buffer);
    assert_eq!(buf_value, buf_value_ret);
}
