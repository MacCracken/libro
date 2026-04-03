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
//! | `sha256` | Use SHA-256 instead of BLAKE3 (for NIST FIPS 180-4 compliance) |
//! | `sqlite` | SQLite-backed audit store with indexed queries |
//! | `signing` | Ed25519 digital signatures per entry |
//! | `streaming` | Real-time pub/sub via majra |
//!
//! None are enabled by default. The default hash algorithm is BLAKE3.
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

#[cfg(feature = "anchoring")]
pub mod anchoring;
pub mod chain;
pub mod entry;
pub mod export;
pub mod file_store;
pub(crate) mod hasher;
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
#[cfg(feature = "timestamping")]
pub mod timestamping;
pub mod verify;

mod error;
pub use error::LibroError;

#[cfg(feature = "anchoring")]
pub use anchoring::{AnchorVerification, WitnessAnchor, WitnessBackend, WitnessReceipt};
pub use chain::{AuditChain, ChainArchive};
pub use entry::{AuditEntry, EventSeverity};
pub use export::{to_csv, to_jsonl};
pub use file_store::FileStore;
pub use merkle::{ConsistencyProof, MerkleProof, MerkleTree, ProofNode, Side};
pub use query::QueryFilter;
pub use retention::RetentionPolicy;
pub use review::{ChainReview, IntegrityStatus};
#[cfg(feature = "sqlite")]
pub use sqlite_store::SqliteStore;
#[cfg(feature = "streaming")]
pub use streaming::AuditStream;
#[cfg(feature = "timestamping")]
pub use timestamping::{TimestampAttestation, TimestampRequest, TimestampResponse};
pub use verify::verify_chain;

pub type Result<T> = std::result::Result<T, LibroError>;

#[cfg(test)]
mod tests;

// Compile-time assertions: all core public types are Send + Sync.
#[cfg(test)]
mod assert_traits {
    fn _assert_send_sync<T: Send + Sync>() {}
    fn _assert_partial_eq<T: PartialEq>() {}
    fn _assert_clone<T: Clone>() {}
    fn _assert_serde<T: serde::Serialize + serde::de::DeserializeOwned>() {}

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
        _assert_send_sync::<super::ConsistencyProof>();
    }

    #[test]
    fn core_types_are_partial_eq() {
        _assert_partial_eq::<super::AuditEntry>();
        _assert_partial_eq::<super::ChainArchive>();
        _assert_partial_eq::<super::ChainReview>();
        _assert_partial_eq::<super::IntegrityStatus>();
        _assert_partial_eq::<super::MerkleProof>();
        _assert_partial_eq::<super::ConsistencyProof>();
        _assert_partial_eq::<super::ProofNode>();
        _assert_partial_eq::<super::Side>();
        _assert_partial_eq::<super::RetentionPolicy>();
        _assert_partial_eq::<super::EventSeverity>();
    }

    #[test]
    fn core_types_are_clone() {
        _assert_clone::<super::AuditEntry>();
        _assert_clone::<super::ChainArchive>();
        _assert_clone::<super::IntegrityStatus>();
        _assert_clone::<super::MerkleTree>();
        _assert_clone::<super::MerkleProof>();
        _assert_clone::<super::ConsistencyProof>();
        _assert_clone::<super::ProofNode>();
        _assert_clone::<super::RetentionPolicy>();
        _assert_clone::<super::QueryFilter>();
    }

    #[test]
    fn core_types_are_serde() {
        _assert_serde::<super::AuditEntry>();
        _assert_serde::<super::EventSeverity>();
        _assert_serde::<super::ChainArchive>();
        _assert_serde::<super::ChainReview>();
        _assert_serde::<super::IntegrityStatus>();
        _assert_serde::<super::MerkleProof>();
        _assert_serde::<super::ConsistencyProof>();
        _assert_serde::<super::ProofNode>();
        _assert_serde::<super::Side>();
        _assert_serde::<super::QueryFilter>();
        _assert_serde::<super::RetentionPolicy>();
    }

    #[cfg(feature = "signing")]
    #[test]
    fn signing_types_traits() {
        use super::signing::{EntrySignature, SignatureAlgorithm, VerifyingKey};
        _assert_serde::<EntrySignature>();
        _assert_serde::<VerifyingKey>();
        _assert_serde::<SignatureAlgorithm>();
        _assert_partial_eq::<EntrySignature>();
        _assert_partial_eq::<SignatureAlgorithm>();
        _assert_clone::<EntrySignature>();
        _assert_clone::<VerifyingKey>();
        _assert_clone::<SignatureAlgorithm>();
        _assert_send_sync::<SignatureAlgorithm>();
    }

    #[cfg(feature = "anchoring")]
    #[test]
    fn anchoring_types_traits() {
        use super::anchoring::{AnchorVerification, WitnessAnchor, WitnessReceipt};
        _assert_send_sync::<WitnessAnchor>();
        _assert_send_sync::<WitnessReceipt>();
        _assert_send_sync::<AnchorVerification>();
        _assert_serde::<WitnessAnchor>();
        _assert_serde::<WitnessReceipt>();
        _assert_serde::<AnchorVerification>();
        _assert_clone::<WitnessAnchor>();
        _assert_clone::<WitnessReceipt>();
        _assert_clone::<AnchorVerification>();
        _assert_partial_eq::<WitnessAnchor>();
        _assert_partial_eq::<WitnessReceipt>();
        _assert_partial_eq::<AnchorVerification>();
    }

    #[cfg(feature = "timestamping")]
    #[test]
    fn timestamping_types_traits() {
        use super::timestamping::{
            TimestampAttestation, TimestampHashAlgorithm, TimestampRequest, TimestampResponse,
            TimestampStatus, TimestampSubject,
        };
        _assert_send_sync::<TimestampRequest>();
        _assert_send_sync::<TimestampResponse>();
        _assert_send_sync::<TimestampAttestation>();
        _assert_serde::<TimestampRequest>();
        _assert_serde::<TimestampResponse>();
        _assert_serde::<TimestampAttestation>();
        _assert_serde::<TimestampHashAlgorithm>();
        _assert_serde::<TimestampStatus>();
        _assert_serde::<TimestampSubject>();
        _assert_clone::<TimestampRequest>();
        _assert_clone::<TimestampResponse>();
        _assert_clone::<TimestampAttestation>();
        _assert_partial_eq::<TimestampRequest>();
        _assert_partial_eq::<TimestampResponse>();
        _assert_partial_eq::<TimestampAttestation>();
    }
}
