/// The audit report for a feed, created by the `.audit()` method.
#[derive(Debug, PartialEq, Clone)]
pub struct Audit {
    /// The number of valid blocks identified
    pub valid_blocks: usize,
    /// The number of invalid blocks identified
    pub invalid_blocks: usize,
}

impl Audit {
    /// Access the `valid_blocks` field from the proof.
    pub fn valid_blocks(&self) -> usize {
        self.valid_blocks
    }

    /// Access the `invalid_blocks` field from the proof.
    pub fn invalid_blocks(&self) -> usize {
        self.invalid_blocks
    }
}
