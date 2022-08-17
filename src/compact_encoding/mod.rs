//! Compact encoding module. Rust implementation of https://github.com/compact-encoding/compact-encoding.

use std::fmt::Debug;

/// State.
#[derive(Debug)]
pub struct State {
    /// Start position
    pub start: usize,
    /// End position
    pub end: usize,
}

impl State {
    /// Create emtpy state
    pub fn new() -> State {
        State { start: 0, end: 0 }
    }

    /// Create a state with an already known size.
    /// With this, you can/must skip the preencode step.
    pub fn new_with_size(size: usize) -> (State, Box<[u8]>) {
        (
            State {
                start: 0,
                end: size,
            },
            vec![0; size].into_boxed_slice(),
        )
    }

    /// After calling preencode(), this allocates the right size buffer to the heap.
    /// Follow this with the same number of encode() steps to fill the created buffer.
    pub fn create_buffer(&self) -> Box<[u8]> {
        vec![0; self.end].into_boxed_slice()
    }
}

/// Compact Encoding
pub trait CompactEncoding<T>
where
    T: Debug,
{
    /// Preencode
    fn preencode(&mut self, value: &T);

    /// Encode
    fn encode(&mut self, value: &T, buffer: &mut Box<[u8]>);

    // /// Decode
    // fn decode(state: State) -> T;
}

impl CompactEncoding<&str> for State {
    fn preencode(&mut self, value: &&str) {
        self.end += value.len();
    }

    fn encode(&mut self, value: &&str, buffer: &mut Box<[u8]>) {
        let len = value.len();
        buffer[self.start..self.start + len].copy_from_slice(value.as_bytes());
        self.start += len;
    }
}
