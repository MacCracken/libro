# Roadmap

Forward work only. Release detail lives in
[CHANGELOG.md](../../CHANGELOG.md); doc currency lives in
[`doc-health.md`](../doc-health.md).

**Current release: 2.11.1** (cyrius 6.7.18, sigil 3.13.15, patra 1.16.6).

Every open item has a target version:

- **2.11.x patches**: documentation, test hygiene and dependency tracking. No API change.
- **2.12.0 – 2.17.0**: one theme per minor.
- **2.x.x**: items blocked upstream. Each names its unblocker.

The items were collected from this file, the four audit reports, the CHANGELOG's deferrals
since 2.8.0, the guides and `dependency-watch.md`, and each was re-checked against the
2.11.0 tree.

---

## 2.11.2 — Documentation and housekeeping

No behaviour change.

- [ ] **Doc currency pass.** Several docs trail the code by many releases:
  - `doc-health.md` was last refreshed at 2.8.3.
  - `threat-model.md` was last updated 2026-05-10 (post-2.6.0). It still names agnosys
    as the TPM backend (:64-68) and pins cyrius 6.6.6 / sigil 3.12.9 / patra 1.13.8
    (:380-394), and T10 still says patra "1.1.1" (:281). It lists "RFC 3161 token" as an
    asset (:86), although the token is not verified (see 2.12.0). None of the 2.8.11 /
    2.8.12 defect classes appear.
  - `compliance/standards-mapping.md` still shows:
    - the TPM via agnosys (:69);
    - "scalars emit verbatim" (:57), false since 2.8.12;
    - stdlib `json`/`bigint` instead of `bayan` (:230-234);
    - a Timestamp Integrity section (:74-95) without the unverified-token limit.
  - `dependency-watch.md` says libro "inherits" sigil's parallel `sv_verify_batch`
    speedup (:180-184). It does not: that API verifies file artifacts against sigil's
    trust store, and libro never calls it (`integration.md` :270-273 has this right).
    Its test counts (:103-104) and capacity numbers (:125-126) are stale.
  - `guides/integration.md` :285-289 still puts hybrid signing "on the 2.3.x roadmap";
    it shipped in 2.3.0.
  - `guides/tpm-anchors.md` keeps its own "Roadmap" section. Those items now live under
    2.16.0 below.
- [ ] **Quickstart's in-repo include list.** `guides/quickstart.md` :56 includes the
  monolithic `lib/sigil.cyr`, the ~13 MB x509/RSA fold (CLAUDE.md quirk #9), and has no
  `lib/thread_local.cyr` before it, which SIGILLs at runtime (quirk #5). It should mirror
  `src/main.cyr`'s thin surface.
- [ ] **ADR status.** 0001–0004 still read "Accepted", and 0003 "Accepted (interim)",
  though the plans they recorded as blocked on sigil or patra have shipped (0002 :21,
  0003 :20, 0004 :21). Each needs a superseded-or-current call.
- [ ] **The bayan premise, corrected.** Measured at 2.11.0:
  - libro's library modules call **no** bayan function. The canonical-JSON hasher has its
    own decoder (`json_str_decode`, `src/entry.cyr`), and only `fuzz/fuzz_libro.fcyr`
    calls `json_parse`.
  - The monolith reaches the build through sigil-mldsa's `# Requires` block, which uses
    bayan's `u256_*` helpers.
  - DCE now removes dead code rather than NOP-filling it: 1,252,936 B with `CYRIUS_DCE=0`,
    679,496 B with `=1`. The 2.8.11 audit's "~200 KB" for thinning predates both facts.

  Measured at 2.11.1 (cyrius 6.7.18, bayan 1.11.1): what bayan contributes after DCE is
  data, not code. Since bayan 1.9.0 the monolith's `# Requires` pulls all seven
  `lib/unicode/` files for its markdown parser, and their tables are top-level string
  `var`s that DCE keeps. `string_data` went 58,232 → 400,629 B and the binary
  679,496 → 988,864 B, with code size flat; old pins under 6.7.18 measure the same. About
  307 KB of it is `lib/unicode/`.

  What is left:
  - Correct `src/main.cyr`'s include comment, which still says libro's surface is
    `json_parse`/`json_get`.
  - Any thinning is upstream, not a libro manifest edit. sigil-mldsa calls bayan's legacy
    `u256_*` compat names, which only the monolith defines, so moving it to `bayan-bigint`
    means calling `bayan_u256_*`. The other route is DCE dropping global initializers that
    no live code reads.

  CLAUDE.md was corrected when this roadmap was reorganized.
- [ ] **The cc3-era repros in `tests/`.** `fixup_limit_repro.cyr`, `patra.cyr`,
  `patra_crash_repro.cyr` and `patra_standalone.cyr` document cc3 crashes resolved long
  ago, and their headers still give `cc3` run instructions. `patra.cyr` says "full suite
  pending codegen investigation", though PatraStore has run in the main suite since
  1.1.0. All four still build and exit 0 on 6.7.18; only `patra_uninit.cyr` runs in CI.
  Each needs a retire-or-relabel call.
- [ ] **`cyrius lint` untracked deferrals.** Six notes:
  - five are `XXX` false positives, on `\uXXXX` and on the junk-suffix test data in
    `src/entry.cyr`, `src/file_store.cyr` and `src/main.cyr`;
  - one is `not yet` in `src/timestamping.cyr` :421.

  Cross-reference or `#skip-lint` them so a real deferral stands out.
- [ ] **Re-measure the ML-DSA and hybrid sign benches on a quiet box.**
  `mldsa65_sign_entry` and `hybrid_sign_entry` have been flagged as noise since 2.8.11.
  In the 2.11.0 A/B their per-run ranges were still 3.35–5.90 ms and 4.47–9.68 ms, about
  1.8× and 2.2×.

## 2.11.x — Dependency tracking

Patch releases as upstream lands.

- [ ] **`_patrastore_ensure_table`'s existence probe after patra 1.16.1.** patra 1.16.1
  checks the table name before the 63-table cap, so a full database that holds
  `audit_entries` now answers `PATRA_ERR_EXISTS`, and the `SELECT COUNT(*)` probe runs only
  when `CREATE TABLE` fails some other way. It is not redundant: per patra 1.16.2 an
  autocommit statement whose sync fails answers `PATRA_ERR_IO` and has still been applied,
  and the probe then finds the table and opens the store. Whether a store should open in
  that state is open. (The other half of the 2.10.8 filing, `patra_initialized()`, was
  adopted in 2.11.1.)
- [ ] **Re-measure appends when patra makes its WAL probe free.** patra 1.16.0's
  per-statement recovery probe costs libro +5–7% on `patra_append_50_*` (2.11.0
  CHANGELOG). 1.16.1–1.16.6 do not change it (2.11.1's A/B: −1.2% / −2.0%, ranges
  overlapping). The fix is on patra's roadmap.

## 2.12.0 — Timestamp binding

- [ ] **Bind an RFC 3161 token to the hash it claims to attest.** This is the half of
  the 2.8.11 audit's open finding that needs no upstream work. Today `ts_attestation_new`
  checks DER shape, a granted PKIStatus and a token's presence, and it "does NOT yet parse
  TSTInfo". So a genuine response for one hash can be presented alongside another. Three
  checks close that:
  - parse the token's TSTInfo eContent;
  - require `messageImprint` to equal the attested hash, under the stated algorithm;
  - require the nonce to echo the request's.

  This is DER parsing libro already does. It does not authenticate the TSA; that is the
  CMS-signature half under 2.x.x. Update `ts_attestation_new`'s scope comment, the threat
  model and standards-mapping to the new boundary.

## 2.13.0 — Allocation

- [ ] **A reusable scratch arena for the canonical hasher.** `alloc()` is the bump
  allocator and is never reclaimed, so hashing cost is paid again on every verify of
  every entry. Measured at 2.11.0:
  - a 500-key object costs **×16** its size (5,281 B in, 89,472 B allocated);
  - `{"service":"sshd"}` costs 176 B;
  - `{}` costs 128 B, of which 112 B is `hasher_finalize_hex`.

  2.10.0's inline path (16 keys or fewer) took the small-object case from 608 B. It left
  about 64 B per number and 24 B per key. Raising `CJH_INLINE_KEYS` means going back to a
  merge sort (2.10.0 notes). A per-call arena the emitters draw from and reset is a
  structural change to how libro allocates. `test_canonical_hash_allocation_bounded`
  (bound 40×) guards the regression either way.
- [ ] **A ranged, allocator-aware read from the patra store.** The store's reads take no
  offset or limit, and each copies every matching row into the global allocator:
  - `patrastore_load_all`;
  - `patrastore_query`, which filters after a full load;
  - `patrastore_by_source`.

  `memstore_load_page` and `chain_page` page in memory only. agnostic needs the newest
  entries of the trail, paged. It works around the gap by keeping its newest 1024 entries
  in fixed slots (agnostic ADR 0011), so its audit view cannot reach older ones.
  `patrastore_load_page_a(a, s, offset, limit)` would let that copy go: `LIMIT`/`OFFSET`,
  or by index, into the caller's arena. patra 1.16.0 changed `LIMIT 0` to return no rows,
  so the zero-limit case needs a test.

## 2.14.0 — Query

- [ ] **RFC 6901 JSON Pointer queries** (`/entries/0/hash` etc.). Unblocked: `lib/bayan.cyr`
  exports `bayan_json_v_pointer` / `bayan_json_v_pointer_cstr`. Prefer the `bayan_*`
  prefix over the compat-shim names (CLAUDE.md quirk #7). It adds a public API surface
  that feeds `QueryFilter`, and it would be libro's first direct bayan call since the
  hasher took its own decoder.
- [ ] **Structured-audit query DSL.** `QueryFilter` is composable but code-only. A
  parseable string form would serve CLI tools and config-driven retention policies.
  JSON Pointers (above) are the substrate.

## 2.15.0 — Storage and archival

- [ ] **An explicit compaction tool**, driving `chain_apply_retention` and `chain_export`
  together for offline archival.
- [ ] **Column-family-style secondary indexes in PatraStore.** Today there is one index
  per query shape (`patrastore_create_source_index`). A generic model would support
  consumer-defined indexes. patra's STR indexes (1.7+) are the building block.

## 2.16.0 — TPM anchors

All of these are from `guides/tpm-anchors.md` and the threat model. They need
`-D LIBRO_TPM` and the `tpm` feature.

- [ ] **A re-seal API**: `tpm_anchor_reseal(ta, new_pcrs)`, for the firmware-update flow.
  Today consumers call `tpm_anchor_new` again against the same inner anchor.
- [ ] **PCR baseline integration.** Pair the seal with sigil's `pcr_measurement_new`
  baselines, so a verifier can tell a legitimate PCR change from an unexpected one.
- [ ] **Sealed-anchor JSON serialization.** `tpm_anchor` carries opaque sigil handles,
  which do not serialize, so sealed anchors cannot ship across hosts that share a TPM.
- [ ] **Key rotation and sealed-file cleanup.** There is no key-rotation API, and old
  sealed files are not removed (`tpm-anchors.md` :190-195). Per-key TPM sealing is listed
  in `threat-model.md` :211-216.
- [ ] **A real-TPM CI runner.** CI covers the API contract under "no TPM device" only; a
  `LIBRO_TPM` build on a TPM-equipped runner would cover the seal/unseal round trip.

## 2.17.0 — Canonical form, version 2

Both items change the hash preimage, so they ship together, and the discriminator ships
first. Through 2.8.12 every preimage change broke every existing chain at once (2.8.11 and
2.8.12 each forced a full re-export and re-anchor, by design: the old forms were unsound).

- [ ] **A preimage version discriminator.** A version field inside the preimage, so old and
  new entries verify side by side after a future correction instead of every consumer
  re-anchoring. It is unblocked, and nothing in the tree implements it yet.
- [ ] **Full RFC 8785 number formatting.** libro normalizes numbers to an *exact plain
  decimal* using digit-string arithmetic (2.8.12; ADR 0007 :65-69). JCS mandates
  ECMAScript `Number::toString`, the shortest round trip, which renders `1e100` as `1e+100`
  rather than 101 digits. The two agree on every short value, which covers every value an
  audit payload carries. Two things stand in the way:
  - **A guaranteed-shortest formatter.** The 6.7.9 stdlib has bayan's
    `bayan_f64_to_json`, Grisu2, which is documented as "shortest-or-near-shortest", not
    exact.
  - **A semantic decision.** JCS rounds through an IEEE double, so integers past 2^53 and
    long decimals would canonicalize to different, lossy values. libro's exact form does
    not do that.

## 2.x.x — Blocked upstream

- [ ] **RFC 3161 TSA signature verification**: the CMS/PKCS#7 half of the 2.8.11 audit
  finding. **Unblocker: sigil.** sigil has the pieces but no public, general
  SignedData-verify API: `_ac_find_signer_cert` and `_ac_verify_sig` are private to
  Authenticode, and `x509_parse` / `x509_verify_chain` are public. The TSTInfo binding
  above (2.12.0) does not depend on it. The x509/RSA path is what made `build/libro`
  ~14 MB at 2.8.0 (quirk #9), so this belongs behind an opt-in feature, like `tpm`.
- [ ] **FIPS 140-3.** sigil is not a validated module (standards-mapping :142-154,
  POA&M). **Unblocker: sigil.**
- [ ] **JSON streaming for very large proofs.** `proof_to_json` builds in memory. The 6.7.9
  stdlib has no incremental JSON emitter: bayan's `json_stream_*` is a SAX-style
  *parser*. **Unblocker: a bayan emitter, or the call to write a libro-local
  incremental writer over the `_sb_*` path.** It is reactive: no consumer has reported
  memory pressure.
- [ ] **Typed `#derive(accessors)` getters.** Once a getter carries its field's type, the
  `libro_empty_str()` sites revert to `str_from("")` and the
  "assigning non-pointer to typed pointer" diagnostic becomes load-bearing.
  `dependency-watch.md` :28-56 also covers the argument-passing check that would have
  caught the 2026-08-22 audit's Finding 9. **Unblocker: cyrius.**
- [ ] **Per-entry `Str` residue on a retaining `chain_append`.** The streaming `nokeep`
  path closed it in 2.9.0 / 2.10.0. The retaining path needs an allocator-level fix
  (2.8.4 notes). **Unblocker: cyrius's allocator.**
- [ ] **Multi-node chain sync (federated audit).** **Unblocker: an AGNOS-level federation
  protocol.** libro would layer a second meta-chain over the existing `WitnessAnchor`
  primitive for cross-node consistency.
- [ ] **Conflict resolution for concurrent appends.** Follows multi-node sync. Today
  libro is single-writer per chain; FileStore's `flock` and PatraStore's patra-level
  locking handle single-node, multi-process use.

## Tracked in other repos

- **MCP tools (`libro_query`, `libro_verify`, `libro_export`)** live in bote.
- **patra issues filed from libro 2.10.8**, to be repaired there:
  `2026-10-14-patra-open-faults-without-patra-init.md` and
  `2026-10-14-create-table-full-masks-exists.md`. libro's side is under 2.11.x.
