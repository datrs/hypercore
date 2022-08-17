use hypercore::compact_encoding::{CompactEncoding, State};

#[test]
fn cenc_create() -> std::io::Result<()> {
    let str_value_1 = "foo";
    let str_value_2 = "bar";

    let mut state = State::new();
    assert_eq!(state.start, 0);
    assert_eq!(state.end, 0);
    state.preencode(&str_value_1);
    assert_eq!(state.start, 0);
    assert_eq!(state.end, 3);
    state.preencode(&str_value_2);
    assert_eq!(state.start, 0);
    assert_eq!(state.end, 6);
    let mut buffer = state.create_buffer();
    assert_eq!(buffer.len(), 6);
    state.encode(&str_value_1, &mut buffer);
    assert_eq!(state.start, 3);
    assert_eq!(state.end, 6);
    state.encode(&str_value_2, &mut buffer);
    assert_eq!(state.start, 6);
    assert_eq!(state.end, 6);
    Ok(())
}
