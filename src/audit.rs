/// The audit report for a feed, created by the `.audit()` method.
#[derive(Debug, PartialEq, Clone)]
pub struct Audit {
    /// The number of valid blocks identified
    pub valid_blocks: u64,
    /// The number of invalid blocks identified
    pub invalid_blocks: u64,
}

impl Audit {
    /// Access the `valid_blocks` field from the proof.
    pub fn valid_blocks(&self) -> u64 {
        self.valid_blocks
    }

    /// Access the `invalid_blocks` field from the proof.
    pub fn invalid_blocks(&self) -> u64 {
        self.invalid_blocks
    }
}
