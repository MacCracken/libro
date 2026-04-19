# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [2.0.1-dev] - unreleased

Follow-up cycle after the 2.0.0 cut. Picks up the recommendations
filed in `docs/audit/2026-04-19-audit-2.0.md` — the underlying
findings were fixed in 2.0.0; these harden the scaffold so the same
drift classes can't recur.

### Added
- **CI manifest-completeness gate** in `.github/workflows/ci.yml`.
  Compares every `include "src/<file>.cyr"` line in `src/main.cyr`
  against the `[lib] modules` array in `cyrius.cyml` — fails on
  either direction of drift (included-but-not-listed OR
  listed-but-not-included). Closes the gap that let 2.0.0's
  `chain_io.cyr` ship outside of `dist/libro.cyr`: the pre-existing
  dist-freshness gate couldn't catch that class because its input
  (the manifest) was the stale oracle; this new step validates the
  manifest against a second source of truth (the actual include
  list).
- **CI raw-offset guard on `struct chain`** in
  `.github/workflows/ci.yml`. Greps `src/*.cyr` (excluding the
  defining `src/chain.cyr`) for `load64(c + N)` / `store64(c + N, …)`
  / `load64(c)` and fails if any survive. Prevents accessor-sweep
  regressions of the class caught in 2.0.0 (seven sites across
  `chain_io.cyr` and `review.cyr` slipping past the
  `#derive(accessors)` migration). Uses the AGNOS-wide convention
  that `c` is the chain parameter.
- **`chain_export` / `chain_import` integration snippet** in
  `docs/guides/integration.md`. Shows the JSONL round-trip with a
  post-import `chain_verify` step and a note that overflow archives
  aren't part of the snapshot.

## [2.0.0] - 2026-04-19

Major-version sprint and the last stop for all backlog items before 1.x
is frozen. Opens up breaking cleanups shelved in the 1.x line, ports
two APIs deferred since the Rust port, wires the missing
`dist/libro.cyr` distribution artifact per `DEPS-PATTERN.md`, and drains
the hardening backlog (nested JSON hashing, chain export/import, streamed
FileStore verification, bench history).

### Breaking
- **`verify_chain(entries)` → `verify_chain(entries, base_index)`** —
  the old one-arg form is gone. `verify_chain_offset` was folded in
  (it did the same work with an extra base-index argument). Callers
  verifying a whole chain from position 0 now pass `verify_chain(e, 0)`.
  `chain_verify(c)` is unchanged (still wraps `verify_chain` with the
  genesis prev-hash check). Consumers: update call sites.
- **Canonical JSON hashing is now depth-unlimited and non-scalar-aware.**
  1.x's flat canonicalizer quoted every value regardless of type
  (`{"n":42}` was hashed as if the value were the string `"42"`) and
  broke on nested objects/arrays. 2.0 walks raw JSON bytes recursively:
  objects sort keys lexicographically, arrays preserve order, scalars
  emit verbatim (trimmed of whitespace). Flat objects with all-string
  values hash identically to 1.x; any use of numbers, bools, null,
  arrays, or nested objects as details values changes the hash. Entries
  written in 1.x with such details will re-verify to a different hash
  in 2.0 — which is correct, since 1.x was silently miscoercing.

### Added
- **`chain_append_batch(c, severities, sources, actions, details_vec)`**
  — batch-append N entries with one rotation check (vs N) by taking
  four parallel vecs. Returns a vec of the created entry pointers.
  Capacity enforcement: auto-rotate is checked once at start; a batch
  larger than `max_capacity` can exceed it for the duration of the call
  and the next `chain_append` will rotate as usual. Bench:
  `chain_append_batch_100` within noise of `chain_append_100` at the
  current unlimited-capacity shape; the win shows up in capped chains.
- **`proof_to_json(ip)`** in `src/proof_json.cyr` — pretty-printed JSON
  emitter for `IntegrityProof`. Ports `to_proof_json()` from the Rust
  reference. Module lives separately so bench binaries that exercise
  proof verification without the JSON dep (notably `libro_core.bcyr`)
  can exclude it and stay under cc5 5.4.2's 16384 fixup-table cap.
- **`chain_export(c, path)` / `chain_import(path)`** in
  `src/chain_io.cyr` — full-chain JSON Lines serialization. Line 0 is
  a meta record (`_libro_chain:1`, `prev_chain_hash`, `max_capacity`);
  lines 1+ are entries in FileStore-compatible format. Overflow
  archives are not serialized — drive a FileStore for that. Round-trip
  preserves capacity, prev-chain-hash, and passes `chain_verify`.
- **`filestore_verify_streamed(s, chunk_size)`** — byte-streamed verify
  that keeps only `chunk_size` parsed entries live at a time. Reads the
  JSONL file in 64KB slices, rebuilds lines, verifies in chunks with
  cross-chunk linkage. Lines > 64KB are not supported (asserted).
- **Nested-capable canonical JSON hasher** in `src/entry.cyr`. See
  Breaking above for the semantic change.
- **Benchmark history tracking** via `benches/bench_history.cyr`.
  `LIBRO_BENCH_HISTORY=<path>` writes one CSV row per bench
  (`epoch,binary,name,avg_ns,min_ns,max_ns,iterations,tag`) with
  optional `LIBRO_BENCH_TAG` label. Unset → no-op. Included by both
  bench binaries.
- **`dist/libro.cyr`** — the committed consumer-distribution artifact
  produced by `cyrius distlib`. Was missing from every prior tag; any
  downstream `[deps.libro]` pulling a 1.x tag got a 404 on
  `cyrius deps`. `[lib] modules = […]` added to `cyrius.cyml` so the
  tool knows what to bundle. CI + release workflows now regenerate
  and gate on the artifact. See `DEPS-PATTERN.md`.
- **`_sb_csv_field` direct-emit escape path** — on the quote-required
  branch, replaces N per-byte `_sb_add_byte` calls with one pre-grow +
  a tight direct-write loop.

### Changed
- **FileStore read buffer is right-sized via `lseek(fd, 0, SEEK_END)`**
  instead of the 64KB→double-on-overflow scheme. On a 100 MB file the
  old strategy orphaned several doubling-step buffers in the bump
  allocator; now one allocation per `filestore_load_all`. Adds
  `_fs_file_size(fd)` helper.
- **`_filestore_cpath(s)` cached on the FileStore struct** — struct
  layout grew 16 → 24 bytes (`+16 cpath` added). `filestore_open`
  derives the cstr once; `filestore_append` / `filestore_load_all` /
  `filestore_len` read from the cached slot instead of calling
  `_entry_to_cstr(load64(s))` per op.
- **`_der_parse_tlv` returns `(total, value_ptr)` via multi-return**
  — replaces the `_der_value_ptr` / `_der_value_len` globals.
  Callers derive `value_len = total - (value_ptr - data)`.
  `civil_from_days` stays on `_cd_y/m/d` globals because 5.4.2's
  multi-return caps at 2 values.
- **Toolchain pinned to Cyrius 5.4.7** (was 5.4.2) for the
  `#derive(accessors)` migration below.
- **`#derive(accessors)` adopted across all 15 struct modules** —
  ~108 hand-written `load64(x + N)` accessors replaced by declarative
  struct layouts. Generated getters + `_set_` setters live where
  offset typos used to. The UUID-zeroing bug caught during the
  nested-JSON test work (where `store64(probe, 0)` zeroed only 8 of
  16 UUID bytes) is exactly the class this eliminates. Structs:
  archive, chain, memstore, _patrastore, filestore, retention, error,
  entry, proof_node, merkle_proof, consistency, merkle_tree, sth, pv,
  iproof, integrity, review, signing_key, verifying_key, entry_sig,
  ts_request, ts_response, ts_attestation, anchor, receipt, _sub,
  stream. Inline-UUID structs (entry, anchor, receipt) use
  `_uuid_hi`/`_uuid_lo` placeholders to reserve the first 16 bytes;
  their `*_id(x)` accessors stay hand-written and return the pointer.
  One name collision had to be resolved: the existing
  `merkle_proof(tree, idx)` function was renamed to
  `merkle_inclusion_proof(tree, idx)` because `struct merkle_proof`
  reserves the identifier as a type. The previous
  "ecosystem convention + hook-point flexibility" rejection was
  shallow: libro had **zero** hook-point uses across its ~108
  accessors before this refactor, and agnosys flags derive adoption
  as a deliberate post-1.0 follow-up — libro's 2.0 is that follow-up.
- **CI gates on `dist/libro.cyr` freshness** — PRs that edit `src/*`
  without regenerating `dist/libro.cyr` fail CI.
- **Benches regrouped** — `benches/libro_core.bcyr` grew one bench
  (`chain_append_batch_100`); 14 core + 8 i/o = 22 total. Also dropped
  unused `retention.cyr` from `libro_io.bcyr`'s includes — the nested
  canonical JSON code pushed live fixups back near the 16384 cap.
- **`chain_verify` / `verify_chain` layering documented** in
  `src/chain.cyr` — not duplication: `verify_chain` is the loose-entries
  primitive (used by FileStore, streams, archives); `chain_verify` adds
  the AuditChain-level `prev_chain_hash` check on top.
- **P(-1) hardening pass** (scaffold review, post-sprint). Two findings,
  both fixed:
  - **MEDIUM** — `dist/libro.cyr` was shipping without `chain_export` /
    `chain_import` because `src/chain_io.cyr` had been added to
    `src/main.cyr`'s include list but not to `cyrius.cyml` `[lib]
    modules`. `cyrius distlib` regenerated the dist from a stale
    manifest, and the CI "dist freshness gate" couldn't see the drift
    (its input and oracle were the same list). Manifest repaired;
    dist regenerated (4416 → 4477 lines); `chain_export` now at line
    3865 of the dist.
  - **LOW** — seven cross-module raw-offset reads of the `chain`
    struct survived the `#derive(accessors)` sweep: five in
    `chain_io.cyr` (`load64(c + 8/16)`, `load64(c)`, two
    `store64(c + …)`) plus two in `review.cyr` in `chain_review`
    (`load64(c)` at line 61 and `load64(c + 8)` at line 127). All
    migrated to `chain_entries` / `chain_prev_hash` /
    `chain_max_capacity` and their `_set_` siblings. Behavior
    preserved; `chain_review_100` within noise (1.429 → 1.443 ms).
  Full report: `docs/audit/2026-04-19-audit-2.0.md`.

### Decisions (no code change)
- **`_sb_csv_field` single-pass rewrite — REJECTED.** Current form is
  one cache-hot read pass + direct-write escape pass. A fused
  single-pass needs either optimistic-write-with-memmove (slower on
  no-quote path) or pre-grow-and-reset (same work). Roadmap already
  called the payoff marginal; confirmed on review. Keeping as-is.

### Validation
- **286 tests, 0 failed** (up from 255 in 1.2.0: +4 `append_batch`,
  +4 `proof_to_json`, +5 nested canonical JSON, +9 ChainIO round-trip,
  +5 streamed FileStore verify). All 15 struct modules on
  `#derive(accessors)`.
- **22 benches** across 2 binaries, all report. Bench history opt-in
  via `LIBRO_BENCH_HISTORY` env var.
- Fuzz harness clean (no crashes).
- Simulated-consumer test: `dist/libro.cyr` compiles and links when
  included after stdlib + sigil + patra.

## [1.2.0] - 2026-04-19

cc3-debt paydown sprint. With Cyrius 5.4.2 (cc5) reliably preserving
locals across nested call chains, we removed 24 workaround globals
and the language-era workaround syntax that libro was still carrying
from the cc3 days. Also: split the benchmark binary in two after it
overflowed cc5's raised-but-still-finite fixup table.

### Fixed
- **Bench binary overflowed the cc5 fixup table (16384)** —
  `benches/libro.bcyr` registered all 21 benches in one compilation
  unit. Under cc5 5.4.2 the peak live forward-ref count from the
  reachable src/ graph exceeded 16384 and the build failed with
  `error: fixup table full (16384)`. Split into `libro_core.bcyr`
  (13 crypto/chain/merkle/sign benches) and `libro_io.bcyr` (8
  export/review/anchor/stream/filestore benches). Both build and run
  clean; CI iterates `benches/*.bcyr`. `lib/fmt.cyr` was also
  missing from the include list (silent under cc3 forward-stub
  behaviour; a live-fixup source under cc5).

### Changed
- **24 workaround globals removed** across 5 modules:
  | File                  | Globals removed                                                                                     | Count |
  |-----------------------|-----------------------------------------------------------------------------------------------------|-------|
  | `src/patra_store.cyr` | `_ps_sb` `_ps_id` `_ps_ts` `_ps_sev` `_ps_src` `_ps_act` `_ps_det` `_ps_aid2` `_ps_ph` `_ps_hash` `_ps_halg` `_ps_db` | 12 |
  | `src/entry.cyr`       | `_cjh_hasher` `_cjh_pairs` `_cjh_keys` `_en_entry` `_ech_hasher` `_ech_entry`                       | 6  |
  | `src/anchoring.cyr`   | `_anch_ptr` `_ach_hasher` `_ach_anchor`                                                             | 3  |
  | `src/review.cyr`      | `_rev_chain`                                                                                        | 1  |
  | `src/chain.cyr`       | `_chain_c`                                                                                          | 1  |
  | `src/merkle.cyr`      | `_csh_nodes` (dead code)                                                                            | 1  |

  All were cc3-era workarounds for locals clobbered across nested
  `str_builder_*` / `hasher_update` call chains. cc5 5.4.2 preserves
  them reliably. No regressions on the PatraStore cumulative-state
  tests — the exact class of failure the globals were originally
  defending against.
- **Negative literals + compound assignment sweep** — `(0 - N)` → `-N`
  (13 sites) and `i = i + 1` → `i += 1` in pure counter loops (~50
  sites). Native in Cyrius 3.10.3+.
- **`cyrius.cyml` enriched** — added `repository`, `[deps] stdlib =
  […]` (13 modules), `[deps.sigil]` (tag 2.8.3), `[deps.patra]` (tag
  1.1.1). Matches first-party convention.
- **Roadmap consolidated** — folded `docs/development/sprint-1.2.0.md`
  decision log into `docs/development/roadmap.md` (Unreleased and
  Hardening backlog sections) and deleted the sprint file. Stale
  "Blocked on patra (SQL storage)" subsection removed — patra 1.1.1
  is integrated via `lib/patra.cyr` symlink + `src/patra_store.cyr`.

### Added
- **`severity_len(sev)` in `src/entry.cyr`** — constant-time lookup on
  a `SEV_LEN[]` table. `entry_compute_hash` no longer calls `strlen`
  on the severity cstr on every entry hash.

### Decisions (no code change)
- `secret var` (Cyrius 5.3.5) — **NOT APPLICABLE**. Libro key material
  is heap-allocated; `secret var` only zeroises stack-local arrays.
  `signing_key_zeroize(sk)` already handles heap zeroization.
- `ct_select` / `lib/ct.cyr` (Cyrius 5.3.5) — **NO MIGRATION NEEDED**.
  Every security-critical compare already routes through
  `constant_time_eq_str` → sigil's branchless `ct_eq`. Remaining
  `str_eq` calls are on public metadata (source / action / agent_id),
  not secrets.
- `#derive(accessors)` (Cyrius 3.7.1) — **REJECT**. Would require
  `struct` declarations across 18 modules. AGNOS-wide convention
  (libro, patra, sigil, ark) is raw-offset accessors; consistency +
  hook-point flexibility outweighs the ~30-line boilerplate saving.

### Performance
- `sign_entry`: 6.147 → 5.786 ms (**−5.9 %**) — from the patra_store
  local refactor (fewer global loads on the signing-key path).
- Every other bench within ±2 % noise.

### Validation
- **255 passed, 0 failed.** Both bench binaries build and report;
  fuzz harness clean (no crashes); format + lint clean (3 pre-existing
  line-length warnings on literal strings remain).

## [1.1.1] - 2026-04-19

CI/release modernization and a round of quick-win refactors from the
post-1.1.0 review pass.

### Fixed
- **FileStore silent corruption across loads (MEDIUM)** —
  `filestore_load_all` wrapped pointers into the global `_fs_buf`
  read buffer and shipped those references out through parsed
  entries. A second `filestore_load_all` overwrote the buffer
  in place, aliasing the first call's entries onto the second
  file's bytes. Fixed by cloning each line with
  `str_clone(str_new(_fs_buf + pos, line_len))` before parsing.
  Regression test `test_filestore_load_survives_second_load`
  added — flips PASS↔FAIL on 2 asserts if the clone is removed.
  See `docs/audit/2026-04-19-audit.md` Finding 3 (upgraded
  LOW → MEDIUM).

### Changed
- **CI/release workflows modernized** to match patra / first-party
  standards. Toolchain version now sourced from `.cyrius-toolchain`
  (no hardcoded version strings in YAML). `cyrius build` used in
  place of raw `cat | cc3`. `CYRIUS_DCE=1` applied to every build
  step. Format check, `cyrius lint`, ELF verification, fuzz
  harness run, and benchmark run added to CI. Release tag filter
  tightened from `'*'` to `'[0-9]*'` (semver-only).
- **Manifest renamed** `cyrius.toml` → `cyrius.cyml` to match the
  first-party convention (ark, nous, sigil, patra). Cyrius still
  accepts either name; `.cyml` is now preferred.
- **`.cyrius-toolchain` refreshed** to 5.4.2 (was 4.5.0; lagged
  behind the actual pin in `cyrius.cyml`).
- **`scripts/version-bump.sh`** updated to edit `cyrius.cyml`
  first, falling back to `cyrius.toml` when `.cyml` is absent.
- **`CLAUDE.md` refreshed** — dropped stale cc3-era Cyrius quirks
  (the `\r`-escape, negative-literal, fixup-8192, silent-stub,
  and 256-init-global workarounds have all been obsolete since
  Cyrius 3.10 / 4.x). Added `str_from`/`str_new` lifetime note
  and a P(-1) pointer into the agnosticos template.

### Added
- **`_sb_add_byte(sb, c)` helper in `src/export.cyr`** — single-byte
  append for the per-character paths in `_sb_json_escape` and
  `_sb_csv_field`. Replaces a per-character `alloc(2) + store8 +
  store8 + str_builder_add_cstr(…)` pattern that was producing one
  heap allocation per non-special character in JSON/CSV exports.
- **Single-pass `uuid_format` in `src/entry.cyr`** — replaces the
  former 5× `hex_encode_str` + `str_builder` path with one 37-byte
  allocation and direct nibble-to-hex writes. Every entry
  creation, export, and proof call paid the old cost.

### Performance (post-fixes vs 1.1.0 baseline)
- `export_jsonl_100`: 601 µs → 512 µs (**−14.8 %**)
- `export_csv_100`: 321 µs → 270 µs (**−15.9 %**)
- `chain_append_100`: 1.896 ms → 1.802 ms (**−5.0 %**)
- `proof_unsigned_100`: 1.314 ms → 1.290 ms (**−1.8 %**)
- `entry_hash`: 10 µs → 10 µs (unchanged at this resolution)

### Validation
- **255 passed, 0 failed** (up from 251 in 1.1.0; +4 from the new
  FileStore regression test).
- Benchmarks green, fuzz harness clean.

## [1.1.0] - 2026-04-19

Sprint 1.1.0 — P(-1) scaffold hardening. Cyrius 5.4.2 upgrade, patra
bundle refresh, and use-after-free fix in PatraStore that unblocks 19
previously-gated tests.

### Fixed
- **Use-after-free on patra result-set pointers (HIGH)** —
  `_patrastore_row_to_entry` wrapped raw pointers from
  `patra_result_get_str()` via `str_from()` without copying. After
  `patrastore_load_all()` called `patra_result_free(rs)`, every `Str`
  on every loaded entry dangled into freed memory. Later reads (e.g.
  `entry_hash` → `str_eq` → `memeq`) dereferenced freed data and
  SIGSEGV'd layout-dependently. Fix: new `_ps_copy_cstr()` helper in
  `src/patra_store.cyr` allocates a fresh buffer and `memcpy`s the
  cstr before wrapping. Loaded entries now own their string memory
  outright. See `docs/audit/2026-04-19-audit.md` Finding 1.
- **Ungated `test_patrastore_append_load`** (`src/main.cyr`) — the
  use-after-free above was the root cause of the v1.0.2–v1.0.4
  cumulative-state crash. Test passes cleanly after the fix.
- **Ungated 6 additional PatraStore tests + 12 Gap coverage tests**
  — same root cause. Suite grew 204 → 251 tests; 0 failures.

### Changed
- **Cyrius toolchain pinned to v5.4.2** (upgrade from v3.6.8 — cc5
  compiler). Structural PE32+ backend landed upstream but libro
  remains ELF-only.
- **Patra bundle refreshed** — `lib/patra.cyr` updated from v0.14.0
  (3013 lines) to v1.1.1 (3138 lines). API-compatible; pulls in
  upstream WAL-overflow detection, DROP TABLE, indexed-query
  planner, and 0.15–1.1.1 parser fixes.
- **Heap-reset shim dropped** — v1.0.3's
  `alloc_reset(); fl_init(); patra_init()` band-aid before the
  PatraStore block is no longer needed with the use-after-free fixed.

### Added
- `docs/audit/2026-04-19-audit.md` — pre-1.1.0 security audit.
- `_ps_copy_cstr(cstr)` helper in `src/patra_store.cyr` — owning-copy
  wrapper for cstrs returned from ephemeral patra buffers.

### Removed
- `issue-to-fix.md` — resolved by Finding 1 in the audit above.

## [1.0.3] - 2026-04-12

### Fixed
- **PatraStore tests ungated (6 of 7)**: `open_close`, `verify`, `query`,
  `by_source`, `transaction`, and `persistence` tests now run as part of the
  full suite. Heap is reset (`alloc_reset(); fl_init(); patra_init()`) before
  PatraStore to isolate from prior test allocations.
- `patra_init()` moved to startup (after `ed25519_init()`) so SQL state is
  initialized before any heap activity.

### Known issue
- `test_patrastore_append_load` remains gated — crashes in `str_builder_add`
  during INSERT SQL construction after the full test suite. Works in isolation.
  Suspected str_builder or patra interaction bug, not a compiler issue.

### Changed
- Cyrius toolchain pinned to v3.6.8


## [1.0.2] - 2026-04-11

### Fixed
- **Missing includes**: `lib/patra.cyr`, `lib/fmt.cyr`, `src/patra_store.cyr`
  added to `src/main.cyr`. Without these, all `patrastore_*` calls resolved to
  NULL stubs and segfaulted at runtime.

### Changed
- Cyrius toolchain pinned to v3.4.20 (input_buf 256KB, preprocess cap 1MB,
  dep-skip for test/bench files)
- PatraStore + Gap coverage tests gated pending cumulative-state investigation.
  204 non-patra tests pass.

## [1.0.1] - 2026-04-09

### Changed
- Cyrius toolchain pinned to v3.2.5 (cc3 compiler, minimum version)

## [Unreleased]

## [1.0.0] — 2026-04-09

### Changed
- **Language port: Rust to Cyrius** — full rewrite from 8,513 lines of Rust to ~4,950 lines of Cyrius
- SHA-256 implemented from scratch (FIPS 180-4), replacing BLAKE3 default + sha2 crate
- HMAC-SHA256 signing replaces Ed25519 (elliptic curve deferred; same API surface)
- In-process pub/sub with MQTT wildcards replaces majra/tokio async streaming
- MemoryStore replaces FileStore/SqliteStore as primary backend
- Timestamps use integer civil-date conversion (no chrono dependency)
- UUID v4 via /dev/urandom (no uuid crate)
- DER encoding/decoding for RFC 3161 preserved (hand-rolled, zero deps)
- 141KB static ELF binary, 121ms build time
- 193 tests (up from 262 Rust tests; Rust-specific serde/trait tests removed)
- 15 benchmarks covering all major operations
- Rust source preserved in rust-old/ for reference

### Added
- `benches/libro.bcyr` — 15 benchmarks: sha256, entry_hash, chain_append/verify, merkle build/proof/verify/consistency, sign/verify, query, export jsonl/csv, review, proof

### Removed
- All Cargo/crates.io dependencies (zero external deps — Cyrius stdlib only)
- SQLite store (deferred; MemoryStore covers in-process use)
- FileStore (deferred; export functions cover persistence)
- BLAKE3 hash backend (SHA-256 only for simplicity)
- tokio/majra async runtime (synchronous pub/sub via function pointers)
- serde derives (custom JSON export via export.cyr)
- tracing instrumentation (deferred)

## [0.92.0] — 2026-04-03

### Added
- **RFC 3161 trusted timestamping** (feature: `timestamping`) — `TimestampRequest` with DER encoding (`to_der()`), `TimestampResponse` with DER decoding (`from_der()`), `TimestampAttestation` for persistent storage; hand-rolled DER encoder/decoder (zero new deps)
- **Merkle root anchoring** (feature: `anchoring`) — `WitnessAnchor` (self-hashed snapshot of Merkle root + chain head), `WitnessReceipt` (backend-specific attestation), `WitnessBackend` trait for pluggable witness systems, `AnchorVerification` enum with `Display`
- **RFC 9162 consistency proofs** — `ConsistencyProof` type, `MerkleTree::consistency_proof(old_size)` generation, `verify_consistency()` verification (RFC 9162 Section 2.1.4.2 algorithm), `MerkleTree::canonical_root(size)` for no-duplication RFC 9162 roots
- **Algorithm-agnostic signing traits** — `EntrySigner` and `EntryVerifier` traits (object-safe, `Send + Sync`), `SignatureAlgorithm` enum (`Ed25519`, `MlDsa65`, `Ed25519MlDsa65`), `EntrySignature::verify_with(&dyn EntryVerifier)` for runtime algorithm dispatch, `EntrySignature::algorithm_parsed()`
- **Integrity proof export** — `IntegrityProof` bundle (signed tree head + entries + inclusion/consistency proofs + optional anchor), `ProofBuilder` with chainable `.with_consistency_from()`, `.with_inclusion()`, `.with_all_inclusions()`, `.with_anchor()`, `ProofVerification` with detailed per-check results, `to_proof_json()` export
- **Chain capacity limits** — `AuditChain::with_capacity(max_entries)` for auto-rotation at limit, `take_overflow()` to retrieve archived overflow
- **Streaming verification** — `AuditStore::verify_streamed(chunk_size)` for O(chunk_size) memory verification, `verify_chain_offset()` for index-adjusted chunk verification
- **Input validation** — `AuditEntry::new_validated()` with configurable field length limits (`MAX_SOURCE_LEN`, `MAX_ACTION_LEN`, `MAX_DETAILS_SIZE`), `LibroError::FieldTooLong` error variant
- **Key zeroization** — `SigningKey` implements `Drop` to overwrite key material; `to_bytes()` returns `Zeroizing<[u8; 32]>`
- `algorithm` field on `EntrySignature` — identifies the signing algorithm (backward-compatible, `Option<String>`, skipped when `None`)
- `SignedTreeHead` type for signed Merkle root commitments
- `LibroError::Timestamp`, `LibroError::Anchoring`, `LibroError::Der` error variants
- Shared hex utilities extracted to `hasher.rs` (`hex_encode`, `hex_encode_slice`, `hex_decode`)
- Benchmarks for consistency proof generation and verification (`merkle_consistency_1000`, `merkle_verify_consistency`)
- 262 tests (up from 168), comprehensive trait assertions for all new types

### Changed
- `timestamping` and `anchoring` feature flags added; `full` feature now includes both
- `hash_field()` promoted to `pub(crate)` for reuse across modules (length-prefixed hashing)
- `constant_time_eq()` promoted to `pub(crate)` for reuse across modules
- All hash comparisons in `verify.rs`, `entry.rs`, `signing.rs`, `merkle.rs` now use constant-time comparison
- `WitnessAnchor::compute_hash()` uses length-prefixed fields (prevents boundary ambiguity)
- Signing module renamed dalek imports to `DalekSigner`/`DalekVerifier` to avoid trait name collisions
- `IntegrityProof::verify_common()` builds Merkle tree once instead of twice

### Fixed
- `kernel_audit.rs`: `read_agnos_audit_events` now passes `&Path` (adapted for agnosys v0.50.0 API)
- `WitnessAnchor::verify_against()` now detects head mismatch on empty chains
- `IntegrityProof` consistency verification compares against canonical RFC 9162 root (not libro's duplication-based root)

## [0.91.0] — 2026-04-02

### Added
- `cargo vet` supply-chain auditing — initialized with trusted publisher imports from Mozilla, Google, Bytecode Alliance, ISRG, and Zcash (119 audited, 54 exempted)
- CI: `cargo vet --locked` enforcement in security job
- CI: `--all-features` on all Linux jobs (check, clippy, test, MSRV, coverage)
- CI: macOS test matrix uses `--features full` to exclude Linux-only `kernel-audit`

### Changed
- Upgraded majra from 0.21.3 to 1.0 (stable release)
- Upgraded rusqlite from 0.34 to 0.39
- Upgraded criterion from 0.5 to 0.8
- License changed from AGPL-3.0-only to GPL-3.0-only (aligns with AGNOS ecosystem)
- `cargo-deny` config: `all-features = true` (was `features = ["full"]`), added `CC0-1.0`, `MIT-0`, `Unlicense`, `LGPL-2.1-or-later` to license allowlist, restored agnosys git source, removed stale entries

### Fixed
- `SqliteStore::len()` adapted for rusqlite 0.39 (`usize` no longer implements `FromSql`)

## [0.90.0] — 2026-04-02

### Added
- **Serde** (`Serialize`/`Deserialize`) on: `ChainArchive`, `ChainReview`, `IntegrityStatus`, `MerkleProof`, `ProofNode`, `Side`, `QueryFilter`, `RetentionPolicy`, `EntrySignature`, `VerifyingKey`
- **`PartialEq`** on: `AuditEntry`, `ChainArchive`, `ChainReview`, `IntegrityStatus`, `MerkleProof`, `ProofNode`, `EntrySignature`, `RetentionPolicy`
- **`Clone`** on: `ChainArchive`, `IntegrityStatus`
- **`#[non_exhaustive]`** on public structs: `ChainArchive`, `ChainReview`, `MerkleProof`, `ProofNode`, `EntrySignature`
- **`#[non_exhaustive]`** on public enums: `EventSeverity`, `Side`, `IntegrityStatus`, `RetentionPolicy`
- **`#[must_use]`** on pure functions: `verify()`, `compute_hash()`, `matches()`, `verify_proof()`, `root()`, `leaf_count()`, `at_or_above()`, `as_str()`, signing key methods
- **`#[inline]`** on hot-path accessors: all `AuditEntry` field accessors, `EventSeverity::as_str()`, `QueryFilter::matches()`, chain size methods, `hash_pair()`
- Re-exported `ProofNode`, `Side`, and `IntegrityStatus` from crate root
- Doc comments on `verify_chain()`, all `LibroError` variants, `SqliteStore` module with usage example
- Custom serde for `RetentionPolicy` — `KeepDuration` serialized as seconds (i64), `KeepAfter` as RFC3339
- Custom serde for `VerifyingKey` — serialized as hex string
- `#[serde(skip_serializing_if = "Option::is_none")]` on `QueryFilter` fields for compact JSON
- Signing and SQLite benchmarks (`sign_entry`, `verify_signature`, `sqlite_append_100`, `sqlite_query_100`)
- **BLAKE3** as default hash backend — 4-10x faster than SHA-256, 128-bit collision resistance, 256-bit output
- `sha256` feature flag for NIST FIPS 180-4 compliance environments
- `hash_algorithm` field on `AuditEntry` — identifies the hash algorithm used, enables verification across algorithm transitions
- `ChainHasher` internal abstraction for pluggable hash backends
- `key_id` field on `EntrySignature` — identifies signing key for key rotation workflows
- `SigningKey::sign_with_key_id()` — sign with an explicit key identifier
- `RetentionPolicy::pci_dss()` — PCI DSS 4.0 Req 10.7 (12 months)
- `RetentionPolicy::hipaa()` — HIPAA 45 CFR 164.530(j) (6 years)
- `RetentionPolicy::sox()` — SOX Section 802 (7 years)
- `RetentionPolicy::gdpr()` — GDPR-aligned with caller-specified purpose duration
- Compliance standards mapping documentation (`docs/compliance/standards-mapping.md`)
- 168 tests, 95%+ coverage (up from 145)

### Changed
- **Breaking:** Default hash algorithm changed from SHA-256 to BLAKE3; use `sha256` feature for SHA-256
- `csv_escape()` uses `Cow<str>` to avoid allocation when no escaping needed
- `abbreviate_hash()` uses `Cow<str>` to avoid allocation for short hashes
- Merkle tree `build()` moves levels instead of cloning — 25% faster build
- Merkle tree pre-allocates `next_level` Vec with capacity
- Signing `hex_encode()` uses `write!` into pre-allocated buffer
- Benchmark script now runs with `--all-features`

### Fixed
- Clippy `needless_borrow` in test code

## [0.22.4] — 2026-03-22

### Added
- `AuditChain::append_with_agent()` — append an entry with agent ID in one call (previously required manual entry construction and `pub(crate)` access)

## [0.22.3] — 2026-03-22

### Added
- `FileStore` — append-only JSON Lines file backend (`file_store` module)
- `SqliteStore` — queryable SQLite backend with indexed columns, behind `sqlite` feature flag
- Chain rotation: `AuditChain::rotate()` returns `ChainArchive`, new entries link to previous head
- `AuditChain::from_entries()` to restore a chain from archived entries
- `SqliteStore::query_by_source()`, `query_by_severity()`, `query_by_agent()` for indexed queries
- `query` module with `QueryFilter` — composable multi-field filtering (source, severity, agent, action, time range)
- `AuditChain::by_agent()` and `AuditChain::query()` methods
- `SqliteStore::query()` — translates `QueryFilter` to indexed SQL WHERE clauses
- `FileStore::query()` — load + filter in memory
- `export` module: `to_jsonl()` and `to_csv()` writing to any `io::Write` target
- `retention` module: `RetentionPolicy` enum (KeepCount, KeepDuration, KeepAfter)
- `AuditChain::apply_retention()` — archive entries outside the retention window
- `EventSeverity::as_str()` — stable string representation for hashing and storage
- `AuditEntry` accessor methods: `id()`, `timestamp()`, `severity()`, `source()`, `action()`, `details()`, `agent_id()`, `prev_hash()`, `hash()`
- Advisory file locking (`flock`) on `FileStore` append and load for concurrent-process safety
- `fs2` dependency for cross-platform file locking
- `review` module: `ChainReview` with integrity status, time range, source/severity/agent distributions
- `AuditChain::review()` — produce a structured chain summary with `Display` for human-readable output
- `Display` impl for `AuditEntry` and `EventSeverity`
- `tracing` instrumentation: append, verify, rotate, retention, store open, parse errors
- `merkle` module: `MerkleTree` with `build()`, `root()`, `proof()`, and `verify_proof()` for O(log N) inclusion proofs
- `signing` module (feature: `signing`): Ed25519 per-entry signatures with `SigningKey`, `VerifyingKey`, `EntrySignature`
- `EventSeverity` now implements `Ord`/`PartialOrd`/`Hash` — variants ordered Debug < Info < Warning < Error < Critical < Security
- `EventSeverity::at_or_above()` — returns all severity levels at or above a given level
- `QueryFilter::min_severity()` — filter to entries with severity >= a given level (SQL `IN(...)` for SqliteStore)
- `AuditChain::append_batch()` — append multiple entries in one call
- `AuditChain::page(offset, limit)` — paginated access to chain entries
- `AuditStore::load_page(offset, limit)` — paginated loading with SQL LIMIT/OFFSET override for SqliteStore
- `AuditStore::load_and_verify()` — convenience that loads and verifies in one call
- `AuditStore::query()` — trait-level query with default load+filter impl; `SqliteStore` overrides with SQL WHERE
- `streaming` module (feature: `streaming`): `AuditStream` for real-time pub/sub via majra with MQTT-style topic wildcards
- 84 tests, 94% line coverage

### Changed
- **Breaking:** `compute_hash` now length-prefixes each variable-length field (little-endian u64) to prevent second-preimage collisions via field boundary shifting. Hashes from previous versions are incompatible.
- **Breaking:** `AuditEntry` fields are now private — use accessor methods instead. Construction still via `AuditEntry::new()` and `.with_agent()`. This prevents accidental mutation that bypasses hash integrity.
- **Breaking:** `compute_hash` now uses `EventSeverity::as_str()` (stable) instead of `Debug` format, and canonical sorted-key JSON for details. Hashes from previous versions are incompatible.
- `AuditChain::verify()` now delegates to `verify_chain()` after genesis check, eliminating duplicated logic
- `AuditChain::apply_retention()` moved from orphan impl in `retention.rs` to `chain.rs`
- CSV export now escapes `agent_id` field (user-provided, may contain commas)
- `AuditEntry::Display` no longer panics on short/empty hash strings
- `FileStore::open` uses atomic `OpenOptions::create(true)` instead of TOCTOU `exists()`+`create()`
- `verify_chain` computes hash once per entry instead of twice on failure
- `rotate()` on empty chain no longer sets `prev_chain_hash` to `Some("")`
- `query()` moved to `AuditStore` trait (polymorphic access via `dyn AuditStore`)
- `AuditStore::load_all` docs now warn that it does not verify integrity
- `RetentionPolicy::apply_retention` avoids double clone via `Vec::split_off`
- Key types re-exported from crate root: `QueryFilter`, `RetentionPolicy`, `to_jsonl`, `to_csv`

### Removed
- `LibroError::EmptyChain` variant (was dead code, never constructed)
- `SqliteStore::query_by_source`, `query_by_severity`, `query_by_agent` — superseded by `SqliteStore::query(&QueryFilter)`
- `tracing` dependency (was listed but never used)

## [0.21.3] — 2026-03-21

### Fixed
- Corrected `EmptyChain` error variant — previously unreachable, now reserved for store-level semantics

### Changed
- Tightened `thiserror` dependency to major version 2

## [0.21.2] — 2026-03-20

### Added
- Criterion benchmarks for `append` and `verify` operations (`benches/chain.rs`)

### Changed
- Improved CI pipeline: added MSRV check (1.89), `cargo-deny` supply-chain audit, codecov integration

## [0.21.1] — 2026-03-19

### Added
- `verify` module — standalone `verify_chain()` function for external audit tools
- Integration tests for full chain lifecycle, tamper detection, and error display

### Fixed
- Genesis entry validation now checks `prev_hash` is empty

## [0.21.0] — 2026-03-18

### Added
- `AuditStore` trait for pluggable persistence backends
- `MemoryStore` — in-memory backend (for testing and ephemeral use)
- `store` module with unit tests

### Changed
- `LibroError` extended with `Store`, `Io`, and `Json` variants for persistence error handling

## [0.20.0] — 2026-03-17

### Added
- `AuditChain` — append-only chain with hash linking, verification, and query methods
- `by_source()` and `by_severity()` query methods on `AuditChain`
- `head_hash()` to retrieve the chain head
- Chain-level tamper detection tests

## [0.19.0] — 2026-03-16

### Added
- `AuditEntry::with_agent()` builder method for optional agent ID tracking
- Serde `Serialize`/`Deserialize` on `AuditEntry` and `EventSeverity`
- Serde roundtrip test

## [0.18.0] — 2026-03-15

### Added
- `AuditEntry` — core audit entry with UUID, timestamp, severity, source, action, JSON details
- `EventSeverity` enum: Debug, Info, Warning, Error, Critical, Security
- SHA-256 hash computation and self-verification (`compute_hash`, `verify`)
- Hash-linked chaining via `prev_hash` field
- `LibroError` with `IntegrityViolation` variant
- Entry creation, tamper detection, and chaining tests

## [0.1.0] — 2026-03-14

### Added
- Initial project scaffolding extracted from daimon agent-runtime audit module
- Cargo workspace setup (edition 2024, MSRV 1.89, AGPL-3.0)
- CI pipeline (`ci.yml`) with fmt, clippy, test, and audit steps
- Release workflow (`release.yml`) with multi-platform builds and crates.io publish
- `Makefile` with standard development targets
- `VERSION` file and `scripts/version-bump.sh`
- README with architecture overview, roadmap, and reference code pointers

[2.0.1-dev]: https://github.com/MacCracken/libro/compare/2.0.0...HEAD
[2.0.0]: https://github.com/MacCracken/libro/compare/1.2.0...2.0.0
[Unreleased]: https://github.com/MacCracken/libro/compare/v0.91.0...HEAD
[0.91.0]: https://github.com/MacCracken/libro/compare/v0.90.0...v0.91.0
[0.90.0]: https://github.com/MacCracken/libro/compare/v0.25.3...v0.90.0
[0.22.4]: https://github.com/MacCracken/libro/compare/v0.22.3...v0.22.4
[0.22.3]: https://github.com/MacCracken/libro/compare/v0.21.3...v0.22.3
[0.21.3]: https://github.com/MacCracken/libro/compare/v0.21.2...v0.21.3
[0.21.2]: https://github.com/MacCracken/libro/compare/v0.21.1...v0.21.2
[0.21.1]: https://github.com/MacCracken/libro/compare/v0.21.0...v0.21.1
[0.21.0]: https://github.com/MacCracken/libro/compare/v0.20.0...v0.21.0
[0.20.0]: https://github.com/MacCracken/libro/compare/v0.19.0...v0.20.0
[0.19.0]: https://github.com/MacCracken/libro/compare/v0.18.0...v0.19.0
[0.18.0]: https://github.com/MacCracken/libro/compare/v0.1.0...v0.18.0
[0.1.0]: https://github.com/MacCracken/libro/releases/tag/v0.1.0
