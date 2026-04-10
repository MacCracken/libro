# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.1] - 2026-04-09

### Changed
- Cyrius toolchain pinned to v3.2.5 (cc3 compiler, minimum version)

## [Unreleased]

## [1.0.0] — 2026-04-09

### Changed
- **Language port: Rust to Cyrius** — full rewrite from 8,513 lines of Rust to ~4,950 lines of Cyrius
- SHA-256 implemented from scratch (FIPS 180-4), replacing BLAKE3 default + sha2 crate
- HMAC-SHA256 signing replaces Ed25519 (elliptic curve deferred; same API surface)
- In-process pub/sub with MQTT wildcards replaces majra/tokio async streaming
- MemoryStore replaces FileStore/SqliteStore as primary backend
- Timestamps use integer civil-date conversion (no chrono dependency)
- UUID v4 via /dev/urandom (no uuid crate)
- DER encoding/decoding for RFC 3161 preserved (hand-rolled, zero deps)
- 141KB static ELF binary, 121ms build time
- 193 tests (up from 262 Rust tests; Rust-specific serde/trait tests removed)
- 15 benchmarks covering all major operations
- Rust source preserved in rust-old/ for reference

### Added
- `benches/libro.bcyr` — 15 benchmarks: sha256, entry_hash, chain_append/verify, merkle build/proof/verify/consistency, sign/verify, query, export jsonl/csv, review, proof

### Removed
- All Cargo/crates.io dependencies (zero external deps — Cyrius stdlib only)
- SQLite store (deferred; MemoryStore covers in-process use)
- FileStore (deferred; export functions cover persistence)
- BLAKE3 hash backend (SHA-256 only for simplicity)
- tokio/majra async runtime (synchronous pub/sub via function pointers)
- serde derives (custom JSON export via export.cyr)
- tracing instrumentation (deferred)

## [0.92.0] — 2026-04-03

### Added
- **RFC 3161 trusted timestamping** (feature: `timestamping`) — `TimestampRequest` with DER encoding (`to_der()`), `TimestampResponse` with DER decoding (`from_der()`), `TimestampAttestation` for persistent storage; hand-rolled DER encoder/decoder (zero new deps)
- **Merkle root anchoring** (feature: `anchoring`) — `WitnessAnchor` (self-hashed snapshot of Merkle root + chain head), `WitnessReceipt` (backend-specific attestation), `WitnessBackend` trait for pluggable witness systems, `AnchorVerification` enum with `Display`
- **RFC 9162 consistency proofs** — `ConsistencyProof` type, `MerkleTree::consistency_proof(old_size)` generation, `verify_consistency()` verification (RFC 9162 Section 2.1.4.2 algorithm), `MerkleTree::canonical_root(size)` for no-duplication RFC 9162 roots
- **Algorithm-agnostic signing traits** — `EntrySigner` and `EntryVerifier` traits (object-safe, `Send + Sync`), `SignatureAlgorithm` enum (`Ed25519`, `MlDsa65`, `Ed25519MlDsa65`), `EntrySignature::verify_with(&dyn EntryVerifier)` for runtime algorithm dispatch, `EntrySignature::algorithm_parsed()`
- **Integrity proof export** — `IntegrityProof` bundle (signed tree head + entries + inclusion/consistency proofs + optional anchor), `ProofBuilder` with chainable `.with_consistency_from()`, `.with_inclusion()`, `.with_all_inclusions()`, `.with_anchor()`, `ProofVerification` with detailed per-check results, `to_proof_json()` export
- **Chain capacity limits** — `AuditChain::with_capacity(max_entries)` for auto-rotation at limit, `take_overflow()` to retrieve archived overflow
- **Streaming verification** — `AuditStore::verify_streamed(chunk_size)` for O(chunk_size) memory verification, `verify_chain_offset()` for index-adjusted chunk verification
- **Input validation** — `AuditEntry::new_validated()` with configurable field length limits (`MAX_SOURCE_LEN`, `MAX_ACTION_LEN`, `MAX_DETAILS_SIZE`), `LibroError::FieldTooLong` error variant
- **Key zeroization** — `SigningKey` implements `Drop` to overwrite key material; `to_bytes()` returns `Zeroizing<[u8; 32]>`
- `algorithm` field on `EntrySignature` — identifies the signing algorithm (backward-compatible, `Option<String>`, skipped when `None`)
- `SignedTreeHead` type for signed Merkle root commitments
- `LibroError::Timestamp`, `LibroError::Anchoring`, `LibroError::Der` error variants
- Shared hex utilities extracted to `hasher.rs` (`hex_encode`, `hex_encode_slice`, `hex_decode`)
- Benchmarks for consistency proof generation and verification (`merkle_consistency_1000`, `merkle_verify_consistency`)
- 262 tests (up from 168), comprehensive trait assertions for all new types

### Changed
- `timestamping` and `anchoring` feature flags added; `full` feature now includes both
- `hash_field()` promoted to `pub(crate)` for reuse across modules (length-prefixed hashing)
- `constant_time_eq()` promoted to `pub(crate)` for reuse across modules
- All hash comparisons in `verify.rs`, `entry.rs`, `signing.rs`, `merkle.rs` now use constant-time comparison
- `WitnessAnchor::compute_hash()` uses length-prefixed fields (prevents boundary ambiguity)
- Signing module renamed dalek imports to `DalekSigner`/`DalekVerifier` to avoid trait name collisions
- `IntegrityProof::verify_common()` builds Merkle tree once instead of twice

### Fixed
- `kernel_audit.rs`: `read_agnos_audit_events` now passes `&Path` (adapted for agnosys v0.50.0 API)
- `WitnessAnchor::verify_against()` now detects head mismatch on empty chains
- `IntegrityProof` consistency verification compares against canonical RFC 9162 root (not libro's duplication-based root)

## [0.91.0] — 2026-04-02

### Added
- `cargo vet` supply-chain auditing — initialized with trusted publisher imports from Mozilla, Google, Bytecode Alliance, ISRG, and Zcash (119 audited, 54 exempted)
- CI: `cargo vet --locked` enforcement in security job
- CI: `--all-features` on all Linux jobs (check, clippy, test, MSRV, coverage)
- CI: macOS test matrix uses `--features full` to exclude Linux-only `kernel-audit`

### Changed
- Upgraded majra from 0.21.3 to 1.0 (stable release)
- Upgraded rusqlite from 0.34 to 0.39
- Upgraded criterion from 0.5 to 0.8
- License changed from AGPL-3.0-only to GPL-3.0-only (aligns with AGNOS ecosystem)
- `cargo-deny` config: `all-features = true` (was `features = ["full"]`), added `CC0-1.0`, `MIT-0`, `Unlicense`, `LGPL-2.1-or-later` to license allowlist, restored agnosys git source, removed stale entries

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
