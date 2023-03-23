use thiserror::Error;

/// Common error type for the hypercore interface
#[derive(Error, Debug)]
pub enum HypercoreError {
    /// Invalid signature
    #[error("Given signature was invalid.")]
    InvalidSignature,
    /// Unexpected IO error occured
    #[error("Unrecoverable input/output error occured.{}",
          .context.as_ref().map_or_else(String::new, |ctx| format!(" Context: {}.", ctx)))]
    IO {
        /// Context for the error
        context: Option<String>,
        /// Original source error
        #[source]
        source: std::io::Error,
    },
}

impl From<std::io::Error> for HypercoreError {
    fn from(err: std::io::Error) -> Self {
        Self::IO {
            context: None,
            source: err,
        }
    }
}
