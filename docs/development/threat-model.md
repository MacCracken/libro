# Libro Threat Model

**Last updated:** 2026-05-10 (post-2.6.0)
**Status:** Living document — updated on material change to trust
boundaries or defended classes.

Libro is a cryptographic audit-chain library: hash-linked event
logging with integrity verification, polymorphic per-entry signing
(Ed25519 / ML-DSA-65 / hybrid), optional RFC 3161 timestamp
attestations, witness-backend anchoring, and optional TPM-sealed
anchor attestation. This document catalogues what libro does and
does not defend against, so consumers (daimon, aegis, stiva, sigil,
ark) can reason about residual risk in their own deployments.

## Scope

**In scope:** Integrity of audit data across the libro API surface
— entries, chains, proofs, signatures (Ed25519 / ML-DSA-65 / hybrid),
stored artifacts (FileStore, PatraStore), streamed verification,
chain import/export, TPM-sealed anchors (opt-in).

**Out of scope:** Threats to the operator's own honesty (libro is
append-only audit; it cannot force a caller to log what they did);
compromise of upstream crypto primitives (sigil's SHA-256, Ed25519,
and ML-DSA-65 are trusted as correct); side-channel attacks beyond
the constant-time compares libro already uses; fault injection;
rowhammer; the future federated / multi-node deployment modes.

## Trust boundaries

```
             ┌──────────────────────────────────┐
             │  Calling application (daimon,    │
             │  aegis, stiva, …)                │
             └────────┬─────────────────────────┘
                      │ trusted: provides event data
                      ↓
     ┌──────────────────────────────────────────┐
     │              libro library               │
     │  ┌────────────┐  ┌───────────────────┐   │
     │  │ entry      │  │ chain / verify /  │   │
     │  │ hasher     │  │ signing / proof / │   │
     │  │            │  │ merkle / anchor / │   │
     │  │            │  │ timestamping      │   │
     │  └────────────┘  └───────────────────┘   │
     │         ▲                ▲               │
     │         │                │               │
     └─────────┼────────────────┼───────────────┘
               │                │
   ┌───────────┴─────┐   ┌──────┴──────┐   ┌──────────┐
   │ sigil           │   │ filesystem  │   │ patra DB │
   │ (trusted crypto)│   │(adversarial)│   │(adversar)│
   └─────────────────┘   └─────────────┘   └──────────┘
```

- **Caller → libro:** Caller is trusted for event-content accuracy.
  libro ensures anything the caller submits is then hash-linked
  authentically — it does not validate *what* is being logged,
  only that the log's structure is tamper-evident.
- **libro → sigil:** sigil is trusted. Its SHA-256 (FIPS 180-4),
  Ed25519 (RFC 8032), ML-DSA-65 (NIST FIPS 204), `sigil_verify_hybrid`
  (AND-mode), and `ct_eq` (branchless constant-time compare) are
  taken as correct. A sigil CVE would propagate.
- **libro → agnosys (opt-in):** under `-D LIBRO_TPM`, agnosys is
  the backend for `tpm_seal` / `tpm_unseal` (shells out to
  tpm2-tools). Treated as trusted at the seal/unseal interface;
  agnosys's own threat model applies to the kernel boundary.
  Default builds do not pull this surface in.
- **libro → filesystem (FileStore, chain_io):** Adversarial. Files
  can be modified out-of-band. Hash-linkage + signatures are the
  defense; libro parsers assume nothing about input well-formedness.
- **libro → patra DB (PatraStore):** Adversarial. Same as
  filesystem; patra's SQL layer handles parameter escaping, libro's
  `_ps_*` helpers wrap values before insertion.
- **libro → kernel (kernel_audit on AGNOS):** kernel is trusted.
  `/proc/agnos/audit` is a read-only-to-libro sink; compromise of
  the kernel is out of scope.

## Assets

| Asset                        | Protection                                          |
|------------------------------|-----------------------------------------------------|
| Chain integrity              | SHA-256 hash-linkage, `verify_chain`, `chain_verify`|
| Entry authenticity           | Polymorphic per-entry sigs: Ed25519 / ML-DSA-65 / hybrid (2.2 / 2.3) |
| Chain-head authenticity      | Signed Tree Head (STH) in `IntegrityProof`          |
| Timestamp trustworthiness    | RFC 3161 token in `ts_attestation`                  |
| Signing key secrecy          | `secret var` stack window during keygen (2.1.1); heap-resident sk cleared via alg-aware `signing_key_zeroize` (2.2/2.3 cover both slots) |
| Cross-chain consistency      | Witness anchoring (`WitnessAnchor` meta-chain)      |
| Anchor authenticity (opt-in) | TPM-sealed self-hash under PCR policy (2.5.0, `-D LIBRO_TPM`) |

## Threats and mitigations

### T1 — Tampered chain (MITIGATED)

**Attack:** Attacker with read+write access to chain storage
(MemoryStore in another process, FileStore, PatraStore) modifies
an entry's `source`, `action`, `details`, or `prev_hash`.

**Mitigation:** Each entry's `hash` is a length-prefixed SHA-256
over all its fields. Altering any field invalidates `hash`.
`prev_hash` links to the previous entry's `hash`, so a tampered
entry also breaks the link from *its successor*. `verify_chain`
detects both in a single pass. The length-prefixed hashing
(`hash_field`) prevents second-preimage via field-boundary shifting.

**Residual:** An attacker who can also forge signatures (T2) and
rewrite every downstream entry's `prev_hash` could produce a
self-consistent tampered chain. Defense against this requires an
external anchor (T8).

### T2 — Entry forgery (MITIGATED)

**Attack:** Attacker constructs a fake `EntrySignature` for an
entry they control.

**Mitigation:** `sign_entry(sk, e)` produces a signature over the
entry hash; `verify_entry_signature(vk, e, sig)` dispatches on
the verifying-key's algorithm (the trust anchor — not the
signature's claimed algorithm). Three algorithm options as of
2.3.0:

- **Ed25519** (RFC 8032) — 128-bit security baseline.
- **ML-DSA-65** (NIST FIPS 204, 2.2.0) — post-quantum, ~192-bit
  classical security and quantum-resistance.
- **Hybrid Ed25519 + ML-DSA-65** (2.3.0) — both primitives must
  validate. Verifier accepts the entry only if both primitives
  pass (sigil's `sigil_verify_hybrid` AND-mode).

Without the signing key, forgery is computationally infeasible
under each primitive's security model. Hybrid mode resists a
break of *either* primitive: an attacker who could forge Ed25519
(post-quantum) still can't produce a valid ML-DSA portion.

**Residual:** If a signing key is exposed, all past and future
signatures *under that algorithm* are forgeable. See T6. Key
rotation (`key_id` on `EntrySignature`) limits blast radius.
Hybrid mode shifts the residual: a key compromise on one
algorithm only invalidates that algorithm's signatures, but the
hybrid AND-mode verify fails the moment either side is broken
— consumers must rotate explicitly, hybrid doesn't auto-degrade
to single-algorithm.

### T3 — Second-preimage on hash (MITIGATED IN 2.0)

**Attack:** Attacker constructs a different logical entry whose
hash matches a target.

**Mitigation:** SHA-256 (128-bit collision resistance). Length-
prefixed field hashing prevents boundary-shift collisions. **2.0
closed a known second-preimage primitive** (ADR 0007): the 1.x
canonicalizer quoted every JSON value as a string, making
`{"n": 42}` and `{"n": "42"}` hash identically. The 2.0 nested /
scalar-aware canonicalizer emits types faithfully. Consumers
relying on value-type distinctness in `details` need 2.0 or later.

### T4 — Timing attack on hash / signature compare (MITIGATED)

**Attack:** Attacker measures compare-time of `prev_hash`,
`entry_hash`, or `signature` to recover partial bytes.

**Mitigation:** Every security-critical comparison routes through
sigil's branchless `ct_eq` via `constant_time_eq_str`. Remaining
`str_eq` calls are on public metadata (`source`, `action`,
`agent_id`) where timing leakage is not exploitable.

### T5 — Denial of service via malformed input (MITIGATED)

**Attack:** Attacker provides malformed `details`, corrupt JSONL
file, or truncated input to crash or hang the parser / verifier.

**Mitigation:**
- Fuzz harness (`fuzz/fuzz_libro.fcyr`, 11 targets as of 2.0.3)
  asserts no-crash on random inputs across `sha256`, `hex_decode`,
  `der_parse`, `entry_create`, `chain_ops`, `sig_verify`,
  `json_parse`, `topic_match`, `chain_import`,
  `filestore_verify_streamed`, `canonical_json_hash`.
- `filestore_verify_streamed` uses a bounded 64KB buffer per chunk
  — unbounded-memory DoS is not possible.
- Lines > 64KB return an explicit error rather than continuing.
- Malformed entries are skipped, not abort-the-verify (parse
  returns 0, line is not included in chunk).

**Residual:** A known hang bug in `filestore_verify_streamed` on
unterminated-tail input was found by fuzz and fixed in 2.0.3
(Finding 4 in the 2.0 audit). The shared `flock` leak that
compounded it is also resolved. Any future similar bug in the
streaming / import parsers is our primary residual-risk class and
is why these paths have dedicated fuzz coverage.

### T6 — Signing-key material exposure (MITIGATED, hardware path opt-in)

**Attack:** Attacker reads the signing key from a core dump,
swap, or memory scan.

**Mitigation:**
- **Stack entropy window** (2.1.1) — `signing_key_generate*` reads
  32 bytes of CSPRNG into a `secret var seed_stack[32]` and
  `memcpy`s to the heap. `secret var` is compiler-guaranteed
  zeroize-on-return, so the seed never lingers on the stack past
  keygen.
- **Heap-resident sk material** — stored on the heap (not the
  stack). `signing_key_zeroize(sk)` is alg-aware: clears 64 bytes
  for Ed25519, 4032 bytes for ML-DSA-65, BOTH slots (4096 bytes +
  64 bytes of seeds) for hybrid. Consumers are expected to call
  zeroize in a `defer`-equivalent or on shutdown.
- **Entropy gathering hardened** (2.1.1) — `random_bytes` via
  `getrandom(2)` replaces the prior `/dev/urandom` open/read/close
  path; cleaner under Landlock policies that block `/dev/urandom`
  traversal.

**Residual:** libro does not memlock the key pages; the OS may
swap them. libro does not defend against pre-zeroize memory reads.
Consumers wanting hardware-backed key sealing can run libro behind
TPM-backed PCR policy gates (2.5.0 `tpm_anchor` covers anchor
sealing; per-key TPM sealing is a future extension if a consumer
asks). See [`docs/guides/tpm-anchors.md`](../guides/tpm-anchors.md)
for the trust model.

### T7 — Side-channel leakage (OUT OF SCOPE for 2.x)

**Attack:** Cache-timing, power-analysis, acoustic, EM.

**Mitigation:** Not defended at the libro layer. sigil's Ed25519
is constant-time by construction (RFC 8032). SHA-256 is generally
considered side-channel resistant under software implementation.
ML-DSA-65 (sigil 3.0) follows the FIPS 204 deterministic-rejection-
loop pattern; the side-channel posture is sigil's responsibility
and out of libro scope.

**Residual:** All software side channels below the crypto-primitive
level. Consumers needing hardware-rooted protection use the 2.5.0
TPM-sealed anchor (`-D LIBRO_TPM`) — though the seal protects
*anchor* integrity, not the underlying signing-key side channels.

### T8 — Chain rollback / replay (CONSUMER-MITIGATED)

**Attack:** Attacker presents an older, internally-valid chain to
a consumer that has seen a newer state.

**Mitigation:** libro exposes `chain_head_hash(c)` for consumer-
side head-tracking, `IntegrityProof` bundles with STH for
third-party verification, and `WitnessAnchor` / `WitnessReceipt`
for external anchoring (e.g., publish chain-head hashes to a
tamper-evident public log or a sibling signed chain). libro does
not enforce rollback detection itself — the consumer must
persist the last-observed head and refuse older states.

**Residual:** Consumer-responsibility risk. libro provides the
primitives; consumer misuse (e.g., not checking `chain_head_hash`
against persisted state) leaves this undefended.

### T9 — Compromised signing key → post-compromise entries (MITIGATED)

**Attack:** After key compromise, attacker signs forged entries
that appear valid.

**Mitigation:** `EntrySignature.key_id` identifies the signing
key used for each entry. Rotation replaces the active key; old
entries retain their pre-rotation `key_id` and verify against the
archived key. A compromise-detected rotation invalidates only
entries signed with the compromised `key_id`.

**Residual:** Detecting the compromise is not libro's job. Once
detected, rotation is the response; libro supports it but does
not trigger it.

### T10 — PatraStore SQL injection (MITIGATED)

**Attack:** Malicious input in `source`, `action`, `details`
escapes parameterization and executes arbitrary SQL.

**Mitigation:** patra's query layer uses parameterized statements
for all user-supplied values. libro's `_ps_*` helpers
(`_ps_copy_cstr`, `_ps_bind_*`) never concatenate user input into
SQL text — only the CREATE TABLE DDL is assembled at runtime, and
that uses fixed identifiers.

**Residual:** A bug in patra's parameter-binding could bypass.
patra has its own fuzz harness and test suite at the `patra-core`
level; libro pins a specific patra tag (`1.1.1` at time of
writing) so an upstream regression is detectable.

### T11 — Concurrent-access corruption (MITIGATED for FileStore)

**Attack:** Two processes writing the same FileStore interleave
entries, producing corrupt JSONL.

**Mitigation:** FileStore acquires an advisory `flock` (exclusive
on append, shared on load) via `file_lock_exclusive` /
`file_lock_shared`. Processes honoring advisory locks serialize.

**Residual:** Advisory locks are cooperative. A process that
doesn't acquire the lock can still corrupt. Mandatory-lock
semantics are platform-dependent and not relied on. PatraStore
multi-writer coordination is delegated to patra's own locking
(patra uses SQLite-style file locking).

### T12 — Kernel-audit source spoofing (OUT OF SCOPE)

**Attack:** Attacker compromises `/proc/agnos/audit` to inject or
alter events read by `kernel_audit_read`.

**Mitigation:** Kernel is in the trusted computing base. If an
attacker can write to kernel `/proc`, libro cannot defend against
anything.

### T13 — Quantum cryptanalysis of Ed25519 (MITIGATED, opt-in)

**Attack:** A future cryptographically-relevant quantum computer
runs Shor's algorithm against Ed25519 signatures, recovering
signing keys and forging arbitrary entries — invalidating audit
chains that rely on Ed25519 alone for non-repudiation.

**Mitigation:** Consumers with retention windows that meaningfully
overlap with the CRQC timeline opt their chains into
**ML-DSA-65** (FIPS 204) or **hybrid Ed25519 + ML-DSA-65** signing.
Both landed in 2.2.0 / 2.3.0 via sigil 3.0.0. ML-DSA-65 is the
NIST-standardized lattice-based signature scheme; hybrid mode adds
a second independent cryptographic assumption so a break of
*either* algorithm leaves the other witnessing the entry.

**Residual:** Audit chains created before 2.2.0 (or 2.x consumers
who stayed on `signing_key_generate()` without migrating) remain
Ed25519-only. Migration uses chain boundaries (rotation under
retention policy or a fresh log file) to switch new chains to
hybrid or ML-DSA-65 without rehashing history. The pre-migration
chain's authenticity post-CRQC is the consumer's risk-acceptance
decision.

### T14 — Anchor tampering (MITIGATED for software, hardware-sealed opt-in)

**Attack:** Attacker tampers with a stored `WitnessAnchor` to
present a fabricated chain-head snapshot.

**Mitigation:** Every anchor self-hashes (`anchor_compute_hash`
length-prefixes the UUID, merkle_root, entry_count, chain_head,
hash_alg, created_at, prev_anchor_hash); `anchor_verify_integrity`
detects byte-level tampering. The 2.5.0 opt-in `tpm_anchor`
adds a TPM 2.0 seal over the anchor's self-hash under PCR policy
(default PCR 0 + PCR 7 — firmware + Secure Boot config) — proves
"this anchor was created on this TPM at this PCR state, AND the
anchor data hasn't been tampered with since".

**Residual:** TPM sealing is opt-in (`-D LIBRO_TPM`). It does NOT
prove chain correctness (verify against the tree separately), host
honesty (a compromised host with TPM control at seal time can
produce a valid seal for arbitrary content), or identity (combine
with Ed25519/ML-DSA-65 entry signing for attribution). PCR rotation
invalidates seals — re-seal is consumer-managed. See
[`docs/guides/tpm-anchors.md`](../guides/tpm-anchors.md).

## Residual risk summary

| Class                              | Status              | Next step                 |
|------------------------------------|---------------------|---------------------------|
| Post-compromise key use            | Mitigated           | Rotation discipline       |
| Pre-zeroize memory read            | Mitigated (2.1.1)   | secret-var + getrandom    |
| Cache / power side channels        | Out of scope        | Sigil-side                |
| Multi-node consistency             | Not implemented     | Future (blocked)          |
| PQ resistance (Ed25519 break)      | Mitigated (2.2/2.3) | Hybrid sig migration      |
| Consumer rollback discipline       | Consumer            | Integration guidance      |
| Anchor tampering                   | Mitigated, hardware-sealed opt-in (2.5.0) | TPM anchor |
| Unknown parser bug in import paths | Fuzz-covered        | Ongoing fuzz sweep        |

## Unsafe code

None in the libro sense. Cyrius doesn't have the Rust `unsafe`
keyword; every operation is pointer-arithmetic-capable by default.
The equivalent discipline is enforced through:
- `#derive(accessors)` on every `struct` (ADR 0005) — cross-module
  readers never reach into struct bytes by hand.
- CI raw-offset guard (`.github/workflows/ci.yml`) — fails the
  build if raw offsets appear on known struct parameter names
  outside their defining file.
- `str_from` / `str_new` discipline documented in `CLAUDE.md` —
  pointers from ephemeral buffers are copied before wrapping.

## Supply chain

- **Cyrius toolchain pin** in `cyrius.cyml` `cyrius = "6.1.23"`.
  CI reads this field and installs the exact toolchain via the
  canonical `scripts/install.sh` flow. No wildcard ranges.
- **sigil pin** in `cyrius.cyml` `[deps.sigil] tag = "3.7.8"`.
  `cyrius deps` resolves deterministically. Pins libro to sigil's
  full FIPS 204 ML-DSA stack.
- **patra pin** in `cyrius.cyml` `[deps.patra] tag = "1.11.0"`.
  Same as above. Pins the prepared-statement / group-commit /
  STR-btree feature set.
- **agnosys pin** in `cyrius.cyml` `[deps.agnosys] tag = "1.4.1"`.
  Direct pin (promoted from transitive-via-sigil in 2.5.0).
  Default builds DCE the TPM surface; opt-in via `-D LIBRO_TPM`.
- **Zero third-party deps** beyond the Cyrius toolchain + sigil +
  patra + agnosys. No transitive dependency graph to audit.

## Change log

Last full restructure: 2.6.0 docs-pass (added T13 PQ-resistance /
T14 anchor-tampering, refreshed T2 / T6 / T7 for the 2.x crypto +
hardware additions, refreshed the supply-chain pins).
