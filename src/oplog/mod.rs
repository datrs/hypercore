use crate::common::BufferSlice;
use crate::compact_encoding::{CompactEncoding, State};
use crate::PartialKeypair;
use anyhow::{anyhow, Result};
use std::convert::{TryFrom, TryInto};

mod entry;
mod header;

pub use entry::{Entry, EntryBitfieldUpdate, EntryTreeUpgrade};
pub use header::{Header, HeaderTree};

/// Oplog.
///
/// There are two memory areas for an `Header` in `RandomAccessStorage`: one is the current
/// and one is the older. Which one is used depends on the value stored in the eigth byte's
/// eight bit of the stored headers.
#[derive(Debug)]
pub struct Oplog {
    header_bits: [bool; 2],
    entries_length: u64,
    entries_byte_length: u64,
}

/// Oplog
#[derive(Debug)]
pub struct OplogOpenOutcome {
    pub oplog: Oplog,
    pub header: Header,
    pub slices_to_flush: Box<[BufferSlice]>,
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
                entries_length: 0,
                entries_byte_length: 0,
            };
            Ok(OplogOpenOutcome {
                oplog,
                header,
                slices_to_flush: Box::new([]),
            })
        } else if let Some(mut h2_outcome) = h2_outcome {
            // This shouldn't happen because the first header is saved to the first slot
            // but Javascript supports this so we should too.
            let header_bits: [bool; 2] = [!h2_outcome.header_bit, h2_outcome.header_bit];
            let oplog = Oplog {
                header_bits,
                entries_length: 0,
                entries_byte_length: 0,
            };
            Ok(OplogOpenOutcome {
                oplog,
                header: h2_outcome.state.decode(&existing),
                slices_to_flush: Box::new([]),
            })
        } else {
            // There is nothing in the oplog, start from new.
            Ok(Self::new(key_pair))
        }
    }

    /// Appends a entry to the Oplog.
    pub fn append(&mut self, entry: Entry, atomic: bool) -> Result<Box<[BufferSlice]>> {
        self.append_batch(&[entry], atomic)
    }

    /// Appends a batch of entries to the Oplog.
    pub fn append_batch(&mut self, batch: &[Entry], atomic: bool) -> Result<Box<[BufferSlice]>> {
        let len = batch.len();
        let header_bit = self.get_current_header_bit();
        // Leave room for leaders
        let mut state = State::new_with_start_and_end(0, len * 8);

        for entry in batch.iter() {
            state.preencode(entry);
        }

        let mut buffer = state.create_buffer();
        for i in 0..len {
            let entry = &batch[i];
            state.start += 8;
            let start = state.start;
            let partial_bit: bool = atomic && i < len - 1;
            state.encode(entry, &mut buffer);
            Self::prepend_leader(
                state.start - start,
                header_bit,
                partial_bit,
                &mut state,
                &mut buffer,
            );
        }

        self.entries_length += len as u64;
        self.entries_byte_length += buffer.len() as u64;

        Ok(vec![BufferSlice {
            index: OplogSlot::Entries as u64 + self.entries_byte_length,
            data: Some(buffer),
        }]
        .into_boxed_slice())
    }

    fn new(key_pair: PartialKeypair) -> OplogOpenOutcome {
        let oplog = Oplog {
            header_bits: INITIAL_HEADER_BITS,
            entries_length: 0,
            entries_byte_length: 0,
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
        let truncate_index = OplogSlot::Entries as u64 + oplog.entries_byte_length;
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
            ]
            .into_boxed_slice(),
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

    /// Gets the current header bit
    fn get_current_header_bit(&self) -> bool {
        self.header_bits[0] != self.header_bits[1]
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
