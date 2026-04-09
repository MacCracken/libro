# Architecture Overview

## Module Map

```
libro (Cyrius library, single-file compilation)
├── entry           — AuditEntry, EventSeverity, UUID v4, RFC 3339 timestamps
├── chain           — AuditChain: append, verify, query, rotate, retain, paginate
├── verify          — Standalone chain verification with constant-time comparison
├── store           — MemoryStore: append, load, verify, streaming verification
├── query           — QueryFilter: composable multi-field filtering
├── export          — JSON Lines and CSV export with field escaping
├── retention       — RetentionPolicy: count, duration, absolute; compliance presets
├── review          — ChainReview: structured summary with integrity status
├── merkle          — MerkleTree: inclusion proofs, RFC 9162 consistency proofs
├── signing         — HMAC-SHA256 signing, key generation, entry signatures
├── anchoring       — WitnessAnchor: self-hashing snapshots, meta-chain
├── timestamping    — RFC 3161 DER encoding/decoding, timestamp attestations
├── proof           — IntegrityProof: signed tree heads, proof bundles
├── streaming       — MQTT-style pub/sub with wildcard topic matching
├── kernel_audit    — AGNOS /proc/agnos/audit interface
├── sha256          — FIPS 180-4 SHA-256 (from scratch)
├── hasher          — ChainHasher: hex encode/decode, length-prefixed hashing
└── error           — Error types and structured error objects
```

## Design Principles

- **Append-only** — entries cannot be modified or deleted after creation
- **Integrity by construction** — entry construction always computes hash
- **Zero dependencies** — Cyrius stdlib only, no external packages
- **Deterministic hashing** — canonical JSON (sorted keys), length-prefixed fields
- **Constant-time security** — all hash comparisons use bitwise OR accumulation
- **Structured tracing** — sakshi instrumentation on all key operations

## Data Flow

```
Event → entry_new() → hash computed → chain_append()
                                           │
                                     MemoryStore
                                           │
                                    memstore_load_all / memstore_query
                                           │
                            ┌──────────────┼──────────────┐
                            ▼              ▼              ▼
                      verify_chain   merkle_build    chain_review
                            │              │              │
                            ▼              ▼              ▼
                    integrity check  inclusion proofs  summary report
```

## Hash Algorithm

SHA-256 (FIPS 180-4) with length-prefixed variable fields:

```
hash = SHA-256(
    id (16 bytes, fixed, no prefix)
    || LE_u64(len(timestamp_rfc3339)) || timestamp_rfc3339
    || LE_u64(len(severity_str))     || severity_str
    || LE_u64(len(source))           || source
    || LE_u64(len(action))           || action
    || canonical_json(details)
    || LE_u64(len(agent_id))         || agent_id
    || LE_u64(len(prev_hash))        || prev_hash
)
```

Length prefixes are 8-byte little-endian unsigned integers, preventing second-preimage attacks via field boundary shifting.

## Consumers

| Project | Audit Domain |
|---------|-------------|
| daimon | Agent lifecycle (register, sandbox, deregister) |
| aegis | Security events (policy violations, intrusions) |
| stiva | Container lifecycle (create, start, stop, kill) |
| sigil | Trust decisions (signatures, key rotation) |
| ark | Package events (install, update, remove) |
