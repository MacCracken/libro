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

## Planned

### Integration (via bote — planned for bote v0.23.3)
- [ ] MCP tools: `libro_query`, `libro_verify`, `libro_export` — implemented in [bote](https://github.com/MacCracken/bote) as registered tool handlers using libro as a dependency
- [ ] Audit integration: bote dispatches audit events to libro chain

### Hardware Security
- [ ] Remote attestation (TPM-backed chain sealing) — requires `tss-esapi` + TPM hardware

### Distributed
- [ ] Multi-node chain sync (federated audit across fleet) — requires networking/consensus infrastructure
