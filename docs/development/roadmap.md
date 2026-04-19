# Roadmap

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

- [ ] Nested JSON canonical hashing (depth > 1)
- [ ] Benchmark history tracking (CSV append per run)
- [ ] Chain export/import (full chain serialization to file)
- [ ] Streaming verification for FileStore
- [ ] `append_batch()` port from Rust
- [ ] FileStore large-file buffer sizing — `_fs_buf` doubles from 64 KB; on a 100 MB file we alloc several giant buffers and orphan them (bump allocator never frees). mmap, pre-allocate at max expected size, or document max-store-size assumption.
- [ ] `_sb_csv_field` single-pass — currently scans once to decide if quoting is needed, then again to escape. Payoff marginal (CSV already sub-ms); clarity win.
- [ ] Cache `_entry_to_cstr` on the store struct — `filestore_open` / `filestore_append` / `filestore_len` each re-derive the cstr from the same path.
- [ ] Resolve `chain_verify` / `verify_chain` duplication — minor overlap between `src/chain.cyr` and `src/verify.cyr`. Document the split or merge.

## Blocked — Waiting on Ecosystem

#### Future
- [ ] Post-quantum signatures (ML-DSA) via sigil
- [ ] Hybrid signing: Ed25519 + PQ for transition period
- [ ] Remote attestation (TPM-backed chain sealing)
- [ ] Multi-node chain sync (federated audit across fleet)
- [ ] Conflict resolution for concurrent appends
- [ ] MCP tools via bote: `libro_query`, `libro_verify`, `libro_export`
