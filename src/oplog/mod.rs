use crate::common::BufferSlice;
use crate::compact_encoding::{CompactEncoding, State};
use crate::crypto::{PublicKey, SecretKey};
use crate::PartialKeypair;
use anyhow::{anyhow, Result};
use std::convert::{TryFrom, TryInto};

/// Oplog header.
#[derive(Debug)]
pub struct Header {
    pub(crate) types: HeaderTypes,
    // TODO: This is a keyValueArray in JS
    pub(crate) user_data: Vec<String>,
    pub(crate) tree: HeaderTree,
    pub(crate) signer: PartialKeypair,
    pub(crate) hints: HeaderHints,
    pub(crate) contiguous_length: u64,
}

impl Header {
    /// Creates a new Header from given key pair
    pub fn new(key_pair: PartialKeypair) -> Header {
        Header {
            types: HeaderTypes::new(),
            user_data: vec![],
            tree: HeaderTree::new(),
            signer: key_pair,
            hints: HeaderHints { reorgs: vec![] },
            contiguous_length: 0,
        }
        // Javascript side, initial header
        //
        // header = {
        //   types: { tree: 'blake2b', bitfield: 'raw', signer: 'ed25519' },
        //   userData: [],
        //   tree: {
        //     fork: 0,
        //     length: 0,
        //     rootHash: null,
        //     signature: null
        //   },
        //   signer: opts.keyPair || crypto.keyPair(),
        //   hints: {
        //     reorgs: []
        //   },
        //   contiguousLength: 0
        // }
    }
}

/// Oplog header types
#[derive(Debug, PartialEq)]
pub struct HeaderTypes {
    pub(crate) tree: String,
    pub(crate) bitfield: String,
    pub(crate) signer: String,
}
impl HeaderTypes {
    pub fn new() -> HeaderTypes {
        HeaderTypes {
            tree: "blake2b".to_string(),
            bitfield: "raw".to_string(),
            signer: "ed25519".to_string(),
        }
    }
}

impl CompactEncoding<HeaderTypes> for State {
    fn preencode(&mut self, value: &HeaderTypes) {
        self.preencode(&value.tree);
        self.preencode(&value.bitfield);
        self.preencode(&value.signer);
    }

    fn encode(&mut self, value: &HeaderTypes, buffer: &mut Box<[u8]>) {
        self.encode(&value.tree, buffer);
        self.encode(&value.bitfield, buffer);
        self.encode(&value.signer, buffer);
    }

    fn decode(&mut self, buffer: &Box<[u8]>) -> HeaderTypes {
        let tree: String = self.decode(buffer);
        let bitfield: String = self.decode(buffer);
        let signer: String = self.decode(buffer);
        HeaderTypes {
            tree,
            bitfield,
            signer,
        }
    }
}

/// Oplog header tree
#[derive(Debug, PartialEq)]
pub struct HeaderTree {
    pub(crate) fork: u64,
    pub(crate) length: u64,
    pub(crate) root_hash: Box<[u8]>,
    pub(crate) signature: Box<[u8]>,
}

impl HeaderTree {
    pub fn new() -> HeaderTree {
        HeaderTree {
            fork: 0,
            length: 0,
            root_hash: Box::new([]),
            signature: Box::new([]),
        }
    }
}

impl CompactEncoding<HeaderTree> for State {
    fn preencode(&mut self, value: &HeaderTree) {
        self.preencode(&value.fork);
        self.preencode(&value.length);
        self.preencode(&value.root_hash);
        self.preencode(&value.signature);
    }

    fn encode(&mut self, value: &HeaderTree, buffer: &mut Box<[u8]>) {
        self.encode(&value.fork, buffer);
        self.encode(&value.length, buffer);
        self.encode(&value.root_hash, buffer);
        self.encode(&value.signature, buffer);
    }

    fn decode(&mut self, buffer: &Box<[u8]>) -> HeaderTree {
        let fork: u64 = self.decode(buffer);
        let length: u64 = self.decode(buffer);
        let root_hash: Box<[u8]> = self.decode(buffer);
        let signature: Box<[u8]> = self.decode(buffer);
        HeaderTree {
            fork,
            length,
            root_hash,
            signature,
        }
    }
}

/// NB: In Javascript's sodium the secret key contains in itself also the public key, so to
/// maintain binary compatibility, we store the public key in the oplog now twice.
impl CompactEncoding<PartialKeypair> for State {
    fn preencode(&mut self, value: &PartialKeypair) {
        self.end += 1 + 32;
        match &value.secret {
            Some(_) => {
                // Also add room for the public key
                self.end += 1 + 64;
            }
            None => {
                self.end += 1;
            }
        }
    }

    fn encode(&mut self, value: &PartialKeypair, buffer: &mut Box<[u8]>) {
        let public_key_bytes: Box<[u8]> = value.public.as_bytes().to_vec().into_boxed_slice();
        self.encode(&public_key_bytes, buffer);
        match &value.secret {
            Some(secret_key) => {
                let mut secret_key_bytes: Vec<u8> = Vec::with_capacity(64);
                secret_key_bytes.extend_from_slice(secret_key.as_bytes());
                secret_key_bytes.extend_from_slice(&public_key_bytes);
                let secret_key_bytes: Box<[u8]> = secret_key_bytes.into_boxed_slice();
                self.encode(&secret_key_bytes, buffer);
            }
            None => {
                buffer[self.start] = 0;
                self.start += 1;
            }
        }
    }

    fn decode(&mut self, buffer: &Box<[u8]>) -> PartialKeypair {
        let public_key_bytes: Box<[u8]> = self.decode(buffer);
        let secret_key_bytes: Box<[u8]> = self.decode(buffer);
        let secret: Option<SecretKey> = if secret_key_bytes.len() == 0 {
            None
        } else {
            Some(SecretKey::from_bytes(&secret_key_bytes[0..32]).unwrap())
        };

        PartialKeypair {
            public: PublicKey::from_bytes(&public_key_bytes).unwrap(),
            secret,
        }
    }
}

/// Oplog header hints
#[derive(Debug)]
pub struct HeaderHints {
    pub(crate) reorgs: Vec<String>,
}

impl CompactEncoding<HeaderHints> for State {
    fn preencode(&mut self, value: &HeaderHints) {
        self.preencode(&value.reorgs);
    }

    fn encode(&mut self, value: &HeaderHints, buffer: &mut Box<[u8]>) {
        self.encode(&value.reorgs, buffer);
    }

    fn decode(&mut self, buffer: &Box<[u8]>) -> HeaderHints {
        HeaderHints {
            reorgs: self.decode(buffer),
        }
    }
}

impl CompactEncoding<Header> for State {
    fn preencode(&mut self, value: &Header) {
        self.end += 1; // Version
        self.preencode(&value.types);
        self.preencode(&value.user_data);
        self.preencode(&value.tree);
        self.preencode(&value.signer);
        self.preencode(&value.hints);
        self.preencode(&value.contiguous_length);
    }

    fn encode(&mut self, value: &Header, buffer: &mut Box<[u8]>) {
        buffer[self.start] = 0; // Version
        self.start += 1;
        self.encode(&value.types, buffer);
        self.encode(&value.user_data, buffer);
        self.encode(&value.tree, buffer);
        self.encode(&value.signer, buffer);
        self.encode(&value.hints, buffer);
        self.encode(&value.contiguous_length, buffer);
    }

    fn decode(&mut self, buffer: &Box<[u8]>) -> Header {
        let version: u8 = buffer[self.start];
        self.start += 1;
        if version != 0 {
            panic!("Unknown oplog version {}", version);
        }
        let types: HeaderTypes = self.decode(buffer);
        let user_data: Vec<String> = self.decode(buffer);
        let tree: HeaderTree = self.decode(buffer);
        let signer: PartialKeypair = self.decode(buffer);
        let hints: HeaderHints = self.decode(buffer);
        let contiguous_length: u64 = self.decode(buffer);

        Header {
            types,
            user_data,
            tree,
            signer,
            hints,
            contiguous_length,
        }
    }
}

/// Oplog.
///
/// There are two memory areas for an `Header` in `RandomAccessStorage`: one is the current
/// and one is the older. Which one is used depends on the value stored in the eigth byte's
/// eight bit of the stored headers.
#[derive(Debug)]
pub struct Oplog {
    header_bits: [bool; 2],
    entries_len: u64,
}

/// Oplog
#[derive(Debug)]
pub struct OplogOpenOutcome {
    pub oplog: Oplog,
    pub header: Header,
    pub slices_to_flush: Vec<BufferSlice>,
}

enum OplogSlot {
    FirstHeader = 0,
    SecondHeader = 4096,
    Entries = 4096 * 2,
}

#[derive(Debug)]
struct ValidateLeaderOutcome {
    state: State,
    header_bit: bool,
    partial_bit: bool,
}

// The first set of bits is [1, 0], see `get_next_header_oplog_slot_and_bit_value` for how
// they change.
const INITIAL_HEADER_BITS: [bool; 2] = [true, false];

impl Oplog {
    /// Opens an existing Oplog from existing byte buffer or creates a new one.
    #[allow(dead_code)]
    pub fn open(key_pair: PartialKeypair, existing: Box<[u8]>) -> Result<OplogOpenOutcome> {
        // First read and validate both headers stored in the existing oplog
        let h1_outcome = Self::validate_leader(OplogSlot::FirstHeader as usize, &existing)?;
        let h2_outcome = Self::validate_leader(OplogSlot::SecondHeader as usize, &existing)?;

        // Depending on what is stored, the state needs to be set accordingly.
        // See `get_next_header_oplog_slot_and_bit_value` for details on header_bits.
        if let Some(mut h1_outcome) = h1_outcome {
            let (header, header_bits): (Header, [bool; 2]) =
                if let Some(mut h2_outcome) = h2_outcome {
                    let header_bits = [h1_outcome.header_bit, h2_outcome.header_bit];
                    let header: Header = if header_bits[0] == header_bits[1] {
                        h1_outcome.state.decode(&existing)
                    } else {
                        h2_outcome.state.decode(&existing)
                    };
                    (header, header_bits)
                } else {
                    (
                        h1_outcome.state.decode(&existing),
                        [h1_outcome.header_bit, h1_outcome.header_bit],
                    )
                };
            let oplog = Oplog {
                header_bits,
                entries_len: 0,
            };
            Ok(OplogOpenOutcome {
                oplog,
                header,
                slices_to_flush: vec![],
            })
        } else if let Some(mut h2_outcome) = h2_outcome {
            // This shouldn't happen because the first header is saved to the first slot
            // but Javascript supports this so we should too.
            let header_bits: [bool; 2] = [!h2_outcome.header_bit, h2_outcome.header_bit];
            let oplog = Oplog {
                header_bits,
                entries_len: 0,
            };
            Ok(OplogOpenOutcome {
                oplog,
                header: h2_outcome.state.decode(&existing),
                slices_to_flush: vec![],
            })
        } else {
            // There is nothing in the oplog, start from new.
            Ok(Self::new(key_pair))
        }
    }

    fn new(key_pair: PartialKeypair) -> OplogOpenOutcome {
        let oplog = Oplog {
            header_bits: INITIAL_HEADER_BITS,
            entries_len: 0,
        };

        // The first 8 bytes will be filled with `prepend_leader`.
        let data_start_index: usize = 8;
        let mut state = State::new_with_start_and_end(data_start_index, data_start_index);

        // Get the right slot and header bit
        let (oplog_slot, header_bit) =
            Oplog::get_next_header_oplog_slot_and_bit_value(&oplog.header_bits);

        // Preencode a new header
        let header = Header::new(key_pair);
        state.preencode(&header);

        // Create a buffer for the needed data
        let mut buffer = state.create_buffer();

        // Encode the header
        state.encode(&header, &mut buffer);

        // Finally prepend the buffer's 8 first bytes with a CRC, len and right bits
        Self::prepend_leader(
            state.end - data_start_index,
            header_bit,
            false,
            &mut state,
            &mut buffer,
        );

        // The oplog is always truncated to the minimum byte size, which is right after
        // the all of the entries in the oplog finish.
        let truncate_index = OplogSlot::Entries as u64 + oplog.entries_len;
        OplogOpenOutcome {
            oplog,
            header,
            slices_to_flush: vec![
                BufferSlice {
                    index: oplog_slot as u64,
                    data: Some(buffer),
                },
                BufferSlice {
                    index: truncate_index,
                    data: None,
                },
            ],
        }
    }

    /// Prepends given `State` with 4 bytes of CRC followed by 4 bytes containing length of
    /// following buffer, 1 bit indicating which header is relevant to the entry (or if used to
    /// wrap the actual header, then the header bit relevant for saving) and 1 bit that tells if
    /// the written batch is only partially finished. For this to work, the state given must have
    /// 8 bytes in reserve in the beginning, so that state.start can be set back 8 bytes.
    fn prepend_leader(
        len: usize,
        header_bit: bool,
        partial_bit: bool,
        state: &mut State,
        buffer: &mut Box<[u8]>,
    ) {
        // The 4 bytes right before start of data is the length in 8+8+8+6=30 bits. The 31st bit is
        // the partial bit and 32nd bit the header bit.
        state.start = state.start - len - 4;
        let len_u32: u32 = len.try_into().unwrap();
        let partial_bit: u32 = if partial_bit { 2 } else { 0 };
        let header_bit: u32 = if header_bit { 1 } else { 0 };
        let combined: u32 = (len_u32 << 2) | header_bit | partial_bit;
        state.encode_u32(combined, buffer);

        // Before that, is a 4 byte CRC32 that is a checksum of the above encoded 4 bytes and the
        // content.
        state.start = state.start - 8;
        let checksum = crc32fast::hash(&buffer[state.start + 4..state.start + 8 + len]);
        state.encode_u32(checksum, buffer);
    }

    /// Validates that leader at given index is valid, and returns header and partial bits and
    /// `State` for the header/entry that the leader was for.
    fn validate_leader(index: usize, buffer: &Box<[u8]>) -> Result<Option<ValidateLeaderOutcome>> {
        if buffer.len() < index + 8 {
            return Ok(None);
        }
        let mut state = State::new_with_start_and_end(index, buffer.len());
        let stored_checksum: u32 = state.decode_u32(buffer);
        let combined: u32 = state.decode_u32(buffer);
        let len = usize::try_from(combined >> 2)
            .expect("Attempted converting to a 32 bit usize on below 32 bit system");

        // NB: In the Javascript version IIUC zero length is caught only with a mismatch
        // of checksums, which is silently interpreted to only mean "no value". That doesn't sound good:
        // better to throw an error on mismatch and let the caller at least log the problem.
        if len == 0 || state.end - state.start < len {
            return Ok(None);
        }
        let header_bit = combined & 1 == 1;
        let partial_bit = combined & 2 == 2;

        state.start = index + 8;
        state.end = state.start + len;
        let calculated_checksum = crc32fast::hash(&buffer[index + 4..state.end]);
        if calculated_checksum != stored_checksum {
            return Err(anyhow!("Checksums do not match"));
        };

        Ok(Some(ValidateLeaderOutcome {
            header_bit,
            partial_bit,
            state,
        }))
    }

    /// Based on given header_bits, determines if saving the header should be done to the first
    /// header slot or the second header slot and the bit that it should get.
    fn get_next_header_oplog_slot_and_bit_value(header_bits: &[bool; 2]) -> (OplogSlot, bool) {
        // Writing a header to the disk is most efficient when only one area is saved.
        // This makes it a bit less obvious to find out which of the headers is older
        // and which newer. The bits indicate the header slot index in this way:
        //
        // [true, false] => [false, false] => [false, true] => [true, true] => [true, false] ...
        //      First    =>     Second     =>     First     =>    Second    =>    First
        if header_bits[0] != header_bits[1] {
            // First slot
            (OplogSlot::FirstHeader, !header_bits[0])
        } else {
            // Second slot
            (OplogSlot::SecondHeader, !header_bits[1])
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        compact_encoding::{CompactEncoding, State},
        crypto::generate_keypair,
        oplog::{Header, HeaderTree, HeaderTypes},
        PartialKeypair,
    };

    #[test]
    fn encode_header_types() {
        let mut enc_state = State::new_with_start_and_end(8, 8);
        let header_types = HeaderTypes::new();
        enc_state.preencode(&header_types);
        let mut buffer = enc_state.create_buffer();
        enc_state.encode(&header_types, &mut buffer);
        let mut dec_state = State::from_buffer(&buffer);
        dec_state.start = 8;
        let header_types_ret: HeaderTypes = dec_state.decode(&buffer);
        assert_eq!(header_types, header_types_ret);
    }

    #[test]
    fn encode_partial_key_pair() {
        let mut enc_state = State::new();
        let key_pair = generate_keypair();
        let key_pair = PartialKeypair {
            public: key_pair.public,
            secret: Some(key_pair.secret),
        };
        enc_state.preencode(&key_pair);
        let mut buffer = enc_state.create_buffer();
        // Pub key: 1 byte for length, 32 bytes for content
        // Sec key: 1 byte for length, 64 bytes for data
        let expected_len = 1 + 32 + 1 + 64;
        assert_eq!(buffer.len(), expected_len);
        assert_eq!(enc_state.end, expected_len);
        assert_eq!(enc_state.start, 0);
        enc_state.encode(&key_pair, &mut buffer);
        let mut dec_state = State::from_buffer(&buffer);
        let key_pair_ret: PartialKeypair = dec_state.decode(&buffer);
        assert_eq!(key_pair.public, key_pair_ret.public);
        assert_eq!(
            key_pair.secret.unwrap().as_bytes(),
            key_pair_ret.secret.unwrap().as_bytes()
        );
    }

    #[test]
    fn encode_tree() {
        let mut enc_state = State::new();
        let tree = HeaderTree::new();
        enc_state.preencode(&tree);
        let mut buffer = enc_state.create_buffer();
        enc_state.encode(&tree, &mut buffer);
        let mut dec_state = State::from_buffer(&buffer);
        let tree_ret: HeaderTree = dec_state.decode(&buffer);
        assert_eq!(tree, tree_ret);
    }

    #[test]
    fn encode_header() {
        let mut enc_state = State::new();
        let key_pair = generate_keypair();
        let key_pair = PartialKeypair {
            public: key_pair.public,
            secret: Some(key_pair.secret),
        };
        let header = Header::new(key_pair);
        enc_state.preencode(&header);
        let mut buffer = enc_state.create_buffer();
        enc_state.encode(&header, &mut buffer);
        let mut dec_state = State::from_buffer(&buffer);
        let header_ret: Header = dec_state.decode(&buffer);
        assert_eq!(header.signer.public, header_ret.signer.public);
        assert_eq!(header.tree.fork, header_ret.tree.fork);
        assert_eq!(header.tree.length, header_ret.tree.length);
        assert_eq!(header.types, header_ret.types);
    }
}
