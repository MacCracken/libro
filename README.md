# Libro

> **Libro** (Italian: book) — cryptographic audit chain for tamper-proof event logging

[![License: GPL-3.0](https://img.shields.io/badge/license-GPL--3.0--only-blue.svg)](LICENSE)
[![Language: Cyrius](https://img.shields.io/badge/language-Cyrius-orange.svg)](https://github.com/MacCracken/cyrius)

Libro provides an append-only, SHA-256 hash-linked audit chain where every event is chained to the previous entry's hash. Any modification to any entry breaks the chain, making tampering detectable. Cyrius-native with no third-party deps — crypto comes from [sigil](https://github.com/MacCracken/sigil), SQL storage from [patra](https://github.com/MacCracken/patra), structured tracing from sakshi.

## Architecture

```
libro (Cyrius library, single-file compilation)
  ├── 21 library modules under src/ + 1 opt-in (src/tpm_anchor.cyr, -D LIBRO_TPM)
  ├── SHA-256 (FIPS 180-4) + Ed25519 (RFC 8032) via sigil
  ├── SQL persistence via patra (v1.13.9 bundled)
  ├── Nested scalar-aware canonical JSON hashing
  ├── Distribution artifact: committed dist/libro.cyr per DEPS-PATTERN.md
  └── CI-enforced gates: manifest completeness, raw-offset guards,
                         per-file allowlist, dist freshness, version parity

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
- **Chain verification** — constant-time hash comparison via sigil's `ct_eq`
- **Severity levels** — Debug, Info, Warning, Error, Critical, Security (ordered)
- **Agent tracking** — optional `agent_id` per entry
- **Storage backends**
  - `MemoryStore` — in-memory with streaming verification
  - `FileStore` — append-only JSON Lines with flock locking
  - `PatraStore` — SQL-backed via patra, indexed queries
- **Chain rotation** — archive old entries, link new chain to previous head
- **Auto-rotation** — capacity-based rotation with overflow archive tracking
- **Batch append** — `chain_append_batch` for N entries in one rotation check
- **Composable queries** — filter by source, severity, agent, action, time range (ANDed)
- **Export** — JSON Lines and CSV to any file descriptor
- **Chain import/export** — portable JSONL snapshot round-trip (`chain_export` / `chain_import`)
- **Streamed verification** — `filestore_verify_streamed` bounded-memory over large files
- **Retention policies** — keep N, keep by duration, keep after timestamp; PCI DSS / HIPAA / SOX / GDPR presets
- **Merkle tree** — O(log N) inclusion proofs, RFC 9162 consistency proofs, canonical roots
- **Ed25519 signatures** — per-entry signing via sigil, key rotation via `key_id`
- **Integrity proofs** — signed tree heads, inclusion/consistency proofs, anchor bundles; JSON export via `proof_to_json`
- **Witness anchoring** — self-hashing anchors with meta-chain support
- **RFC 3161 timestamping** — hand-rolled DER encode/decode for trusted timestamps
- **Streaming** — MQTT-style pub/sub with wildcard topic matching (`*`, `#`)
- **Kernel audit** — AGNOS `/proc/agnos/audit` integration
- **Structured tracing** — sakshi instrumentation on all key operations

## Coverage

- **373 inline tests** across unit / integration / layout-invariant / gap coverage
- **33 benchmarks** across three bench binaries (`libro_core` 18 + `libro_io` 12 + `libro_proof` 3)
- **12 fuzz targets** in a single harness (sha256, hex decode, DER, entry create, chain ops, sig verify, JSON parse, topic match, chain_import, filestore_verify_streamed, canonical_json_hash, proof_from_json)
- **CI history** — each run emits bench rows to `bench-history.csv` tagged with commit SHA, retained as a workflow artifact

## Quick Start

```bash
# Cyrius toolchain (read pin from cyrius.cyml)
cyriusup install "$(grep -E '^cyrius[[:space:]]*=' cyrius.cyml | sed -E 's/.*"([^"]+)".*/\1/')"

# Build (DCE matches CI/release)
CYRIUS_DCE=1 cyrius build src/main.cyr build/libro

# Run tests (518 tests default / 530 with -D LIBRO_TPM, 0 failures expected)
./build/libro

# Run benchmarks
CYRIUS_DCE=1 cyrius build benches/libro_core.bcyr  build/libro_bench_core  && ./build/libro_bench_core
CYRIUS_DCE=1 cyrius build benches/libro_io.bcyr    build/libro_bench_io    && ./build/libro_bench_io
CYRIUS_DCE=1 cyrius build benches/libro_proof.bcyr build/libro_bench_proof && ./build/libro_bench_proof

# Run fuzz (12 targets, no-crash assertions, ~10 s)
CYRIUS_DCE=1 cyrius build fuzz/fuzz_libro.fcyr build/fuzz_libro && timeout 30 ./build/fuzz_libro
```

### Usage Example

```cyrius
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
var proof = merkle_inclusion_proof(tree, 0);
assert(merkle_verify_proof(proof) == 1, "proof valid");

# Portable chain snapshot
chain_export(c, str_from("audit.jsonl"));
var restored = chain_import(str_from("audit.jsonl"));
assert(chain_verify(restored) == 0, "round-trip preserves integrity");
```

## Library modules (21)

| Module           | Description |
|------------------|-------------|
| `error`          | Structured error types with field / index / expected-vs-actual |
| `hasher`         | SHA-256 wrapper, hex encode/decode, length-prefixed field hashing (delegates to sigil) |
| `entry`          | `AuditEntry` — UUID v4, RFC 3339 timestamps, severity, nested-scalar canonical JSON |
| `verify`         | Standalone `verify_chain(entries, base_index)` for loose entries / streams / archives |
| `query`          | `QueryFilter` — composable multi-field filtering |
| `retention`      | `RetentionPolicy` — count / duration / absolute; PCI / HIPAA / SOX / GDPR presets |
| `chain`          | `AuditChain` — append, batch append, rotate, auto-rotate, verify, query, pagination |
| `store`          | `MemoryStore` — in-memory backend with streaming verification |
| `export`         | JSON Lines and CSV export with field escaping |
| `review`         | `ChainReview` — structured summary with integrity status, source/severity/agent distributions |
| `merkle`         | `MerkleTree`, `MerkleProof`, `ConsistencyProof` — RFC 9162 + inclusion proofs |
| `signing`        | Ed25519 signing via sigil, key generation, entry signatures, `key_id` rotation |
| `anchoring`      | `WitnessAnchor` — self-hashing snapshots, anchor meta-chain |
| `timestamping`   | RFC 3161 DER encode/decode, timestamp requests / responses / attestations |
| `proof`          | `IntegrityProof` — signed tree heads, inclusion/consistency proofs, anchor bundles |
| `kernel_audit`   | AGNOS `/proc/agnos/audit` read interface |
| `file_store`     | Append-only JSON Lines backend with flock, streaming verify |
| `chain_io`       | Portable chain snapshot — `chain_export` / `chain_import` JSONL round-trip |
| `patra_store`    | SQL-backed backend via patra with indexed queries |
| `streaming`      | MQTT-style pub/sub with `*` and `#` wildcards |
| `proof_json`     | JSON emitter for `IntegrityProof` — separate module; `libro_proof` benches it (`proof_to_json_25`) since 2.7.2 |

## Project structure

```
src/main.cyr             Entry point + 518/530 inline tests (default / LIBRO_TPM)
src/*.cyr                21 library modules + src/tpm_anchor.cyr (opt-in via -D LIBRO_TPM)
benches/libro_core.bcyr  18 core benchmarks (sha256/chain/merkle/sign/PQ/hybrid)
benches/libro_io.bcyr    12 i/o benchmarks (export/review/anchor/stream/filestore/patra perf)
benches/libro_proof.bcyr  3 proof-path benchmarks (unsigned + signed + to_json)
benches/bench_history.cyr Opt-in CSV history emitter (LIBRO_BENCH_HISTORY env var)
fuzz/fuzz_libro.fcyr    1 harness, 12 fuzz targets
dist/libro.cyr          Consumer distribution artifact (cyrius distlib, committed)
lib/                    Vendored stdlib copies + sigil + patra bundles
scripts/version-bump.sh Syncs VERSION + cyrius.cyml version field
docs/                   Architecture, guides, threat model, compliance, ADRs, audits
```

## Documentation

**Getting started**
- [Quick Start Guide](docs/guides/quickstart.md)
- [Testing Guide](docs/guides/testing.md)
- [Integration Patterns](docs/guides/integration.md)

**Design**
- [Architecture Overview](docs/architecture/overview.md)
- [Threat Model](docs/development/threat-model.md)
- [Compliance Mapping](docs/compliance/standards-mapping.md)
- [Roadmap](docs/development/roadmap.md)
- [Distribution contract (DEPS-PATTERN.md)](DEPS-PATTERN.md)

**Architecture Decision Records**
- [ADR 0001 — Cyrius port from Rust](docs/adr/0001-cyrius-port.md)
- [ADR 0002 — SHA-256 only (BLAKE3 dropped)](docs/adr/0002-sha256-only.md)
- [ADR 0003 — HMAC signing (superseded by 1.0.2 Ed25519 via sigil)](docs/adr/0003-hmac-signing.md)
- [ADR 0004 — MemoryStore as primary backend](docs/adr/0004-memorystore-primary.md)
- [ADR 0005 — `#derive(accessors)` adoption (2.0)](docs/adr/0005-derive-accessors.md)
- [ADR 0006 — dist artifact contract (2.0)](docs/adr/0006-dist-artifact-contract.md)
- [ADR 0007 — Nested scalar-aware canonical-JSON hashing (2.0)](docs/adr/0007-canonical-json-hashing.md)

**Security audits**
- [2026-04-19 — Pre-1.1.0 audit](docs/audit/2026-04-19-audit.md)
- [2026-04-19 — 2.0 hardening audit + post-release addenda](docs/audit/2026-04-19-audit-2.0.md)

**Policies**
- [Security Policy](SECURITY.md)
- [Contributing Guide](CONTRIBUTING.md)
- [Code of Conduct](CODE_OF_CONDUCT.md)
- [Changelog](CHANGELOG.md)

## License

GPL-3.0-only — see [LICENSE](LICENSE) for details.
