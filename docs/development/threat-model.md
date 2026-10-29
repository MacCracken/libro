# Libro Threat Model

**Last updated:** 2026-10-28 (libro 2.11.3)
**Status:** Living document — updated on material change to trust
boundaries or defended classes.

Libro is a cryptographic audit-chain library: hash-linked event
logging with integrity verification, polymorphic per-entry signing
(Ed25519 / ML-DSA-65 / hybrid), optional RFC 3161 timestamp
attestations, witness-backend anchoring, and optional TPM-sealed
anchor attestation. This document catalogues what libro does and
does not defend against, so consumers (daimon, stiva, bote, argonaut,
nein and the other repos that pin `[deps.libro]`) can reason about
residual risk in their own deployments.

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
             │  bote, stiva, …)                 │
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
  Ed25519 (RFC 8032), ML-DSA-65 (NIST FIPS 204) and
  `sigil_verify_hybrid` (AND-mode) are taken as correct. A sigil CVE
  would propagate. libro links a thin sigil surface
  (`dist/sigil-mldsa.cyr` + `src/{sha_ni,sha256,hex}.cyr`), not the
  monolithic bundle, so sigil's x509/RSA/authenticode code is not in
  the binary. Constant-time comparison (`ct_eq_bytes_lens`) comes
  from the Cyrius stdlib's `lib/ct.cyr`, not from sigil.
- **libro → sigil TPM (opt-in):** under `-D LIBRO_TPM`,
  `src/tpm_anchor.cyr` calls `tpm_seal` / `tpm_unseal` /
  `tpm_detect` from sigil's `dist/sigil-tpm.cyr`, which shells out
  to tpm2-tools. That module resolves only under the optional `tpm`
  feature (`[deps.sigil_tpm]` in `cyrius.cyml`). Treated as trusted
  at the seal/unseal interface; the boundary between sigil,
  tpm2-tools and the TPM device is sigil's. Default builds carry no
  TPM surface at all.
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
| Chain-head authenticity      | Signed Tree Head (STH) in `IntegrityProof`; signature over `tree_head_digest(root, tree_size, timestamp, algorithm)` (2.8.12) |
| Timestamp evidence (limited) | RFC 3161 response in `ts_attestation`: DER shape, granted PKIStatus and token presence checked; the token's TSA signature and hash binding are **not** verified (T17) |
| Signing key secrecy          | `secret var` stack window during keygen (2.1.1); heap-resident sk cleared via alg-aware `signing_key_zeroize` (2.2/2.3 cover both slots) |
| Cross-chain consistency      | Witness anchoring (`WitnessAnchor` meta-chain)      |
| Anchor authenticity (opt-in) | TPM-sealed self-hash under PCR policy (2.5.0, `-D LIBRO_TPM`) |

## Threats and mitigations

### T1 — Tampered chain (MITIGATED)

**Attack:** Attacker with read+write access to chain storage
(MemoryStore in another process, FileStore, PatraStore) modifies
an entry's `source`, `action`, `details`, or `prev_hash`.

**Mitigation:** Each entry's `hash` is a SHA-256 over all its
fields: the 16 UUID bytes, then length-prefixed timestamp, severity,
source, action, canonical `details` (T15), agent_id, prev_hash and
`hash_algorithm`. (`hash_algorithm` joined the preimage in 2.8.11;
before that it could be rewritten on disk and `entry_verify` still
passed.) Altering any field invalidates `hash`.
`prev_hash` links to the previous entry's `hash`, so a tampered
entry also breaks the link from *its successor*. `verify_chain`
detects both in a single pass. The length-prefixed hashing
(`hash_field`) prevents second-preimage via field-boundary shifting.

Two load-and-verify holes are closed. Since 2.8.12 `verify_chain` with
`base_index` 0 requires the first entry to be a genesis (empty
`prev_hash`), so entries deleted from the *front* of a log are
detected by the store load-and-verify paths and the proof-bundle
verifier; a segment that legitimately starts mid-chain passes its
expected first link to `verify_chain_from`. Since 2.8.11 a missing or
unreadable log yields an error object (`libro_is_error`,
`filestore_load_all_or_err`) instead of an empty vec that verified
clean. Findings: `docs/audit/2026-08-22-audit.md` (2, 6) and
`docs/audit/2026-08-22-audit-2.md` (5).

**Residual:** An attacker who can also forge signatures (T2) and
rewrite every downstream entry's `prev_hash` could produce a
self-consistent tampered chain. Defense against this requires an
external anchor (T8).

Through 2.11.2 the JSONL readers skipped a line they could not
parse. Mid-file that was caught anyway, since the successor's
`prev_hash` no longer links, but a malformed *last* line read as a
log one entry shorter that verified clean, so breaking the newest
record's JSON hid any edit to it. Since 2.11.3 the verifiers
(`filestore_verify_streamed`, `filestore_load_and_verify`) return a
`LIBRO_ERR_JSON` error naming the record, and `chain_import` fails.
The plain loaders (`filestore_load_all`, `filestore_load_all_or_err`,
`filestore_query`) still skip it, with a warning, so a damaged log
stays readable; a consumer that loads and then runs `verify_chain`
itself keeps the old exposure. Truncation proper (records removed,
not broken) is still detectable only against a signed tree head or an
anchor (T8).

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
Two later classes in the same family are covered separately: T15
(canonical-form collisions in `details`, 2.8.11 / 2.8.12) and T16
(Merkle second-preimage and proof binding, 2.8.11 / 2.8.12).

### T4 — Timing attack on hash / signature compare (MITIGATED)

**Attack:** Attacker measures compare-time of `prev_hash`,
`entry_hash`, or `signature` to recover partial bytes.

**Mitigation:** Every security-critical comparison routes through
`constant_time_eq_str`, which wraps the Cyrius stdlib's branchless
`ct_eq_bytes_lens` (`lib/ct.cyr`). Remaining
`str_eq` calls are on public metadata (`source`, `action`,
`agent_id`) where timing leakage is not exploitable.

### T5 — Denial of service via malformed input (MITIGATED)

**Attack:** Attacker provides malformed `details`, corrupt JSONL
file, or truncated input to crash or hang the parser / verifier.

**Mitigation:**
- Fuzz harness (`fuzz/fuzz_libro.fcyr`, 12 targets)
  asserts no-crash on random inputs across `sha256`, `hex_decode`,
  `der_parse`, `entry_create`, `chain_ops`, `sig_verify`,
  `json_parse`, `topic_match`, `chain_import`,
  `filestore_verify_streamed`, `canonical_json_hash`,
  `proof_from_json`.
- `filestore_verify_streamed` uses a bounded 64KB buffer per chunk
  — unbounded-memory DoS is not possible.
- Lines > 64KB return an explicit error rather than continuing.
- A record that does not parse ends the verify with a
  `LIBRO_ERR_JSON` error naming it (2.11.3); it used to be skipped,
  which is the T1 gap below.
- The canonical-JSON validator caps nesting at `CJH_MAX_DEPTH`
  (128) before any emit runs, and object keys sort with a merge
  sort (2.8.11). Before that, ~130 KB of nested array opens crashed
  append and every later verify, and attacker-chosen keys cost
  O(n²) comparisons on every verify.
- Signature length and hex validity are checked for every
  algorithm, hybrid included, before anything reaches sigil
  (2.8.11 / 2.8.12); a malformed signature in an attacker-supplied
  proof used to crash `proof_verify_signed`. `ts_request_to_der`
  validates the digest's length, hex and algorithm before sizing a
  buffer (2.8.12; a SHA-512 digest overflowed a fixed one).

**Residual:** A known hang bug in `filestore_verify_streamed` on
unterminated-tail input was found by fuzz and fixed in 2.0.3
(Finding 4 in the 2.0 audit). The shared `flock` leak that
compounded it is also resolved. The 2026-08-22 audits found more of
the same class (crash or corruption on untrusted input:
`2026-08-22-audit.md` Findings 9–14, `2026-08-22-audit-2.md`
Findings 7–8), all fixed in 2.8.11 / 2.8.12. Bugs of this kind in
the streaming / import parsers remain the primary residual-risk
class and are why these paths have dedicated fuzz coverage.

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
sealing; per-key TPM sealing is not implemented, and is tracked
under roadmap 2.16.0). See [`docs/guides/tpm-anchors.md`](../guides/tpm-anchors.md)
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

**Mitigation:** Every user-supplied value reaches patra as a bound
parameter of a prepared statement (`patra_prepare` +
`patra_bind_text`): the INSERT in `patrastore_append` (since 2.8.1)
and the `WHERE src = ?` lookup in `patrastore_by_source` (since
2.8.11; until then the read path interpolated its argument into the
SQL text, so a quote returned zero rows and a crafted value could
widen the predicate). The only SQL assembled at runtime is the
CREATE TABLE / CREATE INDEX DDL, from fixed identifiers.

**Residual:** A bug in patra's parameter-binding could bypass.
patra has its own fuzz harnesses (SQL among them) and test suite;
libro pins a specific patra tag (`[deps.patra] tag` in
`cyrius.cyml`, 1.16.6 at libro 2.11.2) so an upstream regression is
detectable.

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
anchor data hasn't been tampered with since". Inside a proof bundle
the anchor is also bound to the signed tree head: its `merkle_root`
and `entry_count` must match (2.8.12, T16), so a self-consistent
anchor for a different tree no longer reports `ANCHOR_VALID`.

**Residual:** TPM sealing is opt-in (`-D LIBRO_TPM`). It does NOT
prove chain correctness (verify against the tree separately), host
honesty (a compromised host with TPM control at seal time can
produce a valid seal for arbitrary content), or identity (combine
with Ed25519/ML-DSA-65 entry signing for attribution). PCR rotation
invalidates seals — re-seal is consumer-managed. See
[`docs/guides/tpm-anchors.md`](../guides/tpm-anchors.md).

### T15 — Canonical-form and serialization ambiguity (MITIGATED 2.8.11 / 2.8.12)

**Attack:** Attacker who can write a stored `details` (JSONL reload,
PatraStore row, proof import) changes its content without changing
the entry hash; or a crafted field value is read back as a different
record.

**Mitigation:**
- **Malformed `details` cannot collide** (2.8.11). Through 2.8.10 the
  canonical-JSON hasher skipped bytes it did not recognise, so
  `{"a":1}`, `{ZZZZ"a":1}`, `{"a":1 JUNK}`, `{"a" 1}` and
  `{"a":1}XXXXXXX` hashed identically: text injected into a stored
  `details` left the entry hash, chain linkage, Merkle root and
  tree-head signature unchanged. The document is now validated
  strictly first (`canonical_json_is_valid`); anything malformed
  takes a `0x00`-tagged, length-prefixed hash of the raw bytes,
  distinct from every canonical emission and injective among
  malformed inputs.
- **One logical value, one digest** (2.8.12). Strings are decoded
  and re-emitted with minimal escaping, with object keys sorted by
  their decoded form; numbers are emitted as an exact plain decimal
  (`1` == `1.0` == `1e0` == `10e-1`); an escaped surrogate pair
  decodes to real UTF-8 (not CESU-8) and a lone surrogate is
  rejected; an empty `details` no longer shares the preimage of the
  literal `null`. See ADR 0007's 2.8.11 and 2.8.12 amendments.
- **Every serialized field is escaped** (2.8.11 / 2.8.12) in the
  FileStore JSONL and export writers, and the JSONL reader rejects a
  duplicated key, so a crafted `timestamp`, `source` or other field
  cannot inject sibling keys or split one record into two. The other
  two text sinks are encoded too: `log_syscall` percent-encodes
  `action` and `data` before the newline-delimited kernel audit sink,
  and the streaming topic percent-encodes `source` and `action`, so a
  `/` cannot forge a topic level.

**Residual:** libro's number form is exact where RFC 8785 mandates
the shortest round trip. They agree on every short decimal
expansion, not on every value (`1e100`, for one), so a consumer re-deriving a
digest with a JCS library can disagree; changing either form is a
preimage change (roadmap 2.17.0). Chains, proofs and anchors written
before 2.8.12 do not verify, and there is no migration path: the old
preimage was unsound, so consumers re-export and re-anchor. Findings:
`docs/audit/2026-08-22-audit.md` (1, 11, 25) and
`docs/audit/2026-08-22-audit-2.md` (1, 2, 6).

### T16 — Merkle second-preimage and self-referential proofs (MITIGATED 2.8.11 / 2.8.12)

**Attack:** Attacker presents an internal Merkle node as a leaf;
exploits odd-node duplication so two different leaf sets share a
root (the CVE-2012-2459 shape); ships a proof bundle whose
inclusion, consistency or anchor evidence is internally consistent
but describes a different tree from the signed head; or restates
when a signed head was attested.

**Mitigation:**
- **Domain separation** (2.8.11, RFC 9162 §2.1.1): a leaf hashes as
  SHA-256 of `0x00` followed by the entry hash, an internal node as
  SHA-256 of `0x01` followed by left and right. `merkle_leaf_hash`
  is public so a consumer can compute the expected leaf.
- **Odd-node promotion** (2.8.11, RFC 9162 §2.1.2): `merkle_build`
  promotes an odd trailing node instead of duplicating it, so `{d}`
  and `{d, d}` no longer share a root and `merkle_build` agrees with
  `merkle_canonical_root` at every size.
- **Proof evidence bound to the signed head.** The bundle verifier
  checks inclusion with `merkle_verify_proof_against` (2.8.11),
  consistency with `merkle_verify_consistency_against`, and an
  anchor's `merkle_root` and `entry_count` against the head (2.8.12).
  Each used to be checked only against itself.
  `merkle_verify_inclusion(mp, root, entry_hash)` (2.8.12) also binds
  the leaf to a specific entry.
- **The tree-head signature covers the whole head** (2.8.12): it is
  taken over `tree_head_digest(root, tree_size, timestamp,
  algorithm)`, length-prefixed. It used to cover the root alone, so
  `timestamp` was unauthenticated. The embedded `verifying_key` is
  excluded on purpose: the verifier supplies its own trusted key.

**Residual:** `merkle_verify_proof` and `merkle_verify_consistency`
remain public and check a proof only against itself. Consumer code
that verifies proofs outside `proof_verify_signed` /
`proof_verify_unsigned` must use the `_against` /
`merkle_verify_inclusion` forms with a trusted root. Findings:
`docs/audit/2026-08-22-audit.md` (3, 4, 5) and
`docs/audit/2026-08-22-audit-2.md` (3, 4).

### T17 — Unverified RFC 3161 token (PARTIAL — token not verified)

**Attack:** Attacker presents a timestamp attestation as evidence
that a hash (a Merkle root, say) existed at a given time, using a
response a trusted TSA never issued for that hash: a genuine granted
response obtained for a different hash, or a fabricated one.

**Mitigation (syntactic only):** `ts_response_from_der` checks the
DER tags of the outer SEQUENCE, the PKIStatusInfo and the status
INTEGER and rejects an overflowing length; `ts_attestation_new`
refuses a response that is not granted or carries no token (2.8.11;
before that, junk whose second byte read as a plausible length could
parse as granted).

**Residual:** libro does **not** verify the TSA's CMS signature over
the token, and does not parse TSTInfo, so it does not confirm that
the token's `messageImprint` matches the attested hash or that the
nonce echoes the request. An attestation proves "a syntactically
valid granted response was obtained", not "a trusted TSA attested to
THIS hash at THAT time". `ts_attestation_verify_hash` compares the
hash recorded in the attestation, not anything inside the token, and
the attestation's `created_at` is libro's local clock. A consumer
relying on RFC 3161 evidence verifies the stored token itself
against the TSA's certificate chain. TSTInfo binding (`messageImprint`
and nonce) is tracked as roadmap 2.12.0; CMS signature verification
is under roadmap 2.x.x, blocked on sigil, which has no public
SignedData-verify API. See `ts_attestation_new`'s scope comment in
`src/timestamping.cyr` and `docs/audit/2026-08-22-audit.md`
(Finding 15, "Not fixed in 2.8.11").

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
| Canonical-form / Merkle preimage   | Mitigated (2.8.11 / 2.8.12) | ADR 0007; preimage version discriminator tracked under roadmap 2.17.0 |
| Proof binding in consumer code     | Consumer            | `_against` / `merkle_verify_inclusion` verifiers |
| RFC 3161 token authenticity        | Not verified        | Binding: roadmap 2.12.0; CMS signature: 2.x.x, blocked on sigil |

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

- **Cyrius toolchain pin** in `cyrius.cyml` `cyrius = "6.7.18"`.
  CI reads this field and installs the exact toolchain via the
  canonical `scripts/install.sh` flow. No wildcard ranges.
- **sigil pin** in `cyrius.cyml` `[deps.sigil] tag = "3.13.15"`.
  `cyrius deps` resolves deterministically. Pins libro to sigil's
  full FIPS 204 ML-DSA stack. The pin names a **thin sub-surface**
  (`dist/sigil-mldsa.cyr` + `src/{sha_ni,sha256,hex}.cyr`), not the
  monolithic bundle — a narrower attack surface as well as a smaller
  binary, since the unused x509/RSA/authenticode fold is never linked.
- **sigil TPM pin** in `cyrius.cyml` `[deps.sigil_tpm] tag = "3.13.15"`,
  marked `optional` and gated behind the `tpm` feature. With the
  feature off it is not cloned, copied, or compiled — the default
  build carries **no TPM surface at all**, rather than a DCE-stripped
  one. Opt in via `cyrius deps --features tpm` + `-D LIBRO_TPM`.
- **patra pin** in `cyrius.cyml` `[deps.patra] tag = "1.16.6"`.
  Same as above. Pins the prepared-statement / group-commit /
  STR-btree feature set.
- **Tag pins are the contract.** The sigil and patra tags equal the
  versions cyrius 6.7.18 folds (sigil 3.13.15, patra 1.16.6), so
  libro tests the same code a consumer of `dist/libro.cyr` compiles.
  `cyrius.lock` is gitignored; CI and release resolve from the tags
  with `cyrius deps`.
- **Stdlib** modules (the `[deps] stdlib` list in `cyrius.cyml`) ship
  with the toolchain release. No libro library module calls bayan:
  the canonical-JSON hasher has its own decoder (`json_str_decode`,
  `src/entry.cyr`) and FileStore and chain import share libro's own
  strict JSONL parser, so no bayan parser sits on the integrity
  path. bayan still reaches every build through the `# Requires`
  block of `dist/sigil-mldsa.cyr`, which uses its `u256_*` helpers.
- **Zero third-party deps** beyond the Cyrius toolchain + sigil +
  patra. No transitive dependency graph to audit. (`agnosys` was
  dropped in 2.7.5, when the TPM primitives were re-sourced from
  sigil 3.9.0; 2.8.0 moved them behind the optional
  `[deps.sigil_tpm]` fold. See `development/dependency-watch.md`.)
- **Compiler-integrity note.** The toolchain is itself in the trust
  boundary: cyrius 6.4.80 fixed a constant-fold defect that silently
  miscompiled ~10 % of systematic 3-term constant expressions while
  every gate stayed green, and it was live at libro's prior pin.
  Libro was verified clear by scanning for the failing expression
  *shape*, not by a passing suite. Treat "tests pass" as insufficient
  evidence of correct codegen across any toolchain bump that names a
  silent-wrong-value fix.

## Change log

Last full restructure: 2.6.0 docs-pass (added T13 PQ-resistance /
T14 anchor-tampering, refreshed T2 / T6 / T7 for the 2.x crypto +
hardware additions, refreshed the supply-chain pins).

2.11.2 docs pass (2026-10-28): added T15 canonical-form ambiguity,
T16 Merkle second-preimage / proof binding and T17 unverified
RFC 3161 token from the 2026-08-22 audits (2.8.11 / 2.8.12); TPM
backend agnosys → sigil `dist/sigil-tpm.cyr`; constant-time compare
attributed to stdlib `lib/ct.cyr`; T1 / T4 / T5 / T10 / T14 and the
supply-chain pins refreshed to cyrius 6.7.18, sigil 3.13.15, patra
1.16.6.
