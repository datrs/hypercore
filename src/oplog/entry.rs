use compact_encoding::{take_array, take_array_mut, write_array, CompactEncoding, EncodingError};

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

impl CompactEncoding for EntryTreeUpgrade {
    fn encoded_size(&self) -> Result<usize, EncodingError> {
        Ok(sum_encoded_size!(self, fork, ancestors, length, signature))
    }

    fn encode<'a>(&self, buffer: &'a mut [u8]) -> Result<&'a mut [u8], EncodingError> {
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

impl CompactEncoding for BitfieldUpdate {
    fn encoded_size(&self) -> Result<usize, EncodingError> {
        Ok(1 + sum_encoded_size!(self, start, length))
    }

    fn encode<'a>(&self, buffer: &'a mut [u8]) -> Result<&'a mut [u8], EncodingError> {
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

/// Oplog Entry
#[derive(Debug)]
pub(crate) struct Entry {
    // TODO: This is a keyValueArray in JS
    pub(crate) user_data: Vec<String>,
    pub(crate) tree_nodes: Vec<Node>,
    pub(crate) tree_upgrade: Option<EntryTreeUpgrade>,
    pub(crate) bitfield: Option<BitfieldUpdate>,
}

impl CompactEncoding for Entry {
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

    fn encode<'a>(&self, buffer: &'a mut [u8]) -> Result<&'a mut [u8], EncodingError> {
        let (flag_buf, mut rest) = take_array_mut::<1>(buffer)?;
        let mut flags = 0u8;
        if !self.user_data.is_empty() {
            flags |= 1;
            rest = self.user_data.encode(rest)?;
        }
        if !self.tree_nodes.is_empty() {
            flags |= 2;
            rest = self.tree_nodes.encode(rest)?;
        }
        if let Some(tree_upgrade) = &self.tree_upgrade {
            flags |= 4;
            rest = tree_upgrade.encode(rest)?;
        }
        if let Some(bitfield) = &self.bitfield {
            flags |= 8;
            rest = bitfield.encode(rest)?;
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
