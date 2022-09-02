//! Compact encoding module. Rust implementation of https://github.com/compact-encoding/compact-encoding.

use std::convert::TryFrom;
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
        State::new_with_start_and_end(0, 0)
    }

    /// Create a state with an already known size.
    /// With this, you can/must skip the preencode step.
    pub fn new_with_size(size: usize) -> (State, Box<[u8]>) {
        (
            State::new_with_start_and_end(0, size),
            vec![0; size].into_boxed_slice(),
        )
    }

    /// Create a state with a start and end already known.
    pub fn new_with_start_and_end(start: usize, end: usize) -> State {
        State { start, end }
    }

    /// Create a state from existing buffer.
    pub fn from_buffer(buffer: &[u8]) -> State {
        State::new_with_start_and_end(0, buffer.len())
    }

    /// After calling preencode(), this allocates the right size buffer to the heap.
    /// Follow this with the same number of encode() steps to fill the created buffer.
    pub fn create_buffer(&self) -> Box<[u8]> {
        vec![0; self.end].into_boxed_slice()
    }

    /// Preencode a string slice
    pub fn preencode_str(&mut self, value: &str) {
        self.preencode_usize_var(&value.len());
        self.end += value.len();
    }

    /// Encode a string slice
    pub fn encode_str(&mut self, value: &str, buffer: &mut [u8]) {
        let len = value.len();
        self.encode_usize_var(&len, buffer);
        buffer[self.start..self.start + len].copy_from_slice(value.as_bytes());
        self.start += len;
    }

    /// Decode a String
    pub fn decode_string(&mut self, buffer: &[u8]) -> String {
        let len = self.decode_usize_var(buffer);
        let value = std::str::from_utf8(&buffer[self.start..self.start + len])
            .expect("string is invalid UTF-8");
        self.start += len;
        value.to_string()
    }

    /// Preencode a variable length usigned int
    pub fn preencode_uint_var<T: From<u32> + Ord>(&mut self, uint: &T) {
        self.end += if *uint < T::from(U16_SIGNIFIER.into()) {
            1
        } else if *uint <= T::from(0xffff) {
            3
        } else if *uint <= T::from(0xffffffff) {
            5
        } else {
            9
        };
    }

    /// Decode a fixed length u16
    pub fn decode_u16(&mut self, buffer: &[u8]) -> u16 {
        let value: u16 =
            ((buffer[self.start] as u16) << 0) | ((buffer[self.start + 1] as u16) << 8);
        self.start += 2;
        value
    }

    /// Encode a variable length u32
    pub fn encode_u32_var(&mut self, value: &u32, buffer: &mut [u8]) {
        if *value < U16_SIGNIFIER.into() {
            let bytes = value.to_le_bytes();
            buffer[self.start] = bytes[0];
            self.start += 1;
        } else if *value <= 0xffff {
            buffer[self.start] = U16_SIGNIFIER;
            self.start += 1;
            self.encode_uint16_bytes(&value.to_le_bytes(), buffer);
        } else {
            buffer[self.start] = U32_SIGNIFIER;
            self.start += 1;
            self.encode_u32(*value, buffer);
        }
    }

    /// Encode u32 to 4 LE bytes.
    pub fn encode_u32(&mut self, uint: u32, buffer: &mut [u8]) {
        self.encode_uint32_bytes(&uint.to_le_bytes(), buffer);
    }

    /// Decode a variable length u32
    pub fn decode_u32_var(&mut self, buffer: &[u8]) -> u32 {
        let first = buffer[self.start];
        self.start += 1;
        if first < U16_SIGNIFIER {
            first.into()
        } else if first == U16_SIGNIFIER {
            self.decode_u16(buffer).into()
        } else {
            self.decode_u32(buffer).into()
        }
    }

    /// Decode a fixed length u32
    pub fn decode_u32(&mut self, buffer: &[u8]) -> u32 {
        let value: u32 = ((buffer[self.start] as u32) << 0)
            | ((buffer[self.start + 1] as u32) << 8)
            | ((buffer[self.start + 2] as u32) << 16)
            | ((buffer[self.start + 3] as u32) << 24);
        self.start += 4;
        value
    }

    /// Encode a variable length u64
    pub fn encode_u64_var(&mut self, value: &u64, buffer: &mut [u8]) {
        if *value < U16_SIGNIFIER.into() {
            let bytes = value.to_le_bytes();
            buffer[self.start] = bytes[0];
            self.start += 1;
        } else if *value <= 0xffff {
            buffer[self.start] = U16_SIGNIFIER;
            self.start += 1;
            self.encode_uint16_bytes(&value.to_le_bytes(), buffer);
        } else if *value <= 0xffffffff {
            buffer[self.start] = U32_SIGNIFIER;
            self.start += 1;
            self.encode_uint32_bytes(&value.to_le_bytes(), buffer);
        } else {
            buffer[self.start] = U64_SIGNIFIER;
            self.start += 1;
            self.encode_uint64_bytes(&value.to_le_bytes(), buffer);
        }
    }

    /// Encode u64 to 8 LE bytes.
    pub fn encode_u64(&mut self, uint: u64, buffer: &mut [u8]) {
        self.encode_uint64_bytes(&uint.to_le_bytes(), buffer);
    }

    /// Decode a variable length u64
    pub fn decode_u64_var(&mut self, buffer: &[u8]) -> u64 {
        let first = buffer[self.start];
        self.start += 1;
        if first < U16_SIGNIFIER {
            first.into()
        } else if first == U16_SIGNIFIER {
            self.decode_u16(buffer).into()
        } else if first == U32_SIGNIFIER {
            self.decode_u32(buffer).into()
        } else {
            self.decode_u64(buffer)
        }
    }

    /// Decode a fixed length u64
    pub fn decode_u64(&mut self, buffer: &[u8]) -> u64 {
        let value: u64 = ((buffer[self.start] as u64) << 0)
            | ((buffer[self.start + 1] as u64) << 8)
            | ((buffer[self.start + 2] as u64) << 16)
            | ((buffer[self.start + 3] as u64) << 24)
            | ((buffer[self.start + 4] as u64) << 32)
            | ((buffer[self.start + 5] as u64) << 40)
            | ((buffer[self.start + 6] as u64) << 48)
            | ((buffer[self.start + 7] as u64) << 56);
        self.start += 8;
        value
    }

    /// Preencode a byte buffer
    pub fn preencode_buffer(&mut self, value: &Box<[u8]>) {
        let len = value.len();
        self.preencode_usize_var(&len);
        self.end += len;
    }

    /// Encode a byte buffer
    pub fn encode_buffer(&mut self, value: &[u8], buffer: &mut [u8]) {
        let len = value.len();
        self.encode_usize_var(&len, buffer);
        buffer[self.start..self.start + len].copy_from_slice(value);
        self.start += len;
    }

    /// Decode a byte buffer
    pub fn decode_buffer(&mut self, buffer: &[u8]) -> Box<[u8]> {
        let len = self.decode_usize_var(buffer);
        let value = buffer[self.start..self.start + len]
            .to_vec()
            .into_boxed_slice();
        self.start += value.len();
        value
    }

    /// Preencode a string array
    pub fn preencode_string_array(&mut self, value: &Vec<String>) {
        let len = value.len();
        self.preencode_usize_var(&len);
        for string_value in value.into_iter() {
            self.preencode_str(string_value);
        }
    }

    /// Encode a String array
    pub fn encode_string_array(&mut self, value: &Vec<String>, buffer: &mut [u8]) {
        let len = value.len();
        self.encode_usize_var(&len, buffer);
        for string_value in value {
            self.encode_str(string_value, buffer);
        }
    }

    /// Decode a String array
    pub fn decode_string_array(&mut self, buffer: &[u8]) -> Vec<String> {
        let len = self.decode_usize_var(buffer);
        let mut value = Vec::with_capacity(len);
        for _ in 0..len {
            value.push(self.decode_string(buffer));
        }
        value
    }

    fn encode_uint16_bytes(&mut self, bytes: &[u8], buffer: &mut [u8]) {
        buffer[self.start] = bytes[0];
        buffer[self.start + 1] = bytes[1];
        self.start += 2;
    }

    fn encode_uint32_bytes(&mut self, bytes: &[u8], buffer: &mut [u8]) {
        self.encode_uint16_bytes(bytes, buffer);
        buffer[self.start] = bytes[2];
        buffer[self.start + 1] = bytes[3];
        self.start += 2;
    }

    fn encode_uint64_bytes(&mut self, bytes: &[u8], buffer: &mut [u8]) {
        self.encode_uint32_bytes(bytes, buffer);
        buffer[self.start] = bytes[4];
        buffer[self.start + 1] = bytes[5];
        buffer[self.start + 2] = bytes[6];
        buffer[self.start + 3] = bytes[7];
        self.start += 4;
    }

    fn preencode_usize_var(&mut self, value: &usize) {
        // TODO: This repeats the logic from above that works for u8 -> u64, but sadly not usize
        self.end += if *value < U16_SIGNIFIER.into() {
            1
        } else if *value <= 0xffff {
            3
        } else if *value <= 0xffffffff {
            5
        } else {
            9
        };
    }

    fn encode_usize_var(&mut self, value: &usize, buffer: &mut [u8]) {
        if *value <= 0xfc {
            let bytes = value.to_le_bytes();
            buffer[self.start] = bytes[0];
            self.start += 1;
        } else if *value <= 0xffff {
            buffer[self.start] = U16_SIGNIFIER;
            self.start += 1;
            self.encode_uint16_bytes(&value.to_le_bytes(), buffer);
        } else if *value <= 0xffffffff {
            buffer[self.start] = U32_SIGNIFIER;
            self.start += 1;
            self.encode_uint32_bytes(&value.to_le_bytes(), buffer);
        } else {
            buffer[self.start] = U64_SIGNIFIER;
            self.start += 1;
            self.encode_uint64_bytes(&value.to_le_bytes(), buffer);
        }
    }

    fn decode_usize_var(&mut self, buffer: &[u8]) -> usize {
        let first = buffer[self.start];
        self.start += 1;
        // NB: the from_le_bytes needs a [u8; 2] and that can't be efficiently
        // created from a byte slice.
        if first < U16_SIGNIFIER {
            first.into()
        } else if first == U16_SIGNIFIER {
            self.decode_u16(buffer).into()
        } else if first == U32_SIGNIFIER {
            usize::try_from(self.decode_u32(buffer))
                .expect("Attempted converting to a 32 bit usize on below 32 bit system")
        } else {
            usize::try_from(self.decode_u64(buffer))
                .expect("Attempted converting to a 64 bit usize on below 64 bit system")
        }
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
    fn encode(&mut self, value: &T, buffer: &mut [u8]);

    /// Decode
    fn decode(&mut self, buffer: &[u8]) -> T;
}

impl CompactEncoding<String> for State {
    fn preencode(&mut self, value: &String) {
        self.preencode_str(value)
    }

    fn encode(&mut self, value: &String, buffer: &mut [u8]) {
        self.encode_str(value, buffer)
    }

    fn decode(&mut self, buffer: &[u8]) -> String {
        self.decode_string(buffer)
    }
}

impl CompactEncoding<u32> for State {
    fn preencode(&mut self, value: &u32) {
        self.preencode_uint_var(value)
    }

    fn encode(&mut self, value: &u32, buffer: &mut [u8]) {
        self.encode_u32_var(value, buffer)
    }

    fn decode(&mut self, buffer: &[u8]) -> u32 {
        self.decode_u32_var(buffer)
    }
}

impl CompactEncoding<u64> for State {
    fn preencode(&mut self, value: &u64) {
        self.preencode_uint_var(value)
    }

    fn encode(&mut self, value: &u64, buffer: &mut [u8]) {
        self.encode_u64_var(value, buffer)
    }

    fn decode(&mut self, buffer: &[u8]) -> u64 {
        self.decode_u64_var(buffer)
    }
}

impl CompactEncoding<usize> for State {
    fn preencode(&mut self, value: &usize) {
        self.preencode_usize_var(value)
    }

    fn encode(&mut self, value: &usize, buffer: &mut [u8]) {
        self.encode_usize_var(value, buffer)
    }

    fn decode(&mut self, buffer: &[u8]) -> usize {
        self.decode_usize_var(buffer)
    }
}

impl CompactEncoding<Box<[u8]>> for State {
    fn preencode(&mut self, value: &Box<[u8]>) {
        self.preencode_buffer(value);
    }

    fn encode(&mut self, value: &Box<[u8]>, buffer: &mut [u8]) {
        self.encode_buffer(value, buffer);
    }

    fn decode(&mut self, buffer: &[u8]) -> Box<[u8]> {
        self.decode_buffer(buffer)
    }
}

impl CompactEncoding<Vec<String>> for State {
    fn preencode(&mut self, value: &Vec<String>) {
        self.preencode_string_array(value);
    }

    fn encode(&mut self, value: &Vec<String>, buffer: &mut [u8]) {
        self.encode_string_array(value, buffer);
    }

    fn decode(&mut self, buffer: &[u8]) -> Vec<String> {
        self.decode_string_array(buffer)
    }
}
