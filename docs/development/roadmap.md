# Roadmap

Forward work only. Release detail lives in
[CHANGELOG.md](../../CHANGELOG.md); doc currency lives in
[`doc-health.md`](../doc-health.md).

## Active patch line — 2.6.x

Open items, sequenced fastest-to-land first, reactive items last.

- [ ] **P2 — Migrate `constant_time_eq_str` to `ct_eq_bytes_lens`.**
  `src/hasher.cyr:51` (and the regenerated `dist/libro.cyr:116`)
  still calls bare `ct_eq(a, alen, b, blen)`. Sigil 3.0.2 retired
  the name in favour of `ct_eq_bytes_lens` (same signature, same
  branchless constant-time semantics — pure rename); sigil 3.0.1's
  dist already ships under a banner where the alias is gone, so
  consumers pinned to libro 2.5+ link against a missing symbol
  and have to ship a one-line shim (see argonaut 1.5.1's
  `src/compat.cyr` + `[deps.argonaut_compat]` self-reference).
  Self-contained fix: rename the call site + `test_ct_eq`
  helpers in `src/main.cyr:159–163`; regenerate dist; bump
  sigil floor pin in `cyrius.cyml` to whatever first ships the
  rename. No upstream blocker.
- [ ] **RFC 6901 JSON Pointer queries** (`/entries/0/hash` etc.).
  Cyrius 5.7.40+ ships the parser. Adds a public API surface that
  feeds into `QueryFilter` for CLI tools or config-driven retention
  policies; substrate for the structured-audit query DSL listed
  under Ideas.
- [ ] **Re-investigate `proof_to_json` bench-context control-flow
  hijack.** Carried open since 2.0.5. Re-tested against cyrius
  5.10.34 in the 2.1.1 / 2.2.0 / 2.5.0 release passes; bug
  persists with changed symptom (was ~25 Hz `main()` re-entry,
  now SIGILL on first bench iteration). Test-suite path is
  clean — only the bench harness manifests the bug. Re-test
  against whatever cyrius is current next time it's picked up.
- [ ] **JSON streaming for very large proofs.** Cyrius 5.7.40–42
  shipped `json_stream_*` emitters. Libro's `proof_to_json` is
  in-memory string building; a streaming refactor helps when
  proof size becomes a memory concern (large chains with deep
  inclusion-proof sets). Reactive item — pick up when a consumer
  reports memory pressure.

## Ecosystem-blocked

Items genuinely blocked on upstream capability. Each names the
unblocker.

- [ ] **Parallel batch verify hot path.** Sigil 3.0.0 shipped the
  parallel `sv_verify_batch` infrastructure but the workers
  serialize on the full-call mutex (correctness-only, 0.96–1.04×
  serial throughput in 3.0). Unblocker: sigil 3.1's alloc-free
  verify-hot-path rewrite.
- [ ] **Multi-node chain sync (federated audit).** Unblocker: an
  AGNOS-level federation protocol. Libro would layer a second
  meta-chain over the existing `WitnessAnchor` primitive for
  cross-node consistency. No upstream ETA.
- [ ] **Conflict resolution for concurrent appends.** Follows
  multi-node sync. Currently libro is single-writer per chain;
  FileStore's `flock` and PatraStore's patra-level locking handle
  single-node multi-process.

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
