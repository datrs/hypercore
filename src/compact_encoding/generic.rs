//! Generic compact encodings
use super::{CompactEncoding, State};

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

impl CompactEncoding<u8> for State {
    fn preencode(&mut self, _: &u8) {
        self.end += 1;
    }

    fn encode(&mut self, value: &u8, buffer: &mut [u8]) {
        buffer[self.start] = *value;
        self.start += 1;
    }

    fn decode(&mut self, buffer: &[u8]) -> u8 {
        let value = buffer[self.start];
        self.start += 1;
        value
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

impl CompactEncoding<Vec<u8>> for State {
    fn preencode(&mut self, value: &Vec<u8>) {
        self.preencode_buffer_vec(value);
    }

    fn encode(&mut self, value: &Vec<u8>, buffer: &mut [u8]) {
        self.encode_buffer(value, buffer);
    }

    fn decode(&mut self, buffer: &[u8]) -> Vec<u8> {
        self.decode_buffer_vec(buffer).to_vec()
    }
}

impl CompactEncoding<Vec<u32>> for State {
    fn preencode(&mut self, value: &Vec<u32>) {
        let len = value.len();
        self.preencode_usize_var(&len);
        self.end += len * 4;
    }

    fn encode(&mut self, value: &Vec<u32>, buffer: &mut [u8]) {
        let len = value.len();
        self.encode_usize_var(&len, buffer);
        for entry in value {
            self.encode_u32(*entry, buffer);
        }
    }

    fn decode(&mut self, buffer: &[u8]) -> Vec<u32> {
        let len = self.decode_usize_var(buffer);
        let mut value: Vec<u32> = Vec::with_capacity(len);
        for _ in 0..len {
            value.push(self.decode_u32(&buffer));
        }
        value
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
