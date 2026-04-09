# Libro

> **Libro** (Italian: book) — cryptographic audit chain for tamper-proof event logging

[![License: GPL-3.0](https://img.shields.io/badge/license-GPL--3.0--only-blue.svg)](LICENSE)
[![Language: Cyrius](https://img.shields.io/badge/language-Cyrius-orange.svg)](https://github.com/MacCracken/cyrius)

Libro provides an append-only, SHA-256 hash-linked audit chain where every event is chained to the previous entry's hash. Any modification to any entry breaks the chain, making tampering detectable. Zero external dependencies — Cyrius stdlib only.

## Architecture

```
libro (Cyrius library)
  ├── 19 modules, single-file compilation via include
  ├── SHA-256 (FIPS 180-4, implemented from scratch)
  ├── HMAC-SHA256 signing (Ed25519 deferred)
  └── zero external dependencies

Consumers:
  daimon ──→ libro (agent lifecycle audit)
  aegis  ──→ libro (security events)
  stiva  ──→ libro (container lifecycle)
  sigil  ──→ libro (trust decisions)
  ark    ──→ libro (package operations)
```

## Features

- **Hash-linked entries** — SHA-256 chain with length-prefixed field hashing
- **Append-only** — no update, no delete; immutable audit trail
- **Chain verification** — constant-time hash comparison, linkage checks
- **Severity levels** — Debug, Info, Warning, Error, Critical, Security (ordered)
- **Agent tracking** — optional agent_id per entry
- **Storage backend** — `MemoryStore` with streaming verification (O(chunk_size) memory)
- **Chain rotation** — archive old entries, link new chain to previous head
- **Auto-rotation** — capacity-based rotation with overflow archive tracking
- **Composable queries** — filter by source, severity, agent, action, time range (ANDed)
- **Export** — JSON Lines and CSV to any file descriptor
- **Retention policies** — keep N, keep by duration, keep after timestamp; PCI DSS / HIPAA / SOX / GDPR presets
- **Merkle tree** — O(log N) inclusion proofs, RFC 9162 consistency proofs, canonical roots
- **Digital signatures** — HMAC-SHA256 signing with key rotation support
- **Integrity proofs** — signed tree heads, inclusion/consistency proofs, anchor bundles
- **Witness anchoring** — self-hashing anchors with meta-chain support
- **RFC 3161 timestamping** — DER encoding/decoding for trusted timestamps
- **Streaming** — MQTT-style pub/sub with wildcard topic matching
- **Kernel audit** — AGNOS `/proc/agnos/audit` integration
- **Structured tracing** — sakshi instrumentation on all key operations
- **193 tests, 15 benchmarks** — comprehensive coverage

## Quick Start

```bash
# Install Cyrius (if not already installed)
cyriusup install 2.7.2 && cyriusup use 2.7.2

# Build
cyrius build src/main.cyr build/libro

# Run tests (193 tests)
./build/libro

# Run benchmarks (15 benchmarks)
cyrius build benches/libro.bcyr build/libro_bench && ./build/libro_bench
```

### Usage Example

```cyrius
# Include libro modules (via single-file compilation)
include "src/chain.cyr"    # brings all dependencies

# Create an audit chain
var c = chain_new();

# Append entries
chain_append(c, SEV_INFO, str_from("daimon"), str_from("agent.register"),
    str_from("{\"agent_id\":\"web-01\",\"sandbox\":\"landlock\"}"));

chain_append(c, SEV_SECURITY, str_from("aegis"), str_from("intrusion.detected"),
    str_from("{\"source\":\"10.0.0.5\",\"port\":22}"));

# Verify chain integrity
var err = chain_verify(c);
assert(err == 0, "chain valid");

# Query security events
var q = query_new();
query_min_severity(q, SEV_SECURITY);
var alerts = chain_query(c, q);

# Build Merkle tree + inclusion proof
var tree = merkle_build(chain_entries(c));
var proof = merkle_proof(tree, 0);
assert(merkle_verify_proof(proof) == 1, "proof valid");

# Export to CSV
var fd = file_open("audit.csv", O_WRONLY | O_CREAT | O_TRUNC, 0x1A4);
export_csv(chain_entries(c), fd);
file_close(fd);
```

## Modules

| Module | Description |
|--------|-------------|
| `entry` | `AuditEntry` — UUID v4, RFC 3339 timestamps, severity, canonical JSON hashing |
| `chain` | `AuditChain` — append, rotate, auto-rotate, verify, query, pagination |
| `verify` | Chain integrity verification with constant-time comparison |
| `store` | `MemoryStore` — in-memory backend with streaming verification |
| `query` | `QueryFilter` — composable multi-field filtering (source, severity, agent, action, time) |
| `export` | JSON Lines and CSV export with field escaping |
| `retention` | `RetentionPolicy` — count, duration, absolute; PCI DSS / HIPAA / SOX / GDPR presets |
| `review` | `ChainReview` — structured summary with integrity status and distributions |
| `merkle` | `MerkleTree` — inclusion proofs, RFC 9162 consistency proofs, canonical roots |
| `signing` | HMAC-SHA256 signing, key generation, entry signatures, key rotation |
| `anchoring` | `WitnessAnchor` — self-hashing snapshots, anchor chaining, verification |
| `timestamping` | RFC 3161 DER encoding/decoding, timestamp requests/responses/attestations |
| `proof` | `IntegrityProof` — signed tree heads, inclusion/consistency proofs, anchor bundles |
| `streaming` | MQTT-style pub/sub with `*` and `#` wildcards |
| `kernel_audit` | AGNOS kernel audit interface (`/proc/agnos/audit`) |
| `sha256` | FIPS 180-4 SHA-256 (from scratch, 32-bit masked arithmetic) |
| `hasher` | ChainHasher wrapper, hex encode/decode, length-prefixed field hashing |
| `error` | Error types and structured error objects |

## Project Structure

```
src/main.cyr           Entry point + 193 tests
src/*.cyr              Library modules (18 files)
benches/libro.bcyr     15 benchmarks
lib/                   Vendored Cyrius stdlib
build/                 Compiled binaries (gitignored)
docs/                  Architecture, guides, compliance, ADRs
```

## Documentation

- [Quick Start Guide](docs/guides/quickstart.md)
- [Testing Guide](docs/guides/testing.md)
- [Integration Patterns](docs/guides/integration.md)
- [Architecture Overview](docs/architecture/overview.md)
- [Threat Model](docs/development/threat-model.md)
- [Compliance Mapping](docs/compliance/standards-mapping.md)
- [Roadmap](docs/development/roadmap.md)
- [Changelog](CHANGELOG.md)

## License

GPL-3.0-only — see [LICENSE](LICENSE) for details.
