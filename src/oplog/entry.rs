use crate::{
    compact_encoding::{CompactEncoding, State},
    Node,
};

impl CompactEncoding<Node> for State {
    fn preencode(&mut self, value: &Node) {
        self.preencode(&value.index);
        self.preencode(&value.length);
        self.preencode_fixed_32();
    }

    fn encode(&mut self, value: &Node, buffer: &mut [u8]) {
        self.encode(&value.index, buffer);
        self.encode(&value.length, buffer);
        self.encode_fixed_32(&value.hash, buffer);
    }

    fn decode(&mut self, buffer: &[u8]) -> Node {
        let index: u64 = self.decode(buffer);
        let length: u64 = self.decode(buffer);
        let hash: Box<[u8]> = self.decode_fixed_32(buffer);
        Node::new(index, hash.to_vec(), length)
    }
}

impl CompactEncoding<Vec<Node>> for State {
    fn preencode(&mut self, value: &Vec<Node>) {
        let len = value.len();
        self.preencode(&len);
        for val in value.into_iter() {
            self.preencode(val);
        }
    }

    fn encode(&mut self, value: &Vec<Node>, buffer: &mut [u8]) {
        let len = value.len();
        self.encode(&len, buffer);
        for val in value {
            self.encode(val, buffer);
        }
    }

    fn decode(&mut self, buffer: &[u8]) -> Vec<Node> {
        let len: usize = self.decode(buffer);
        let mut value = Vec::with_capacity(len);
        for _ in 0..len {
            value.push(self.decode(buffer));
        }
        value
    }
}

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

/// Entry bitfield update
#[derive(Debug)]
pub struct EntryBitfieldUpdate {
    pub(crate) drop: bool,
    pub(crate) start: u64,
    pub(crate) length: u64,
}

impl CompactEncoding<EntryBitfieldUpdate> for State {
    fn preencode(&mut self, value: &EntryBitfieldUpdate) {
        self.end += 1;
        self.preencode(&value.start);
        self.preencode(&value.length);
    }

    fn encode(&mut self, value: &EntryBitfieldUpdate, buffer: &mut [u8]) {
        let flags: u8 = if value.drop { 1 } else { 0 };
        buffer[self.start] = flags;
        self.start += 1;
        self.encode(&value.start, buffer);
        self.encode(&value.length, buffer);
    }

    fn decode(&mut self, buffer: &[u8]) -> EntryBitfieldUpdate {
        let flags = self.decode_u8(buffer);
        let start: u64 = self.decode(buffer);
        let length: u64 = self.decode(buffer);
        EntryBitfieldUpdate {
            drop: flags == 1,
            start,
            length,
        }
    }
}

/// Oplog Entry
#[derive(Debug)]
pub struct Entry {
    // userData: null,
    // treeNodes: batch.nodes,
    // treeUpgrade: batch,
    // bitfield: {
    //   drop: false,
    //   start: batch.ancestors,
    //   length: values.length
    // }
    // TODO: This is a keyValueArray in JS
    pub(crate) user_data: Vec<String>,
    pub(crate) tree_nodes: Vec<Node>,
    pub(crate) tree_upgrade: Option<EntryTreeUpgrade>,
    pub(crate) bitfield: Option<EntryBitfieldUpdate>,
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

        let bitfield: Option<EntryBitfieldUpdate> = if flags & 4 != 0 {
            let value: EntryBitfieldUpdate = self.decode(buffer);
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
