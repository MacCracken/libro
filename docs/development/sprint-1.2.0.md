# Sprint 1.2.0 plan

Scope locked 2026-04-19, after the v1.1.1 quick-win round. Goals:

1. **Pay down the cc3-era debt** now that libro is on Cyrius 5.4.2 (cc5).
2. **Harden crypto material handling** with the `secret var` primitive.
3. **Tighten the hot-path** on severity formatting.
4. **Decide** on the `#derive(accessors)` question without committing code until we're sure.

Every item below is carried over from the 1.1.0/1.1.1 review passes (agent survey +
internal audit) so nothing from that round gets lost across sessions. Items are
listed in execution order — cheap/low-risk first, risky/decision-heavy last.

Landing criteria for the sprint: 255+ tests still green, benches no worse on the
hot paths (`entry_hash`, `chain_append_100`, `chain_verify_100`, `sign_entry`,
`verify_sig`, `export_*`), CHANGELOG + roadmap + VERSION + `cyrius.cyml` in sync.

---

## Item 1 — Severity length table (refactor-B.12)

**Source:** Agent 2's refactor sweep, 1.1.0 review.

**Problem.** `entry_compute_hash` length-prefixes every field with `strlen(severity_as_str(sev))` on every entry hash. Severity names are 5 short constants (`"Debug"`, `"Info"`, `"Warning"`, `"Error"`, `"Critical"`, `"Security"`). `strlen` on them is deterministic but wasted work.

**Fix.** Add `SEV_LEN[]` parallel to the severity enum in `src/entry.cyr` (or wherever `severity_as_str` lives). Replace `strlen(severity_as_str(...))` hot-path calls with a direct `sev_len(sev)` lookup. Keep `severity_as_str` for cstring emission.

**Effort / payoff.** Small / Low — real but tiny on `bench_entry_hash` (already 10 µs); clarity improvement is the main win.

**Test.** Existing entry-hash tests; values must match pre-change hashes (hashing is deterministic, no format change).

---

## Item 2 — Cosmetic sweep: negative literals + compound assignment (language-A.4)

**Source:** Agent 1 survey. Cyrius 3.10.3 introduced both natively; libro still carries the cc3-era workarounds. `CLAUDE.md` was updated in 1.1.1 to note these quirks are obsolete, but the source still uses the old forms.

**Scope.**
- `(0 - N)` → `-N` in literals.
- `i = i + 1` / `i = i - 1` → `i += 1` / `i -= 1` etc., where it's safe (pure counter loops, byte-at-a-time scans).
- Skip places where the RHS has side effects or the existing form documents intent.

**Effort / payoff.** Small / Low — purely cosmetic. Done in one pass via targeted grep + visual review. Runs clean under `cyrfmt --check`.

**Test.** Full suite; behaviour must be byte-identical. Running benches on a before/after binary as a spot-check that codegen didn't regress.

**Risk.** Low. Pure syntax migration. Only caveat: Cyrius's single-pass nature means some compound-assign forms may codegen slightly differently — benches confirm no regression.

---

## Item 3 — `secret var` for Ed25519 signing-key material (language-A.1)

**Source:** Agent 1 survey. Cyrius 5.3.5 added `secret var name[N];` which threads a zeroise body into the function's defer chain.

**Scope.**
- Locate Ed25519 signing-key buffers in libro's `src/signing.cyr` and any 32-byte key material we hold locally (not sigil's own — that's upstream).
- Mark those locals `secret var`.
- Verify the zeroise actually happens (run under `strace` / inspect the generated asm once, or rely on the cyrius `tests/tcyr/secret.tcyr` reference behaviour).

**Effort / payoff.** Small / High — pure defense-in-depth win, no API change, no perf impact on the happy path.

**Test.** Existing signing tests must continue to pass. Optionally add a test that re-reads the stack after signing and asserts zero — but this is brittle; the Cyrius upstream test suite already covers the primitive.

**Blockers.** If sigil holds the private key internally and libro only ever holds a handle, there may be nothing for libro to zeroise. In that case, log the outcome ("no libro-side key material") in the audit file and close out the item.

---

### Item 3 — outcome: NOT APPLICABLE (decided 1.2.0)

Audit findings:

- `src/signing.cyr` allocates Ed25519 key material via `alloc()` — 64-byte secret key, 32-byte public key, 32-byte seed — **all heap**, not stack.
- Cyrius 5.3.5's `secret var` only zeroises **stack-local arrays** (it threads a defer body into the function epilogue). It has no effect on heap buffers.
- `grep 'var .*\[' src/*.cyr` returns zero hits — libro holds no stack arrays anywhere in its source, let alone crypto-sensitive ones.
- Heap zeroization is already handled correctly by `signing_key_zeroize(sk)` in `src/signing.cyr:59`, which `memset(load64(sk), 0, 64)` and `memset(load64(sk + 40), 0, 32)` the sk_bytes and seed buffers.

**Decision.** Close the item as NOT APPLICABLE. No code change. If libro ever refactors `sign_entry` to hold a temporary sk copy on the stack, revisit — that copy would be a valid `secret var` candidate.

---

## Item 4 — `ct_select` / constant-time compare audit (language-A.2)

**Source:** Agent 1 survey. Cyrius 5.3.5 shipped `lib/ct.cyr` with `ct_select(cond, a, b)` (branchless).

**Scope.**
- Check `src/hasher.cyr:constant_time_eq_str` → `ct_eq(str_data(a), str_len(a), str_data(b), str_len(b))` (already constant-time via sigil).
- Grep libro's `src/*.cyr` for any hash / MAC / signature comparison that uses plain `str_eq` or `==` where a constant-time check is required (signature verification paths especially).
- If any are found, migrate to `constant_time_eq_str` or a `ct_select`-based helper.

**Effort / payoff.** Small / Medium — mostly audit; patching is 1-2 sites at most.

**Test.** Existing ct_eq tests. Add a test that exercises the migrated site with deliberately-equal-then-unequal buffers to prove both branches work.

---

### Item 4 — outcome: CLEAN (decided 1.2.0)

Audit of every `str_eq` / `constant_time_eq_str` call in `src/*.cyr`:

**Constant-time already (security-critical compares):** `src/verify.cyr:30,62` (chain linkage), `src/entry.cyr:508` (entry self-verify), `src/signing.cyr:155` (signature hash match), `src/proof.cyr:209` (rebuilt vs signed root), `src/merkle.cyr:223,339,389,390` (Merkle roots), `src/chain.cyr:190` (chain prev-hash), `src/store.cyr:91` (store tail hash), `src/timestamping.cyr:424` (timestamp hash), `src/anchoring.cyr:127,137,148` (anchor).

**Non-CT `str_eq` calls — reviewed and safe:** `src/query.cyr:73,87,93` and `src/chain.cyr:208,240` are metadata filtering (source / action / agent_id). These fields are public log metadata, not secrets. Timing leaks on them do not expose anything that isn't already visible in the emitted log.

**Primitive check:** sigil's `ct_eq` (lib/sigil.cyr) uses the standard XOR-OR accumulator — no early return, no data-dependent branches. Comparable to `ct_select`'s behaviour.

**Decision.** No migration needed. Close the item.

---

## Item 5 — Workaround-global audit (language-A.3 / refactor-B.11)

**Source:** Both agents flagged this; cc3 clobbered locals across function calls, forcing libro to spill intermediate values to globals.

**Known workaround-global sites.**
- `src/patra_store.cyr:76–86` — ten `_ps_*` globals (`_ps_id`, `_ps_ts`, `_ps_sev`, `_ps_src`, `_ps_act`, `_ps_det`, `_ps_aid2`, `_ps_ph`, `_ps_hash`, `_ps_halg`, `_ps_db`, `_ps_sb`). Introduced to survive the nested `str_builder_*` call chain in `patrastore_append`.
- `src/entry.cyr:` — `_cjh_hasher`, `_cjh_pairs`, `_cjh_keys`, `_en_entry`. Similar reason, for canonical-JSON hashing and entry construction.

**Approach — one field at a time.**
1. Convert one global back to a local.
2. Rebuild, run full suite.
3. If green, commit; if not, revert and annotate the global with `# keep global — cc5 still clobbers this across <fn>` so future audits don't re-attempt.

**Effort / payoff.** Medium / Medium — incremental, regression risk per field. Clarity and maintainability win is substantial; perf impact is marginal (globals may actually be faster than stack spills in some cases — measure).

**Test.** Full suite after each conversion. The PatraStore cumulative-state tests are sensitive to this; if any of the 7 PatraStore tests or 12 Gap-coverage tests start failing again, revert immediately.

**Exit condition.** Document in `CLAUDE.md` "Known Cyrius Compiler Quirks" which globals must remain as workarounds under cc5 5.4.2.

---

### Item 5 — outcome: 25 workaround globals removed (landed 1.2.0)

| File                  | Globals removed                                                                                     | Count |
|-----------------------|-----------------------------------------------------------------------------------------------------|-------|
| `src/patra_store.cyr` | `_ps_sb` `_ps_id` `_ps_ts` `_ps_sev` `_ps_src` `_ps_act` `_ps_det` `_ps_aid2` `_ps_ph` `_ps_hash` `_ps_halg` `_ps_db` | 12 |
| `src/entry.cyr`       | `_cjh_hasher` `_cjh_pairs` `_cjh_keys` `_en_entry` `_ech_hasher` `_ech_entry`                       | 6  |
| `src/anchoring.cyr`   | `_anch_ptr` `_ach_hasher` `_ach_anchor`                                                             | 3  |
| `src/review.cyr`      | `_rev_chain`                                                                                        | 1  |
| `src/chain.cyr`       | `_chain_c`                                                                                          | 1  |
| `src/merkle.cyr`      | `_csh_nodes` (dead code)                                                                            | 1  |
| **Total**             |                                                                                                     | **24** |

**Result.** cc5 5.4.2 preserves locals across the `str_builder_*`, `hasher_update`, and `_filestore_*` call chains that used to force globals under cc3. 255/255 tests green after every conversion. Benches within ±2 % noise (sign_entry actually −5.9 %). No regressions on the PatraStore cumulative-state tests — the exact class of failure the globals were originally defending against.

**Globals intentionally kept** (not workarounds):

- **Immutable string constants**: `_csv_header`, `_ps_create_sql`, `_ps_select`, `_ps_count`, `_oid_sha256/384/512`. Globals are the right home for these.
- **Cached scratch buffers** (one-time-alloc, reused): `_ts_buf`, `_fs_buf` + `_fs_buf_size`, `_hf_lebuf`. Performance globals.
- **Multi-return carriers**: `_cd_y/m/d` (civil_from_days), `_der_value_ptr/len` (DER parser). Could be migrated to Cyrius 3.7.2's native `return (a, b)` / `var x, y = fn()` multi-return but that's a larger refactor for a future sprint.
- **Single-function scratch state**: `_fsp_pairs`, `_sp_path`, `_sp_nodes`. Used inside one function (or a recursive helper). Candidates for a future pass but lower-risk to keep.

**Updated CLAUDE.md guidance:** the "Globals for workaround state" principle should be demoted. cc5 5.4.2 is reliable enough that locals are the default; globals are for genuinely shared or long-lived state.

---

## Item 6 — `#derive(accessors)` — decide, don't blindly apply (language-A.5)

**Source:** Agent 1 survey. Cyrius 3.7.1+ supports `#derive(accessors)` to auto-generate field getters/setters.

**Candidate sites.**
- `src/entry.cyr` — roughly 15 hand-written `entry_id`, `entry_timestamp`, `entry_severity`, … getters.
- `src/chain.cyr`, `src/query.cyr`, `src/retention.cyr` — smaller accessor sets.

**Decision to make.** Does replacing hand-written accessors with `#derive(accessors)` actually help libro? Concerns:

- **Pros.** Eliminates ~30 lines of boilerplate. Matches first-party convention.
- **Cons.** Hand-written accessors are a natural hook for validation (e.g. bounds checks, lazy materialization). `#derive(accessors)` may not allow easy interception. Every accessor replaced is a place where we can't inline a sakshi trace, a fast-path cache, or a contract assertion.

**Action.** Research `#derive(accessors)` semantics in Cyrius 5.4.2. Check whether ark, sigil, patra use it. Write a 5-line recommendation (`accept` / `reject` / `partial`) into this file. **Defer the actual code change** until after the decision is documented — this item is a research task, not an edit task.

**Effort / payoff.** Small research / Medium decision weight — the outcome shapes the next sprint.

---

### Item 6 — outcome: REJECT (decided 1.2.0)

`#derive(accessors)` (Cyrius 3.7.1) requires a `struct Name { field1; field2; ... }` declaration to generate `Name_field(p)` / `Name_set_field(p, v)` helpers.

**Survey of first-party code:** `grep '^struct\|#derive' src/*.cyr` returns **zero hits** in libro and zero hits in patra. The AGNOS-wide convention is raw-offset access via `load64(p + OFFSET)` + hand-written accessors (`entry_id(e)`, `patra_result_get_str(rs, row, col)`, etc.). Sigil, ark, nous follow the same pattern.

**Cost of adoption:** adding `struct` declarations to libro's 18 modules and migrating accessors would:

1. Break visual + code-review consistency with every other first-party repo.
2. Give up the hook points where hand-written accessors let us add sakshi tracing, bounds checks, or lazy materialization.
3. Save roughly 30 lines of boilerplate — not worth the refactor cost or the consistency break.

**Decision.** REJECT for libro. Revisit if AGNOS-wide convention changes.

---

## Carry-forward (not in scope for 1.2.0, tracked here so they're not lost)

### Previously-deferred v1.1 backlog items

- [ ] **Nested JSON canonical hashing (depth > 1)** — `src/entry.cyr` comment labels this P-2. Flat JSON is done; nested is deferred pending consumer need.
- [ ] **Benchmark history tracking** — CSV append per `./build/libro_bench` run, like cyrius's `bench-history.csv`.
- [ ] **Chain export/import** — full chain serialization to file (not just append-only JSONL).
- [ ] **Streaming verification for FileStore** — O(chunk) memory verify on large files.
- [ ] **`append_batch()` port from Rust** — batch-append multiple entries in one call.

### Carry-forward from the 1.1.0/1.1.1 agent reviews

- [ ] **FileStore buffer sizing on large files** (refactor-B.6) — `_fs_buf` doubles from 64 KB; on a 100 MB file we allocate several giant buffers and orphan them (bump allocator never frees). Either mmap, pre-allocate at max expected size, or document the max-store-size assumption.
- [ ] **`csv_field` two-pass** (refactor-B.3) — the current `_sb_csv_field` scans once to check if quoting is needed, then again to escape. Collapse into a single pass. Payoff is marginal (CSV exports are already sub-1ms) but clarity improves.
- [ ] **`_entry_to_cstr` caching on repeat calls** (refactor-B.10) — `filestore_open` / `filestore_append` / `filestore_len` each call `_entry_to_cstr(load64(s))` on the same path. Cache the cstr on the store struct.
- [ ] **`chain_verify` / `verify_chain` redundancy** (refactor-B.9) — minor duplication between `src/chain.cyr:chain_verify` and `src/verify.cyr:verify_chain`. Document the reason for the split or merge.

### Ecosystem-blocked

- [ ] **Post-quantum signatures (ML-DSA) via sigil** — blocked on sigil.
- [ ] **Hybrid Ed25519 + PQ signing** — blocked on sigil.
- [ ] **Remote attestation (TPM-backed chain sealing)** — blocked on agnostic-os TPM access.
- [ ] **Multi-node chain sync (federated audit across fleet)** — design work not started.
- [ ] **MCP tools via bote**: `libro_query`, `libro_verify`, `libro_export`.
