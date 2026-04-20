# Roadmap

Release detail lives in [CHANGELOG.md](../../CHANGELOG.md). This
document is the forward-looking plan — open threads, unblocked
hardening candidates, and ecosystem-blocked items.

## Release history (brief)

| Version | Focus | Headline |
|---------|-------|----------|
| **2.0.4** | Docs + CI + hardening | ADRs 0005–0007, threat-model rewrite, bench-history in CI, per-file raw-offset allowlist, struct-layout invariant tests (chain / iproof / anchor). 316 tests. |
| **2.0.3** | Fuzz + HIGH bug fix | `fuzz_chain_import` / `fuzz_filestore_verify_streamed` / `fuzz_canonical_json_hash` added. Streaming verifier infinite-loop on unterminated input fixed (Finding 4). |
| **2.0.2** | Accessor-sweep tail + CI extension | `src/proof_json.cyr` raw-offset migration. Specific-struct CI guard extended from 1 to 7 (struct, param) pairs. +7 assertions, 293 tests. |
| **2.0.1** | Audit follow-ups | CI manifest-completeness gate, raw-offset guard (chain), `chain_export`/`chain_import` integration snippet. |
| **2.0.0** | Major sprint | Breaking: `verify_chain(entries, base_index)`, nested scalar-aware canonical JSON. Added: `chain_append_batch`, `proof_to_json`, `chain_export`/`chain_import`, `filestore_verify_streamed`, bench history, `#derive(accessors)` across all structs. Toolchain 5.4.2 → 5.4.7. 286 tests. |
| 1.2.0 | cc3-debt paydown | 24 workaround globals removed. Bench binary split (cc5 16384 fixup cap). |
| 1.1.1 | CI modernization | Dist-freshness gate, `cyrius.toml` → `cyrius.cyml`, FileStore UAF fix. |
| 1.1.0 | P(-1) + patra | Cyrius 5.4.2 pin, patra 1.1.1 refresh, PatraStore UAF fix (unblocks 19 gated tests). |
| 1.0.2 | Sigil migration | SHA-256 + Ed25519 delegated to sigil. `src/sha256.cyr` deleted. 8 fuzz harnesses added. |
| 1.0.1 | FileStore | Append-only JSONL backend with flock. |
| 1.0.0 | Cyrius port | Full rewrite from Rust (8513 → ~5000 LOC). |

The Rust-era history (0.90.0 ← 0.22.4 ← 0.21.x ← …) lives in
CHANGELOG for archaeological interest.

## Open — unblocked (hardening candidates)

These are hardening-adjacent threads that could be picked up without
any upstream dependency. Listed in rough order of leverage.

- [ ] **TPM-backed `WitnessAnchor` sealing (agnosys / sigil-tpm
  integration).** Sigil 2.8.4 already ships `src/tpm.cyr` as a thin
  wrapper over `agnosys 1.0.0`'s TPM primitives
  (`tpm_available`, `tpm_seal_data`, `tpm_unseal_data`, `tpm_random`).
  It's not bundled into `dist/sigil.cyr` because the dist is
  self-contained and TPM requires pulling agnosys as a separate
  dep. The libro-side design question is whether to (a) ship an
  opt-in `src/tpm_anchor.cyr` that pulls agnosys + sigil.tpm and
  wraps TPM-sealing into `WitnessAnchor`, or (b) stay slim and let
  consumers compose libro + sigil.tpm + agnosys at their own
  level. The integrity-proof structure (signed tree head +
  `WitnessAnchor`) is already factored to accept a hardware
  attestation as an additional proof field, so option (a) is
  mostly a new module + CI-dep-pin update. Previously listed as
  ecosystem-blocked — that was wrong; sigil has the primitives.
- [ ] **Investigate bench-context `proof_to_json` control-flow hijack.**
  Calling `proof_to_json(ip)` inside `bench_run` causes `main()` to
  re-enter repeatedly (observed ~25 Hz — banner prints thousands of
  times, no benchmark ever completes). Ruled out: the function
  itself (tests pass), the include of `proof_json.cyr` alone (builds
  clean with DCE dropping the unused function), the underlying
  proof-build path (`proof_build_unsigned` / `_signed` benches in
  `libro_proof.bcyr` run fine). Triggered specifically by combining
  `proof_json.cyr` include + a call site from inside a bench. Could
  be stack corruption in the JSON string builder, a DCE interaction
  stripping a transitive helper, or a Cyrius codegen issue with the
  bench-harness call pattern. Real bug — the `proof_to_json` bench
  was dropped from `libro_proof.bcyr` pending root cause.
- [ ] **Extend the raw-offset guard to the remaining ambiguous-param
  structs.** 2.0.4's per-file allowlist closes the "any new
  raw-offset param name" regression class; 2.0.1/2.0.2's specific-
  struct guard covers 7 unambiguous (struct, param) pairs. Roughly
  15 more derived structs use single-letter parameter names (`a`,
  `e`, `r`, `s`, `t`) that overlap across files. Unlocking guard
  coverage on those needs either (a) codebase-wide parameter-name
  rename to disambiguate, or (b) a per-file allowed-offsets map that
  the guard can cross-check against each struct's `#derive`
  declaration. Option (b) is the right fix but needs more tooling
  than a shell grep.
- [ ] **Struct-layout invariant tests for more structs.** 2.0.4
  shipped three (chain / iproof / anchor — covering the shape
  spectrum). Expanding to the other ~24 derived structs would mean
  ~100 more assertions; low effort, diminishing returns beyond the
  shape-spectrum trio but useful for confidence after toolchain
  bumps.
- [ ] **`proof_from_json` round-trip.** 2.0 ships
  `proof_to_json(ip)` but there's no parser to re-hydrate a saved
  proof. Would close the loop for archival workflows (consumer
  saves a signed proof as JSON, later re-verifies it without access
  to the original chain). Pairs naturally with a fuzz target on the
  new parser.

## Open — ecosystem-blocked

These items are on the roadmap for visibility but blocked on
upstream capability. Each has a named unblocker chain that's been
verified against the upstream's own roadmap — if sigil (or another
named dep) doesn't have it scheduled, it doesn't belong here.

- [ ] **Post-quantum signatures (ML-DSA-65, NIST FIPS 204).**
  Unblocker chain: Cyrius stdlib → sigil 3.0 → libro. Cyrius needs
  `lib/keccak.cyr` (SHAKE-128/256 for ML-DSA's XOF step). Sigil
  2.8.4 already stubs the enum values (`SIG_ALG_ML_DSA_65`,
  `SIG_ALG_HYBRID`) ready for the crypto to land. **Status:** stalled
  upstream — keccak was originally slated for Cyrius 5.2.x but has
  been pushed back behind Windows-target support work and an
  ongoing bug/issue pass. No near-term ETA. Libro's
  `EntrySignature.algorithm` + `key_id` already support algorithm
  dispatch, so the libro-side migration remains a one-sprint job
  once sigil ships `src/mldsa.cyr`.
- [ ] **Hybrid signing (Ed25519 + ML-DSA-65).** Same unblocker chain
  as above. Sigil has this explicitly scheduled for 3.0 as a
  `TrustPolicy` with `required_signature_algorithms` — libro would
  produce entries with two signatures during the transition period,
  matching sigil's `SigilVerifier` hybrid policy.
- [ ] **Multi-node chain sync (federated audit).** Unblocker: an
  AGNOS-level federation protocol. Libro would layer a second
  meta-chain over the existing `WitnessAnchor` primitive for
  cross-node consistency.
- [ ] **Conflict resolution for concurrent appends.** Follows
  multi-node sync. Currently libro is single-writer per chain;
  FileStore's `flock` and PatraStore's patra-level locking handle
  single-node multi-process.

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
  config-driven retention policies.
- Column-family-style secondary indexes in PatraStore. Currently
  one index per query shape; a generic column-family model would
  support arbitrary consumer-defined indexes.
- Explicit compaction tool driving `chain_apply_retention` +
  `chain_export` together for offline archival.
