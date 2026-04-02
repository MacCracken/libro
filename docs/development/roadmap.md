# Roadmap

## Completed

### Phase 1 — Persistence
- [x] Hash-linked audit entries (SHA-256) with length-prefixed field separators
- [x] Append-only chain with verification
- [x] Severity levels (Debug through Security)
- [x] Agent ID tracking
- [x] AuditStore trait with memory backend
- [x] File-based store (append-only JSON Lines with flock)
- [x] SQLite store (indexed, behind `sqlite` feature flag)
- [x] Chain rotation (archive + new chain linked to previous head)

### Phase 2 — Query & Export
- [x] Composable query filters (source, severity, agent, action, time range)
- [x] Query on AuditStore trait (polymorphic, SQL-optimized for SqliteStore)
- [x] Export to JSON Lines and CSV
- [x] Retention policies (KeepCount, KeepDuration, KeepAfter)

### Phase 3 — Advanced
- [x] Merkle tree for efficient partial verification (O(log N) proofs)
- [x] Ed25519 digital signatures per entry (behind `signing` feature flag)
- [x] Chain review with structured summary and Display impls
- [x] Tracing instrumentation (append, verify, rotate, retention, store ops)

### Phase 4 — Ergonomics & Streaming
- [x] `EventSeverity` ordering (`Ord`/`PartialOrd`) with `min_severity` query filter
- [x] Batch append (`append_batch`) for multiple entries in one call
- [x] Chain pagination (`page`, `load_page` on `AuditStore` trait, SQL LIMIT/OFFSET for SqliteStore)
- [x] Streaming subscription via majra pub/sub (behind `streaming` feature flag)

### Phase 5 — v1 API Stabilization (0.90.0)
- [x] Serde on all wire-facing types: `MerkleProof`, `ProofNode`, `Side`, `EntrySignature`, `VerifyingKey`, `ChainArchive`, `ChainReview`, `IntegrityStatus`, `QueryFilter`, `RetentionPolicy`
- [x] `PartialEq` on all data types: `AuditEntry`, `ChainArchive`, `ChainReview`, `IntegrityStatus`, `MerkleProof`, `ProofNode`, `EntrySignature`, `RetentionPolicy`
- [x] `Clone` on `ChainArchive`, `IntegrityStatus`
- [x] `#[non_exhaustive]` on all public enums and structs with public fields
- [x] `#[must_use]` on all pure/verification functions
- [x] `#[inline]` on hot-path accessors
- [x] Re-export `ProofNode`, `Side`, `IntegrityStatus` from crate root
- [x] Complete documentation coverage
- [x] 166 tests, 95%+ coverage

## Post-v1 (infrastructure-dependent)

### Integration (via bote)
- [ ] MCP tools: `libro_query`, `libro_verify`, `libro_export` — implemented in [bote](https://github.com/MacCracken/bote) as registered tool handlers using libro as a dependency
- [ ] Audit integration: bote dispatches audit events to libro chain

### Hardware Security
- [ ] Remote attestation (TPM-backed chain sealing) — requires `tss-esapi` + TPM hardware

### Distributed
- [ ] Multi-node chain sync (federated audit across fleet) — requires networking/consensus infrastructure
