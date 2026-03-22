//! # Libro — Cryptographic Audit Chain
//!
//! Libro (Italian/Spanish: book, record) provides tamper-proof event logging
//! with hash-linked entries. Every event is chained to the previous via SHA-256,
//! making any modification detectable.
//!
//! ## Modules
//!
//! - [`entry`] — Audit entries with hash linking
//! - [`chain`] — The audit chain: append, verify, query
//! - [`store`] — Persistence backends (memory, file, custom)
//! - [`verify`] — Chain integrity verification

pub mod chain;
pub mod entry;
pub mod store;
pub mod verify;

mod error;
pub use error::LibroError;

pub use chain::AuditChain;
pub use entry::{AuditEntry, EventSeverity};
pub use verify::verify_chain;

pub type Result<T> = std::result::Result<T, LibroError>;

#[cfg(test)]
mod tests;
