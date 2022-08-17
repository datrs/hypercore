use hypercore::compact_encoding::{CompactEncoding, State};

#[test]
fn cenc_create() {
    let str_value_1 = "foo";
    let str_value_2 = "bar";

    let mut enc_state = State::new();
    enc_state.preencode_str(&str_value_1);
    enc_state.preencode_str(&str_value_2);
    let mut buffer = enc_state.create_buffer();
    enc_state.encode_str(&str_value_1, &mut buffer);
    enc_state.encode_str(&str_value_2, &mut buffer);
    let mut dec_state = State::from_buffer(&buffer);
    let str_value_1_ret: String = dec_state.decode(&buffer);
    assert_eq!(str_value_1, str_value_1_ret);
}
