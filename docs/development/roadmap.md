# Roadmap

Release detail lives in [CHANGELOG.md](../../CHANGELOG.md). This
document is the forward-looking plan — open threads, the next four
minors, and ecosystem-blocked items.

## Release history (brief)

| Version | Focus | Headline |
|---------|-------|----------|
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

## Planned minors (2.1.x → 2.5.x)

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
  2.1.1 development; **deferred indefinitely.** The homogeneous
  groups in libro's test surface (10× `test_layout_*`, 5× canonical
  JSON, ~5× SHA-256 vectors) each exercise *different accessor
  functions per case* — collapsing them via `test_each` requires
  fn-pointer indirection per field, which costs more LOC than it
  saves and obscures the intent of the layout-invariant probes.
  `test_each` is the right shape for "input → expected output"
  tables (`json_pointer` corpus, etc.); libro's test surface
  doesn't have that shape in any meaningful concentration.
- [ ] **Investigate bench-context `proof_to_json` control-flow
  hijack.** Carried from 2.0.5. Calling `proof_to_json(ip)`
  inside `bench_run` causes `main()` to re-enter ~25 Hz. Real
  bug; the bench was dropped from `libro_proof.bcyr`. Could be
  stack corruption, DCE interaction, or a codegen issue. The
  cyrius 5.10.x bug-pass cycle may have closed it incidentally —
  re-test before deeper diagnosis.

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

## Open — unblocked (not yet slotted)

These remain on the menu but don't fit a specific minor yet. Pick
one up as 2.x.0 fillers or absorb into a related minor.

- [ ] **`proof_from_json` round-trip.** 2.0 ships `proof_to_json`
  but no parser to re-hydrate a saved proof. Closes the loop for
  archival workflows. Pairs naturally with a fuzz target on the
  new parser. The 2.0.x release line landed a partial-verify
  round-trip — finishing the round-trip is mostly mechanical.
- [ ] **JSON streaming for very large proofs.** Cyrius 5.7.40–42
  shipped `json_stream_*` emitters. Libro's `proof_to_json` is
  in-memory string building. If proof size becomes a concern
  (large chains with deep inclusion-proof sets), refactor to
  streaming. Not actionable until a consumer hits the wall.
- [ ] **RFC 6901 JSON Pointer queries** (`/entries/0/hash` etc.).
  Cyrius 5.7.40+ ships the parser. Could feed into `QueryFilter`
  for CLI tools or config-driven retention policies. Speculative.
- [ ] **Extend the raw-offset guard to ambiguous-param structs.**
  ~15 derived structs use single-letter param names that overlap
  across files. Needs either codebase-wide rename or per-file
  allowed-offsets map cross-checked against `#derive` declarations.
  Option (b) is the right fix but needs more tooling than a shell
  grep.
- [ ] **Struct-layout invariant tests for the remaining structs.**
  Three landed in 2.0.4. Expanding to the other ~24 derived
  structs is ~100 more assertions; low effort, diminishing returns
  beyond the shape-spectrum trio but useful confidence after
  toolchain bumps.

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

## Out of libro scope (tracked elsewhere)

- **MCP tools (`libro_query`, `libro_verify`, `libro_export`) via
  bote.** Lives in the bote repo — libro's API is stable; the MCP
  surface is a wrapper concern and shouldn't grow libro's module
  count.

## Future (speculative)

Items without a clear owner or scheduled unblocker. Drop or
promote to "Open" if they reach actionable state.

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
