use crate::{
    common::BitfieldUpdate,
    compact_encoding::{CompactEncoding, State},
    Node,
};

/// Entry tree upgrade
#[derive(Debug)]
pub struct EntryTreeUpgrade {
    pub(crate) fork: u64,
    pub(crate) ancestors: u64,
    pub(crate) length: u64,
    pub(crate) signature: Box<[u8]>,
}

impl CompactEncoding<EntryTreeUpgrade> for State {
    fn preencode(&mut self, value: &EntryTreeUpgrade) {
        self.preencode(&value.fork);
        self.preencode(&value.ancestors);
        self.preencode(&value.length);
        self.preencode(&value.signature);
    }

    fn encode(&mut self, value: &EntryTreeUpgrade, buffer: &mut [u8]) {
        self.encode(&value.fork, buffer);
        self.encode(&value.ancestors, buffer);
        self.encode(&value.length, buffer);
        self.encode(&value.signature, buffer);
    }

    fn decode(&mut self, buffer: &[u8]) -> EntryTreeUpgrade {
        let fork: u64 = self.decode(buffer);
        let ancestors: u64 = self.decode(buffer);
        let length: u64 = self.decode(buffer);
        let signature: Box<[u8]> = self.decode(buffer);
        EntryTreeUpgrade {
            fork,
            ancestors,
            length,
            signature,
        }
    }
}

impl CompactEncoding<BitfieldUpdate> for State {
    fn preencode(&mut self, value: &BitfieldUpdate) {
        self.end += 1;
        self.preencode(&value.start);
        self.preencode(&value.length);
    }

    fn encode(&mut self, value: &BitfieldUpdate, buffer: &mut [u8]) {
        let flags: u8 = if value.drop { 1 } else { 0 };
        buffer[self.start] = flags;
        self.start += 1;
        self.encode(&value.start, buffer);
        self.encode(&value.length, buffer);
    }

    fn decode(&mut self, buffer: &[u8]) -> BitfieldUpdate {
        let flags = self.decode_u8(buffer);
        let start: u64 = self.decode(buffer);
        let length: u64 = self.decode(buffer);
        BitfieldUpdate {
            drop: flags == 1,
            start,
            length,
        }
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

impl CompactEncoding<Entry> for State {
    fn preencode(&mut self, value: &Entry) {
        self.end += 1; // flags
        if value.user_data.len() > 0 {
            self.preencode(&value.user_data);
        }
        if value.tree_nodes.len() > 0 {
            self.preencode(&value.tree_nodes);
        }
        if let Some(tree_upgrade) = &value.tree_upgrade {
            self.preencode(tree_upgrade);
        }
        if let Some(bitfield) = &value.bitfield {
            self.preencode(bitfield);
        }
    }

    fn encode(&mut self, value: &Entry, buffer: &mut [u8]) {
        let start = self.start;
        self.start += 1;
        let mut flags: u8 = 0;
        if value.user_data.len() > 0 {
            flags = flags | 1;
            self.encode(&value.user_data, buffer);
        }
        if value.tree_nodes.len() > 0 {
            flags = flags | 2;
            self.encode(&value.tree_nodes, buffer);
        }
        if let Some(tree_upgrade) = &value.tree_upgrade {
            flags = flags | 4;
            self.encode(tree_upgrade, buffer);
        }
        if let Some(bitfield) = &value.bitfield {
            flags = flags | 8;
            self.encode(bitfield, buffer);
        }

        buffer[start] = flags;
    }

    fn decode(&mut self, buffer: &[u8]) -> Entry {
        let flags = self.decode_u8(buffer);
        let user_data: Vec<String> = if flags & 1 != 0 {
            self.decode(buffer)
        } else {
            vec![]
        };

        let tree_nodes: Vec<Node> = if flags & 2 != 0 {
            self.decode(buffer)
        } else {
            vec![]
        };

        let tree_upgrade: Option<EntryTreeUpgrade> = if flags & 4 != 0 {
            let value: EntryTreeUpgrade = self.decode(buffer);
            Some(value)
        } else {
            None
        };

        let bitfield: Option<BitfieldUpdate> = if flags & 4 != 0 {
            let value: BitfieldUpdate = self.decode(buffer);
            Some(value)
        } else {
            None
        };

        Entry {
            user_data,
            tree_nodes,
            tree_upgrade,
            bitfield,
        }
    }
}
