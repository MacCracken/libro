# Architecture Overview

## Module Map

```
libro
├── entry           — AuditEntry, EventSeverity, hash computation
├── chain           — AuditChain: append, verify, query, rotate, retain, paginate
├── verify          — Standalone chain verification
├── store           — AuditStore trait + MemoryStore
├── file_store      — FileStore (JSON Lines, flock)
├── sqlite_store    — SqliteStore (indexed, SQL queries)        [feature: sqlite]
├── query           — QueryFilter (composable filtering)
├── export          — to_jsonl, to_csv
├── retention       — RetentionPolicy (KeepCount, KeepDuration, KeepAfter)
├── review          — ChainReview (structured summary)
├── merkle          — MerkleTree, MerkleProof, verify_proof
├── signing         — Ed25519 per-entry signatures              [feature: signing]
├── streaming       — AuditStream (majra pub/sub)               [feature: streaming]
└── error           — LibroError
```

## Feature Flags

| Flag | Dependencies | Description |
|------|-------------|-------------|
| `sqlite` | `rusqlite` (bundled) | SQLite-backed audit store with indexed queries |
| `signing` | `ed25519-dalek`, `rand_core` | Ed25519 digital signatures per entry |
| `streaming` | `majra`, `tokio` | Real-time pub/sub via majra topic hierarchy |

None are enabled by default. The core library (chain, entry, verify, query, export, merkle, retention, review, file_store) has no optional dependencies.

## Design Principles

- **Append-only** — entries cannot be modified or deleted after creation
- **Integrity by construction** — `AuditEntry` fields are private; construction always computes hash
- **Feature-gated** — heavy dependencies (SQLite, crypto, async) are opt-in
- **Zero unsafe** — no `unsafe` blocks anywhere
- **Thread-safe** — all public types are `Send + Sync`
- **Deterministic hashing** — canonical JSON (sorted keys), length-prefixed fields, stable severity representation

## Data Flow

```
Event → AuditEntry::new() → hash computed → AuditChain::append()
                                                  │
                                    ┌─────────────┼─────────────┐
                                    ▼             ▼             ▼
                              MemoryStore    FileStore     SqliteStore
                                    │             │             │
                                    └─────────────┼─────────────┘
                                                  ▼
                                          load_all / query
                                                  │
                                    ┌─────────────┼─────────────┐
                                    ▼             ▼             ▼
                              verify_chain   MerkleTree    ChainReview
```

## Hash Algorithm

SHA-256 with length-prefixed variable fields:

```
hash = SHA-256(
    id (16 bytes, fixed)
    || len(timestamp_rfc3339) || timestamp_rfc3339
    || len(severity_str)     || severity_str
    || len(source)           || source
    || len(action)           || action
    || canonical_json(details)
    || len(agent_id)         || agent_id
    || len(prev_hash)        || prev_hash
)
```

## Consumers

| Project | Audit Domain |
|---------|-------------|
| daimon | Agent lifecycle (register, sandbox, deregister) |
| aegis | Security events (policy violations, intrusions) |
| stiva | Container lifecycle (create, start, stop, kill) |
| sigil | Trust decisions (signatures, key rotation) |
| ark | Package events (install, update, remove) |
