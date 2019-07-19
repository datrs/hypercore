/// A message sent over the network.
#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    start: usize,
    length: Option<usize>,
}
