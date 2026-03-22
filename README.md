# Libro

> **Libro** (Italian/Spanish: book, record) — cryptographic audit chain for tamper-proof event logging

[![License: AGPL-3.0](https://img.shields.io/badge/license-AGPL--3.0-blue.svg)](LICENSE)

Libro provides an append-only, hash-linked audit chain where every event is chained to the previous via SHA-256. Any modification to any entry breaks the chain, making tampering detectable.

## Architecture

```
libro (this crate)
  └── sha2 (SHA-256 hash linking)

Consumers:
  daimon ──→ libro (agent lifecycle audit: register, sandbox, deregister)
  aegis  ──→ libro (security events: policy violations, intrusion attempts)
  stiva  ──→ libro (container lifecycle: create, start, stop, kill)
  sigil  ──→ libro (trust decisions: signature verification, key rotation)
  ark    ──→ libro (package events: install, update, remove)
```

## Features

- **Hash-linked entries** — each entry contains SHA-256 of the previous, forming a tamper-proof chain
- **Append-only** — no update, no delete. Immutable audit trail
- **Chain verification** — verify integrity of entire chain or any subsequence
- **Severity levels** — Debug, Info, Warning, Error, Critical, Security
- **Agent tracking** — optional agent_id per entry for per-agent audit trails
- **Storage backends** — `AuditStore` trait with memory backend (file/SQLite planned)
- **Structured details** — arbitrary JSON payload per entry

## Quick Start

```rust
use libro::{AuditChain, EventSeverity};

let mut chain = AuditChain::new();

chain.append(
    EventSeverity::Info,
    "daimon",
    "agent.register",
    serde_json::json!({ "agent_id": "web-agent-01", "sandbox": "landlock" }),
);

chain.append(
    EventSeverity::Security,
    "aegis",
    "intrusion.detected",
    serde_json::json!({ "source": "10.0.0.5", "port": 22, "attempts": 5 }),
);

// Verify chain integrity
chain.verify().expect("chain is valid");

// Tampering breaks the chain
// chain.entries()[0].details = serde_json::json!("tampered");
// chain.verify() → Err(IntegrityViolation)
```

## Modules

| Module | Description |
|--------|-------------|
| `entry` | `AuditEntry` with UUID, timestamp, severity, source, action, JSON details, hash linking |
| `chain` | `AuditChain` — append, query, verify, head hash |
| `store` | `AuditStore` trait + `MemoryStore` (file/SQLite backends planned) |
| `verify` | Standalone chain verification (for external audit tools) |

## Roadmap

### Done
- [x] Hash-linked audit entries (SHA-256)
- [x] Append-only chain with verification
- [x] Severity levels (Debug through Security)
- [x] Agent ID tracking
- [x] AuditStore trait with memory backend
- [x] 17 tests

### Phase 1 — Persistence
- [ ] File-based store (append-only log file, one JSON entry per line)
- [ ] SQLite store (for queryable audit history)
- [ ] Chain rotation (archive old entries, start new chain linked to previous)

### Phase 2 — Query & Export
- [ ] Query by time range, severity, source, agent_id
- [ ] Export to JSON Lines, CSV
- [ ] Streaming — subscribe to new entries (via majra pub/sub)
- [ ] Retention policies (keep N days, keep N entries)

### Phase 3 — Advanced
- [ ] Merkle tree for efficient partial verification
- [ ] Digital signatures per entry (ed25519)
- [ ] Remote attestation (TPM-backed chain sealing)
- [ ] Multi-node chain sync (federated audit across fleet)
- [ ] MCP tools: `libro_query`, `libro_verify`, `libro_export`

## Reference Code

| Source | What to Reference | Path | Maturity |
|--------|------------------|------|----------|
| **Daimon** audit module | Existing cryptographic audit hash chain in agent-runtime | `userland/agent-runtime/src/` (audit-related modules) | **High** — production code, libro was extracted from this |
| **Aegis** | Security event types, severity patterns | `userland/agent-runtime/src/aegis.rs` | **High** — 55 tests |
| **Sigil** | Trust verification events, signature chain patterns | `userland/agent-runtime/src/sigil.rs` | **High** — 46 tests |

## License

AGPL-3.0 — see [LICENSE](LICENSE) for details.
