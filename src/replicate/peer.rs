// use sparse_bitfield::Bitfield;

use super::Message;

/// A peer on the network.
// omitted fields: [
//  feed,
//  stream,
//  inflightRequests,
// ]
#[derive(Debug, Clone, PartialEq)]
pub struct Peer {
  // remote_id: usize,
// remote_length: usize,
// remote_bitfield: Bitfield,
// remote_is_want: bool,
// remote_is_downloading: bool,
// is_live: bool,
// is_sparse: bool,
// is_downloading: bool,
// is_uploading: bool,
// max_requests: u16,
}

impl Peer {
  /// Check if the peer has a message.
  pub fn have(&mut self, _msg: &Message) {
    unimplemented!();
  }

  /// Tell a peer you no longer have a message.
  pub fn unhave(&mut self, _msg: &Message) {
    unimplemented!();
  }

  /// Update.
  pub fn update(&mut self) {
    unimplemented!();
  }
}
