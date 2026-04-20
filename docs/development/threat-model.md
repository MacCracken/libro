# Libro Threat Model

**Last updated:** 2026-04-19 (post-2.0.3)
**Status:** Living document — updated on material change to trust
boundaries or defended classes.

Libro is a cryptographic audit-chain library: hash-linked event
logging with integrity verification, optional Ed25519 signing,
optional RFC 3161 timestamp attestations, and witness-backend
anchoring. This document catalogues what libro does and does not
defend against, so consumers (daimon, aegis, stiva, sigil, ark)
can reason about residual risk in their own deployments.

## Scope

**In scope:** Integrity of audit data across the libro API surface
— entries, chains, proofs, signatures, stored artifacts (FileStore,
PatraStore), streamed verification, chain import/export.

**Out of scope:** Threats to the operator's own honesty (libro is
append-only audit; it cannot force a caller to log what they did);
compromise of upstream crypto primitives (sigil's SHA-256 and
Ed25519 are trusted as correct); side-channel attacks beyond the
constant-time compares libro already uses; fault injection;
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
  Ed25519 (RFC 8032), and `ct_eq` (branchless constant-time
  compare) are taken as correct. A sigil CVE would propagate.
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
| Entry authenticity           | Ed25519 per-entry signatures (`sign_entry`)         |
| Chain-head authenticity      | Signed Tree Head (STH) in `IntegrityProof`          |
| Timestamp trustworthiness    | RFC 3161 token in `ts_attestation`                  |
| Signing key secrecy          | Heap-only storage, `signing_key_zeroize` on disposal|
| Cross-chain consistency      | Witness anchoring (`WitnessAnchor` meta-chain)      |

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

**Mitigation:** `sign_entry(sk, e)` produces an Ed25519 signature
over the entry hash. `verify_entry_signature(vk, e, sig)` checks
via sigil's `ed25519_verify`. Without the signing key, forgery is
computationally infeasible (Ed25519 provides 128-bit security).

**Residual:** If the signing key is exposed, all past and future
signatures are forgeable. See T6. Key rotation (`key_id` on
`EntrySignature`) limits blast radius.

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

### T6 — Signing-key material exposure (PARTIAL)

**Attack:** Attacker reads the signing key from a core dump,
swap, or memory scan.

**Mitigation:** `SigningKey` stores key material on the heap (not
the stack). `signing_key_zeroize(sk)` overwrites both the secret-
key buffer and the seed before freeing. Consumers are expected to
call `zeroize` in a `defer`-equivalent or on shutdown.

**Residual:** libro does not memlock the key pages; the OS may
swap them. libro does not defend against pre-zeroize memory reads.
Hardware-backed key storage (TPM sealing via agnosys + sigil.tpm)
is unblocked but not yet integrated — see
`docs/development/roadmap.md` "Open — unblocked".

### T7 — Side-channel leakage (OUT OF SCOPE for 2.x)

**Attack:** Cache-timing, power-analysis, acoustic, EM.

**Mitigation:** Not defended at the libro layer. sigil's Ed25519
is constant-time by construction (RFC 8032). SHA-256 is generally
considered side-channel resistant under software implementation.

**Residual:** All software side channels below the crypto-primitive
level. Hardware-backed deployments (TPM sealing via agnosys +
sigil.tpm) are unblocked but not yet integrated — see the roadmap.

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

## Residual risk summary

| Class                              | Status          | Next step                 |
|------------------------------------|-----------------|---------------------------|
| Post-compromise key use            | Mitigated       | Rotation discipline       |
| Pre-zeroize memory read            | Unmitigated     | TPM backing (unblocked, not integrated) |
| Cache / power side channels        | Out of scope    | Hardware-backed (blocked) |
| Multi-node consistency             | Not implemented | Future (blocked)          |
| PQ resistance                      | Not implemented | ML-DSA via sigil (blocked)|
| Consumer rollback discipline       | Consumer        | Integration guidance      |
| Unknown parser bug in import paths | Fuzz-covered    | Ongoing fuzz sweep        |

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

- **Cyrius toolchain pin** in `cyrius.cyml` `cyrius = "5.4.7"`.
  CI reads this field and installs the exact toolchain. No
  wildcard ranges.
- **sigil pin** in `cyrius.cyml` `[deps.sigil] tag = "2.8.3"`.
  `cyrius deps` resolves deterministically.
- **patra pin** in `cyrius.cyml` `[deps.patra] tag = "1.1.1"`.
  Same as above.
- **Zero third-party deps** beyond the Cyrius toolchain + sigil +
  patra. No transitive dependency graph to audit.

## Review cadence

This document is reviewed on every minor release (X.Y.0 → X.(Y+1).0)
and whenever a finding files in `docs/audit/`. Last full review:
2.0.0. Post-2.0 sprints have appended findings inline; a full
restructure is due at 2.1.0.
