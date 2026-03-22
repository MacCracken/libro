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
- **Storage backends** — `AuditStore` trait with memory, file (JSON Lines), and SQLite backends
- **Chain rotation** — archive old entries, start new chain linked to previous head
- **Composable queries** — filter by source, severity, agent, action, time range (all ANDed)
- **Export** — JSON Lines and CSV to any `io::Write` target
- **Retention policies** — keep N entries, keep by duration, keep after timestamp
- **Merkle tree** — build from chain, O(1) root comparison, O(log N) inclusion proofs
- **Digital signatures** — Ed25519 per-entry signing and verification (feature: `signing`)
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
| `store` | `AuditStore` trait + `MemoryStore` |
| `file_store` | `FileStore` — append-only JSON Lines persistence |
| `sqlite_store` | `SqliteStore` — queryable SQLite persistence (feature: `sqlite`) |
| `query` | `QueryFilter` — composable, multi-field entry filtering |
| `export` | Export to JSON Lines and CSV (`to_jsonl`, `to_csv`) |
| `retention` | `RetentionPolicy` — keep N entries, keep by age, keep after timestamp |
| `review` | `ChainReview` — structured chain summary with integrity, distributions, time range |
| `merkle` | `MerkleTree` — O(log N) inclusion proofs for partial verification |
| `signing` | Ed25519 per-entry signatures (feature: `signing`) |
| `verify` | Standalone chain verification (for external audit tools) |

## Roadmap

See [docs/development/roadmap.md](docs/development/roadmap.md) for full roadmap and completed milestones.

## Reference Code

| Source | What to Reference | Path | Maturity |
|--------|------------------|------|----------|
| **Daimon** audit module | Existing cryptographic audit hash chain in agent-runtime | `userland/agent-runtime/src/` (audit-related modules) | **High** — production code, libro was extracted from this |
| **Aegis** | Security event types, severity patterns | `userland/agent-runtime/src/aegis.rs` | **High** — 55 tests |
| **Sigil** | Trust verification events, signature chain patterns | `userland/agent-runtime/src/sigil.rs` | **High** — 46 tests |

## License

AGPL-3.0 — see [LICENSE](LICENSE) for details.
