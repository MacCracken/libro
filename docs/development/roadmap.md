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

- [ ] **Third bench binary `benches/libro_proof.bcyr`.** Deferred in
  2.0.2 when `proof_to_json` wouldn't fit in either existing bench
  binary under cc5's 16384 fixup-table cap. A minimal third binary
  (just `error` / `hasher` / `entry` / `verify` / `chain` / `merkle` /
  `signing` / `proof` / `proof_json`) would unblock perf tracking for
  the proof-building and JSON-emission paths.
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
upstream capability. Each has a named unblocker.

- [ ] **Post-quantum signatures (ML-DSA / CRYSTALS-Dilithium).**
  Unblocks when: sigil exposes ML-DSA primitives. Libro's
  `EntrySignature.algorithm` field + `key_id` already support
  algorithm dispatch, so the signing-side migration is local.
- [ ] **Hybrid signing (Ed25519 + PQ).** Unblocks with the above.
  Would produce entries with two signatures for the transition
  period.
- [ ] **Remote attestation (TPM-backed chain sealing).** Unblocks
  when: sigil or a sibling crate exposes TPM attestation primitives
  in Cyrius. Libro's integrity-proof structure (signed tree head +
  WitnessAnchor) is already factored to accept a hardware attestation
  as an additional proof field.
- [ ] **Multi-node chain sync (federated audit).** Unblocks when: an
  AGNOS-level federation protocol lands. Libro would layer a second
  meta-chain over the existing `WitnessAnchor` primitive for
  cross-node consistency.
- [ ] **Conflict resolution for concurrent appends.** Unblocks with
  multi-node sync. Currently libro is single-writer; FileStore's
  `flock` and PatraStore's patra-level locking are sufficient for
  single-node multi-process.

## Out of libro scope (tracked elsewhere)

- **MCP tools (`libro_query`, `libro_verify`, `libro_export`) via
  bote.** Lives in the bote repo — libro's API is stable; the MCP
  surface is a wrapper concern and shouldn't grow libro's module
  count.

## Future (speculative)

Items without a clear owner or timeline. Drop or promote to "Open"
if they reach actionable state.

- Structured-audit query DSL (current `QueryFilter` is composable but
  code-only; a parseable string form could enable CLI tools or
  config-driven retention policies).
- Column-family-style secondary indexes in PatraStore (currently one
  index per query shape; a generic column-family model would support
  arbitrary consumer-defined indexes).
- An explicit compaction tool that drives `chain_apply_retention` +
  `chain_export` together for offline archival.
