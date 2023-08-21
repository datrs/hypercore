use futures::future::Either;
use std::convert::{TryFrom, TryInto};

use crate::common::{BitfieldUpdate, Store, StoreInfo, StoreInfoInstruction};
use crate::encoding::{CompactEncoding, HypercoreState};
use crate::tree::MerkleTreeChangeset;
use crate::{HypercoreError, Node, PartialKeypair};

mod entry;
mod header;

pub(crate) use entry::{Entry, EntryTreeUpgrade};
pub(crate) use header::{Header, HeaderTree};

pub(crate) const MAX_OPLOG_ENTRIES_BYTE_SIZE: u64 = 65536;
const HEADER_SIZE: usize = 4096;

/// Oplog.
///
/// There are two memory areas for an `Header` in `RandomAccessStorage`: one is the current
/// and one is the older. Which one is used depends on the value stored in the eigth byte's
/// eight bit of the stored headers.
#[derive(Debug)]
pub(crate) struct Oplog {
    header_bits: [bool; 2],
    pub(crate) entries_length: u64,
    pub(crate) entries_byte_length: u64,
}

/// Oplog create header outcome
#[derive(Debug)]
pub(crate) struct OplogCreateHeaderOutcome {
    pub(crate) header: Header,
    pub(crate) infos_to_flush: Box<[StoreInfo]>,
}

/// Oplog open outcome
#[derive(Debug)]
pub(crate) struct OplogOpenOutcome {
    pub(crate) oplog: Oplog,
    pub(crate) header: Header,
    pub(crate) infos_to_flush: Box<[StoreInfo]>,
    pub(crate) entries: Option<Box<[Entry]>>,
}

impl OplogOpenOutcome {
    pub(crate) fn new(oplog: Oplog, header: Header, infos_to_flush: Box<[StoreInfo]>) -> Self {
        Self {
            oplog,
            header,
            infos_to_flush,
            entries: None,
        }
    }
    pub(crate) fn from_create_header_outcome(
        oplog: Oplog,
        create_header_outcome: OplogCreateHeaderOutcome,
    ) -> Self {
        Self {
            oplog,
            header: create_header_outcome.header,
            infos_to_flush: create_header_outcome.infos_to_flush,
            entries: None,
        }
    }
}

#[repr(usize)]
enum OplogSlot {
    FirstHeader = 0,
    SecondHeader = HEADER_SIZE,
    Entries = HEADER_SIZE * 2,
}

#[derive(Debug)]
struct ValidateLeaderOutcome {
    state: HypercoreState,
    header_bit: bool,
    partial_bit: bool,
}

// The first set of bits is [1, 0], see `get_next_header_oplog_slot_and_bit_value` for how
// they change.
const INITIAL_HEADER_BITS: [bool; 2] = [true, false];

impl Oplog {
    /// Opens an existing Oplog from existing byte buffer or creates a new one.
    pub(crate) fn open(
        key_pair: &Option<PartialKeypair>,
        info: Option<StoreInfo>,
    ) -> Result<Either<StoreInfoInstruction, OplogOpenOutcome>, HypercoreError> {
        match info {
            None => Ok(Either::Left(StoreInfoInstruction::new_all_content(
                Store::Oplog,
            ))),
            Some(info) => {
                let existing = info.data.expect("Could not get data of existing oplog");
                // First read and validate both headers stored in the existing oplog
                let h1_outcome = Self::validate_leader(OplogSlot::FirstHeader as usize, &existing)?;
                let h2_outcome =
                    Self::validate_leader(OplogSlot::SecondHeader as usize, &existing)?;

                // Depending on what is stored, the state needs to be set accordingly.
                // See `get_next_header_oplog_slot_and_bit_value` for details on header_bits.
                let mut outcome: OplogOpenOutcome = if let Some(mut h1_outcome) = h1_outcome {
                    let (header, header_bits): (Header, [bool; 2]) =
                        if let Some(mut h2_outcome) = h2_outcome {
                            let header_bits = [h1_outcome.header_bit, h2_outcome.header_bit];
                            let header: Header = if header_bits[0] == header_bits[1] {
                                (*h1_outcome.state).decode(&existing)?
                            } else {
                                (*h2_outcome.state).decode(&existing)?
                            };
                            (header, header_bits)
                        } else {
                            (
                                (*h1_outcome.state).decode(&existing)?,
                                [h1_outcome.header_bit, h1_outcome.header_bit],
                            )
                        };
                    let oplog = Oplog {
                        header_bits,
                        entries_length: 0,
                        entries_byte_length: 0,
                    };
                    OplogOpenOutcome::new(oplog, header, Box::new([]))
                } else if let Some(mut h2_outcome) = h2_outcome {
                    // This shouldn't happen because the first header is saved to the first slot
                    // but Javascript supports this so we should too.
                    let header_bits: [bool; 2] = [!h2_outcome.header_bit, h2_outcome.header_bit];
                    let oplog = Oplog {
                        header_bits,
                        entries_length: 0,
                        entries_byte_length: 0,
                    };
                    OplogOpenOutcome::new(
                        oplog,
                        (*h2_outcome.state).decode(&existing)?,
                        Box::new([]),
                    )
                } else if let Some(key_pair) = key_pair {
                    // There is nothing in the oplog, start from fresh given key pair.
                    Self::fresh(key_pair.clone())?
                } else {
                    // The storage is empty and no key pair given, erroring
                    return Err(HypercoreError::EmptyStorage {
                        store: Store::Oplog,
                    });
                };

                // Read headers that might be stored in the existing content
                if existing.len() > OplogSlot::Entries as usize {
                    let mut entry_offset = OplogSlot::Entries as usize;
                    let mut entries: Vec<Entry> = Vec::new();
                    let mut partials: Vec<bool> = Vec::new();
                    while let Some(mut entry_outcome) =
                        Self::validate_leader(entry_offset, &existing)?
                    {
                        let entry: Entry = entry_outcome.state.decode(&existing)?;
                        entries.push(entry);
                        partials.push(entry_outcome.partial_bit);
                        entry_offset = (*entry_outcome.state).end();
                    }

                    // Remove all trailing partial entries
                    while !partials.is_empty() && partials[partials.len() - 1] {
                        entries.pop();
                    }
                    outcome.entries = Some(entries.into_boxed_slice());
                }
                Ok(Either::Right(outcome))
            }
        }
    }

    /// Appends an upgraded changeset to the Oplog.
    pub(crate) fn append_changeset(
        &mut self,
        changeset: &MerkleTreeChangeset,
        bitfield_update: Option<BitfieldUpdate>,
        atomic: bool,
        header: &Header,
    ) -> Result<OplogCreateHeaderOutcome, HypercoreError> {
        let mut header: Header = header.clone();
        let entry = self.update_header_with_changeset(changeset, bitfield_update, &mut header)?;

        Ok(OplogCreateHeaderOutcome {
            header,
            infos_to_flush: self.append_entries(&[entry], atomic)?,
        })
    }

    pub(crate) fn update_header_with_changeset(
        &mut self,
        changeset: &MerkleTreeChangeset,
        bitfield_update: Option<BitfieldUpdate>,
        header: &mut Header,
    ) -> Result<Entry, HypercoreError> {
        let tree_nodes: Vec<Node> = changeset.nodes.clone();
        let entry: Entry = if changeset.upgraded {
            let hash = changeset
                .hash
                .as_ref()
                .expect("Upgraded changeset must have a hash before appended");
            let signature = changeset
                .signature
                .expect("Upgraded changeset must be signed before appended");
            let signature: Box<[u8]> = signature.to_bytes().into();
            header.tree.root_hash = hash.clone();
            header.tree.signature = signature.clone();
            header.tree.length = changeset.length;

            Entry {
                user_data: vec![],
                tree_nodes,
                tree_upgrade: Some(EntryTreeUpgrade {
                    fork: changeset.fork,
                    ancestors: changeset.ancestors,
                    length: changeset.length,
                    signature,
                }),
                bitfield: bitfield_update,
            }
        } else {
            Entry {
                user_data: vec![],
                tree_nodes,
                tree_upgrade: None,
                bitfield: bitfield_update,
            }
        };
        Ok(entry)
    }

    /// Clears a segment, returns infos to write to storage.
    pub(crate) fn clear(
        &mut self,
        start: u64,
        end: u64,
    ) -> Result<Box<[StoreInfo]>, HypercoreError> {
        let entry: Entry = Entry {
            user_data: vec![],
            tree_nodes: vec![],
            tree_upgrade: None,
            bitfield: Some(BitfieldUpdate {
                drop: true,
                start,
                length: end - start,
            }),
        };
        self.append_entries(&[entry], false)
    }

    /// Flushes pending changes, returns infos to write to storage.
    pub(crate) fn flush(
        &mut self,
        header: &Header,
        clear_traces: bool,
    ) -> Result<Box<[StoreInfo]>, HypercoreError> {
        let (new_header_bits, infos_to_flush) = if clear_traces {
            // When clearing traces, both slots need to be cleared, hence
            // do this twice, but for the first time, ignore the truncate
            // store info, to end up with three StoreInfos.
            let (new_header_bits, infos_to_flush) =
                Self::insert_header(header, 0, self.header_bits, clear_traces)?;
            let mut combined_infos_to_flush: Vec<StoreInfo> =
                infos_to_flush.into_vec().drain(0..1).into_iter().collect();
            let (new_header_bits, infos_to_flush) =
                Self::insert_header(header, 0, new_header_bits, clear_traces)?;
            combined_infos_to_flush.extend(infos_to_flush.into_vec());
            (new_header_bits, combined_infos_to_flush.into_boxed_slice())
        } else {
            Self::insert_header(header, 0, self.header_bits, clear_traces)?
        };
        self.entries_byte_length = 0;
        self.entries_length = 0;
        self.header_bits = new_header_bits;
        Ok(infos_to_flush)
    }

    /// Appends a batch of entries to the Oplog.
    fn append_entries(
        &mut self,
        batch: &[Entry],
        atomic: bool,
    ) -> Result<Box<[StoreInfo]>, HypercoreError> {
        let len = batch.len();
        let header_bit = self.get_current_header_bit();
        // Leave room for leaders
        let mut state = HypercoreState::new_with_start_and_end(0, len * 8);

        for entry in batch.iter() {
            state.preencode(entry)?;
        }

        let mut buffer = state.create_buffer();
        for (i, entry) in batch.iter().enumerate() {
            (*state).add_start(8)?;
            let start = state.start();
            let partial_bit: bool = atomic && i < len - 1;
            state.encode(entry, &mut buffer)?;
            Self::prepend_leader(
                state.start() - start,
                header_bit,
                partial_bit,
                &mut state,
                &mut buffer,
            )?;
        }

        let index = OplogSlot::Entries as u64 + self.entries_byte_length;
        self.entries_length += len as u64;
        self.entries_byte_length += buffer.len() as u64;

        Ok(vec![StoreInfo::new_content(Store::Oplog, index, &buffer)].into_boxed_slice())
    }

    fn fresh(key_pair: PartialKeypair) -> Result<OplogOpenOutcome, HypercoreError> {
        let entries_length: u64 = 0;
        let entries_byte_length: u64 = 0;
        let header = Header::new(key_pair);
        let (header_bits, infos_to_flush) =
            Self::insert_header(&header, entries_byte_length, INITIAL_HEADER_BITS, false)?;
        let oplog = Oplog {
            header_bits,
            entries_length,
            entries_byte_length,
        };
        Ok(OplogOpenOutcome::from_create_header_outcome(
            oplog,
            OplogCreateHeaderOutcome {
                header,
                infos_to_flush,
            },
        ))
    }

    fn insert_header(
        header: &Header,
        entries_byte_length: u64,
        current_header_bits: [bool; 2],
        clear_traces: bool,
    ) -> Result<([bool; 2], Box<[StoreInfo]>), HypercoreError> {
        // The first 8 bytes will be filled with `prepend_leader`.
        let data_start_index: usize = 8;
        let mut state = HypercoreState::new_with_start_and_end(data_start_index, data_start_index);

        // Get the right slot and header bit
        let (oplog_slot, header_bit) =
            Oplog::get_next_header_oplog_slot_and_bit_value(&current_header_bits);
        let mut new_header_bits = current_header_bits;
        match oplog_slot {
            OplogSlot::FirstHeader => new_header_bits[0] = header_bit,
            OplogSlot::SecondHeader => new_header_bits[1] = header_bit,
            _ => {
                panic!("Invalid oplog slot");
            }
        }

        // Preencode the new header
        (*state).preencode(header)?;

        // If clearing, lets add zeros to the end
        let end = if clear_traces {
            let end = state.end();
            state.set_end(HEADER_SIZE);
            end
        } else {
            state.end()
        };

        // Create a buffer for the needed data
        let mut buffer = state.create_buffer();

        // Encode the header
        (*state).encode(header, &mut buffer)?;

        // Finally prepend the buffer's 8 first bytes with a CRC, len and right bits
        Self::prepend_leader(
            end - data_start_index,
            header_bit,
            false,
            &mut state,
            &mut buffer,
        )?;

        // The oplog is always truncated to the minimum byte size, which is right after
        // all of the entries in the oplog finish.
        let truncate_index = OplogSlot::Entries as u64 + entries_byte_length;
        Ok((
            new_header_bits,
            vec![
                StoreInfo::new_content(Store::Oplog, oplog_slot as u64, &buffer),
                StoreInfo::new_truncate(Store::Oplog, truncate_index),
            ]
            .into_boxed_slice(),
        ))
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
        state: &mut HypercoreState,
        buffer: &mut Box<[u8]>,
    ) -> Result<(), HypercoreError> {
        // The 4 bytes right before start of data is the length in 8+8+8+6=30 bits. The 31st bit is
        // the partial bit and 32nd bit the header bit.
        let start = (*state).start();
        (*state).set_start(start - len - 4)?;
        let len_u32: u32 = len.try_into().unwrap();
        let partial_bit: u32 = if partial_bit { 2 } else { 0 };
        let header_bit: u32 = if header_bit { 1 } else { 0 };
        let combined: u32 = (len_u32 << 2) | header_bit | partial_bit;
        state.encode_u32(combined, buffer)?;

        // Before that, is a 4 byte CRC32 that is a checksum of the above encoded 4 bytes and the
        // content.
        let start = state.start();
        state.set_start(start - 8)?;
        let checksum = crc32fast::hash(&buffer[state.start() + 4..state.start() + 8 + len]);
        state.encode_u32(checksum, buffer)?;
        Ok(())
    }

    /// Validates that leader at given index is valid, and returns header and partial bits and
    /// `State` for the header/entry that the leader was for.
    fn validate_leader(
        index: usize,
        buffer: &[u8],
    ) -> Result<Option<ValidateLeaderOutcome>, HypercoreError> {
        if buffer.len() < index + 8 {
            return Ok(None);
        }
        let mut state = HypercoreState::new_with_start_and_end(index, buffer.len());
        let stored_checksum: u32 = state.decode_u32(buffer)?;
        let combined: u32 = state.decode_u32(buffer)?;
        let len = usize::try_from(combined >> 2)
            .expect("Attempted converting to a 32 bit usize on below 32 bit system");

        // NB: In the Javascript version IIUC zero length is caught only with a mismatch
        // of checksums, which is silently interpreted to only mean "no value". That doesn't sound good:
        // better to throw an error on mismatch and let the caller at least log the problem.
        if len == 0 || state.end() - state.start() < len {
            return Ok(None);
        }
        let header_bit = combined & 1 == 1;
        let partial_bit = combined & 2 == 2;

        let new_start = index + 8;
        state.set_end(new_start + len);
        state.set_start(new_start)?;

        let calculated_checksum = crc32fast::hash(&buffer[index + 4..state.end()]);
        if calculated_checksum != stored_checksum {
            return Err(HypercoreError::InvalidChecksum {
                context: "Calculated signature does not match oplog signature".to_string(),
            });
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
