# Roadmap

## Unreleased — ready to ship (you pick the version)

cc3-debt paydown sprint. All changes landed in the working tree,
255/255 tests green, benches within noise. See
`docs/development/sprint-1.2.0.md` for the full plan + decision log.

**Source changes (already in working tree, not yet tagged):**

- [x] **24 workaround globals removed** — patra_store 12, entry 6, anchoring 3, review 1, chain 1, merkle 1 dead. All were cc3-era workarounds for locals clobbered across nested `str_builder_*` / `hasher_update` call chains; cc5 5.4.2 preserves them reliably.
- [x] **Negative literals + compound assignment sweep** — `(0 - N)` → `-N` (13 sites) and `i = i + 1` → `i += 1` in simple counter loops (~50 sites). Native in Cyrius 3.10.3+.
- [x] **`severity_len(sev)` added** in `src/entry.cyr`; `entry_compute_hash` no longer calls `strlen` on the constant severity cstr every entry.
- [x] **`cyrius.cyml` enriched** — added `repository`, `[deps] stdlib = […]` (13 modules), `[deps.sigil]` (tag 2.8.3), `[deps.patra]` (tag 1.1.1). Matches patra / first-party convention.
- [x] **Sprint 1.2.0 plan captured** in `docs/development/sprint-1.2.0.md` — every idea from the 1.1.x reviews is recorded there so nothing is lost across sessions.

**Decisions recorded (no code change):**

- [x] `secret var` (Cyrius 5.3.5) — NOT APPLICABLE. Libro key material is heap-allocated; `secret var` only zeroises stack-local arrays. `signing_key_zeroize(sk)` already handles heap zeroization.
- [x] `ct_select` / `lib/ct.cyr` (Cyrius 5.3.5) — NO MIGRATION NEEDED. Every security-critical compare already routes through `constant_time_eq_str` → sigil's branchless `ct_eq`. Remaining `str_eq` calls are metadata, not secrets.
- [x] `#derive(accessors)` (Cyrius 3.7.1) — REJECT. Would require `struct` declarations across 18 modules. AGNOS-wide convention (libro, patra, sigil, ark) is raw-offset accessors; consistency + hook-point flexibility outweighs the ~30-line boilerplate saving.

**Validation:** 255 passed, 0 failed. `sign_entry` 6.147 → 5.786 ms (−5.9 %) from the patra_store local refactor; every other bench within ±2 % noise. Fuzz clean.

**Version bump is yours to call** — none of this has touched `VERSION` / `cyrius.cyml` version / CHANGELOG yet.

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

## v1.1 — Hardening (not blocked)

- [ ] Nested JSON canonical hashing (depth > 1)
- [ ] Benchmark history tracking (CSV append per run)
- [ ] Chain export/import (full chain serialization to file)
- [ ] Streaming verification for FileStore
- [ ] `append_batch()` port from Rust

## Blocked — Waiting on Ecosystem

#### Blocked on patra (SQL storage)
- [ ] **Patra SQL backend** — both libro and patra depend on sigil for SHA-256 via stdlib. Once patra drops its bundled SHA-256 and uses sigil from stdlib (same as libro), the conflict resolves and libro can include patra for indexed SQL storage, transactions, and WAL crash recovery.

#### Future
- [ ] Post-quantum signatures (ML-DSA) via sigil
- [ ] Hybrid signing: Ed25519 + PQ for transition period
- [ ] Remote attestation (TPM-backed chain sealing)
- [ ] Multi-node chain sync (federated audit across fleet)
- [ ] Conflict resolution for concurrent appends
- [ ] MCP tools via bote: `libro_query`, `libro_verify`, `libro_export`
