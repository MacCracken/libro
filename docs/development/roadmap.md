# Roadmap

Forward work only. Release detail lives in
[CHANGELOG.md](../../CHANGELOG.md); doc currency lives in
[`doc-health.md`](../doc-health.md).

## Active patch line — 2.7.x

Open items, sequenced fastest-to-land first, reactive items last.

- [ ] **RFC 6901 JSON Pointer queries** (`/entries/0/hash` etc.).
  Cyrius 5.7.40+ ships the parser. Adds a public API surface that
  feeds into `QueryFilter` for CLI tools or config-driven retention
  policies; substrate for the structured-audit query DSL listed
  under Ideas.
- [ ] **JSON streaming for very large proofs.** Cyrius 5.7.40–42
  shipped `json_stream_*` emitters. Libro's `proof_to_json` is
  in-memory string building; a streaming refactor helps when
  proof size becomes a memory concern (large chains with deep
  inclusion-proof sets). Reactive item — pick up when a consumer
  reports memory pressure.

## Ecosystem-blocked

Items genuinely blocked on upstream capability. Each names the
unblocker.

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
