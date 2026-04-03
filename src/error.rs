use thiserror::Error;

/// Errors that can occur during libro operations.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum LibroError {
    /// A chain entry's hash does not match its content, or the chain linkage is broken.
    ///
    /// `index` identifies the first invalid entry. `expected` and `actual` contain
    /// the mismatched hash values.
    #[error("chain integrity violated at entry {index}: expected hash {expected}, got {actual}")]
    IntegrityViolation {
        index: usize,
        expected: String,
        actual: String,
    },
    /// An error from a storage backend (file, SQLite, or custom store).
    #[error("store error: {0}")]
    Store(String),
    /// An I/O error from file operations.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// A JSON serialization or deserialization error.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    /// An input field exceeds the allowed maximum length.
    #[error("field `{field}` too long: {len} bytes (max {max})")]
    FieldTooLong {
        field: &'static str,
        len: usize,
        max: usize,
    },
    /// An RFC 3161 timestamp request or response is malformed.
    #[error("timestamp error: {0}")]
    Timestamp(String),
    /// A witness anchoring operation failed.
    #[error("anchoring error: {0}")]
    Anchoring(String),
    /// DER encoding/decoding error.
    #[error("DER encoding error: {0}")]
    Der(String),
}
