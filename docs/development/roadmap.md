# Roadmap

Release detail lives in [CHANGELOG.md](../../CHANGELOG.md). This
document is the forward-looking plan — open threads, the next four
minors, and ecosystem-blocked items.

## Release history (brief)

| Version | Focus | Headline |
|---------|-------|----------|
| **2.6.1** | Layout-invariant coverage | 15 new `test_layout_*` fns + 1 TPM-gated. Total layout coverage 10 → 25 (26 with TPM). 502 tests default / 514 with `-D LIBRO_TPM`. |
| **2.6.0** | proof_from_json full round-trip + doc-health.md | Lossless inclusion paths (`{"h":<hex>,"s":<0/1>}`); 443 tests; new docs/doc-health.md ledger. |
| **2.5.0** | Opt-in TPM-sealed `WitnessAnchor` | New `src/tpm_anchor.cyr` behind `-D LIBRO_TPM`; agnosys promoted to direct pin. |
| **2.4.0** | PatraStore performance tier | Prepared SELECT/COUNT, sync-mode controls, append_batch, opt-in STR src_idx. |
| **2.3.0** | Hybrid signing | Ed25519 + ML-DSA-65 AND-mode. Struct slots extended for both algorithms. |
| **2.2.0** | Post-quantum signing | ML-DSA-65 (NIST FIPS 204) entry signing via sigil 3.0. |
| **2.1.0** | Toolchain + dep refresh | cyrius 5.4.7 → 5.10.34, sigil 2.8.3 → 3.0.1, patra 1.1.1 → 1.9.3. `[deps].stdlib` extended for sigil 3.0's bundle (ct/keccak/thread/agnosys + transitive). `lib/` un-tracked (agnosys pattern); CI install rewritten to use the canonical `scripts/install.sh`. `cyrius.cyml [package].version = "${file:VERSION}"`. 373 tests. |
| 2.0.5 | 2.0.4 follow-ups | Three more `#derive(accessors)` layout-invariant tests; third bench binary `libro_proof.bcyr`. |
| 2.0.4 | Docs + CI + hardening | ADRs 0005–0007, threat-model rewrite, bench-history in CI, per-file raw-offset allowlist, struct-layout invariant tests (chain / iproof / anchor). 316 tests. |
| 2.0.3 | Fuzz + HIGH bug fix | `fuzz_chain_import` / `fuzz_filestore_verify_streamed` / `fuzz_canonical_json_hash` added. Streaming verifier infinite-loop on unterminated input fixed (Finding 4). |
| 2.0.2 | Accessor-sweep tail + CI extension | `src/proof_json.cyr` raw-offset migration. Specific-struct CI guard extended from 1 to 7 (struct, param) pairs. +7 assertions, 293 tests. |
| 2.0.1 | Audit follow-ups | CI manifest-completeness gate, raw-offset guard (chain), `chain_export`/`chain_import` integration snippet. |
| 2.0.0 | Major sprint | Breaking: `verify_chain(entries, base_index)`, nested scalar-aware canonical JSON. Added: `chain_append_batch`, `proof_to_json`, `chain_export`/`chain_import`, `filestore_verify_streamed`, bench history, `#derive(accessors)` across all structs. 286 tests. |
| 1.2.0 | cc3-debt paydown | 24 workaround globals removed. Bench binary split (cc5 16384 fixup cap). |
| 1.1.x | CI modernization + patra | Cyrius 5.4.2 pin, patra 1.1.1, dist-freshness gate, FileStore/PatraStore UAF fixes. |
| 1.0.x | Cyrius port + sigil migration | Full Rust → Cyrius rewrite. SHA-256/Ed25519 delegated to sigil. FileStore append-only JSONL backend. |

The Rust-era history (0.90.0 ← 0.22.4 ← …) lives in CHANGELOG for
archaeological interest.

## Planned minors (2.1.x → 2.6.x)

Sequencing reflects unblocked-ness and visible value to consumers.
Each minor is one focused theme; patches under it are small grinds
(hardening, test-quality, doc passes).

### 2.1.x — toolchain-bump follow-ups (current line)

Small hardening grinds on top of the 2.1.0 dep refresh. Each is
isolated and ships as a patch.

- [ ] **`secret var` migration in `src/signing.cyr`.** Cyrius 5.5.12+
  guarantees zeroize-on-return for `secret var`. Libro's
  `signing_key_zeroize()` currently does this manually — move
  the seed and secret-key buffers to `secret var`, simplify the
  zeroize fn to a no-op (kept as API surface), and confirm the
  layout-invariant test for `signing_key` still passes. Small.
- [ ] **`getrandom` for `WitnessAnchor` nonce + `signing_key_generate`
  seed.** Cyrius 5.7.35 ships `lib/random.cyr`. Replace the raw
  `/dev/urandom` reads in `signing.cyr` and `timestamping.cyr`
  with the wrapper — a portability + auditability win, no perf
  delta. Small.
- [ ] **Landlock policy doc for PatraStore.** Cyrius 5.7.35 ships
  `lib/security.cyr` with Landlock enums + syscall wrappers.
  Document a hardening recipe (deny path traversal outside the
  intended `.patra` directory) under `docs/guides/integration.md`
  for consumers; libro itself stays unopinionated. Doc-only.
- [x] ~~**`lib/test.cyr` table-driven refactor.**~~ Investigated in
  2.1.1 development; not pursued for that release. Today's
  homogeneous groups in libro's test surface (10× `test_layout_*`,
  5× canonical JSON, ~5× SHA-256 vectors) each exercise *different
  accessor functions per case* — collapsing them via `test_each`
  requires fn-pointer indirection per field, which costs more LOC
  than it saves and obscures the intent of the layout-invariant
  probes. `test_each` is the right shape for "input → expected output"
  tables (`json_pointer` corpus, etc.); libro's test surface
  doesn't have that shape in any meaningful concentration.

### 2.2.x — post-quantum signing (ML-DSA-65)

NIST FIPS 204 ML-DSA-65 entry signing. Sigil 3.0.0 shipped the
full crypto stack (`src/mldsa*.cyr`, 8 modules, ~60 KB) — the
upstream blocker is gone.

Scope:

- [ ] Add `SIG_ALG_ML_DSA_65 = 1` dispatch in `EntrySignature.algorithm`
  (slot already reserved by sigil's enum).
- [ ] `signing_key_generate_mldsa()` + `sign_entry_mldsa(sk, e)` +
  matching `verify_entry_signature` dispatch on `algorithm`.
- [ ] Round-trip + tamper-rejection tests mirroring the existing
  Ed25519 battery (~20 assertions).
- [ ] Bench binary entry: `mldsa65_sign` / `mldsa65_verify`
  (sigil's published numbers: ~4.91 ms sign, ~2.23 ms verify —
  bench locally to confirm under libro's call shape).
- [ ] Roadmap doc + `docs/guides/integration.md` migration note for
  consumers wanting to opt new chains into PQ from day one.

Effort: medium (~80 LOC + test battery + bench). Sigil dispatch
is a thin wrapper; libro's algorithm-enum infrastructure already
supports it.

### 2.3.x — hybrid signing (Ed25519 + ML-DSA-65)

Layered on top of 2.2.x. Producers sign with both algorithms;
verifiers accept the policy-required subset. Sigil 3.0.0 ships
`sigil_verify_hybrid(...)` and `SIG_ALG_HYBRID = 2`.

Scope:

- [ ] `EntrySignature` grows from one signature blob to two slots
  (Ed25519 + ML-DSA-65) when `algorithm = SIG_ALG_HYBRID`.
  Storage layout decision: extend the struct vs. a sibling struct
  — needs an ADR.
- [ ] `sign_entry_hybrid(sk_ed, sk_mldsa, e)` produces both
  signatures; `verify_entry_signature` dispatches `sigil_verify_hybrid`
  with the consumer's required-algorithm policy.
- [ ] Migration tests: chain that begins Ed25519-only, rotates
  to hybrid, eventually ML-DSA-only. Verifiers at each stage
  accept the appropriate subset.
- [ ] `chain_export` / `chain_import` round-trip preserves
  hybrid-signature shape.
- [ ] Bench: hybrid-verify cost vs. Ed25519-only and ML-DSA-only
  baselines.

Effort: medium-large. The split from 2.2.x exists because hybrid
introduces a new entry-storage shape — ergonomic decisions belong
in their own release rather than bundled with the algorithm
introduction.

### 2.4.x — PatraStore performance

Patra 1.7–1.9's perf surface is currently unused by libro. The
write hot path (`patrastore_append`) re-tokenizes + re-parses SQL
on every append; the durability hot path issues an fdatasync per
mutating exec.

Scope:

- [ ] **Prepared statements in `patrastore_append`** —
  `patra_prepare(db, "INSERT INTO ...")` once, dispatch via
  `patra_exec_prepared` per entry, `patra_finalize` on
  `patrastore_close`. Patra benches show ~36% per-insert win
  (`insert_1k_prepared` 14 µs vs `insert_1k_exec` 22 µs); libro's
  batch-append path is the same shape.
- [ ] **`patra_set_sync_mode(SYNC_BATCH)` for `chain_append_batch`** —
  amortize fdatasync across the batch, switch back to `SYNC_FULL`
  before returning. Patra benches show ~64× speedup on real-disk
  btrfs/nvme (`insert_500_sync_full` 19.5 ms vs
  `insert_500_sync_batch` 306 µs amortized).
- [ ] **Optional STR-indexed columns for source/action queries.**
  Patra 1.7.0 ships STR-keyed B-tree indexes. If query benches
  show source/action-filtered queries above scan baseline, add
  `CREATE INDEX src_idx ON audit_entries (src)` to
  `_patrastore_ensure_table`. Defer if benches don't justify.
- [ ] Bench-history rows for the new paths so the win is on record.
- [ ] Update `docs/guides/integration.md` with the sync-mode
  knob for consumers doing bulk imports.

Effort: medium. Localized to `src/patra_store.cyr` and the
batch-append entry in `chain.cyr`.

### 2.5.x — TPM-sealed `WitnessAnchor` (opt-in)

Hardware-backed anchor sealing. Sigil 2.8.4 ships `src/tpm.cyr`
(thin wrapper over agnosys 1.0's TPM primitives). Kept opt-in so
consumers without a TPM (or running rootless) don't pay the
agnosys dep cost.

Scope:

- [ ] New module `src/tpm_anchor.cyr` (opt-in via build define
  `LIBRO_TPM=1`). Wraps `WitnessAnchor` to optionally include a
  `tpm_seal_data` blob over the signed tree-head.
- [ ] `[deps.agnosys]` added to `cyrius.cyml` (currently transitive
  via sigil — needs to be a direct dep).
- [ ] `tpm_anchor_verify` checks the seal against the active TPM
  via `tpm_unseal_data`. On a host without TPM, falls through to
  software-only verification with a warning.
- [ ] Tests: TPM-available + TPM-unavailable paths (the latter is
  the common consumer case; needs a stub).
- [ ] Doc: `docs/guides/tpm-anchors.md` covering the trust model
  (TPM seal proves "this anchor was created on a host with this
  TPM at this PCR state" — not "this chain is correct").

Effort: medium. Opt-in keeps the default build surface unchanged.

### 2.6.x — proof-format follow-ups + hardening grinds

The 2.6.0 line closes `proof_from_json` round-trip (lossless
inclusion paths) and lands the `docs/doc-health.md` ledger.
Subsequent patches on the line pick up the previously-unslotted
items in the order below — sequenced for fastest-to-land + lowest
risk first, with reactive items at the tail.

- [x] ~~**`proof_from_json` round-trip.**~~ Shipped in 2.6.0. The
  inclusion-path JSON shape switched from bare hex strings to
  `{"h":"<hex>","s":<0|1>}` objects so the side bit survives.
  `merkle_verify_proof` now passes against every parsed inclusion;
  legacy emissions still parse (degraded to SIDE_LEFT defaults) so
  archival proofs from older libro versions remain readable.
- [x] ~~**2.6.1 — Struct-layout invariant tests for the remaining
  structs.**~~ Shipped in 2.6.1. 15 new layout tests covering
  `archive`, `error`, `integrity`, `review`, `receipt`, `memstore`,
  `stream`, `ts_request`, `ts_response`, `ts_attestation`,
  `retention`, `proof_node`, `merkle_proof`, `consistency`, `pv`;
  plus a 16th gated behind `#ifdef LIBRO_TPM` covering `tpm_anchor`.
  Total layout coverage 10 → 25 (26 with TPM). +59 assertions
  default / +71 with `-D LIBRO_TPM`. Internal `_`-prefixed structs
  (`_patrastore`, `_sub`) intentionally not covered — not
  public-surface.
- [ ] **2.6.2 — Extend the raw-offset guard to ambiguous-param
  structs.** ~15 derived structs use single-letter param names that
  overlap across files. CI tooling work — needs either a codebase-
  wide rename or a per-file allowed-offsets map cross-checked
  against `#derive` declarations. Option (b) is the right fix but
  needs more tooling than a shell grep.
- [ ] **2.6.3 — RFC 6901 JSON Pointer queries** (`/entries/0/hash`
  etc.). Cyrius 5.7.40+ ships the parser. Adds a public API surface
  that feeds into `QueryFilter` for CLI tools or config-driven
  retention policies; substrate for the structured-audit query DSL
  listed under Ideas.
- [ ] **2.6.x — Re-investigate `proof_to_json` bench-context
  control-flow hijack.** Carried since 2.0.5. Re-tested in
  2.1.1 / 2.2.0 / 2.5.0 against cyrius 5.10.34; bug persists with
  changed symptom (was ~25 Hz `main()` re-entry, now SIGILL on
  first bench iteration). Test-suite path is fine. Re-test against
  whatever cyrius is current at that patch's time.
- [ ] **2.6.x — JSON streaming for very large proofs.** Cyrius
  5.7.40–42 shipped `json_stream_*` emitters. Libro's
  `proof_to_json` is in-memory string building; a streaming
  refactor helps when proof size becomes a memory concern (large
  chains with deep inclusion-proof sets). Reactive — pick up when
  a consumer reports memory pressure.

## Open — ecosystem-blocked

Items still genuinely blocked on upstream capability. Each has a
named unblocker; if the unblocker isn't actually scheduled, the
item moves to "Future".

- [ ] **Multi-node chain sync (federated audit).** Unblocker: an
  AGNOS-level federation protocol. Libro would layer a second
  meta-chain over the existing `WitnessAnchor` primitive for
  cross-node consistency. No upstream ETA.
- [ ] **Conflict resolution for concurrent appends.** Follows
  multi-node sync. Currently libro is single-writer per chain;
  FileStore's `flock` and PatraStore's patra-level locking handle
  single-node multi-process.
- [ ] **Parallel batch verify hot path.** Sigil 3.0.0 shipped the
  parallel `sv_verify_batch` infrastructure but the workers
  serialize on the full-call mutex (correctness-only, 0.96–1.04×
  serial throughput). Sigil 3.1's alloc-free verify-hot-path
  rewrite is the actual unblocker; libro stays serial until then.

## Tracked in other repos

- **MCP tools (`libro_query`, `libro_verify`, `libro_export`) via
  bote.** Currently lives in the bote repo.

## Ideas (not slotted)

Items without a named owner or upstream unblocker. Listed for
visibility; no implicit scope decision either way.

- Structured-audit query DSL. `QueryFilter` is composable but
  code-only; a parseable string form could enable CLI tools or
  config-driven retention policies. RFC 6901 pointers (above)
  could be the substrate.
- Column-family-style secondary indexes in PatraStore. Currently
  one index per query shape; a generic column-family model would
  support arbitrary consumer-defined indexes. Patra 1.7+ STR
  indexes are the building block.
- Explicit compaction tool driving `chain_apply_retention` +
  `chain_export` together for offline archival.
