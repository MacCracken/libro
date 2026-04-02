# Compliance Standards Mapping

How libro's features map to major compliance frameworks.

Libro is a library crate — it provides cryptographic primitives and data structures. Infrastructure concerns (network security, access control, backup) are the consumer's responsibility. This document maps what libro provides and what consumers must implement.

## Coverage Matrix

| Standard | Control | Libro Feature | Status |
|----------|---------|---------------|--------|
| **ISO 27001** A.8.15 | Tamper-proof logging | SHA-256 hash-linked chain, `verify_chain()` | Covered |
| **ISO 27001** A.8.24 | Use of cryptography | SHA-256, Ed25519 signing, length-prefixed fields | Covered |
| **NIST 800-53** AU-3 | Content of audit records | `AuditEntry`: agent_id, action, timestamp, source, severity, details | Covered |
| **NIST 800-53** AU-8 | Time stamps | `chrono::Utc::now()` — see [Timestamp Integrity](#timestamp-integrity) | Partial |
| **NIST 800-53** AU-9 | Protection of audit info | Hash chain tamper detection; `FileStore` flock; `SqliteStore` Mutex | Covered |
| **NIST 800-53** AU-10 | Non-repudiation | Ed25519 per-entry signatures with `key_id` for rotation | Covered |
| **NIST 800-53** AU-11 | Audit record retention | `RetentionPolicy` with compliance presets | Covered |
| **NIST 800-53** SC-12 | Key management | `SigningKey`/`VerifyingKey` with `key_id` rotation support | Partial |
| **SOC 2** CC7.2 | Monitoring | Append-only chain, streaming pub/sub for real-time events | Covered |
| **SOC 2** CC6.1 | Logical access | See [Consumer Responsibilities](#consumer-responsibilities) | Consumer |
| **PCI DSS 4.0** 10.2 | Audit log content | Structured entries: who, what, when, where, outcome | Covered |
| **PCI DSS 4.0** 10.3 | Tamper detection | Hash chain — modification breaks all successor hashes | Covered |
| **PCI DSS 4.0** 10.5 | Log integrity | File locking, hash verification, Merkle proofs | Covered |
| **PCI DSS 4.0** 10.7 | Log retention | `RetentionPolicy::pci_dss()` — 12 months | Covered |
| **HIPAA** 164.312(b) | Audit controls | Structured logging with severity, source, action, details | Covered |
| **HIPAA** 164.312(c) | Integrity controls | SHA-256 hash verification, Ed25519 signatures | Covered |
| **HIPAA** 164.530(j) | Record retention | `RetentionPolicy::hipaa()` — 6 years | Covered |
| **GDPR** Art 5(1)(f) | Integrity & confidentiality | Hash chain integrity; encryption is consumer's responsibility | Partial |
| **GDPR** Art 30 | Records of processing | Structured entries: source, action, agent, timestamp, details | Covered |
| **SOX** Section 802 | Record retention | `RetentionPolicy::sox()` — 7 years | Covered |
| **eIDAS** Art 34 | Qualified timestamps | See [Timestamp Integrity](#timestamp-integrity) | Not covered |
| **RFC 6962** | Verifiable logs | Merkle tree with O(log N) inclusion proofs | Covered |

## Cryptographic Guarantees

### What Libro Provides

| Property | Mechanism | Strength |
|----------|-----------|----------|
| **Tamper detection** | SHA-256 hash-linked chain | Any modification to any entry invalidates all successor hashes |
| **Non-repudiation** | Ed25519 per-entry signatures | Binds entry content to a specific signing key |
| **Efficient verification** | Merkle tree inclusion proofs | O(log N) proof that an entry exists in the chain |
| **Second-preimage resistance** | Length-prefixed field hashing | Prevents field boundary shifting attacks |
| **Deterministic hashing** | Canonical JSON (sorted keys) | Same logical content always produces the same hash |
| **Key rotation readiness** | `key_id` on `EntrySignature` | Identifies which key produced each signature |

### Algorithms

| Purpose | Algorithm | Standard | Quantum-resistant? |
|---------|-----------|----------|--------------------|
| Entry hashing | SHA-256 | FIPS 180-4 | Yes |
| Entry signing | Ed25519 | RFC 8032 | No (post-quantum migration path planned) |
| Key encoding | Hex (lowercase) | — | N/A |

## Timestamp Integrity

Libro uses `chrono::Utc::now()` which reads the system clock. This is **not** a trusted timestamp in the RFC 3161 sense.

**Consumer responsibilities:**
- Ensure system clocks are NTP-synchronized (RFC 5905)
- For legal/regulatory evidence, consider anchoring Merkle roots to an external Timestamp Authority (TSA) per RFC 3161
- For eIDAS qualified timestamps, use a qualified TSA service and store the TSA response alongside the chain

**What libro provides:**
- Monotonic entry ordering within a single chain (enforced by hash linking)
- `RetentionPolicy::KeepAfter` and `QueryFilter::after`/`before` for time-based operations
- RFC 3339 timestamp format in serialized entries

## Consumer Responsibilities

These controls are outside libro's scope as a library crate. Consumers MUST implement:

### Access Control (NIST AU-9, PCI DSS 10.5.1-10.5.2, ISO 27001 A.8.15)
- Restrict read/write access to `FileStore` paths and `SqliteStore` database files
- Ensure only authorized processes can call `AuditChain::append()`
- Use OS-level file permissions, SELinux/AppArmor, or container isolation

### Transport Security (NIST SC-8, PCI DSS 4.1)
- Encrypt audit data in transit using TLS 1.2+
- Libro's streaming feature (majra pub/sub) operates in-process; network transport is the consumer's concern

### Backup and Disaster Recovery (PCI DSS 10.5.3, ISO 27001 A.8.13)
- Back up `FileStore` files and `SqliteStore` databases to offsite/immutable storage
- Consider WORM (Write-Once Read-Many) storage for compliance-critical chains
- Test restore procedures regularly

### Encryption at Rest (HIPAA 164.312(a)(2)(iv), PCI DSS 3.4)
- Encrypt store files at the filesystem or volume level
- Libro does not encrypt entries at rest — it provides integrity (hashing) not confidentiality

### Log Shipping and Centralization (SOC 2, PCI DSS 10.5.3)
- Ship audit logs to a central SIEM or log aggregation service
- Use libro's `to_jsonl()` or `to_csv()` export for SIEM ingestion
- Consider streaming integration for real-time forwarding

### Retention Enforcement
- Libro provides `RetentionPolicy` with compliance presets but does NOT automatically enforce them
- Consumers must schedule periodic `chain.apply_retention()` calls
- Archived entries should be stored in immutable/offsite storage before deletion

## FIPS 140-3 Considerations

Libro uses `sha2` (pure Rust) and `ed25519-dalek` (pure Rust). These are **not** FIPS 140-3 validated modules.

For FedRAMP or federal information systems:
- Replace cryptographic backends with FIPS-validated modules (e.g., via `aws-lc-rs` or a validated OpenSSL binding)
- Or document a Plan of Action and Milestones (POA&M) for cryptographic module validation

## Post-Quantum Migration Path

SHA-256 is considered quantum-resistant. Ed25519 is **not**.

Planned (post-v1):
- Feature-gated post-quantum signature scheme (e.g., CRYSTALS-Dilithium)
- Hybrid signing: Ed25519 + PQ signature for transition period
- `key_id` field enables gradual key migration

## Comparison with Industry Solutions

| Feature | Libro | Trillian | Rekor (Sigstore) | CloudTrail |
|---------|-------|----------|------------------|------------|
| Hash-linked chain | SHA-256 | SHA-256 | SHA-256 | Proprietary |
| Merkle proofs | O(log N) | O(log N) | O(log N) | No |
| Digital signatures | Ed25519 | Various | Various | AWS KMS |
| Trusted timestamps | Consumer | Consumer | RFC 3161 | AWS |
| Key rotation | key_id | Built-in | Built-in | KMS |
| Append-only store | File + SQLite | Cloud Spanner | Redis + Cloud | S3 |
| Streaming | majra pub/sub | No | No | EventBridge |
| Retention policies | Presets | Consumer | Consumer | Configurable |
| Query/filter | Composable | Map fn | Basic API | Athena SQL |
| Embeddable library | **Yes** | No (service) | No (service) | No (service) |
