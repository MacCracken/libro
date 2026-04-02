# Roadmap

## v1.0 Criteria

- [x] Stable public API — all types have appropriate traits, serde, `#[non_exhaustive]`
- [x] Cryptographic hardening — BLAKE3 default, SHA-256 fallback, hash algorithm versioning
- [x] Key rotation support — `key_id` on signatures, `sign_with_key_id()`
- [x] Compliance documentation — standards mapping, consumer responsibilities
- [x] Retention presets — named constructors for PCI DSS, HIPAA, SOX, GDPR
- [x] P(-1) scaffold hardening — benchmarks, audit, cleanliness checks
- [ ] Consumer integration validation — at least one AGNOS crate (daimon, aegis, stiva, sigil, or ark) uses libro 0.90.0 in production
- [ ] Release candidate period — 2 weeks minimum with no API changes

## Post-v1

### Integration (via bote) — DONE (bote 0.91.0)
- [x] MCP tools: `libro_query`, `libro_verify`, `libro_export`
- [x] Audit integration: bote dispatches audit events to libro chain

### Cryptographic Evolution
- [ ] Post-quantum signature scheme (CRYSTALS-Dilithium or ML-DSA) via feature flag
- [ ] Hybrid signing: Ed25519 + PQ for transition period
- [ ] RFC 3161 trusted timestamping integration (optional TSA client)
- [ ] Merkle root anchoring to external witness (periodic root hash export)

### Hardware Security
- [ ] Remote attestation (TPM-backed chain sealing) — requires `tss-esapi` + TPM hardware
- [ ] FIPS 140-3 validated cryptographic backend (via `aws-lc-rs` or validated OpenSSL)

### Distributed
- [ ] Multi-node chain sync (federated audit across fleet)
- [ ] Conflict resolution for concurrent appends
- [ ] Consensus-backed append (Raft/PBFT for multi-writer chains)
