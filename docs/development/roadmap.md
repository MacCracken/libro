# Roadmap

## v1.0.1 — Released 2026-04-09

- [x] Cyrius toolchain pinned to v3.2.5 (cc3 compiler, minimum version)
- [x] FileStore — append-only JSON Lines backend with flock locking
- [x] Vendored stdlib updated to Cyrius 3.2.5 (36 modules, includes patra, chrono, csv, http, base64)
- [x] 202 tests, 15 benchmarks
- [x] CI/CD updated for cc3

## v1.0.0 — Released 2026-04-09

- [x] Full Cyrius port — 19 modules, 193 tests, 15 benchmarks
- [x] SHA-256 (FIPS 180-4), length-prefixed field hashing
- [x] HMAC-SHA256 signing with key rotation support
- [x] Merkle tree — inclusion proofs, RFC 9162 consistency proofs
- [x] Integrity proofs — signed tree heads, inclusion/consistency bundles, anchor support
- [x] Witness anchoring — self-hashing anchors, meta-chain
- [x] RFC 3161 timestamping — DER encoding/decoding
- [x] Streaming pub/sub — MQTT-style topic wildcards
- [x] Kernel audit — AGNOS /proc/agnos/audit interface
- [x] Structured tracing — sakshi instrumentation
- [x] CI/CD — GitHub Actions for build, test, bench, security scan, release

## Post-v1.0

### Blocked — Waiting on Ecosystem Dependencies

> **IMPORTANT:** The items below are blocked on other AGNOS repos converting to Cyrius.
> Do NOT attempt to implement these until the dependencies are ready.

#### Blocked on sigil (crypto primitives)
- [ ] **Replace `src/sha256.cyr` with sigil's SHA-256** — sigil will be the single source of crypto primitives across the ecosystem. When sigil converts to Cyrius, libro drops its hand-rolled SHA-256 and includes sigil's verified implementation instead.
- [ ] **Replace `src/signing.cyr` HMAC with sigil's Ed25519** — sigil will provide Ed25519 (and eventually ML-DSA). Libro's HMAC-SHA256 signing is a placeholder; the API surface is designed for drop-in replacement.
- [ ] **`src/hasher.cyr` → sigil dependency** — ChainHasher wraps SHA-256; will delegate to sigil.
- [ ] Post-quantum signatures (ML-DSA) via sigil
- [ ] Hybrid signing: Ed25519 + PQ for transition period

#### Blocked on patra (SQL storage)
- [ ] **Replace MemoryStore/FileStore with patra SQL backend** — patra provides B-tree indexed storage, WAL crash recovery, and SQL queries. Libro's current MemoryStore is in-process only; FileStore is append-only JSON Lines. Patra gives us indexed queries, transactions, and durability.
- [ ] **Note:** patra bundles its own SHA-256 which conflicts with libro's `src/sha256.cyr`. Once both use sigil's SHA-256, the conflict resolves. Do NOT include patra.cyr directly until sigil migration is complete.

### v1.1 — Hardening (not blocked)
- [ ] Nested JSON canonical hashing (depth > 1)
- [ ] Benchmark history tracking (CSV append per run)
- [ ] Fuzz harnesses for DER parser and entry deserialization
- [ ] Chain export/import (full chain serialization to file)
- [ ] Streaming verification for FileStore

### Future
- [ ] Remote attestation (TPM-backed chain sealing)
- [ ] Multi-node chain sync (federated audit across fleet)
- [ ] Conflict resolution for concurrent appends
- [ ] MCP tools via bote: `libro_query`, `libro_verify`, `libro_export`
