use compact_encoding::types::{take_array, take_array_mut, write_array, CompactEncodable};

use crate::encoding::{CompactEncoding, EncodingError, HypercoreState};
use crate::{chain_encoded_bytes, decode, sum_encoded_size};
use crate::{common::BitfieldUpdate, Node};

/// Entry tree upgrade
#[derive(Debug)]
pub(crate) struct EntryTreeUpgrade {
    pub(crate) fork: u64,
    pub(crate) ancestors: u64,
    pub(crate) length: u64,
    pub(crate) signature: Box<[u8]>,
}

impl CompactEncodable for EntryTreeUpgrade {
    fn encoded_size(&self) -> Result<usize, EncodingError> {
        Ok(sum_encoded_size!(self, fork, ancestors, length, signature))
    }

    fn encoded_bytes<'a>(&self, buffer: &'a mut [u8]) -> Result<&'a mut [u8], EncodingError> {
        Ok(chain_encoded_bytes!(
            self, buffer, fork, ancestors, length, signature
        ))
    }

    fn decode(buffer: &[u8]) -> Result<(Self, &[u8]), EncodingError>
    where
        Self: Sized,
    {
        decode!(EntryTreeUpgrade, buffer, {fork: u64, ancestors: u64, length: u64, signature: Box<[u8]>})
    }
}
impl CompactEncoding<EntryTreeUpgrade> for HypercoreState {
    fn preencode(&mut self, value: &EntryTreeUpgrade) -> Result<usize, EncodingError> {
        self.0.preencode(&value.fork)?;
        self.0.preencode(&value.ancestors)?;
        self.0.preencode(&value.length)?;
        self.0.preencode(&value.signature)
    }

    fn encode(
        &mut self,
        value: &EntryTreeUpgrade,
        buffer: &mut [u8],
    ) -> Result<usize, EncodingError> {
        self.0.encode(&value.fork, buffer)?;
        self.0.encode(&value.ancestors, buffer)?;
        self.0.encode(&value.length, buffer)?;
        self.0.encode(&value.signature, buffer)
    }

    fn decode(&mut self, buffer: &[u8]) -> Result<EntryTreeUpgrade, EncodingError> {
        let fork: u64 = self.0.decode(buffer)?;
        let ancestors: u64 = self.0.decode(buffer)?;
        let length: u64 = self.0.decode(buffer)?;
        let signature: Box<[u8]> = self.0.decode(buffer)?;
        Ok(EntryTreeUpgrade {
            fork,
            ancestors,
            length,
            signature,
        })
    }
}

impl CompactEncodable for BitfieldUpdate {
    fn encoded_size(&self) -> Result<usize, EncodingError> {
        Ok(1 + sum_encoded_size!(self, start, length))
    }

    fn encoded_bytes<'a>(&self, buffer: &'a mut [u8]) -> Result<&'a mut [u8], EncodingError> {
        let drop = if self.drop { 1 } else { 0 };
        let rest = write_array(&[drop], buffer)?;
        Ok(chain_encoded_bytes!(self, rest, start, length))
    }

    fn decode(buffer: &[u8]) -> Result<(Self, &[u8]), EncodingError>
    where
        Self: Sized,
    {
        let ([flags], rest) = take_array::<1>(buffer)?;
        let (start, rest) = u64::decode(rest)?;
        let (length, rest) = u64::decode(rest)?;
        Ok((
            BitfieldUpdate {
                drop: flags == 1,
                start,
                length,
            },
            rest,
        ))
    }
}

impl CompactEncoding<BitfieldUpdate> for HypercoreState {
    fn preencode(&mut self, value: &BitfieldUpdate) -> Result<usize, EncodingError> {
        self.0.add_end(1)?;
        self.0.preencode(&value.start)?;
        self.0.preencode(&value.length)
    }

    fn encode(
        &mut self,
        value: &BitfieldUpdate,
        buffer: &mut [u8],
    ) -> Result<usize, EncodingError> {
        let flags: u8 = if value.drop { 1 } else { 0 };
        self.0.set_byte_to_buffer(flags, buffer)?;
        self.0.encode(&value.start, buffer)?;
        self.0.encode(&value.length, buffer)
    }

    fn decode(&mut self, buffer: &[u8]) -> Result<BitfieldUpdate, EncodingError> {
        let flags = self.0.decode_u8(buffer)?;
        let start: u64 = self.0.decode(buffer)?;
        let length: u64 = self.0.decode(buffer)?;
        Ok(BitfieldUpdate {
            drop: flags == 1,
            start,
            length,
        })
    }
}

/// Oplog Entry
#[derive(Debug)]
pub struct Entry {
    // TODO: This is a keyValueArray in JS
    pub(crate) user_data: Vec<String>,
    pub(crate) tree_nodes: Vec<Node>,
    pub(crate) tree_upgrade: Option<EntryTreeUpgrade>,
    pub(crate) bitfield: Option<BitfieldUpdate>,
}

impl CompactEncodable for Entry {
    fn encoded_size(&self) -> Result<usize, EncodingError> {
        let mut out = 1; // flags
        if !self.user_data.is_empty() {
            out += self.user_data.encoded_size()?;
        }
        if !self.tree_nodes.is_empty() {
            out += self.tree_nodes.encoded_size()?;
        }
        if let Some(tree_upgrade) = &self.tree_upgrade {
            out += tree_upgrade.encoded_size()?;
        }
        if let Some(bitfield) = &self.bitfield {
            out += bitfield.encoded_size()?;
        }
        Ok(out)
    }

    fn encoded_bytes<'a>(&self, buffer: &'a mut [u8]) -> Result<&'a mut [u8], EncodingError> {
        let (flag_buf, mut rest) = take_array_mut::<1>(buffer)?;
        let mut flags = 0u8;
        if !self.user_data.is_empty() {
            flags |= 1;
            rest = self.user_data.encoded_bytes(rest)?;
        }
        if !self.tree_nodes.is_empty() {
            flags |= 2;
            rest = self.tree_nodes.encoded_bytes(rest)?;
        }
        if let Some(tree_upgrade) = &self.tree_upgrade {
            flags |= 4;
            rest = tree_upgrade.encoded_bytes(rest)?;
        }
        if let Some(bitfield) = &self.bitfield {
            flags |= 8;
            rest = bitfield.encoded_bytes(rest)?;
        }
        flag_buf[0] = flags;
        Ok(rest)
    }

    fn decode(buffer: &[u8]) -> Result<(Self, &[u8]), EncodingError>
    where
        Self: Sized,
    {
        let ([flags], rest) = take_array::<1>(buffer)?;
        let (user_data, rest) = if flags & 1 != 0 {
            <Vec<String>>::decode(rest)?
        } else {
            (Default::default(), rest)
        };

        let (tree_nodes, rest) = if flags & 2 != 0 {
            <Vec<Node>>::decode(buffer)?
        } else {
            (Default::default(), rest)
        };

        let (tree_upgrade, rest) = if flags & 2 != 0 {
            let (x, rest) = EntryTreeUpgrade::decode(buffer)?;
            (Some(x), rest)
        } else {
            (Default::default(), rest)
        };

        let (bitfield, rest) = if flags & 2 != 0 {
            let (x, rest) = BitfieldUpdate::decode(buffer)?;
            (Some(x), rest)
        } else {
            (Default::default(), rest)
        };

        Ok((
            Self {
                user_data,
                tree_nodes,
                tree_upgrade,
                bitfield,
            },
            rest,
        ))
    }
}

impl CompactEncoding<Entry> for HypercoreState {
    fn preencode(&mut self, value: &Entry) -> Result<usize, EncodingError> {
        self.0.add_end(1)?; // flags
        if !value.user_data.is_empty() {
            self.0.preencode(&value.user_data)?;
        }
        if !value.tree_nodes.is_empty() {
            self.preencode(&value.tree_nodes)?;
        }
        if let Some(tree_upgrade) = &value.tree_upgrade {
            self.preencode(tree_upgrade)?;
        }
        if let Some(bitfield) = &value.bitfield {
            self.preencode(bitfield)?;
        }
        Ok(self.end())
    }

    fn encode(&mut self, value: &Entry, buffer: &mut [u8]) -> Result<usize, EncodingError> {
        let start = self.0.start();
        self.0.add_start(1)?;
        let mut flags: u8 = 0;
        if !value.user_data.is_empty() {
            flags |= 1;
            self.0.encode(&value.user_data, buffer)?;
        }
        if !value.tree_nodes.is_empty() {
            flags |= 2;
            self.encode(&value.tree_nodes, buffer)?;
        }
        if let Some(tree_upgrade) = &value.tree_upgrade {
            flags |= 4;
            self.encode(tree_upgrade, buffer)?;
        }
        if let Some(bitfield) = &value.bitfield {
            flags |= 8;
            self.encode(bitfield, buffer)?;
        }

        buffer[start] = flags;
        Ok(self.0.start())
    }

    fn decode(&mut self, buffer: &[u8]) -> Result<Entry, EncodingError> {
        let flags = self.0.decode_u8(buffer)?;
        let user_data: Vec<String> = if flags & 1 != 0 {
            self.0.decode(buffer)?
        } else {
            vec![]
        };

        let tree_nodes: Vec<Node> = if flags & 2 != 0 {
            self.decode(buffer)?
        } else {
            vec![]
        };

        let tree_upgrade: Option<EntryTreeUpgrade> = if flags & 4 != 0 {
            let value: EntryTreeUpgrade = self.decode(buffer)?;
            Some(value)
        } else {
            None
        };

        let bitfield: Option<BitfieldUpdate> = if flags & 8 != 0 {
            let value: BitfieldUpdate = self.decode(buffer)?;
            Some(value)
        } else {
            None
        };

        Ok(Entry {
            user_data,
            tree_nodes,
            tree_upgrade,
            bitfield,
        })
    }
}
