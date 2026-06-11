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
| **NIST 800-53** SC-12 | Key management | `SigningKey` / `VerifyingKey` with `key_id` rotation; 2.1.1 `secret var` entropy gathering + `getrandom`; alg-aware `signing_key_zeroize` covers Ed25519 / ML-DSA-65 / hybrid slots | Covered |
| **NIST FIPS 204** | ML-DSA digital signatures | `signing_key_generate_mldsa()` + `sign_entry` ML-DSA-65 dispatch via sigil 3.0 (2.2.0+) | Covered |
| **NIST CNSA 2.0** | Post-quantum cryptographic baseline | ML-DSA-65 entry signing + hybrid Ed25519+ML-DSA migration path (2.2 / 2.3) | Covered |
| **NIST 800-53** AU-9 (3) | Cryptographic protection of audit info (hardware-rooted) | Opt-in `tpm_anchor` — TPM-sealed anchor under PCR policy (2.5.0, `-D LIBRO_TPM`) | Opt-in |
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
| **Non-repudiation** | Per-entry signatures: Ed25519 / ML-DSA-65 / hybrid via sigil | Binds entry content to a specific signing key; hybrid mode binds under TWO cryptographic assumptions |
| **Post-quantum readiness** | ML-DSA-65 (FIPS 204) entry signing (2.2.0) | ~192-bit classical + quantum-resistant; CNSA 2.0 aligned |
| **PQ migration path** | Hybrid Ed25519 + ML-DSA-65 signing (2.3.0) | Verifier requires both primitives; chain stays valid through PQ transition |
| **Efficient inclusion verification** | Merkle inclusion proofs | O(log N) proof that an entry exists in the chain |
| **Lossless proof round-trip** | `proof_to_json` / `proof_from_json` with side-bit-preserving inclusion paths (2.6.0) | Saved proofs re-hydrate to verify identically |
| **Append-only consistency** | RFC 9162 consistency proofs | Proves one tree state is a prefix of a later state |
| **Second-preimage resistance** | Length-prefixed field hashing + nested scalar-aware canonical JSON (2.0) | Prevents field boundary shifting AND value-type coercion attacks |
| **Deterministic hashing** | Canonical JSON: sorted keys, depth-unlimited, scalars emit verbatim | Same logical content always produces the same hash |
| **Key rotation readiness** | `key_id` on `EntrySignature` | Identifies which key signed each entry |
| **Hardware-rooted anchor attestation** | TPM 2.0 sealed `WitnessAnchor` under PCR policy (2.5.0, opt-in) | Proves anchor was created on this TPM at this PCR state |

### Algorithms

| Purpose | Algorithm | Standard | Source | PQ-resistant? |
|---------|-----------|----------|--------|---------------|
| Entry hashing | SHA-256 | FIPS 180-4 | sigil (`lib/sigil.cyr`) | Yes |
| Entry signing (default) | Ed25519 | RFC 8032 | sigil 3.0 | No |
| Entry signing (PQ) | ML-DSA-65 | NIST FIPS 204 | sigil 3.0 (`src/mldsa*.cyr`) | Yes |
| Entry signing (hybrid) | Ed25519 + ML-DSA-65 (AND-mode) | RFC 8032 + FIPS 204 | sigil 3.0 (`sigil_verify_hybrid`) | Yes (one of the two binds the entry even if Ed25519 breaks) |
| TPM seal hash bank | SHA-256 | FIPS 180-4 | agnosys + tpm2-tools | N/A |
| Entropy source | `getrandom(2)` | Linux ABI | `lib/random.cyr` (2.1.1) | N/A |
| Key encoding | Hex (lowercase) | — | sigil `hex_encode_str` | N/A |
| Constant-time compare | Branchless `ct_eq` / `ct_eq_bytes` | — | sigil + Cyrius `lib/ct.cyr` | N/A |

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
quantum-resistant under Shor's algorithm against a
cryptographically-relevant quantum computer (CRQC).

**Shipped (2.2.0 / 2.3.0)** — both options are first-class
algorithm choices on `signing_key_generate*`:

- **ML-DSA-65** (NIST FIPS 204, sigil 3.0) — pure post-quantum
  signing. Call `signing_key_generate_mldsa()`. CNSA 2.0
  compliant. Signatures are ~3.3 KB (vs Ed25519's 64 B) but
  verify is ~2 ms (faster than Ed25519's ~6.6 ms in sigil 3.0).
- **Hybrid Ed25519 + ML-DSA-65** — call
  `signing_key_generate_hybrid()`. Each entry carries both
  signatures; verify requires both (sigil's
  `sigil_verify_hybrid` AND-mode). The migration path: a chain
  starts Ed25519-only, rotates to hybrid at a chain boundary,
  eventually rotates to ML-DSA-65-only once the consumer's
  threat model considers Ed25519 retired. `key_id` on
  `EntrySignature` enables the gradual rotation.

See [`docs/guides/integration.md`](../guides/integration.md) for
the migration story and the backward-compatible verify pattern
(a pre-2.3 Ed25519-only vk can still validate the Ed25519 portion
of a hybrid signature, useful while a fleet rolls out the
hybrid-aware verifier).

## Hardware-Rooted Anchor Attestation (2.5.0, opt-in)

For deployments that require hardware-rooted audit attestation
(FedRAMP High, NSA CNSA 2.0 with hardware-anchored claims, certain
HIPAA / SOX deployments under formal forensics policies), libro
2.5.0 ships opt-in TPM 2.0 sealed anchors:

- Build with `-D LIBRO_TPM` to include `src/tpm_anchor.cyr`.
- `tpm_anchor_new(inner, output_dir, pcr_indices)` seals the
  inner `WitnessAnchor`'s self-hash to the host TPM under the
  chosen PCR policy.
- `tpm_anchor_verify(ta)` returns `TPM_ANCHOR_VALID` only when:
  (1) the inner anchor self-verifies, (2) the TPM unseals the
  blob (PCR state matches seal time), AND (3) the unsealed bytes
  equal the inner anchor's hash.
- Default PCR policy: PCR 0 (firmware) + PCR 7 (Secure Boot
  configuration) — AGNOS-aligned conservative default. Stable
  across kernel updates, invalidates only on firmware /
  boot-policy changes.

See [`docs/guides/tpm-anchors.md`](../guides/tpm-anchors.md) for
the full trust model (what it does and doesn't prove), the
runtime requirements (tpm2-tools, `/dev/tpmrm0` ACLs), and
alternative PCR policies for tighter or looser attestation
windows.

## Supply Chain

Libro's supply chain is short and auditable:

- **Cyrius toolchain**: pinned in `cyrius.cyml` `cyrius = "6.1.35"`.
  CI reads this field and installs exactly that toolchain via the
  canonical `scripts/install.sh` flow.
- **sigil**: pinned in `cyrius.cyml` `[deps.sigil] tag = "3.7.10"`
  (`lib/sigil.cyr` resolved by `cyrius deps`). Carries the FIPS 204
  ML-DSA-65 stack libro 2.2+ depends on.
- **patra**: pinned in `cyrius.cyml` `[deps.patra] tag = "1.11.0"`
  (`lib/patra.cyr`). Carries the prepared-statement / group-commit
  / STR-btree features libro 2.4+ uses.
- **agnosys**: pinned in `cyrius.cyml` `[deps.agnosys] tag = "1.4.1"`
  (`lib/agnosys.cyr`). Direct pin since 2.5.0; carries TPM 2.0
  primitives (opt-in via `-D LIBRO_TPM`) and Landlock syscall
  wrappers.
- **stdlib modules** (~22 modules including the 2.1.0 sigil-bundle
  expansion: `assert`, `alloc`, `freelist`, `str`, `string`, `vec`,
  `hashmap`, `syscalls`, `io`, `fs`, `fmt`, `json`, `tagged`,
  `process`, `sakshi`, `bigint`, `chrono`, `ct`, `keccak`, `thread`,
  `random`, `bench`, `fnptr`, `test`) — bundled with the Cyrius
  toolchain release.

No third-party dependencies beyond the Cyrius toolchain. There is
no transitive graph to audit.

## Comparison with Industry Solutions

| Feature | Libro | Trillian | Rekor (Sigstore) | CloudTrail |
|---------|-------|----------|------------------|------------|
| Hash-linked chain | SHA-256 (sigil) | SHA-256 | SHA-256 | Proprietary |
| Merkle proofs | O(log N) inclusion + RFC 9162 consistency | O(log N) | O(log N) | No |
| Lossless proof JSON RT | Yes (2.6.0) | Limited | Limited | No |
| Digital signatures | Ed25519 / ML-DSA-65 / hybrid (sigil 3.0) | Various | Various | AWS KMS |
| Post-quantum signing | **Yes** (FIPS 204 ML-DSA-65 + hybrid) | No (planned) | No (planned) | No |
| Trusted timestamps | RFC 3161 client (consumer pairs TSA) | Consumer | Built-in | AWS |
| Hardware-rooted attestation | **Yes** (TPM-sealed anchors, opt-in) | No | No | AWS Nitro |
| Key rotation | `key_id` on every entry | Built-in | Built-in | KMS |
| Append-only store | MemoryStore / FileStore / PatraStore | Cloud Spanner | Redis + Cloud | S3 |
| PatraStore perf knobs | Prepared SQL + group commit + STR index | Cloud Spanner | Cloud | S3 |
| Streaming | In-process pub/sub (MQTT wildcards) | No | No | EventBridge |
| Retention policies | Presets (PCI / HIPAA / SOX / GDPR) | Consumer | Consumer | Configurable |
| Query/filter | Composable `QueryFilter` (ANDed) | Map fn | Basic API | Athena SQL |
| Embeddable library | **Yes** | No (service) | No (service) | No (service) |
