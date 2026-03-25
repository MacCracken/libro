//! # Libro — Cryptographic Audit Chain
//!
//! Libro (Italian/Spanish: book, record) provides tamper-proof event logging
//! with hash-linked entries. Every event is chained to the previous via SHA-256,
//! making any modification detectable.
//!
//! ## Feature Flags
//!
//! | Flag | Description |
//! |------|-------------|
//! | `sqlite` | SQLite-backed audit store with indexed queries |
//! | `signing` | Ed25519 digital signatures per entry |
//! | `streaming` | Real-time pub/sub via majra |
//!
//! None are enabled by default.
//!
//! ## Modules
//!
//! - [`entry`] — Audit entries with hash linking
//! - [`chain`] — The audit chain: append, verify, query, rotate, retain, paginate
//! - [`store`] — Persistence backends (memory, file, custom)
//! - [`file_store`] — Append-only JSON Lines file backend
//! - [`query`] — Composable query filters
//! - [`export`] — JSON Lines and CSV export
//! - [`retention`] — Retention policies
//! - [`review`] — Structured chain review and summary
//! - [`merkle`] — Merkle tree for efficient partial verification
//! - [`verify`] — Chain integrity verification
//! - [`signing`] — Ed25519 per-entry signatures *(feature: `signing`)*
//! - [`streaming`] — Real-time pub/sub *(feature: `streaming`)*
//! - [`sqlite_store`] — SQLite persistence *(feature: `sqlite`)*

pub mod chain;
pub mod entry;
pub mod export;
pub mod file_store;
pub mod kernel_audit;
pub mod merkle;
pub mod query;
pub mod retention;
pub mod review;
#[cfg(feature = "signing")]
pub mod signing;
#[cfg(feature = "sqlite")]
pub mod sqlite_store;
pub mod store;
#[cfg(feature = "streaming")]
pub mod streaming;
pub mod verify;

mod error;
pub use error::LibroError;

pub use chain::{AuditChain, ChainArchive};
pub use entry::{AuditEntry, EventSeverity};
pub use export::{to_csv, to_jsonl};
pub use file_store::FileStore;
pub use merkle::{MerkleProof, MerkleTree};
pub use query::QueryFilter;
pub use retention::RetentionPolicy;
pub use review::ChainReview;
#[cfg(feature = "sqlite")]
pub use sqlite_store::SqliteStore;
#[cfg(feature = "streaming")]
pub use streaming::AuditStream;
pub use verify::verify_chain;

pub type Result<T> = std::result::Result<T, LibroError>;

#[cfg(test)]
mod tests;

// Compile-time assertions: all core public types are Send + Sync.
#[cfg(test)]
mod assert_traits {
    fn _assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn core_types_are_send_sync() {
        _assert_send_sync::<super::AuditEntry>();
        _assert_send_sync::<super::AuditChain>();
        _assert_send_sync::<super::ChainArchive>();
        _assert_send_sync::<super::QueryFilter>();
        _assert_send_sync::<super::RetentionPolicy>();
        _assert_send_sync::<super::ChainReview>();
        _assert_send_sync::<super::FileStore>();
        _assert_send_sync::<super::MerkleTree>();
        _assert_send_sync::<super::MerkleProof>();
    }
}
