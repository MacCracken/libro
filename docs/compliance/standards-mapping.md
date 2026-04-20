# Compliance Standards Mapping

How libro's features map to major compliance frameworks.

Libro is a **library**, not a deployed service. It provides
cryptographic primitives and data structures; infrastructure
concerns (network security, access control, backup, encryption at
rest) are the consumer's responsibility. This document maps what
libro provides, what consumers must implement, and how the mapping
ties back to libro's actual Cyrius API (not the historical Rust
crate).

## Coverage Matrix

| Standard | Control | Libro Feature | Status |
|----------|---------|---------------|--------|
| **ISO 27001** A.8.15 | Tamper-proof logging | SHA-256 hash-linked chain; `chain_verify(c)` / `verify_chain(entries, base_index)` | Covered |
| **ISO 27001** A.8.24 | Use of cryptography | SHA-256 + Ed25519 via sigil; length-prefixed field hashing | Covered |
| **NIST 800-53** AU-3 | Content of audit records | `AuditEntry`: `agent_id`, `action`, `timestamp`, `source`, `severity`, `details` | Covered |
| **NIST 800-53** AU-8 | Time stamps | `timestamp_rfc3339(get_epoch_secs())` — see [Timestamp Integrity](#timestamp-integrity) | Partial |
| **NIST 800-53** AU-9 | Protection of audit info | Hash chain tamper detection; `FileStore` flock; `PatraStore` via patra locking | Covered |
| **NIST 800-53** AU-10 | Non-repudiation | Ed25519 per-entry signatures (`sign_entry`); `key_id` on `EntrySignature` for rotation | Covered |
| **NIST 800-53** AU-11 | Audit record retention | `RetentionPolicy` with compliance presets | Covered |
| **NIST 800-53** SC-12 | Key management | `SigningKey` / `VerifyingKey` with `key_id` rotation support; `signing_key_zeroize` | Partial |
| **SOC 2** CC7.2 | Monitoring | Append-only chain; `streaming` module for real-time pub/sub | Covered |
| **SOC 2** CC6.1 | Logical access | See [Consumer Responsibilities](#consumer-responsibilities) | Consumer |
| **PCI DSS 4.0** 10.2 | Audit log content | Structured entries: who, what, when, where, outcome | Covered |
| **PCI DSS 4.0** 10.3 | Tamper detection | Hash chain — modification of any entry breaks all successor hashes | Covered |
| **PCI DSS 4.0** 10.5 | Log integrity | File locking, hash verification, Merkle proofs, signed tree heads | Covered |
| **PCI DSS 4.0** 10.7 | Log retention | `retention_pci_dss()` — 12 months | Covered |
| **HIPAA** 164.312(b) | Audit controls | Structured logging with severity, source, action, details | Covered |
| **HIPAA** 164.312(c) | Integrity controls | SHA-256 hash verification; Ed25519 signatures | Covered |
| **HIPAA** 164.530(j) | Record retention | `retention_hipaa()` — 6 years | Covered |
| **GDPR** Art 5(1)(f) | Integrity & confidentiality | Hash-chain integrity; encryption-at-rest is consumer's responsibility | Partial |
| **GDPR** Art 30 | Records of processing | Structured entries: source, action, agent, timestamp, details | Covered |
| **SOX** Section 802 | Record retention | `retention_sox()` — 7 years | Covered |
| **eIDAS** Art 34 | Qualified timestamps | See [Timestamp Integrity](#timestamp-integrity) | Not covered |
| **RFC 9162** | Verifiable logs (CT-style) | Merkle tree with O(log N) inclusion + RFC 9162 consistency proofs | Covered |

## Cryptographic Guarantees

### What Libro Provides

| Property | Mechanism | Strength |
|----------|-----------|----------|
| **Tamper detection** | SHA-256 hash-linked chain | Any modification to any entry invalidates all successor hashes |
| **Non-repudiation** | Ed25519 per-entry signatures via sigil | Binds entry content to a specific signing key |
| **Efficient inclusion verification** | Merkle inclusion proofs | O(log N) proof that an entry exists in the chain |
| **Append-only consistency** | RFC 9162 consistency proofs | Proves one tree state is a prefix of a later state |
| **Second-preimage resistance** | Length-prefixed field hashing + nested scalar-aware canonical JSON (2.0) | Prevents field boundary shifting AND value-type coercion attacks |
| **Deterministic hashing** | Canonical JSON: sorted keys, depth-unlimited, scalars emit verbatim | Same logical content always produces the same hash |
| **Key rotation readiness** | `key_id` on `EntrySignature` | Identifies which key signed each entry |

### Algorithms

| Purpose | Algorithm | Standard | Source | PQ-resistant? |
|---------|-----------|----------|--------|---------------|
| Entry hashing | SHA-256 | FIPS 180-4 | sigil (`lib/sigil.cyr`) | Yes |
| Entry signing | Ed25519 | RFC 8032 | sigil (`lib/sigil.cyr`) | No (PQ migration planned) |
| Key encoding | Hex (lowercase) | — | sigil `hex_encode_str` | N/A |
| Constant-time compare | Branchless `ct_eq` | — | sigil | N/A |

## Timestamp Integrity

Libro uses `get_epoch_secs()` + `timestamp_rfc3339(…)` which read the
system clock. This is **not** a trusted timestamp in the RFC 3161
sense.

**Consumer responsibilities:**
- Ensure system clocks are NTP-synchronized (RFC 5905).
- For legal/regulatory evidence, anchor Merkle roots to an external
  Timestamp Authority (TSA) per RFC 3161. Libro ships the client side
  (`src/timestamping.cyr` — `ts_request_*`, `ts_response_*`,
  `ts_attestation_*` with hand-rolled DER encode/decode).
- For eIDAS qualified timestamps, pair a qualified TSA with
  `ts_attestation` storage.

**What libro provides:**
- Monotonic entry ordering within a single chain (enforced by hash
  linking).
- `RetentionPolicy::KeepAfter` and `QueryFilter` time-range filters
  for time-based operations.
- RFC 3339 timestamps in serialized entries.
- RFC 3161 request/response DER encoding via `src/timestamping.cyr`.

## Consumer Responsibilities

These controls are outside libro's scope as a library. Consumers
MUST implement:

### Access Control (NIST AU-9, PCI DSS 10.5.1–10.5.2, ISO 27001 A.8.15)
- Restrict read/write access to `FileStore` paths and PatraStore DB
  files.
- Ensure only authorized processes can call `chain_append` /
  `chain_append_batch`.
- Use OS-level file permissions, SELinux / AppArmor, or container
  isolation.

### Transport Security (NIST SC-8, PCI DSS 4.1)
- Encrypt audit data in transit using TLS 1.2+.
- Libro's streaming module operates in-process; network transport is
  the consumer's concern.

### Backup and Disaster Recovery (PCI DSS 10.5.3, ISO 27001 A.8.13)
- Back up FileStore files and PatraStore DB files to offsite /
  immutable storage.
- Consider WORM (Write-Once Read-Many) storage for compliance-
  critical chains.
- Test restore procedures regularly. `chain_export` / `chain_import`
  provide a portable JSONL snapshot format suitable for cold
  backups.

### Encryption at Rest (HIPAA 164.312(a)(2)(iv), PCI DSS 3.4)
- Encrypt store files at the filesystem or volume level.
- Libro does not encrypt entries at rest — it provides integrity
  (hashing) not confidentiality.

### Log Shipping and Centralization (SOC 2, PCI DSS 10.5.3)
- Ship audit logs to a central SIEM or log aggregation service.
- Use `export_jsonl` / `export_csv` for SIEM ingestion, or pair with
  `src/streaming.cyr` for real-time forwarding.

### Retention Enforcement
- Libro provides `RetentionPolicy` with compliance presets but does
  NOT automatically enforce them.
- Consumers must schedule periodic `chain_apply_retention(c, policy)`
  calls.
- Archived entries should land in immutable / offsite storage before
  deletion.

## FIPS 140-3 Considerations

Libro delegates all cryptographic primitives to sigil, which is a
pure-Cyrius implementation of SHA-256 (FIPS 180-4) and Ed25519
(RFC 8032). Sigil is **not** a FIPS 140-3 validated cryptographic
module.

For FedRAMP or federal information systems:
- Replace sigil with a FIPS-validated cryptographic module (e.g., a
  Cyrius binding to `aws-lc-rs` or a validated OpenSSL binding if/when
  such a binding exists in the Cyrius ecosystem), or
- Document a Plan of Action and Milestones (POA&M) for cryptographic
  module validation.

## Post-Quantum Migration Path

SHA-256 is considered quantum-resistant (Grover's algorithm reduces
effective strength but 256-bit remains safe). Ed25519 is **not**
quantum-resistant.

Planned (ecosystem-blocked; see `docs/development/roadmap.md`):
- Feature-gated post-quantum signature scheme (ML-DSA / CRYSTALS-
  Dilithium) via sigil.
- Hybrid signing: Ed25519 + PQ signature for the transition period.
- `key_id` on `EntrySignature` enables gradual key migration.

## Supply Chain

Libro's supply chain is short and auditable:

- **Cyrius toolchain**: pinned in `cyrius.cyml` `cyrius = "5.4.7"`.
  CI reads this field and installs exactly that toolchain.
- **sigil**: pinned in `cyrius.cyml` `[deps.sigil] tag = "2.8.3"`
  (`lib/sigil.cyr` symlinked by `cyrius deps`).
- **patra**: pinned in `cyrius.cyml` `[deps.patra] tag = "1.1.1"`
  (`lib/patra.cyr` symlinked by `cyrius deps`).
- **stdlib modules** (`alloc`, `vec`, `str`, `fmt`, `syscalls`, `io`,
  `freelist`, `hashmap`, `json`, `sakshi`, `bigint`, `chrono`,
  `assert`) — bundled with the Cyrius toolchain release.

No third-party dependencies beyond the Cyrius toolchain. There is
no transitive graph to audit.

## Comparison with Industry Solutions

| Feature | Libro | Trillian | Rekor (Sigstore) | CloudTrail |
|---------|-------|----------|------------------|------------|
| Hash-linked chain | SHA-256 (sigil) | SHA-256 | SHA-256 | Proprietary |
| Merkle proofs | O(log N) inclusion + RFC 9162 consistency | O(log N) | O(log N) | No |
| Digital signatures | Ed25519 (sigil) | Various | Various | AWS KMS |
| Trusted timestamps | RFC 3161 client (consumer pairs TSA) | Consumer | Built-in | AWS |
| Key rotation | `key_id` on every entry | Built-in | Built-in | KMS |
| Append-only store | MemoryStore / FileStore / PatraStore | Cloud Spanner | Redis + Cloud | S3 |
| Streaming | In-process pub/sub (MQTT wildcards) | No | No | EventBridge |
| Retention policies | Presets (PCI / HIPAA / SOX / GDPR) | Consumer | Consumer | Configurable |
| Query/filter | Composable `QueryFilter` (ANDed) | Map fn | Basic API | Athena SQL |
| Embeddable library | **Yes** | No (service) | No (service) | No (service) |
