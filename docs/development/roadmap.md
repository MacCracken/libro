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

## Planned

### Streaming & Integration
- [ ] Streaming subscription — subscribe to new entries (via majra pub/sub)
- [ ] MCP tools: `libro_query`, `libro_verify`, `libro_export`

### Hardware Security
- [ ] Remote attestation (TPM-backed chain sealing)

### Distributed
- [ ] Multi-node chain sync (federated audit across fleet)

### Ergonomics
- [ ] `EventSeverity` ordering (`Ord` impl) for "severity >= Warning" queries
- [ ] Batch append (multiple entries in one call, single hash link computation)
- [ ] Chain iterator with pagination for large chains
