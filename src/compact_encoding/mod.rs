//! Compact encoding module. Rust implementation of https://github.com/compact-encoding/compact-encoding.

use std::fmt::Debug;

/// State.
#[derive(Debug)]
pub struct CencState {
    /// Start position
    pub start: usize,
    /// End position
    pub end: usize,
    /// Buffer to hold the encoding
    pub buffer: Vec<u8>,
}

impl CencState {
    /// Create emtpy state
    pub fn new() -> CencState {
        CencState {
            start: 0,
            end: 0,
            buffer: vec![],
        }
    }
}

/// Compact Encoding
pub trait CompactEncoding<T>
where
    T: Debug,
{
    /// Preencode
    fn preencode(state: CencState, value: T) -> CencState;

    // /// Encode
    // fn encode(state: CencState, value: T);

    // /// Decode
    // fn decode(state: CencState) -> T;
}

/// Compact Encoder
#[derive(Debug)]
pub struct CompactEncoder {}

impl CompactEncoding<String> for CompactEncoder {
    fn preencode(mut state: CencState, value: String) -> CencState {
        state.end += value.len();
        state
    }
}

impl CompactEncoding<u64> for CompactEncoder {
    fn preencode(mut state: CencState, _value: u64) -> CencState {
        state.end += 8;
        state
    }
}
