use crate::compact_encoding::{CompactEncoding, State};

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
}

impl CompactEncoding<Entry> for State {
    fn preencode(&mut self, value: &Entry) {}

    fn encode(&mut self, value: &Entry, buffer: &mut [u8]) {}

    fn decode(&mut self, buffer: &[u8]) -> Entry {
        Entry { user_data: vec![] }
    }
}
