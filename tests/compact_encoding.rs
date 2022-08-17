use hypercore::compact_encoding::{CompactEncoding, State};

#[test]
fn cenc_create() {
    let mut state = State::new();
    assert_eq!(state.start, 0);
    assert_eq!(state.end, 0);
    assert_eq!(state.buffer.capacity(), 0);
    state.preencode("test".to_string());
    assert_eq!(state.start, 0);
    assert_eq!(state.end, 4);
    assert_eq!(state.buffer.capacity(), 0);
}
