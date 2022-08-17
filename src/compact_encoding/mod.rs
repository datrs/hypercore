//! Compact encoding module. Rust implementation of https://github.com/compact-encoding/compact-encoding.

use std::fmt::Debug;

const U16_SIGNIFIER: u8 = 0xfd;
const U32_SIGNIFIER: u8 = 0xfe;
const U64_SIGNIFIER: u8 = 0xff;

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

    /// Create a state from existing buffer.
    pub fn from_buffer(buffer: &Box<[u8]>) -> State {
        State {
            start: 0,
            end: buffer.len(),
        }
    }

    /// After calling preencode(), this allocates the right size buffer to the heap.
    /// Follow this with the same number of encode() steps to fill the created buffer.
    pub fn create_buffer(&self) -> Box<[u8]> {
        vec![0; self.end].into_boxed_slice()
    }

    /// Encode u32 to 4 LE bytes.
    pub fn encode_uint32(&mut self, uint: u32, buffer: &mut Box<[u8]>) {
        let bytes = uint.to_le_bytes();
        buffer[self.start] = bytes[0];
        self.start += 1;
        buffer[self.start] = bytes[1];
        self.start += 1;
        buffer[self.start] = bytes[2];
        self.start += 1;
        buffer[self.start] = bytes[3];
        self.start += 1;
    }

    /// Preencode a string slice
    pub fn preencode_str(&mut self, value: &str) {
        self.preencode_usize_var(&value.len());
        self.end += value.len();
    }

    /// Encode a string slice
    pub fn encode_str(&mut self, value: &str, buffer: &mut Box<[u8]>) {
        let len = value.len();
        self.encode_usize_var(&len, buffer);
        buffer[self.start..self.start + len].copy_from_slice(value.as_bytes());
        self.start += len;
    }

    fn preencode_uint_var<T: From<u32> + Ord>(&mut self, uint: &T) {
        self.end += if *uint <= T::from(0xfc) {
            1
        } else if *uint <= T::from(0xffff) {
            3
        } else if *uint <= T::from(0xffffffff) {
            5
        } else {
            9
        };
    }

    fn preencode_usize_var(&mut self, value: &usize) {
        // TODO: This repeats the logic from above that works for u8 -> u64, but sadly not usize
        self.end += if *value <= 0xfc {
            1
        } else if *value <= 0xffff {
            3
        } else if *value <= 0xffffffff {
            5
        } else {
            9
        };
    }

    fn encode_usize_var(&mut self, value: &usize, buffer: &mut Box<[u8]>) {
        if *value <= 0xfc {
            let bytes = value.to_le_bytes();
            buffer[self.start] = bytes[0];
            self.start += 1;
        } else if *value <= 0xffff {
            buffer[self.start] = U16_SIGNIFIER;
            self.start += 1;
            let bytes = value.to_le_bytes();
            buffer[self.start] = bytes[0];
            self.start += 1;
            buffer[self.start] = bytes[1];
            self.start += 1;
        } else if *value <= 0xffffffff {
            buffer[self.start] = U32_SIGNIFIER;
            self.start += 1;
            let bytes = value.to_le_bytes();
            buffer[self.start] = bytes[0];
            self.start += 1;
            buffer[self.start] = bytes[1];
            self.start += 1;
            buffer[self.start] = bytes[2];
            self.start += 1;
            buffer[self.start] = bytes[3];
            self.start += 1;
        } else {
            buffer[self.start] = U64_SIGNIFIER;
            self.start += 1;
            let bytes = value.to_le_bytes();
            buffer[self.start] = bytes[0];
            self.start += 1;
            buffer[self.start] = bytes[1];
            self.start += 1;
            buffer[self.start] = bytes[2];
            self.start += 1;
            buffer[self.start] = bytes[3];
            self.start += 1;
            buffer[self.start] = bytes[4];
            self.start += 1;
            buffer[self.start] = bytes[5];
            self.start += 1;
            buffer[self.start] = bytes[6];
            self.start += 1;
            buffer[self.start] = bytes[7];
            self.start += 1;
        }
    }

    fn decode_usize_var(&mut self, buffer: &Box<[u8]>) -> usize {
        // FIXME: need to check first byte here for signifier etc.
        let value = buffer[self.start];
        self.start += 1;
        value.into()
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

    /// Decode
    fn decode(&mut self, buffer: &Box<[u8]>) -> T;
}

impl CompactEncoding<String> for State {
    fn preencode(&mut self, value: &String) {
        self.preencode_str(value)
    }

    fn encode(&mut self, value: &String, buffer: &mut Box<[u8]>) {
        self.encode_str(value, buffer)
    }

    fn decode(&mut self, buffer: &Box<[u8]>) -> String {
        let len = self.decode_usize_var(buffer);
        let value = std::str::from_utf8(&buffer[self.start..self.start + len])
            .expect("string is invalid UTF-8");
        self.start += len;
        value.to_string()
    }
}

impl CompactEncoding<u32> for State {
    fn preencode(&mut self, value: &u32) {
        self.preencode_uint_var(value)
    }

    fn encode(&mut self, value: &u32, buffer: &mut Box<[u8]>) {
        if *value <= 0xfc {
            let bytes = value.to_le_bytes();
            buffer[self.start] = bytes[0];
            self.start += 1;
        } else if *value <= 0xffff {
            buffer[self.start] = U16_SIGNIFIER;
            self.start += 1;
            let bytes = value.to_le_bytes();
            buffer[self.start] = bytes[0];
            self.start += 1;
            buffer[self.start] = bytes[1];
            self.start += 1;
        } else {
            buffer[self.start] = U32_SIGNIFIER;
            self.start += 1;
            self.encode_uint32(*value, buffer);
        }
    }

    fn decode(&mut self, _buffer: &Box<[u8]>) -> u32 {
        // FIXME
        0
    }
}

impl CompactEncoding<usize> for State {
    fn preencode(&mut self, value: &usize) {
        self.preencode_usize_var(value)
    }

    fn encode(&mut self, value: &usize, buffer: &mut Box<[u8]>) {
        self.encode_usize_var(value, buffer)
    }

    fn decode(&mut self, buffer: &Box<[u8]>) -> usize {
        // FIXME
        buffer.len()
    }
}
