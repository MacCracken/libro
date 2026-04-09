# Roadmap

## v1.0.0 — Released 2026-04-09

- [x] Stable public API — all types, constructors, accessors
- [x] Cryptographic hardening — SHA-256 (FIPS 180-4), length-prefixed field hashing
- [x] HMAC-SHA256 signing with key rotation support
- [x] Compliance documentation — standards mapping, consumer responsibilities
- [x] Retention presets — named constructors for PCI DSS, HIPAA, SOX, GDPR
- [x] Merkle tree — inclusion proofs, RFC 9162 consistency proofs, canonical roots
- [x] Integrity proofs — signed tree heads, inclusion/consistency bundles, anchor support
- [x] Witness anchoring — self-hashing anchors, meta-chain, verification
- [x] RFC 3161 timestamping — DER encoding/decoding, request/response/attestation
- [x] Streaming pub/sub — MQTT-style topic wildcards, in-process delivery
- [x] Kernel audit integration — AGNOS /proc/agnos/audit interface
- [x] Structured tracing — sakshi instrumentation on key operations
- [x] Full Cyrius port — 19 modules, 193 tests, 15 benchmarks
- [x] CI/CD — GitHub Actions for build, test, bench, security scan, release

## Post-v1.0

### v1.1 — Hardening
- [ ] Ed25519 signatures (replace HMAC-SHA256 when Cyrius gains elliptic curve support)
- [ ] FileStore — append-only JSON Lines persistence backend
- [ ] Nested JSON canonical hashing (depth > 1)
- [ ] Benchmark history tracking (CSV append per run)
- [ ] Fuzz harnesses for DER parser and entry deserialization

### v1.2 — Storage
- [ ] SQLite store via Cyrius FFI (when available)
- [ ] Chain export/import (full chain serialization to file)
- [ ] Streaming verification for FileStore

### Future
- [ ] Post-quantum signatures (ML-DSA) via feature flag
- [ ] Hybrid signing: Ed25519 + PQ for transition period
- [ ] Remote attestation (TPM-backed chain sealing)
- [ ] Multi-node chain sync (federated audit across fleet)
- [ ] Conflict resolution for concurrent appends
- [ ] MCP tools via bote: `libro_query`, `libro_verify`, `libro_export`
