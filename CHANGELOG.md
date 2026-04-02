# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.91.0] — 2026-04-02

### Changed
- Upgraded majra from 0.21.3 to 1.0 (stable release)
- Upgraded rusqlite from 0.34 to 0.39
- Upgraded criterion from 0.5 to 0.8
- License changed from AGPL-3.0-only to GPL-3.0-only (aligns with AGNOS ecosystem)
- Updated `cargo-deny` config: added `CC0-1.0`, `MIT-0`, `Unlicense`, `LGPL-2.1-or-later` to license allowlist; removed stale entries

### Fixed
- `SqliteStore::len()` adapted for rusqlite 0.39 (`usize` no longer implements `FromSql`)

## [0.90.0] — 2026-04-02

### Added
- **Serde** (`Serialize`/`Deserialize`) on: `ChainArchive`, `ChainReview`, `IntegrityStatus`, `MerkleProof`, `ProofNode`, `Side`, `QueryFilter`, `RetentionPolicy`, `EntrySignature`, `VerifyingKey`
- **`PartialEq`** on: `AuditEntry`, `ChainArchive`, `ChainReview`, `IntegrityStatus`, `MerkleProof`, `ProofNode`, `EntrySignature`, `RetentionPolicy`
- **`Clone`** on: `ChainArchive`, `IntegrityStatus`
- **`#[non_exhaustive]`** on public structs: `ChainArchive`, `ChainReview`, `MerkleProof`, `ProofNode`, `EntrySignature`
- **`#[non_exhaustive]`** on public enums: `EventSeverity`, `Side`, `IntegrityStatus`, `RetentionPolicy`
- **`#[must_use]`** on pure functions: `verify()`, `compute_hash()`, `matches()`, `verify_proof()`, `root()`, `leaf_count()`, `at_or_above()`, `as_str()`, signing key methods
- **`#[inline]`** on hot-path accessors: all `AuditEntry` field accessors, `EventSeverity::as_str()`, `QueryFilter::matches()`, chain size methods, `hash_pair()`
- Re-exported `ProofNode`, `Side`, and `IntegrityStatus` from crate root
- Doc comments on `verify_chain()`, all `LibroError` variants, `SqliteStore` module with usage example
- Custom serde for `RetentionPolicy` — `KeepDuration` serialized as seconds (i64), `KeepAfter` as RFC3339
- Custom serde for `VerifyingKey` — serialized as hex string
- `#[serde(skip_serializing_if = "Option::is_none")]` on `QueryFilter` fields for compact JSON
- Signing and SQLite benchmarks (`sign_entry`, `verify_signature`, `sqlite_append_100`, `sqlite_query_100`)
- **BLAKE3** as default hash backend — 4-10x faster than SHA-256, 128-bit collision resistance, 256-bit output
- `sha256` feature flag for NIST FIPS 180-4 compliance environments
- `hash_algorithm` field on `AuditEntry` — identifies the hash algorithm used, enables verification across algorithm transitions
- `ChainHasher` internal abstraction for pluggable hash backends
- `key_id` field on `EntrySignature` — identifies signing key for key rotation workflows
- `SigningKey::sign_with_key_id()` — sign with an explicit key identifier
- `RetentionPolicy::pci_dss()` — PCI DSS 4.0 Req 10.7 (12 months)
- `RetentionPolicy::hipaa()` — HIPAA 45 CFR 164.530(j) (6 years)
- `RetentionPolicy::sox()` — SOX Section 802 (7 years)
- `RetentionPolicy::gdpr()` — GDPR-aligned with caller-specified purpose duration
- Compliance standards mapping documentation (`docs/compliance/standards-mapping.md`)
- 168 tests, 95%+ coverage (up from 145)

### Changed
- **Breaking:** Default hash algorithm changed from SHA-256 to BLAKE3; use `sha256` feature for SHA-256
- `csv_escape()` uses `Cow<str>` to avoid allocation when no escaping needed
- `abbreviate_hash()` uses `Cow<str>` to avoid allocation for short hashes
- Merkle tree `build()` moves levels instead of cloning — 25% faster build
- Merkle tree pre-allocates `next_level` Vec with capacity
- Signing `hex_encode()` uses `write!` into pre-allocated buffer
- Benchmark script now runs with `--all-features`

### Fixed
- Clippy `needless_borrow` in test code

## [0.22.4] — 2026-03-22

### Added
- `AuditChain::append_with_agent()` — append an entry with agent ID in one call (previously required manual entry construction and `pub(crate)` access)

## [0.22.3] — 2026-03-22

### Added
- `FileStore` — append-only JSON Lines file backend (`file_store` module)
- `SqliteStore` — queryable SQLite backend with indexed columns, behind `sqlite` feature flag
- Chain rotation: `AuditChain::rotate()` returns `ChainArchive`, new entries link to previous head
- `AuditChain::from_entries()` to restore a chain from archived entries
- `SqliteStore::query_by_source()`, `query_by_severity()`, `query_by_agent()` for indexed queries
- `query` module with `QueryFilter` — composable multi-field filtering (source, severity, agent, action, time range)
- `AuditChain::by_agent()` and `AuditChain::query()` methods
- `SqliteStore::query()` — translates `QueryFilter` to indexed SQL WHERE clauses
- `FileStore::query()` — load + filter in memory
- `export` module: `to_jsonl()` and `to_csv()` writing to any `io::Write` target
- `retention` module: `RetentionPolicy` enum (KeepCount, KeepDuration, KeepAfter)
- `AuditChain::apply_retention()` — archive entries outside the retention window
- `EventSeverity::as_str()` — stable string representation for hashing and storage
- `AuditEntry` accessor methods: `id()`, `timestamp()`, `severity()`, `source()`, `action()`, `details()`, `agent_id()`, `prev_hash()`, `hash()`
- Advisory file locking (`flock`) on `FileStore` append and load for concurrent-process safety
- `fs2` dependency for cross-platform file locking
- `review` module: `ChainReview` with integrity status, time range, source/severity/agent distributions
- `AuditChain::review()` — produce a structured chain summary with `Display` for human-readable output
- `Display` impl for `AuditEntry` and `EventSeverity`
- `tracing` instrumentation: append, verify, rotate, retention, store open, parse errors
- `merkle` module: `MerkleTree` with `build()`, `root()`, `proof()`, and `verify_proof()` for O(log N) inclusion proofs
- `signing` module (feature: `signing`): Ed25519 per-entry signatures with `SigningKey`, `VerifyingKey`, `EntrySignature`
- `EventSeverity` now implements `Ord`/`PartialOrd`/`Hash` — variants ordered Debug < Info < Warning < Error < Critical < Security
- `EventSeverity::at_or_above()` — returns all severity levels at or above a given level
- `QueryFilter::min_severity()` — filter to entries with severity >= a given level (SQL `IN(...)` for SqliteStore)
- `AuditChain::append_batch()` — append multiple entries in one call
- `AuditChain::page(offset, limit)` — paginated access to chain entries
- `AuditStore::load_page(offset, limit)` — paginated loading with SQL LIMIT/OFFSET override for SqliteStore
- `AuditStore::load_and_verify()` — convenience that loads and verifies in one call
- `AuditStore::query()` — trait-level query with default load+filter impl; `SqliteStore` overrides with SQL WHERE
- `streaming` module (feature: `streaming`): `AuditStream` for real-time pub/sub via majra with MQTT-style topic wildcards
- 84 tests, 94% line coverage

### Changed
- **Breaking:** `compute_hash` now length-prefixes each variable-length field (little-endian u64) to prevent second-preimage collisions via field boundary shifting. Hashes from previous versions are incompatible.
- **Breaking:** `AuditEntry` fields are now private — use accessor methods instead. Construction still via `AuditEntry::new()` and `.with_agent()`. This prevents accidental mutation that bypasses hash integrity.
- **Breaking:** `compute_hash` now uses `EventSeverity::as_str()` (stable) instead of `Debug` format, and canonical sorted-key JSON for details. Hashes from previous versions are incompatible.
- `AuditChain::verify()` now delegates to `verify_chain()` after genesis check, eliminating duplicated logic
- `AuditChain::apply_retention()` moved from orphan impl in `retention.rs` to `chain.rs`
- CSV export now escapes `agent_id` field (user-provided, may contain commas)
- `AuditEntry::Display` no longer panics on short/empty hash strings
- `FileStore::open` uses atomic `OpenOptions::create(true)` instead of TOCTOU `exists()`+`create()`
- `verify_chain` computes hash once per entry instead of twice on failure
- `rotate()` on empty chain no longer sets `prev_chain_hash` to `Some("")`
- `query()` moved to `AuditStore` trait (polymorphic access via `dyn AuditStore`)
- `AuditStore::load_all` docs now warn that it does not verify integrity
- `RetentionPolicy::apply_retention` avoids double clone via `Vec::split_off`
- Key types re-exported from crate root: `QueryFilter`, `RetentionPolicy`, `to_jsonl`, `to_csv`

### Removed
- `LibroError::EmptyChain` variant (was dead code, never constructed)
- `SqliteStore::query_by_source`, `query_by_severity`, `query_by_agent` — superseded by `SqliteStore::query(&QueryFilter)`
- `tracing` dependency (was listed but never used)

## [0.21.3] — 2026-03-21

### Fixed
- Corrected `EmptyChain` error variant — previously unreachable, now reserved for store-level semantics

### Changed
- Tightened `thiserror` dependency to major version 2

## [0.21.2] — 2026-03-20

### Added
- Criterion benchmarks for `append` and `verify` operations (`benches/chain.rs`)

### Changed
- Improved CI pipeline: added MSRV check (1.89), `cargo-deny` supply-chain audit, codecov integration

## [0.21.1] — 2026-03-19

### Added
- `verify` module — standalone `verify_chain()` function for external audit tools
- Integration tests for full chain lifecycle, tamper detection, and error display

### Fixed
- Genesis entry validation now checks `prev_hash` is empty

## [0.21.0] — 2026-03-18

### Added
- `AuditStore` trait for pluggable persistence backends
- `MemoryStore` — in-memory backend (for testing and ephemeral use)
- `store` module with unit tests

### Changed
- `LibroError` extended with `Store`, `Io`, and `Json` variants for persistence error handling

## [0.20.0] — 2026-03-17

### Added
- `AuditChain` — append-only chain with hash linking, verification, and query methods
- `by_source()` and `by_severity()` query methods on `AuditChain`
- `head_hash()` to retrieve the chain head
- Chain-level tamper detection tests

## [0.19.0] — 2026-03-16

### Added
- `AuditEntry::with_agent()` builder method for optional agent ID tracking
- Serde `Serialize`/`Deserialize` on `AuditEntry` and `EventSeverity`
- Serde roundtrip test

## [0.18.0] — 2026-03-15

### Added
- `AuditEntry` — core audit entry with UUID, timestamp, severity, source, action, JSON details
- `EventSeverity` enum: Debug, Info, Warning, Error, Critical, Security
- SHA-256 hash computation and self-verification (`compute_hash`, `verify`)
- Hash-linked chaining via `prev_hash` field
- `LibroError` with `IntegrityViolation` variant
- Entry creation, tamper detection, and chaining tests

## [0.1.0] — 2026-03-14

### Added
- Initial project scaffolding extracted from daimon agent-runtime audit module
- Cargo workspace setup (edition 2024, MSRV 1.89, AGPL-3.0)
- CI pipeline (`ci.yml`) with fmt, clippy, test, and audit steps
- Release workflow (`release.yml`) with multi-platform builds and crates.io publish
- `Makefile` with standard development targets
- `VERSION` file and `scripts/version-bump.sh`
- README with architecture overview, roadmap, and reference code pointers

[Unreleased]: https://github.com/MacCracken/libro/compare/v0.91.0...HEAD
[0.91.0]: https://github.com/MacCracken/libro/compare/v0.90.0...v0.91.0
[0.90.0]: https://github.com/MacCracken/libro/compare/v0.25.3...v0.90.0
[0.22.4]: https://github.com/MacCracken/libro/compare/v0.22.3...v0.22.4
[0.22.3]: https://github.com/MacCracken/libro/compare/v0.21.3...v0.22.3
[0.21.3]: https://github.com/MacCracken/libro/compare/v0.21.2...v0.21.3
[0.21.2]: https://github.com/MacCracken/libro/compare/v0.21.1...v0.21.2
[0.21.1]: https://github.com/MacCracken/libro/compare/v0.21.0...v0.21.1
[0.21.0]: https://github.com/MacCracken/libro/compare/v0.20.0...v0.21.0
[0.20.0]: https://github.com/MacCracken/libro/compare/v0.19.0...v0.20.0
[0.19.0]: https://github.com/MacCracken/libro/compare/v0.18.0...v0.19.0
[0.18.0]: https://github.com/MacCracken/libro/compare/v0.1.0...v0.18.0
[0.1.0]: https://github.com/MacCracken/libro/releases/tag/v0.1.0
