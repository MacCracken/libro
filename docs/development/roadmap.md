# Roadmap

## v2.0.0 — 2026-04-19

Major-version sprint. Last stop for 1.x-era backlog before the line is frozen.
Breaking cleanups shelved in 1.x, two deferred Rust-port APIs landed, missing
`dist/libro.cyr` distribution artifact wired per `DEPS-PATTERN.md`, hardening
backlog drained (nested JSON hash, chain export/import, streamed FileStore
verify, bench history).

### Breaking
- [x] `verify_chain(entries)` → `verify_chain(entries, base_index)`; `verify_chain_offset` folded in. `chain_verify(c)` unchanged. Consumers pass `0` for whole-chain verification.
- [x] Canonical JSON hashing rewritten as a recursive byte-walker — depth-unlimited, scalar-aware. 1.x flat canonicalizer quoted every value regardless of type and broke on nested objects/arrays. Flat all-string objects still hash identically; non-string values, arrays, and nested objects now hash differently (and correctly).

### Added
- [x] `chain_append_batch(c, severities, sources, actions, details_vec)` — N entries, one rotation check. Returns vec of created entry pointers. Over-capacity batches tolerated for the call's duration.
- [x] `proof_to_json(ip)` in `src/proof_json.cyr` — ports `to_proof_json()` from Rust. Separate module so `libro_core.bcyr` can exclude it and stay under the cc5 16384 fixup cap.
- [x] `chain_export(c, path)` / `chain_import(path)` in `src/chain_io.cyr` — full-chain JSON Lines serialization. Preserves entries, `prev_chain_hash`, `max_capacity`. Overflow archives intentionally not serialized.
- [x] `filestore_verify_streamed(s, chunk_size)` — byte-streamed verify over a 64KB read buffer, keeps only `chunk_size` entries live at a time. Cross-chunk linkage propagated via tail-hash.
- [x] Bench history CSV via `benches/bench_history.cyr` — `LIBRO_BENCH_HISTORY=<path>` writes one row per bench; `LIBRO_BENCH_TAG` for run labels. No-op when env unset.
- [x] `dist/libro.cyr` (4,488 lines) — produced by `cyrius distlib`, committed. `[lib] modules = […]` in `cyrius.cyml`; CI gates on freshness.
- [x] `_sb_csv_field` quote branch direct-emit — one pre-grow + tight loop vs N `_sb_add_byte` calls.

### Changed / cleanup
- [x] **Toolchain bumped Cyrius 5.4.2 → 5.4.7** for the derive migration below.
- [x] **`#derive(accessors)` across 13 of 15 struct modules** — ~95 hand-written `load64(x + N)` accessors replaced by declarative struct layouts; getters + `_set_` setters generated. Offset-typo class of bug eliminated (including the UUID-zeroing bug caught in the nested-JSON tests). Inline-UUID structs (`entry`, `anchor`, `receipt`) reserve the first 16 bytes with `_uuid_hi`/`_uuid_lo`; their `*_id(x)` pointer-returning accessors stay manual. `merkle.cyr`'s 4 structs kept on manual with a documented cc5 parse-state bug — deterministic, reproduces on both 5.4.2 and 5.4.7, only in `libro_core.bcyr`, flagged upstream.
- [x] FileStore read buffer right-sized via `lseek(fd, 0, SEEK_END)` — one allocation per load.
- [x] `_filestore_cpath` cached on struct (16→24 bytes) — cstr derived once at `filestore_open`.
- [x] `_der_parse_tlv` → multi-return `(total, value_ptr)` — kills `_der_value_ptr` / `_der_value_len` globals. `civil_from_days` stays on globals (5.4.2 multi-return caps at 2).
- [x] `chain_verify` / `verify_chain` layering documented in `src/chain.cyr`.
- [x] `benches/libro_io.bcyr` trimmed — dropped unused `retention.cyr` after nested canonical JSON pushed live fixups back near the 16384 cap.
- [x] Lint clean — 3 pre-existing 120-char-line warnings fixed (SHA-256 test vector + zero-hash prev_hash built via `str_builder`; patra-store CREATE SQL assembled at runtime).

### Validation
- [x] **286 tests, 0 failed** (255 → 286: +4 append_batch, +4 proof_to_json, +5 nested canonical JSON, +9 ChainIO, +5 streamed FileStore verify).
- [x] 22 benches across 2 binaries. Fuzz clean. Simulated-consumer: `dist/libro.cyr` compiles after stdlib + sigil + patra.

### Decisions (no code change)
- [x] `_sb_csv_field` single-pass — REJECTED. Current two-pass form is one cache-hot check + one direct-write escape; a fused version needs optimistic-write-with-memmove or pre-grow-and-reset, neither cleaner.

## v1.2.0 — 2026-04-19

- [x] 24 workaround globals removed — patra_store 12, entry 6, anchoring 3, review 1, chain 1, merkle 1 (dead). cc3-era workarounds for locals clobbered across nested `str_builder_*` / `hasher_update` call chains; cc5 5.4.2 preserves them reliably.
- [x] Negative literals + compound assignment sweep — `(0 - N)` → `-N` (13 sites) and `i = i + 1` → `i += 1` in pure counter loops (~50 sites).
- [x] `severity_len(sev)` added in `src/entry.cyr`; `entry_compute_hash` no longer `strlen`s the severity cstr per entry.
- [x] `cyrius.cyml` enriched — `repository`, `[deps] stdlib = […]` (13 modules), `[deps.sigil]` 2.8.3, `[deps.patra]` 1.1.1.
- [x] **Bench binary split** — single `benches/libro.bcyr` overflowed cc5's 16384 fixup-table cap. Now `libro_core.bcyr` (13 crypto/chain/merkle/sign benches) + `libro_io.bcyr` (8 export/review/anchor/stream/filestore benches); CI iterates `benches/*.bcyr`. Added missing `lib/fmt.cyr` include.
- [x] Roadmap consolidated — `docs/development/sprint-1.2.0.md` folded in and deleted; stale "Blocked on patra" subsection removed.
- [x] Decisions recorded: `secret var` NOT APPLICABLE (heap-only key material), `ct_select` NO MIGRATION (already via sigil's `ct_eq`), `#derive(accessors)` REJECTED (AGNOS raw-offset convention).
- [x] `sign_entry` 6.147 → 5.786 ms (−5.9 %). 255 tests, 0 failed. Fuzz clean.

## v1.1.1 — 2026-04-19

- [x] CI/release modernized (reads toolchain pin from `cyrius.cyml` `cyrius = "..."` field, DCE, semver-only tags, lint + fuzz + bench in CI)
- [x] Manifest `cyrius.toml` → `cyrius.cyml` (first-party convention)
- [x] FileStore UAF across loads (audit Finding 3) fixed + regression test
- [x] CSV/JSON escape per-char alloc eliminated (export jsonl −14.8 %, csv −15.9 %)
- [x] `uuid_format` single-pass (chain_append −5 %)
- [x] CLAUDE.md refreshed — dropped 5 obsolete cc3-era Cyrius quirk notes
- [x] 255 tests, 0 failed

## v1.1.0 — 2026-04-19

- [x] P(-1) scaffold-hardening pass (clean build, lint, bench baseline, audit)
- [x] Cyrius 5.4.2 toolchain pin (cc5)
- [x] Patra bundle refresh — v0.14.0 → v1.1.1 (api-compatible)
- [x] **Fixed use-after-free in `_patrastore_row_to_entry`** — root cause of the long-standing cumulative-state crash in `test_patrastore_append_load`. See `docs/audit/2026-04-19-audit.md` Finding 1.
- [x] Ungated all 7 PatraStore tests
- [x] Ungated all 12 Gap coverage tests
- [x] 251 tests pass (up from 204)

## v1.0.2 — 2026-04-11

- [x] Sigil migration — SHA-256 and Ed25519 from sigil stdlib (dropped `src/sha256.cyr`)
- [x] Real Ed25519 signing via sigil (replaced HMAC-SHA256 placeholder)
- [x] `src/hasher.cyr` delegates to sigil's hex, ct_eq, SHA-256
- [x] 8 fuzz harnesses (SHA-256, hex decode, DER parse, entry create, chain ops, sig verify, JSON parse, topic match)
- [x] 240 tests (up from 202), 21 benchmarks (up from 15)
- [x] Gap coverage: retention KeepDuration/KeepAfter, time-range queries, agent_id query, CSV special chars, entry validation, compliance presets, merkle 16-leaf, stream recv/drain, filestore multi-append
- [x] Cyrius 3.4.0 compatibility verified
- [x] CI/CD updated for Cyrius 3.4.0

## v1.0.1 — 2026-04-09

- [x] FileStore — append-only JSON Lines backend with flock locking
- [x] Cyrius toolchain pinned to cc3 compiler
- [x] 202 tests, 15 benchmarks
- [x] CI/CD updated for cc3

## v1.0.0 — 2026-04-09

- [x] Full Cyrius port from Rust v0.92.0 (8,513 LOC → ~5,000 LOC)
- [x] 19 modules: error, sha256, hasher, entry, verify, query, retention, chain, store, export, review, merkle, signing, anchoring, timestamping, proof, kernel_audit, file_store, streaming
- [x] SHA-256 (FIPS 180-4), length-prefixed field hashing
- [x] Merkle tree — inclusion proofs, RFC 9162 consistency proofs, canonical roots
- [x] Integrity proofs — signed tree heads, inclusion/consistency bundles, anchor support
- [x] Witness anchoring — self-hashing anchors, meta-chain
- [x] RFC 3161 timestamping — DER encoding/decoding
- [x] Streaming pub/sub — MQTT-style topic wildcards
- [x] Kernel audit — AGNOS /proc/agnos/audit interface
- [x] Structured tracing — sakshi instrumentation
- [x] CI/CD — GitHub Actions for build, test, bench, security scan, release

## Rust features NOT yet ported

- [ ] `append_batch()` on AuditChain (batch append multiple entries)
- [ ] `to_proof_json()` export (pretty-print integrity proof as JSON)
- [ ] `SqliteStore` — deferred to patra integration

## Hardening backlog (not blocked)

- [ ] MCP tools via bote (≥ 2.5.1): `libro_query`, `libro_verify`, `libro_export` — bote is no longer blocked. Deferred to post-2.0 (lives in its own repo/PR).

## Blocked — Waiting on Ecosystem

#### Future
- [ ] Post-quantum signatures (ML-DSA) via sigil
- [ ] Hybrid signing: Ed25519 + PQ for transition period
- [ ] Remote attestation (TPM-backed chain sealing)
- [ ] Multi-node chain sync (federated audit across fleet)
- [ ] Conflict resolution for concurrent appends
