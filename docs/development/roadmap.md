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

  ✅ **The open item filed with it is RESOLVED in 2.8.11.**
  `patrastore_load_and_verify` returned the entries vec on success and an error
  object on failure — both non-zero pointers — so `if (r != 0)` read an
  integrity violation as a successful load. The 2.8.11 audit found the same
  shape in three more APIs (`filestore_load_and_verify`, `entry_new_validated`,
  `memstore_verify_streamed`), so the fix is a discriminator rather than a
  per-site signature change: every error constructor stamps a magic at offset 0
  and `libro_is_error(p)` / `libro_is_ok(p)` answer the question. Agnostic's
  `patrastore_load_all` + `verify_chain` workaround still works and is still
  unambiguous; it is no longer necessary.

- [ ] **RFC 3161 token verification (carried over from the 2.8.11 audit).**
  2.8.11 tightened the syntactic half — DER tags are checked, a length that
  overflows is rejected, and a granted response must carry a token — but libro
  still does NOT verify the TSA's CMS signature over the token, does not parse
  TSTInfo to confirm the token's `messageImprint` matches the hash it is
  attesting, and does not check the nonce against the request. So a
  `ts_attestation` proves "a syntactically valid granted response was obtained",
  not "a trusted TSA attested to THIS hash at THAT time", and a genuine response
  for one hash can be presented alongside another.

  Unblocker: CMS/PKCS#7 signature verification against a TSA certificate chain.
  That belongs in **sigil**, not here — libro should call it, not reimplement
  ASN.1 certificate handling. `ts_attestation_new`'s doc comment states the
  current limit so no caller can mistake it.

- [ ] **Hash-algorithm version discriminator.** 2.8.11 changed the entry
  preimage and the Merkle construction with no migration path — the old forms
  were unsound, so re-anchoring was the only honest option, but that is a cost
  every consumer paid at once. A version field inside the preimage would let a
  future correction verify old and new entries side by side instead. Design
  question worth answering before the next preimage change, not after.

- [ ] **Thin `[deps.bayan]` to `dist/bayan-json.cyr`.** libro's entire bayan
  surface is `json_parse` and `json_get`, and it pulls the 641 KB monolith —
  json + bigint + base64 + csv + toml + cyml + u128 + yaml + the 1.5.0 PDF
  writer/reader. Measured at the 2.8.11 bump: the bayan 1.4.2 → 1.5.2 fold alone
  accounts for the bulk of a +207 KB binary growth, and DCE NOPs the dead code
  rather than removing it. `dist/bayan-json.cyr` is 100 KB.

  This is the same shape as the 2.8.0 sigil thinning (14 MB → 724 KB), and bayan
  ships the sub-bundles already. It is NOT a drop-in: `bayan` currently comes
  from `[deps].stdlib`, so moving it to a git dep changes the dep graph, and
  `cyrius.cyml` documents at length why that has bitten this repo before —
  a declared dep overlays the snapshot on every resolve, and a downstream
  consumer holding both the fold and the sub-bundle gets duplicate definitions.
  Needs the transitive consumers (bote → agnosai) checked first.

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
