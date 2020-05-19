/// An event emitted by a Feed.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Event {
    /// A new block has been appended.
    Append,
    /// A new block has been downloaded.
    Download(u64),
}
