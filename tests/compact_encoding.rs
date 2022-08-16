use hypercore::compact_encoding::CompactEncoding;
use hypercore::compact_encoding::{CencState, CompactEncoder};

#[test]
fn cenc_create() {
    let state = CencState::new();
    assert_eq!(state.start, 0);
    assert_eq!(state.end, 0);
    assert_eq!(state.buffer.capacity(), 0);
    let state = CompactEncoder::preencode(state, "test".to_string());
    assert_eq!(state.start, 0);
    assert_eq!(state.end, 4);
    assert_eq!(state.buffer.capacity(), 0);
}
