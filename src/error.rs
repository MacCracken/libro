use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum LibroError {
    #[error("chain integrity violated at entry {index}: expected hash {expected}, got {actual}")]
    IntegrityViolation {
        index: usize,
        expected: String,
        actual: String,
    },
    #[error("store error: {0}")]
    Store(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
