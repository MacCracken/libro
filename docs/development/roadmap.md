# Roadmap

Forward work only. Release detail lives in
[CHANGELOG.md](../../CHANGELOG.md); doc currency lives in
[`doc-health.md`](../doc-health.md).

## Active patch line — 2.8.x

Open items, sequenced fastest-to-land first, reactive items last.

- [x] **✅ FIXED in 2.8.9 — `PatraStore`'s cached prepared statements killed a
  caller on another thread.** Reported by Agnostic 2026-08-21, fixed the same
  day. `patrastore_open` cached its `SELECT`/`COUNT` handles; patra's SQL parse
  scratch is per-thread, so executing one from another thread dereferenced TLS
  that was not there and killed the process — no diagnostic, no error return.

  The cache is gone; both read paths take the `patra_query` fallback they already
  had, which parses on the calling thread. Struct layout unchanged. Guarded by
  `test_patrastore_read_from_another_thread`, mutation-verified: restoring the
  cache makes the suite dump core.

  ⚠ **Still open, filed with it:** `patrastore_load_and_verify` returns the
  entries vec on success and an error object on failure — **both non-zero
  pointers** — so the natural `if (r != 0)` reads an integrity violation as a
  successful load. An integrity check whose failure is indistinguishable from
  success deserves a separate signature (an out-param, or 0-on-success plus an
  out-vec). Agnostic sidesteps it by calling `patrastore_load_all` then
  `verify_chain` directly, which is unambiguous.

- [ ] **RFC 6901 JSON Pointer queries** (`/entries/0/hash` etc.).
  Unblocker satisfied: `lib/bayan.cyr` exports `bayan_json_v_pointer`
  / `bayan_json_v_pointer_cstr` (legacy `json_v_pointer*` names also
  forwarded by the compat shim — prefer the `bayan_*` prefix in new
  code, per CLAUDE.md quirk #7). Adds a public API surface that feeds
  into `QueryFilter` for CLI tools or config-driven retention
  policies; substrate for the structured-audit query DSL listed
  under Ideas.
- [ ] **JSON streaming for very large proofs.** Libro's
  `proof_to_json` is in-memory string building; a streaming refactor
  helps when proof size becomes a memory concern (large chains with
  deep inclusion-proof sets). Reactive item — pick up when a consumer
  reports memory pressure.

  ⚠️ **Premise correction (2.8.3):** this item previously claimed
  "Cyrius 5.7.40–42 shipped `json_stream_*` **emitters**". That is not
  what exists. `lib/bayan.cyr`'s `json_stream_*` set
  (`json_stream_handler_new` / `_on` / `_parse` / `_parse_str`) is a
  SAX-style streaming **parser**, and the only builders
  (`bayan_json_build`, `bayan_json_v_build`, `_build_pretty`) are
  in-memory. There is no incremental output emitter to refactor onto,
  so this item is **not** unblocked as written — it needs either an
  upstream bayan emitter or a libro-local incremental writer over the
  existing `_sb_*` string-builder path. Scoping that is an open call.

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

*(Re-source the TPM primitives from sigil — **shipped in 2.8.0**,
removed from this list. `[deps.agnosys]` is gone; `tpm_seal` /
`tpm_unseal` / `tpm_detect` now resolve from the optional
`[deps.sigil_tpm]` fold behind the `tpm` feature, with
`src/tpm_anchor.cyr` unchanged. See CHANGELOG [2.8.0].)*

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
