//! Compact encoding module. Rust implementation of https://github.com/compact-encoding/compact-encoding.

use std::fmt::Debug;

/// State.
#[derive(Debug)]
pub struct State {
    /// Start position
    pub start: usize,
    /// End position
    pub end: usize,
    /// Buffer to hold the encoding
    pub buffer: Vec<u8>,
}

impl State {
    /// Create emtpy state
    pub fn new() -> State {
        State {
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
    fn preencode(&mut self, value: T);

    // /// Encode
    // fn encode(state: State, value: T);

    // /// Decode
    // fn decode(state: State) -> T;
}

impl CompactEncoding<String> for State {
    fn preencode(&mut self, value: String) {
        self.end += value.len();
    }
}

impl CompactEncoding<u64> for State {
    fn preencode(&mut self, _value: u64) {
        self.end += 8;
    }
}
