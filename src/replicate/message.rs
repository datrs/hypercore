/// A message sent over the network.
#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    start: u64,
    length: Option<u64>,
}
