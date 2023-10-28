use crate::encoding::{CompactEncoding, EncodingError, HypercoreState};
use crate::{common::BitfieldUpdate, Node};

/// Entry tree upgrade
#[derive(Debug)]
pub(crate) struct EntryTreeUpgrade {
    pub(crate) fork: u64,
    pub(crate) ancestors: u64,
    pub(crate) length: u64,
    pub(crate) signature: Box<[u8]>,
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
