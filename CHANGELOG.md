# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [2.8.8] — 2026-08-19 — cyrius 6.5.27 → 6.5.31, patra 1.13.8 → 1.13.9

**518** assertions green (**530** with `tpm`). Binary 813,384 → **821,688 B**
(844,360 B with `tpm`), DCE build, sibling-free at the pinned tags.

### Changed — toolchain pin 6.5.27 → 6.5.31

Picks up the stdlib folds shipped across 6.5.28–6.5.31 (sakshi 2.4.11, patra
1.13.9, yukti 2.3.8, niyama 1.0.7, mabda 4.1.0, ganita 1.1.4, yantra 1.0.3).

### Changed — `[deps.patra]` 1.13.8 → 1.13.9

**This is the half that matters downstream.** 6.5.31 folds patra **1.13.9**, and
`cyrius deps` applies a declared dep's copy *on top of* the `lib sync --full`
snapshot on every resolve. A `[deps.patra]` one patch behind the fold therefore
downgrades `lib/patra.cyr` for every transitive consumer — libro is consumed by
**bote**, which is consumed by **agnosai**, so the stale tag reached three repos
deep.

Measured in a sibling-free tree before this bump: `lib sync --full` landed patra
1.13.9, then `cyrius deps` rewrote it to **1.13.0**. That broke agnosai's CI —
`check-clean.sh` reported `lib: patra.cyr differs from the … snapshot` and the
build lost patra's surface. It is invisible on any machine with a `../patra`
checkout, because `path` wins over `tag`; only CI, which has no siblings,
resolves the tag and takes the downgrade. `deps --verify` does not catch it
either: the lock is regenerated *from* the downgraded file, so it agrees with
itself.

agnosai 2.0.2 carries a defensive `[deps.patra] tag = "1.13.9"` as a shim for
exactly this. With this release that shim can be dropped once bote is rebuilt
against libro 2.8.8.

⚠ **Keep this tag equal to the patra the pinned toolchain folds.** The pairing is
the invariant, not the version number — bumping the cyrius pin without checking
`head -3 ~/.cyrius/versions/<pin>/lib/patra.cyr` reintroduces the same defect.

### Fixed — 6 `src/` files failing `cyrfmt --check`

`src/{chain,main,patra_store,proof_json,signing,tpm_anchor}.cyr` were not
canonically formatted. **Pre-existing, not from the pin bump** — the same 6 fail
identically under 6.5.27 and 6.5.31. They accumulated because the Format check
gate was silently vacuous until it was fixed to loop per-file: `cyrfmt` reads
only `argv[1]` and ignores the rest, so `cyrfmt --check src/*.cyr` only ever
checked `src/anchoring.cyr` and returned 0 while the others drifted.

Swept with `cyrius fmt`; continuation-indent only, `git diff -w` over `src/` is
empty. `dist/libro.cyr` regenerated to match (unchanged at 5,603 lines).

All 20 CI gates verified locally, the seven shell gates extracted verbatim from
`.github/workflows/ci.yml` and run as-is: manifest completeness, three
raw-offset gates, dangerous-pattern scan, required docs, version consistency —
0 failures. Plus fmt, lint (0), DCE build, ELF, 518/530 assertions, fuzz 1/0,
bench 3/0, `distlib --check` current.

## [2.8.7] — 2026-08-18 — patra 1.13.1 → 1.13.8 (the 1.13.x repair arc)

**518** assertions green (**530** with `tpm`), fuzz clean. Binary 800,712 →
**813,384 B** sibling-free at the pinned tags.

### Changed

- **`[deps.patra]` `1.13.1` → `1.13.8`**, picking up patra's seven-release
  repair arc. What libro gains, in the order it matters:

  - **A WAL is now bound to its database** (patra 1.13.8). An orphaned `.wal`
    used to be replayed into whatever file later took that path — a fresh
    database that should hold 1 row could come back holding 30, from a
    *different* database's abandoned transaction. libro's store is exactly the
    shape that meets this: a long-lived `.patra` file that a crash can leave a
    WAL beside, and that an operator may restore over.
  - **Transactions actually hold their lock.** `patrastore_begin` /
    `_commit` / `_rollback` re-export patra's transactions, which until 1.13.3
    released their exclusive flock after the *first* statement — leaving a
    window where another process could commit into the same file mid-transaction
    and a later rollback would write before-images over its committed pages.
  - **The write-ahead log is now write-ahead** (1.13.4): before-images are
    synced before the pages they protect, the header page is WAL-logged (so a
    rollback no longer leaves `TBL_NROWS` stale), and recovery runs under a lock.
  - **`DELETE` no longer desyncs the index from the rows** (1.13.5), and
    **duplicate keys across a leaf split are reachable by index mutations**
    (1.13.6) — both directly relevant to `patrastore_by_source` under
    `patrastore_create_source_index`, where many entries share one `src` key.
  - Memory-safety fixes for malformed `.patra` input, and parser strictness that
    turns silently-truncated statements into errors.

### ⚠ Behaviour change to be aware of

- **A field longer than 255 bytes now fails the append instead of being silently
  truncated.** patra 1.13.6 made an over-long `STR` return `PATRA_ERR_ROWSZ`
  rather than cutting it to 255. This reaches libro through the bind path
  `patrastore_append` already uses — verified: a 300-byte bound value returns 8
  and stores nothing, while 255 still succeeds.

  For a tamper-evident log this is the better failure: the old behaviour hashed
  the full value and stored a truncated one, so the record on disk did not match
  what the chain committed to. But it **is** a change — an entry whose `det`
  (details), `src` or `act` exceeds 255 bytes will now be rejected, and
  `patrastore_append` returns the error and logs `patrastore: insert failed`
  rather than storing a shortened row.

  **`patrastore_append` now bounds all ten fields itself, before binding.** Each
  is checked against patra's 255-byte STR capacity and the offending field is
  named in the log; the append returns `PATRA_ERR_ROWSZ` without touching patra.

  **Deliberately a rejection, not a truncation.** `entry_compute_hash` covers
  the FULL `src` / `act` / `det` / `ts` values, so storing a shortened row would
  put bytes on disk that do not match the hash written beside them — the entry
  would fail verification, and every entry chained after it would be
  unverifiable. Silently truncating is precisely how the pre-2.7 `'`-escaping
  defect corrupted chains: the on-disk record diverging from what the chain
  committed to. Callers that may log long details should truncate deliberately
  at the call site, where the hash is computed over what actually gets stored.

  The bound also makes the guarantee independent of the patra version — built
  against a patra that truncates (≤ 1.13.5), libro now still refuses.

### Verified

- Sibling-free (CI-shape) resolution and build, as the release process requires
  since 2.8.6 — `path = "../patra"` makes the tag inert locally, so a local green
  result is not evidence about CI. Resolved `patra 1.13.8` (commit `0188523`) and
  `sigil 3.12.9`, both commit-pinned.
- Default build 518/518, `--features tpm` build 530/530, fuzz harness clean.

## [2.8.6] — 2026-08-18 — three pins moved, and the sidecar hole's real cause found

512 assertions green (524 with `tpm`), fuzz clean. Binary 800,712 B sibling-free
at the new pins (was 800,696). Lock 112 entries (was 111 under 6.5.20).

### Fixed

- **`dist/libro.deps` was missing its `sakshi` leaf — and 2.8.5's recorded root
  cause was wrong.** The sidecar shipped 26 leaves against 27 declared while
  `dist/libro.cyr` calls `sakshi_*` ~44 times and defines none, so a clean-room
  consumer resolving only from the sidecar was under-declared.

  2.8.5 blamed patra 1.12.12's removed `deps.sakshi` block. **That diagnosis was
  wrong, and being wrong is why the hole survived a release.** The real cause is
  a cyrius bug: `_distlib_named_deps` (`cbt/commands.cyr:2486`) scans the
  manifest for the literal `[deps.` **unanchored**, so it matches inside `#`
  comment prose and adds that name to the "fold, not a stdlib leaf" exclude set.
  libro's own comment block — which discussed `deps.sakshi` at length — is what
  deleted `sakshi` from libro's sidecar. The neighbouring `_distlib_enum_profiles`
  (`:2364`) is line-anchored on purpose and its comment already warned that
  `_distlib_named_deps` is not.

  Proof it is the comments and not the dep graph: **patra shipped the identical
  defect with no git deps at all** — `dist/patra.deps` carried 11 leaves against
  12 declared, missing `sakshi`, unchanged at 1.12.11 / 1.12.12 / 1.13.0 / 1.13.1,
  i.e. straight through the removal boundary that was blamed.

  Fix: never write a bare `[deps.X]` in comment prose; backtick it. Measured —
  libro 26 → 27 leaves, patra 11 → 12, both with `dist/*.cyr` byte-identical.
  Filed upstream against cyrius `distlib`.

- **The CI format gate never checked anything.** `ci.yml` ran
  `cyrfmt --check src/*.cyr`, but `cyrfmt` reads only `argv[1]` and silently
  ignores the rest — so the gate only ever checked `src/anchoring.cyr`, which is
  clean, and returned 0. Proven by reordering: `export.cyr` first exits 1,
  `anchoring.cyr` first exits 0. **Five files were unformatted behind it**
  (`export`, `file_store`, `proof_json`, `signing`, `tpm_anchor`). The gate is
  now a per-file loop, and those five are reformatted — whitespace only,
  `git diff -w` empty, and the DCE binary is byte-identical either side.

### Changed

- **Toolchain pin `6.5.20` → `6.5.27`.** Proven a **zero-byte** change for libro
  by a 2×2 sibling-free build matrix: `{6.5.20, 6.5.27} × {patra 1.13.0, 1.13.1}`
  produces exactly two distinct binaries, factorising purely by the patra tag.
  Of the stdlib files differing across the span libro declares only `fs` and
  `process`, and both diffs are an inert comment banner. The real gains are
  non-codegen: a 6.5.25 deps-resolution fix (`_dep_find_stdlib_dir` used
  `file_exists("src/main.cyr")` as its "am I the cyrius repo?" test, which is
  true for libro, so a warm `./lib` was returned *as* the stdlib), and 6.5.24
  diagnostic line attribution.

- **`[deps.patra]` `1.13.0` → `1.13.1`.** This is the only change in the release
  that moves a byte of the binary. It carries patra's fix for a result buffer
  sized by the whole table rather than the query — up to 41× on indexed equality
  lookups, and flat rather than growing with database size. `patrastore_by_source`
  under `patrastore_create_source_index` is the path that benefits.

- **`[deps.sigil]` / `[deps.sigil_tpm]` `3.12.7` → `3.12.9`.** Security-filed
  upstream (Bellcore verify-after-sign, RSA/bignum de-banking). libro does not
  call the new symbols (`crypto_banks_*`: zero hits in `src/`), but
  `dist/sigil-tpm.cyr` moves, so the `-D LIBRO_TPM` build was verified
  explicitly: **524 passed, 0 failed**.

### Documentation

- `CLAUDE.md` "Current State" was stale at 2.8.3 and **contradicted its own Build
  & Test section** (502/514 vs the real 512/524). Corrected, along with the pin
  (6.4.83 → 6.5.27), patra (1.12.12 → 1.13.1, and 1.11.2 in the tree map), and
  binary/capacity figures (800,712 B, `.bss` 80,280 B, `fn_table` 2240/32768,
  `identifiers` 60336/524288, `var_table` 904/8192).
- **The lock-count note had it backwards.** It called 112 the *polluted* number;
  at 6.5.27 the honest default **is** 112 and a `--features tpm` resolve is 113
  (both measured). Restated as an invariant — tpm is exactly one more than a
  clean full re-sync — so it stops rotting with the stdlib snapshot.
- **Consumer lists named `sigil` and `ark`, neither of which consumes libro.**
  Ten repos actually pin `[deps.libro]`: daimon, aegis, stiva, bote, argonaut,
  phylax, nein, t-ron, kybernet, cyrius-yeomans-descent.
- `docs/guides/testing.md`'s TPM recipe **could not succeed** — it omitted
  `--features tpm` on both `cyrius deps` and `cyrius build`, which fails with
  `undefined variable 'TPM_SHA256'`. Replaced with the verified recipe plus the
  restore step.
- Test counts (502/514 → 512/524) and live dep pins corrected across
  `README`, `CONTRIBUTING`, `quickstart`, `testing`, `tpm-anchors`,
  `dependency-watch`, `threat-model`, and `standards-mapping`. Historical
  release rows and `≥` version floors were deliberately left alone.

### Known issue (not fixed here)

- **`path = "../patra"` / `path = "../sigil"` make the tag inert locally.** A
  sibling checkout fully overrides the git/tag fields and *silently skips
  commit-pin verification* — libro's real `cyrius.lock` has zero `^commit` lines
  where a sibling-free tree has two. Before this release local builds compiled
  patra 1.13.1 + sigil 3.12.9 while CI compiled 1.13.0 + 3.12.7. The tags now
  match what is exercised, but the mechanism remains: **a sibling-free
  reproduction is mandatory on every release**, not optional.

## [2.8.5] — 2026-08-12 — the stale-sakshi overlay is cut at both roots

No behaviour change in libro's own code; 512 assertions green (524 with `tpm`),
`.bss` held at 80,224 bytes. Every change here is a dependency correction.

### Security

- **`[deps.sigil]` / `[deps.sigil_tpm]` 3.12.1 → 3.12.7.** 3.12.1 predates two
  authentication bypasses of the same class: **3.12.5** (PKCS#1 v1.5 signature
  verification) and **3.12.6** (RSA-PSS). ⚠ Both were fixed in `src/bignum.cyr`
  — `bn_mod`'s shared-bank buffers became stack buffers with a self-scrub — not
  in `src/rsa.cyr` as the shape of the CVE suggests. libro's compiled surface
  does not carry `bignum.cyr` or `rsa.cyr`, but three files that it *does* carry
  changed across the range (`bigint_ext.cyr`, `crypto_scratch.cyr`, `mul64.cyr`),
  so this is a real bump rather than a defensive one.

### Fixed

- **`[deps.patra]` 1.12.12 → 1.13.0 — libro was the middle link of a four-level
  defect that silently downgraded `lib/sakshi.cyr` for every consumer.**

  patra 1.12.12's own manifest declared `[deps.sakshi]` at **2.4.2** while the
  toolchain snapshot folds **2.4.10**. `cyrius deps` overlays a git dep's
  resolution ON TOP of the `lib sync --full` snapshot, **recursing through
  sibling manifests**, on **every `cyrius build`** — not only on `deps`:

  ```
  agnosai -> bote -> libro -> [deps.patra] 1.12.12 -> [deps.sakshi] 2.4.2
  ```

  patra **1.13.0** carries zero `[deps.*]` blocks, so the chain terminates.
  bote's defensive `[deps.sakshi]` 2.4.10, which existed only to absorb this,
  is removable as a result.

  ⚠ **Verify after a `cyrius build`, never after the three-step** — the
  three-step ends correct and the next build reverts it. `deps --verify` cannot
  see it either: the lock is written *from disk*, so it records the downgraded
  file's hash. The only signal is an unnamed "1 bundled lib(s) differ" warning.

- **A second stale-sakshi path, not previously identified.** sigil **3.12.1's**
  manifest also declared `[deps.sakshi]`, at **2.4.3**. libro fed the overlay
  through *both* pins; 3.12.7 declares zero `[deps.*]`, closing it.

### Changed

- **Toolchain pinned to cyrius 6.5.20** (was 6.5.10). Its headline fix is a
  **P1**: a `switch`/`match` case body could only be left safely by `return` —
  otherwise a wrong answer with no diagnostic, or a segfault. libro's own `src/`
  contains no statement-position `switch`/`match`, so the exposure was in the
  vendored surface only.

### Known issues

- ⚠ **`dist/libro.deps` no longer declares `sakshi`, and never did on its own
  merits.** The leaf reached the sidecar only through patra 1.12.12's
  `[deps.sakshi]` block — the defect fixed above — so fixing it unmasked a
  latent hole: `dist/libro.cyr` calls `sakshi_*` 44 times and defines none.
  Measured at 27 leaves on 1.12.12, **26** on 1.13.0 and 26 with the block
  removed entirely, so this is not caused by the version chosen. Consumers are
  unaffected today — bote and agnosai each declare `"sakshi"` in their own
  `[deps].stdlib` — but a clean-room consumer relying only on the sidecar would
  fail where 2.8.4 worked. `cyrius distlib`'s self-check cannot catch it:
  undefined *functions* are downgraded to warnings, so only an undefined
  *variable* fails a bundle.
- ⚠ **A bare `cyrius deps` does NOT undo `cyrius deps --features tpm`.** It
  leaves `lib/sigil_tpm_sigil-tpm.cyr` in place and the lock at **112** entries;
  only `rm -rf lib && cyrius lib sync --full && cyrius deps` restores the honest
  **111**. Committing after a TPM build therefore records a lock the default
  build does not use. CLAUDE.md's instruction has been corrected.

## [2.8.4] — 2026-07-28

**A chain that links without retaining.** Additive; no existing behaviour changes.

### Added

- **`chain_new_streaming()`** — a chain that computes and links every entry but
  keeps **none** of them, holding only the head hash in the `prev_hash`
  carry-over slot that `_chain_prev_link` already falls back to (the same slot
  `chain_rotate` uses). Linkage is byte-identical to a retaining chain, so the
  durable chain written to a store verifies exactly as before, and memory is
  O(1) instead of O(events).
- **`entry_free(e)`** — hand an entry's 88-byte struct back to the freelist once
  it has been persisted.
- **`chain_streaming(c)` / `chain_set_streaming(c, v)`** — derived accessors for
  the new `streaming` field on `struct chain`.

### Why

A **write-through** consumer — one that appends an event, hands it straight to a
FileStore, and never reads the chain again — had no way to bound the in-memory
chain. `chain_new()` leaves `max_capacity` at 0, so `_chain_auto_rotate` returns
immediately and rotation never fires; there was no capacity constructor; and
`chain_apply_retention` redistributes into two fresh vecs while the archived
entries stay referenced from `overflow`. libro contains one `fl_free` in ~4,400
lines, none of it on the entry path. So a long-lived writer grew forever.

Found by cyrius-yeomans-descent, whose `audit_event` appends on every login /
save / security event and never reads the chain back.

### Scope — stated plainly

`entry_free` releases the **struct**, not the `Str`s it points at.
`source` / `action` / `details` are caller-owned and never libro's to release;
the ones `entry_new` mints itself (timestamp, hash, algorithm) come from
`str_new`, which allocates through the bump allocator — that memory has no free
at this layer and cannot be reclaimed without an allocator change. So 2.8.4
bounds the entries vec (the unbounded term) and the per-entry struct; a smaller
per-entry `Str` residue remains and needs an allocator-level fix.

On a streaming chain `chain_len` is always 0 and every query / verify function
sees an empty chain. That is the trade, and it is why the mode is opt-in rather
than the default.

### Verification

512 assertions (was 510), 0 failures. The new `test_chain_streaming` checks
linkage **within** each chain — entry N+1 records entry N's hash — rather than
across two chains, since every entry carries a random UUID and its own timestamp
and two chains fed identical inputs legitimately produce different hashes.
Mutation-verified: making `_chain_retain` always push fails 3 assertions;
dropping the prev-hash carry fails 2.

## [2.8.3] — 2026-07-28

**Toolchain pin → 6.4.83 (17-release jump); deps already at latest.** A pure toolchain refresh —
**zero `src/*.cyr` changes**, zero manifest-semantics changes, zero new `[deps] stdlib` entries.
`sigil 3.12.1` and `patra 1.12.12` were re-confirmed as the newest published tags (GitHub API +
local tag list), so the dep pins are unchanged from 2.8.2; only the compiler and its bundled stdlib
snapshot move. The 6.4.65 `thread_local_alloc` floor that coupled the 2.8.2 dep and toolchain bumps
(both sigil and patra call it) stays satisfied by definition at 6.4.83.

### Changed
- **Toolchain pin `6.4.66` → `6.4.83`** (`cyrius.cyml`). `rm -rf ./lib && cyrius deps` re-vendored
  the stdlib snapshot. Every module still lints with **0 warnings**; `cyrfmt --check` clean.
- `dist/libro.cyr` regenerated — **only the version header line changed** (5561 lines, unchanged
  otherwise; `dist/libro.deps` unchanged at 22 stdlib leaves). A simulated downstream consumer built
  outside the repo against `dist/libro.cyr` compiles and runs clean.

### Notes — what the 17-release delta actually carries for libro
- **The `_cfo` constant-fold rewind class (6.4.74 / 6.4.80 / 6.4.81) is the reason to take this
  bump.** 6.4.80 fixed a **CRITICAL** silent-wrong-value defect in the PEXPR tier (`+ - & | ^`):
  a constant expression whose literal subtraction produced a *negative* intermediate silently
  discarded its left operand — `1 - 2 + 3` evaluated to `5`, and 40 of 400 systematic 3-term
  expressions were wrong at 6.4.79. Non-negative intermediates were always correct, which is why it
  hid. **libro was not affected**: a comment- and string-stripped scan of `src/*.cyr`,
  `benches/*.bcyr` and `fuzz/*.fcyr` finds **zero expressions of the failing shape**, and the suite
  reports an identical 502/514 before and after the bump. Recorded here because the defect was live
  at libro's *previous* 6.4.66 pin, so "our tests passed" was not by itself evidence of correctness.
- **6.4.75 (P0) does not reach libro either.** `fn_table` growth past 8192 silently corrupted six
  fn-indexed side tables, and the DCE `live[]` reachability bitmap cleared only 1/4 of its declared
  size — leaving bits 8192..32767 in uninitialised stack, so unreachable-fn counts were
  non-deterministic across identical runs. libro builds with `CYRIUS_DCE=1`, but `CYRIUS_STATS=1`
  puts it at **`fn_table: 2167 / 32768`** — an order of magnitude below the threshold, so neither
  half was ever in range. (6.4.76 separately raised the identifier pool 256 KB → 512 KB; libro sits
  at 58,749.)
- **CVE-32/33/34 (6.4.81) are compile-time, not runtime.** Three unbounded copies in cycc's own
  preprocessor / `READFILE` path, reachable from untrusted *source text*. They harden the compiler
  against hostile `.cyr` input; libro's runtime surface is untouched.
- 6.4.69's f64 JSON round-trip work does **not** apply — libro emits JSON by hand-rolled string
  building (`_sb_json_escape`) and uses no `f64` anywhere in `src/`.
- Tooling fixes worth knowing: `cyrius audit` no longer fmt/lint/doc-walks the consumer's vendored
  `lib/` and now names failing files (6.4.78); `cyrius audit` / `cyrius capacity` no longer compile
  project sources with no stdlib includes, and `capacity --check` is no longer a green placebo
  (6.4.73); `cyrius lib sync` refuses to run inside the cyrius source repo (6.4.77). 6.4.67 added
  chrono `DateTime` / `Duration` / `format()` — additive, libro's existing chrono use is unchanged.

### Notes — measurements
- Default `build/libro` `749 KB → 775 KB` (775,256 B), `-D LIBRO_TPM` `776 KB → 802 KB` (802,024 B).
  The ~26 KB growth on both is 6.4.83 codegen against the newer stdlib snapshot, not a surface
  change — `.bss` stays thin at **80,152 B**, so the 2.8.0 sigil-thinning holds.
- Suite **502 / 514** (default / `-D LIBRO_TPM`) pass, **33** benches run across the three binaries,
  fuzz clean across **12** targets. The `-D LIBRO_TPM` path still needs *both* `cyrius deps
  --features tpm` and `cyrius build --features tpm -D LIBRO_TPM`; the benign `duplicate fn
  '_sigil_random_fill'` warning on that path is expected (sigil-tpm and sigil-mldsa both carry
  random).
- The `sakshi 2.4.3 (pinned: 2.4.6)` shadow warning from `cyrius deps` is pre-existing and benign
  (patra vendors `lib/sakshi.cyr` at 2.4.3); sakshi is a `dist/libro.deps` stdlib leaf, not inlined.

## [2.8.2] — 2026-07-17

**Toolchain pin → 6.4.66 + dep refresh (sigil 3.12.1, patra 1.12.12) + `ERR_*` → `LIBRO_ERR_*`
enum namespacing.** Moves onto the latest cyrius 6.4.x and the latest crypto/storage dep tags, and
clears the 6.4.x error-enum namespace lint note (quirk #8) by prefixing libro's `LibroErr`
constants. The dep and toolchain bumps are **coupled**: sigil 3.12.1 and patra 1.12.12 both call the
stdlib `thread_local_alloc`, which first ships in cyrius **6.4.63** — building the new deps against
the 6.4.62 snapshot fails with `undefined function 'thread_local_alloc'`, so the two move together.

### Changed
- **Toolchain pin `6.4.62` → `6.4.66`** (`cyrius.cyml`). `rm -rf ./lib && cyrius deps` re-vendored
  the stdlib snapshot (which now carries `thread_local_alloc` in `lib/thread_local.cyr`).
- **`sigil` `3.11.1` → `3.12.1`**, **`patra` `1.12.9` → `1.12.12`** (`[deps.sigil]`,
  `[deps.sigil_tpm]`, and `[deps.patra]` tags). Both build clean under 6.4.66. No new `[deps] stdlib`
  entries required. The thin sigil surface (`dist/sigil-mldsa.cyr` + `src/{sha_ni,sha256,hex}.cyr`;
  TPM behind the optional `tpm` feature) is structurally unchanged from 2.8.0.
- **Error enum namespaced: bare `ERR_*` → `LIBRO_ERR_*`** across `src/error.cyr` (the `LibroErr`
  enum definition + its two internal constructors), `src/kernel_audit.cyr`, and `src/main.cyr` (test
  call sites) — 19 references in all. Clears the informational 6.4.x lint note on `src/error.cyr:5-6`
  (proposal `2026-07-11-error-enum-namespace-lint-gate`) proactively, before it can graduate to an
  enforced error. patra's `PATRA_ERR_*` constants are a separate namespace and are left untouched
  (a word-boundary rename guarantees `PATRA_ERR_*`, preceded by `_`, is never matched). No CI change:
  the raw-offset allowlist entry for `src/error.cyr` names the `error` **struct**'s param (`e`), not
  the enum constants, so the rename doesn't touch it — `src/error.cyr` now lints with 0 warnings.

### Notes
- Default `build/libro` `724 KB → 749 KB`, `-D LIBRO_TPM` `751 KB → 776 KB` — the ~25 KB growth is
  6.4.66 codegen + the newer deps, not a surface change; `.bss` stays thin (the 2.8.0 sigil-thinning
  holds). Suite **502 / 514** (default / TPM) pass, 33 benches run, fuzz clean.
- `dist/libro.cyr` regenerated (5561 lines, v2.8.2 — now emits `LIBRO_ERR_*`; `dist/libro.deps`
  unchanged at 22 stdlib leaves).
- The `sakshi 2.4.3 (pinned: 2.4.6)` shadow warning from `cyrius deps` is pre-existing and benign
  (patra vendors `lib/sakshi.cyr` at 2.4.3); sakshi is a `dist/libro.deps` stdlib leaf, not inlined
  into the bundle, so it doesn't affect distribution — build + full suite pass regardless.

## [2.8.1] — 2026-07-13

**`patrastore_append` now writes audit rows with a bound (prepared-statement)
INSERT — a single quote in any field no longer drops the record (argonaut P1).**
The patra-backed audit store built each row by raw SQL string interpolation
(`INSERT INTO audit_entries VALUES ('…')`), so a `'` in a service name, action, or
detail produced malformed SQL → `PATRA_ERR_SYNTAX` → the entry was **silently
dropped**, diverging the on-disk chain from the in-memory one. The audit chain is
PID 1's tamper-evidence surface (argonaut), so a value that can't round-trip is an
integrity hole, not a nit.

### Fixed
- **`patrastore_append` (`src/patra_store.cyr`) uses `patra_prepare` +
  `patra_bind_text` ×10 + `patra_exec_prepared` + `patra_finalize`.** Each of the ten
  columns (`id, ts, sev, src, act, det, aid, ph, hash, halg`) binds as `(ptr, len)`
  and is stored as bytes — never reparsed as SQL — so quotes and any metacharacters
  round-trip verbatim with no escaping. Works on patra **1.12.9+** (the bind API
  predates the fix). Regression: `tests/patra.cyr` gains `test_ps_quote` — an entry
  with quotes in service/action/detail persists and reloads (**8** assertions, was 5).

### Notes
- patra **1.12.10** independently fixed the SQL-string path too (standard `''`
  escaping + `patra_quote_str`), so libro's former raw INSERT would also round-trip
  there; the bound path is the durable, escaping-free fix and is preferred
  regardless of patra version.
- No dep or toolchain change (cyrius 6.4.62, patra 1.12.9, sigil 3.11.1 unchanged).
  `dist/libro.cyr` regenerated (5496 lines).

## [2.8.0] — 2026-07-13

**Toolchain pin → 6.4.62 + dep refresh (sigil 3.11.1, patra 1.12.9) + THIN the sigil surface:
`build/libro` 14 MB → 724 KB.** Adopts the cyrius 6.4 line, moves the crypto/storage deps to
their latest tags, and — the headline — stops pulling the monolithic `dist/sigil.cyr`. libro's
actual sigil surface is SHA-256 + Ed25519 + ML-DSA (+ hybrid) + hex; the full bundle also inlines
the x509/RSA/authenticode path, whose bignum tables carry **~13 MB of static `.bss`** libro never
touches. Because cyrius **auto-includes every active `[deps.*]` module** into the compilation, that
~13 MB landed in every libro binary (and every consumer's). Switching `[deps.sigil]` to the
capability sub-bundles that cover libro's surface drops `build/libro` **~14 MB → ~724 KB** (`.bss`
13,046,672 → 79,152 B); the three benches and the fuzz binary fall the same way (~14 MB → ~0.6 MB),
and downstream consumers inherit it transitively (a libro-consuming binary drops to ~640 KB).

### Changed
- **Toolchain pin `6.3.31` → `6.4.62`** (`cyrius.cyml`). `rm -rf ./lib && cyrius deps` re-vendored
  the stdlib snapshot.
- **`sigil` `3.9.8` → `3.11.1`**, **`patra` `1.12.7` → `1.12.9`** (`[deps.*]` tags; both build clean
  under 6.4.62). No new `[deps] stdlib` entries required.
- **Thin sigil surface (`[deps.sigil]`): `dist/sigil.cyr` → `dist/sigil-mldsa.cyr` +
  `src/sha_ni.cyr` + `src/sha256.cyr` + `src/hex.cyr`.** `sigil-mldsa` carries Ed25519 + ML-DSA +
  hybrid + SHA-512 + `crypto_scratch`; the three standalone modules add SHA-256 and hex (which
  `mldsa` lacks). `sha_ni` precedes `sha256` so the latter's `#ifndef`-guarded `sha_ni` include
  self-skips; `hex.cyr` is standalone (no bignum/x509 deps). `ct_eq*` already come from stdlib
  `lib/ct.cyr`, not sigil. Verified this exact set covers all 16 sigil symbols libro calls, with
  **zero duplicate-symbol warnings**.
- **TPM sigil surface moved behind an optional `tpm` feature** (cyrius ≥ 6.3.1 `optional` +
  `[features]`). `tpm_seal`/`tpm_unseal`/`tpm_detect` live in a new **optional** `[deps.sigil_tpm]`
  fold (`dist/sigil-tpm.cyr`), activated only by `--features tpm` for the `-D LIBRO_TPM` build — so
  the thin default build never links tpm code or `tpm2` strings. Build the TPM variant with
  `cyrius deps --features tpm && cyrius build --features tpm -D LIBRO_TPM src/main.cyr build/libro_tpm`
  (one benign `duplicate fn '_sigil_random_fill' (last wins)` warning there — sigil-tpm and
  sigil-mldsa both carry `random`). CI's LIBRO_TPM step updated accordingly.
- **`src/main.cyr` no longer explicitly `include`s the sigil folds** — they arrive via the
  `[deps.sigil]` auto-include (manifest order puts stdlib `thread_local` before sigil, so the
  crypto_scratch-over-TLS SIGILL rule still holds). This is deliberate: it keeps `cyrius distlib`
  from mis-listing the sigil sub-modules as **stdlib leaves** in `dist/libro.deps`. distlib only
  recognizes a named-dep fold when the module basename equals the dep name (`sigil.cyr` ↔ `sigil`);
  the multi-module thin set defeats that, and an explicit include would have written
  `sigil-mldsa`/`sigil_sha256`/… into the sidecar — a downstream `cyrius deps` then errors
  *"dep libro requires 'sigil-mldsa' but it is not in the cyrius stdlib"*. With no explicit include
  the sidecar stays stdlib-only and sigil resolves as a named dep (verified: consumer `cyrius deps`
  + compile both succeed). The benches/fuzz/standalone repros keep explicit thin includes.

### Notes
- `dist/libro.cyr` is version-restamped only (no bundled-source delta); `dist/libro.deps` unchanged
  at 22 stdlib leaves.
- **New advisory lint note (informational, not enforced).** cyrius 6.4.x's lint proposes leaf libs
  prefix their error enum (`LIBRO_ERR_*`) instead of bare `ERR_*` to avoid the flat enum-const
  collision reserved for the sakshi base logger (proposal
  `2026-07-11-error-enum-namespace-lint-gate`). It surfaces as a `note` on `src/error.cyr` — CI's
  lint step is non-fatal (`cyrius lint … || true`) and the suite is unaffected. The enum-namespace
  rename is a separate concern, not part of this bump (CLAUDE.md quirk #8).
- Verified end-to-end: default build **502/502** (724 KB), `--features tpm -D LIBRO_TPM` **514/514**,
  all three benches + fuzz build-and-run clean (~0.6 MB each), `cyrfmt --check` clean, a downstream
  `cyrius deps` resolves clean, and a simulated-consumer compile (chain create + append) exits 0.

## [2.7.10] — 2026-07-02

**Toolchain pin → 6.3.31 — official freelist agnos-mmap fix.** Pins cyrius `6.3.15` → `6.3.31`
and re-syncs the vendored stdlib, landing the upstream fix for the freelist allocator's agnos
mmap ABI. Before it, `fl_alloc` (the audit chain's allocator, reached via `chain_new` /
`filestore_open`) issued the Linux 6-arg `mmap(addr, len, …)` with the **length in arg2** — but
agnos `mmap#27` reads the length from **arg1** (`kernel/core/syscall.cyr`: `sys_mmap(arg1)`) →
`mmap(0)` → 0 (MAP_FAILED) → the next store SIGSEGV'd. This crashed **libro's audit chain on
agnos** on first use. Surfaced running `cyrius-yeomans-descent` under mirshi (descent's
`persist_init` → `chain_new()` → `fl_alloc(32)`); root-caused + fixed upstream in cyrius 6.3.31
(issue `2026-07-02-freelist-agnos-mmap-abi`).

### Changed
- **Toolchain pin `6.3.15` → `6.3.31`** (`cyrius.cyml`) + `cyrius lib sync` re-vendored the
  stdlib. The synced `lib/freelist.cyr` now dispatches its mmap by target (a `_fl_mmap` helper:
  single-arg `syscall(SYS_MMAP, length)` under `CYRIUS_TARGET_AGNOS`, the Linux/macOS 6-arg form
  otherwise), matching the file's existing `_fl_map_flags` pattern. Non-agnos targets unchanged.

### Notes
- The fix is now the **official stdlib freelist** (byte-identical to `6.3.31`'s), not a local
  hand-patch — it survives future `cyrius lib sync`, and benefits **every** agnos `fl_alloc`
  consumer (libro, sigil), not just descent.
- No `dist/libro.cyr` content change — `freelist` is a stdlib dep, not a bundled `[lib]` module;
  the dist regen only restamps the version.
- Verified: builds host + `--agnos`, full battery **502/502**, and descent's audit chain
  (`persist: player saves + audit chain ready`) runs under mirshi.

## [2.7.9] — 2026-06-30

**Toolchain pin → 6.3.15 + dep refresh** (base-stack agnos-readiness migration,
tier 2). No source change — libro's own code is already agnos-clean and
buffer-safe (the ecosystem `var X[N]` stack-local audit found zero own-code
overflows). The `file_store.cyr` append path (`file_append_locked`) is handled
by the **stdlib** on agnos (6.3.15 `io.cyr` uses an explicit `SEEK_END`+write
under the `LOCK_EX` hold since the agnos kernel doesn't honor `AO_APPEND` yet),
so no libro-side guard is needed. Verified: `src/main.cyr` builds clean on both
host and `--agnos`, and the full test battery passes **502/502**.

### Changed
- Toolchain pin `6.2.48` → `6.3.15`.
- `sigil` dep `3.9.5` → `3.9.8`, `patra` dep `1.12.3` → `1.12.7` (both migrated
  to 6.3.15; `path=` added for local sibling resolution alongside the tags).
- `[deps] stdlib` += `atomic`, `sync` — required by the patra 1.12.7 dist
  (its lock-free queue + mutex primitives); `cyrius deps` errors
  "cannot read ./lib/sync.cyr" without them.

## [2.7.8] — 2026-06-27

**Ship the `dist/libro.deps` dependency sidecar + adopt the cyrius 6.2.48 toolchain.**
`cyrius distlib` (≥6.2.48) now emits `dist/libro.deps` alongside `dist/libro.cyr` — a
sidecar listing the 22 stdlib `lib/` leaves the libro fold needs in scope
(ct/keccak/random/thread/thread_local/fs/process/hashmap/slice/bayan/… captured from
the `src/main.cyr` umbrella; the `sigil`/`patra` named-dep folds are excluded, since
they resolve transitively). A consumer (e.g. descent) can now declare just
`[deps.libro]` and `cyrius deps` auto-resolves the whole crypto/store stdlib surface in
topological order, with **no hand-ordered list and no omit-one→SIGILL trap**. Toolchain
pin 6.2.11 → **6.2.48**; sigil pin 3.9.0 → **3.9.5** (picks up the certpin `run_capture`
5-arg fix + sigil's own sidecar). `dist/libro.cyr` itself is unchanged — the sidecar is
purely additive.

## [2.7.7] - 2026-06-21

**Dep-pin bump: patra 1.11.2 → 1.12.3 (completes the agnos M6 chain).** libro 2.7.6
shipped its own AGNOS syscall-ABI fixes but still pinned the pre-agnos patra
(1.11.2), whose raw `syscall(SYS_FUTEX)` / `syscall(201)` mis-dispatch on the AGNOS
ring-3 target. patra 1.12.3 moved its mutex onto the `mutex_lock`/`mutex_unlock`
abstraction (a no-op on single-core agnos — no `SYS_FUTEX` referenced) and routed
the WAL salt timestamp to `time_unix`#46. Bumping the pin propagates that fix to
libro consumers — descent's `--agnos` build now resolves patra 1.12.3 and compiles
the full chain clean. No libro source change; Linux behavior byte-identical.

### Changed
- **`cyrius.cyml` `[deps.patra]` 1.11.2 → 1.12.3.**

## [2.7.6] - 2026-06-21

**AGNOS syscall-ABI correctness — entropy / wall-clock / file-seek.** libro's
audit-chain primitives hardcoded Linux x86_64 syscall numbers that mis-dispatch
on the AGNOS ring-3 target (agnos reuses ABI numbers — e.g. #8 is `dup`, not
`lseek`), so `cyrius build --agnos` would silently misbehave at runtime.
Source-only; Linux behavior byte-identical (symbolic forms resolve to the same
Linux numbers via the syscall peer).

### Fixed

- **`src/entry.cyr` `uuid_v4` — entropy under `#ifdef CYRIUS_TARGET_AGNOS`.** agnos
  has no `/dev/urandom`; the UUID-v4 seed now draws from `getrandom` #45
  (RDRAND-seeded), same fail-closed contract as the Linux path. The raw
  `syscall(1)` / `syscall(60)` fail-closed write+exit became symbolic `SYS_WRITE` /
  `SYS_EXIT` (correct on both targets — agnos `exit` is #0, not #60).
- **`src/entry.cyr` `get_epoch_secs` — wall-clock under `#ifdef CYRIUS_TARGET_AGNOS`.**
  agnos `time_unix` #46 returns unix seconds in rax (no timespec); Linux keeps
  `clock_gettime` #228.
- **`src/chain_io.cyr` + `src/file_store.cyr` — file size/seek via symbolic
  `SYS_LSEEK`.** Three raw `syscall(8, …)` calls (`_fs_file_size` + two SEEK_SET
  rewinds) were Linux `lseek`, but **#8 is `dup` on agnos** → a dup'd fd returned
  as a byte count, silently corrupting every FileStore size read. Now
  `syscall(SYS_LSEEK, …)` (Linux #8 / agnos #58).

## [2.7.5] - 2026-06-19

**Re-sourced the TPM primitives from sigil (agnosys → agnodrm decomposition).**
libro's only agnosys surface was the TPM primitives (`tpm_seal` / `tpm_unseal` /
`tpm_detect`) used by `src/tpm_anchor.cyr`. The agnosys → agnodrm decomposition
moved the trust stack into **sigil**, which promoted it to first-class in
`dist/sigil.cyr` at 3.9.0.

### Changed
- **Dropped `[deps.agnosys]`**; bumped `[deps.sigil]` **3.7.14 → 3.9.0** (now the
  TPM provider). Removed `include "lib/agnosys.cyr"` from **every harness** that
  carried it — `src/main.cyr`, `fuzz/fuzz_libro.fcyr`, and the three benches
  (`benches/libro_{core,proof,io}.bcyr`). `tpm_anchor.cyr`'s `tpm_seal` /
  `tpm_unseal` / `tpm_detect` calls now resolve from `lib/sigil.cyr` (same symbol
  names, unchanged). No library logic changed. (Those stale agnosys includes were
  no-op duplicates that, against sigil 3.9.0's now-bundled trust/error stack,
  produced last-wins symbol clashes — `luks_config_name`, `syserr_*` — and a
  crash in `fuzz_canonical_json_hash`; dropping them clears both.)
- Verified: `cyrius deps` clean (sigil 3.9.0 vendored, stale `lib/agnosys.cyr`
  pruned), `cyrius build -D LIBRO_TPM` OK, all 7 fuzz harnesses pass (`fuzz_libro`
  no crashes), and the benches build + run clean.

## [2.7.4] - 2026-06-15

**Toolchain + dependency refresh.** Cyrius pin 6.1.35 → 6.2.11;
sigil 3.7.10 → 3.7.14; patra 1.11.0 → 1.11.2; agnosys 1.4.1 → 1.4.3.
No library *logic* changed — only pins, plus a required
`thread_local` include fix in the bench/fuzz/repro harnesses (see
Fixed). libro compiles clean (default + `-D LIBRO_TPM`), lints and
formats clean, full suite passes (**502 default / 514 TPM**), and all
33 benches + the fuzz harness build *and run* clean on the 6.2.11
stack. The regenerated `dist/libro.cyr` is byte-identical to 2.7.3
apart from the version header — the `src/` library modules did not
move.

### Changed

- **Cyrius pin**: `cyrius.cyml` `cyrius = "6.2.11"` (was 6.1.35).
- **Sigil pin**: `[deps.sigil]` `tag = "3.7.14"` (was 3.7.10).
- **Patra pin**: `[deps.patra]` `tag = "1.11.2"` (was 1.11.0).
- **Agnosys pin**: `[deps.agnosys]` `tag = "1.4.3"` (was 1.4.1).

### Fixed

- **SIGILL in the bench + fuzz harnesses (CI-breaking).** The three
  bench binaries (`benches/libro_*.bcyr`) and the fuzz harness
  (`fuzz/fuzz_libro.fcyr`) included `lib/sigil.cyr` **without**
  `lib/thread_local.cyr` before it. `src/main.cyr` got that include in
  2.7.1 (sigil 3.6.0), but the standalone harnesses were never updated —
  a latent landmine that stayed quiet only because sigil 3.7.10's
  `crypto_scratch` didn't exercise the TLS path those harnesses hit.
  sigil 3.7.14 does, so every harness linked clean but **SIGILL'd at
  first crypto use (exit 132, `Illegal instruction`)** — CI's
  `for f in benches/*.bcyr` / `fuzz/*.fcyr` run-step core-dumped. Added
  `include "lib/thread_local.cyr"` (immediately after `lib/thread.cyr`,
  matching main.cyr) to all three benches and the fuzz harness; also to
  the three sigil-using standalone repros under `tests/`
  (`patra.cyr`, `patra_standalone.cyr`, `fixup_limit_repro.cyr`), which
  CI does not run but would SIGILL if invoked. All now run exit 0. This
  is the exact failure mode CLAUDE.md's BIG NOTE warns about — a
  build-only check misses it; the harness must be *run*.

### Notes

- **New benign diagnostic under 6.2.11.** The 6.2.11 linker now warns
  on duplicate global symbols (`duplicate symbol 'ERR_UNKNOWN' / 'ERR_IO'
  redefined with conflicting value (last definition wins)` at
  `lib/agnosys.cyr:82-83`). This is a *pre-existing* name collision
  between libro's `src/error.cyr` error enum and agnosys's ported error
  enum — both define `ERR_IO` / `ERR_UNKNOWN` as flat globals. The
  collision is harmless (each name resolves consistently within the
  build; all 502/514 tests pass) and was simply silent before 6.2.11
  began diagnosing it. Not a regression from the agnosys bump.

## [2.7.3] - 2026-06-11

**Toolchain refresh + stdlib `bayan` carve migration.** Cyrius pin
6.1.23 → 6.1.35; sigil 3.7.8 → 3.7.10 (**required** by the new
toolchain). No source *logic* changed — only the stdlib module names
libro `include`s. libro compiles clean (default + `-D LIBRO_TPM`),
lints and formats clean, full suite passes (**502 default / 514 TPM**)
and all 33 benches + the fuzz harness run clean on the 6.1.35 stack.

### Changed

- **Cyrius pin**: `cyrius.cyml` `cyrius = "6.1.35"` (was 6.1.23).
- **Sigil pin**: `[deps.sigil]` `tag = "3.7.10"` (was 3.7.8) —
  **required, not cosmetic.** cyrius 6.1.35 **hard-errors on a missing
  `include`** (previously a soft skip). sigil's `dist/sigil.cyr`
  inlines the SHA-NI / AES-NI modules *and* retains an opt-in
  `include "src/sha_ni.cyr"` / `include "src/aes_ni.cyr"` — the
  intended path for source-tree consumers that pull in only
  `src/sha256.cyr` and need the hardware-dispatch infrastructure. In
  3.7.8 those opt-in includes were unguarded, so in the bundle (where
  the file is absent from the fold) 6.1.35 aborts the build
  (`cannot open include file: src/sha_ni.cyr`). sigil 3.7.10 wraps
  them in `#ifndef _SIGIL_SHA_NI_INCLUDED` (and the aes_ni equivalent)
  and `#define`s the marker where the bundle inlines the module, so
  the redundant include self-skips — the proper fix for the dual
  consumption model. libro's sigil surface (SHA-256, Ed25519, HMAC,
  `ct_eq_bytes_lens`, hex) is unchanged.
- **stdlib `json` + `bigint` → `bayan` carve (cyrius 6.1.25).** The
  6.1.x line consolidated the former `json`/`bigint`/`base64`/`csv`/
  `toml`/`cyml`/`u128` stdlib modules into one bundled `bayan` dist;
  the standalone `lib/json.cyr` / `lib/bigint.cyr` no longer ship in
  the 6.1.35 snapshot. Migrated `[deps] stdlib` (`"json"`, `"bigint"`
  → `"bayan"`) and every `include "lib/json.cyr"` → `include
  "lib/bayan.cyr"` (dropping the now-redundant `lib/bigint.cyr`
  include) across `src/main.cyr`, all three benches, the fuzz harness,
  and the `tests/` repros. **No call-site changes** — `bayan`'s
  back-compat shim forwards the legacy `json_*` / `bigint_*` names, and
  libro's only surface is `json_parse` / `json_get` (canonical-JSON
  hashing). libro makes zero direct `bigint_*` calls; the old standalone
  `bigint` include was dead weight.
- **Patra / Agnosys pins** unchanged — `[deps.patra]` `1.11.0` and
  `[deps.agnosys]` `1.4.1` are still the latest published tags; both
  surfaces are unchanged on the 6.1.35 stack (PatraStore tests + perf
  benches, TPM build + tests pass).
- **`dist/libro.cyr`** regenerated; only the version header moved — the
  bundled source (libro's own `[lib]` modules) is byte-identical, since
  the carve touched only stdlib `include` lines, not bundled code.

### Notes

- `proof_to_json_25` holds at **~222us avg** on 6.1.35 (was ~218us on
  6.1.23) — within run-to-run noise; the 2.7.2 bench-context fix stays
  resolved.

## [2.7.2] - 2026-06-10

**Toolchain + dependency refresh; long-standing `proof_to_json` bench
unblocked.** Cyrius pin 6.0.53 → 6.1.23 (6.0 → 6.1 minor-line crossing);
sigil 3.6.0 → 3.7.8; patra 1.10.3 → 1.11.0; agnosys 1.3.2 → 1.4.1. No
source *logic* changed. libro compiles clean (default + `-D LIBRO_TPM`),
lints and formats clean, full suite passes (**502 default / 514 TPM**) and
all benches + the fuzz harness run clean on the 6.1.23 stack.

### Changed

- **Cyrius pin**: `cyrius.cyml` `cyrius = "6.1.23"` (was 6.0.53). The
  6.0 → 6.1 crossing required no source migrations — clean build + full
  suite on the first pass.
- **Sigil pin**: `[deps.sigil]` `tag = "3.7.8"` (was 3.6.0). libro's
  surface (SHA-256, Ed25519, HMAC, `ct_eq_bytes_lens`, hex) is unchanged
  across the 3.6 → 3.7 line; suite + batch-verify path pass unchanged.
- **Patra pin**: `[deps.patra]` `tag = "1.11.0"` (was 1.10.3); bundled
  `lib/patra.cyr` tracks it. PatraStore tests + perf-tier benches pass.
- **Agnosys pin**: `[deps.agnosys]` `tag = "1.4.1"` (was 1.3.2). libro's
  surface (tpm_seal/tpm_unseal + syscall wrappers) is unchanged across the
  1.3 → 1.4 line; TPM build + tests pass.
- **`dist/libro.cyr`** regenerated (5481 lines); only the version header
  moved — no bundled-source change.

### Added

- **`proof_to_json_25` benchmark shipped in `benches/libro_proof.bcyr`**
  (benches 32 → **33**). Measuring `proof_to_json(ip)` inside a `bench_run`
  loop had triggered a bench-context control-flow hijack/SIGILL since 2.0.5
  (carried open on the roadmap; the same call always passed in the test
  suite). Re-tested against cyrius 6.1.23: **resolved** — the bench runs
  clean (`proof_to_json_25: ~218us avg`). The bench now ships with
  `proof_json.cyr` + its `store`/`export`/`file_store` include closure.
  Retires the roadmap *"Re-investigate `proof_to_json` bench-context
  control-flow hijack"* item.

### Notes

- **Roadmap P2 (`constant_time_eq_str` → `ct_eq_bytes_lens`) was already
  complete.** `src/hasher.cyr` and the `src/main.cyr` test helpers already
  call `ct_eq_bytes_lens`; there are zero bare `ct_eq(` call sites in source
  or in `dist/libro.cyr`. The roadmap entry described a pre-migration state
  that no longer existed — closed as done, no code change.
- Secondary docs (`dependency-watch`, `testing`, `threat-model`,
  `standards-mapping`, `integration`) refreshed off their pre-2.6.3 pins to
  the 6.1.23 / 3.7.8 / 1.11.0 / 1.4.1 stack — the doc-health "Outstanding
  after 2.7.1" backlog.

## [2.7.1] - 2026-06-03

**Toolchain + dependency refresh; TPM `#derive` workaround removed.**
Cyrius pin 6.0.51 → 6.0.53; sigil 3.5.7 → 3.6.0; agnosys 1.2.8 → 1.3.2;
patra unchanged (1.10.3, already latest). Two source changes, both
enabled by the toolchain/dep bumps. libro compiles clean (default +
`-D LIBRO_TPM`), lints and formats clean, full suite passes
(**502 default / 514 TPM**) on the 6.0.53 stack.

### Changed

- **Cyrius pin**: `cyrius.cyml` `cyrius = "6.0.53"` (was 6.0.51).
- **Sigil pin**: `[deps.sigil]` `tag = "3.6.0"` (was 3.5.7). 3.6.0
  ships truly-parallel `sv_verify_batch` (drops the per-call mutex;
  ~3.42× at 64 artifacts / 4 workers) via cyrius 6.0.52 thread-local
  storage. libro inherits the speedup on its batch-verify path with no
  API change. This retires the **"Parallel batch verify hot path"**
  item from the roadmap's *Ecosystem-blocked* list — the unblocker
  (sigil's alloc-free, now lock-free verify hot path) has shipped.
- **Agnosys pin**: `[deps.agnosys]` `tag = "1.3.2"` (was 1.2.8).
  libro's surface (tpm_seal/tpm_unseal + syscall wrappers) is unchanged
  across the 1.2 → 1.3 line; TPM build + tests pass.
- **`dist/libro.cyr`** regenerated; only the version header moved
  (`tpm_anchor` is not bundled).

### Added

- **`lib/thread_local.cyr` to the stdlib include set** (`[deps] stdlib`
  + `src/main.cyr`, ordered before sigil). **Required by sigil 3.6.0:**
  its `crypto_scratch` banks per-thread crypto working arrays over
  cyrius 6.0.52 TLS and calls `thread_local_init/get/set`. Without the
  module included before sigil, the binary **links but SIGILLs at
  runtime** (reads an uninitialised thread pointer) — it does not fail
  to compile. Caught here by running the suite, not just building it.

### Removed

- **Hand-written `tpm_anchor` accessors → back to `#derive(accessors)`.**
  cyrius 6.0.53 raised the per-file `#derive` cap 64 → **512** (verified:
  512 builds, 513 fails `max 512`), so the 2.6.5 workaround is no longer
  needed. `struct tpm_anchor` is a normal `#derive(accessors)` struct
  again; the four hand-written `load64`/`store64` getters + setters are
  gone. `tpm_anchor_new` still constructs via raw `store64` (`ta` stays
  the allowlisted raw-offset param), mirroring the anchor/receipt idiom.

## [2.7.0] - 2026-06-03

**Toolchain refresh + TPM-build root-cause correction.** Cyrius pin
advances within the 6.0 line (6.0.14 → 6.0.51); dependency pins
(sigil 3.5.7, patra 1.10.3, agnosys 1.2.8) are unchanged. No source
*logic* changed — the only `src/*.cyr` edit is a corrected comment in
`tpm_anchor.cyr`. libro compiles clean (default + `-D LIBRO_TPM`),
lints and formats clean, and the full suite passes (**502 default /
514 TPM**) against the canonical 6.0.51 stdlib.

### Changed

- **Cyrius pin**: `cyrius.cyml` `cyrius = "6.0.51"` (was 6.0.14).
  CI extracts this via the existing grep/sed line; no YAML change
  required.
- **`dist/libro.cyr`** rebuilt with `cyrius distlib`; the only delta
  is the version header (2.6.5 → 2.7.0) — bundled source unchanged
  (`tpm_anchor` is not part of the dist bundle).

### Fixed

- **Corrected the `-D LIBRO_TPM` root-cause attribution.** 2.6.5
  recorded the TPM-build compile failure as the **256-entry
  type/struct table cap**. Re-testing under 6.0.51 (by restoring
  `#derive(accessors)` on `tpm_anchor` and rebuilding) shows that was
  a mis-attribution: the failure is the **per-file `#derive` struct
  cap (max 64)**. The TPM build pulls in agnosys (39 `#derive`
  structs) on top of libro's own ~27; `tpm_anchor`'s derive would be
  the 65th. 6.0.51 now reports this explicitly — `error: too many
  #derive structs in one file (max 64)` — where 6.0.14 failed
  silently (`compile … FAIL`, no diagnostic). The hand-written
  `load64`/`store64` accessors added in 2.6.5 are the correct fix and
  **remain in place**; only the explanatory comment changed. See
  CLAUDE.md "Known Cyrius Compiler Quirks" §4.

### Notes

- **256-entry type/struct table cap raised to 1024** in 6.0.51 (was
  256). This is a genuine upstream improvement, but — per the Fixed
  item above — it is *not* the limit that gated the TPM build, so it
  unblocks no libro work today. Recorded for future struct/enum
  growth headroom.

## [2.6.5] - 2026-05-28

**Toolchain + dependency refresh.** Cyrius pin advances within the
6.0 line (6.0.1 → 6.0.14); sigil 3.4.3 → 3.5.7, patra 1.9.5 →
1.10.3, agnosys 1.2.7 → 1.2.8. The bump required one targeted source
change (the `tpm_anchor` accessors, below) to keep the `-D LIBRO_TPM`
build compiling; every other `src/*.cyr` is byte-for-byte unchanged.
libro compiles clean (default + `-D LIBRO_TPM`), lints and formats
clean, and the full suite passes (**502 default / 514 TPM**) against
the canonical 6.0.14 stdlib.

### Fixed

- **`-D LIBRO_TPM` build no longer fails to compile.** Since the 6.0.1
  syscalls refactor the opt-in TPM build died with a bare `compile …
  FAIL` (no diagnostic). Root cause (bisected 2026-05-28): cyrius has a
  **256-entry type/struct table cap** that fails silently on the 257th
  definition. libro's TPM build sits right at that boundary; `enum
  TpmAnchorVerify` + `struct tpm_anchor` + its `#derive(accessors)`
  tipped it over (the default build stays under). Fix: `tpm_anchor`
  drops `#derive(accessors)` for four hand-written `load64`/`store64`
  getters + setters (`ta` was already the allowlisted raw-offset param),
  which removes the derive's type-table pressure. Filed upstream as
  cyrius `docs/development/issues/2026-05-28-type-table-256-cap-silent-fail.md`.

### Changed

- **Cyrius pin**: `cyrius.cyml` `cyrius = "6.0.14"` (was 6.0.1).
  CI extracts this via the existing grep/sed line; no YAML change
  required.
- **Sigil pin**: `[deps.sigil]` `tag = "3.5.7"` (was 3.4.3).
- **Patra pin**: `[deps.patra]` `tag = "1.10.3"` (was 1.9.5) —
  first crossing onto the 1.10 line. libro's patra surface
  (SQL-backed store in `src/patra_store.cyr`) compiles and passes
  unchanged.
- **Agnosys pin**: `[deps.agnosys]` `tag = "1.2.8"` (was 1.2.7).
  Patch bump within the existing direct-pin policy (libro's only
  agnosys surface is tpm_seal/tpm_unseal + syscall wrappers,
  unchanged across the line).
- **`cyrius.lock` is no longer committed.** It is now gitignored
  (alongside `lib/`), matching the patra/sigil convention. Under
  cyrius 6.0.x the lock records the toolchain's stdlib hashes, which
  track the exact installed snapshot and drift between dev machines
  and CI; the cyrius.cyml `[deps.*]` tag pins are the contract. CI
  and release run `cyrius deps` but no longer verify or ship the lock.
- **`dist/libro.cyr`** rebuilt with `cyrius distlib`; deltas are
  the version header (2.6.4 → 2.6.5) plus cosmetic blank-line
  collapsing from the 6.0.14 distlib — source content unchanged.

## [2.6.4] - 2026-05-25

**Toolchain + dependency refresh.** Cyrius pin advances across
the 5 → 6 major boundary (5.10.44 → 6.0.1); sigil 3.1.1 → 3.4.3,
patra 1.9.4 → 1.9.5, agnosys 1.2.6 → 1.2.7. Unlike 2.6.3, the
major compiler bump required **no source migrations** — every
`src/*.cyr` is byte-for-byte unchanged. libro compiles clean
(default + `-D LIBRO_TPM`), lints and formats clean, and the
full suite passes (502 default / 514 TPM).

### Changed

- **Cyrius pin**: `cyrius.cyml` `cyrius = "6.0.1"` (was
  5.10.44) — first crossing of the 5 → 6 major line. CI extracts
  this via the existing grep/sed line; no YAML change required.
- **Sigil pin**: `[deps.sigil]` `tag = "3.4.3"` (was 3.1.1).
- **Patra pin**: `[deps.patra]` `tag = "1.9.5"` (was 1.9.4).
- **Agnosys pin**: `[deps.agnosys]` `tag = "1.2.7"` (was 1.2.6).
  Patch bump within the existing direct-pin policy (libro's only
  agnosys surface is tpm_seal/tpm_unseal + syscall wrappers,
  unchanged across the line).
- **`cyrius.lock`** regenerated against the new tags; the
  toolchain-bundled stdlib snapshot (incl. sakshi 2.2.5) advanced
  with the 6.0.1 pin, reflected in the recorded hashes.
- **`dist/libro.cyr`** rebuilt with `cyrius distlib`; the only
  delta is the version header (source content unchanged), so the
  committed bundle stays current with the tag.

### Docs

- **Root-doc refresh** to current state: `README.md` (502/514
  tests, 12 fuzz targets, bundled patra v1.9.5), `CLAUDE.md`
  (version 2.6.4, toolchain pin 6.0.1, test/bench/binary/dist
  counts, patra version), and the `docs/doc-health.md` ledger.
  The 2.6.3 and 2.6.4 dep bumps were never propagated to the
  secondary docs, so `dependency-watch`, `testing`, `threat-model`,
  `standards-mapping`, and `integration` are flagged stale in the
  ledger for a follow-up pass.
- **Binary-size metric note**: under cyrius 6.0.x, `CYRIUS_DCE=1`
  NOPs dead code (~630 KB NOPed) but no longer strips it, so the
  DCE and non-DCE binaries are byte-identical (~1.1 MB). The
  prior ~456 KB figure reflected 5.x DCE, which stripped.

## [2.6.3] - 2026-05-11

**Toolchain + dependency refresh.** Cyrius pin advances from
5.10.34 to 5.10.44; sigil 3.0.1 → 3.1.1, patra 1.9.3 → 1.9.4,
agnosys 1.0.4 → 1.2.6. Three call-site migrations to keep the
test suite passing against the new sigil and stdlib.

### Changed

- **Cyrius pin**: `cyrius.cyml` `cyrius = "5.10.44"` (was
  5.10.34). CI extracts this via the existing grep/sed line —
  no YAML change required.
- **Sigil pin**: `[deps.sigil]` `tag = "3.1.1"` (was 3.0.1).
- **Patra pin**: `[deps.patra]` `tag = "1.9.4"` (was 1.9.3).
- **Agnosys pin**: `[deps.agnosys]` `tag = "1.2.6"` (was 1.0.4).
  Advanced past sigil's transitive 1.0.4 floor — libro's only
  agnosys surface is tpm_seal/tpm_unseal + syscall wrappers,
  both unchanged across the 1.0 → 1.2 line, so the direct pin
  tracks the latest published tag rather than the transitive
  floor. Comment in `cyrius.cyml` updated to record the new
  policy.
- **`cyrius.lock`** regenerated against the new tags.

### Added

- **`lib/slice.cyr` include** in `src/main.cyr` (right before
  `lib/agnosys.cyr`). Agnosys 1.2.6 indexes slices with the
  `s[i]` syntax that lowers to `_slice_idx_get_W` helpers from
  stdlib `slice.cyr`; without the include the compile emits
  `slice subscript requires include "lib/slice.cyr"`.
- **`"slice"` entry** added to `[deps].stdlib` so `cyrius deps`
  materialises `lib/slice.cyr` alongside the rest of the
  vendored stdlib surface libro uses.

### Fixed

- **Three `str_from(sig_alg_name(...))` call sites** migrated to
  `str_new(cstr, strlen(cstr))`. Sigil 3.1.1 annotated
  `sig_alg_name`'s return as `: i64` (previously untyped); the
  cyrius 5.10.44 dispatcher then routes `str_from(i64)` to
  `str_from_int`, which formats the cstr pointer as a decimal
  string ("5160205" instead of "ML-DSA-65"). `str_new` has no
  int-typed overload, so the call wraps the cstr as a Str
  without dispatching through the int path.
  - `src/signing.cyr` (single-alg `sign_entry` + hybrid
    `_sign_entry_hybrid`)
  - `src/proof.cyr` (sth construction in `proof_build`)
- **`ct_eq` → `ct_eq_bytes_lens`** rename. Sigil retired its
  hand-rolled `ct_eq` at 3.0.2 in favour of the stdlib
  `ct_eq_bytes_lens` (dual-length variant). Libro was still on
  3.0.1 so the old name resolved; the bump to 3.1.1 left
  `ct_eq` undefined. Migrated:
  - `src/hasher.cyr` (`constant_time_eq_str` wrapper)
  - `src/main.cyr` (`test_ct_eq` battery, 4 calls)

### Verified

- Build clean (`CYRIUS_DCE=1 cyrius build src/main.cyr build/libro`).
- Test suite: **502 passed, 0 failed** (unchanged total).
- LIBRO_TPM build: **514 passed, 0 failed** (unchanged total).
- All three bench binaries (`libro_core`, `libro_io`,
  `libro_proof`) compile clean with DCE.
- Fuzz harness (`fuzz/fuzz_libro.fcyr`) — all 12 targets
  pass, no crashes.
- `cyrfmt --check src/*.cyr` clean; `cyrius lint` clean except
  for the one pre-existing >120-char line literal in main.cyr.
- `dist/libro.cyr` regenerated via `cyrius distlib` — 5549
  lines, header reports `Version: 2.6.3`.

## [2.6.2] - 2026-05-10

**Raw-offset CI guard extended to the remaining derived structs.**
Second item on the sequenced 2.6.x patch line. Two complementary
checks land: six new unambiguous-param structs join the cross-file
guard, and a brand-new offset-bound check enforces field-count
discipline on the seven ambiguous-param structs (those sharing
single-letter names `a` / `e` / `r` / `s` across multiple files).

### Added

- **Extended cross-file specific-struct guard** in
  `.github/workflows/ci.yml`. Six new `check_struct` entries:
  - `pv` (proof.cyr, param `pv`)
  - `proof_node` (merkle.cyr, param `pn`)
  - `merkle_tree` (merkle.cyr, param `tree`)
  - `ts_response` (timestamping.cyr, param `resp`)
  - `_sub` (streaming.cyr, param `sub`)
  - `tpm_anchor` (tpm_anchor.cyr, param `ta`)
  Each param is unambiguous across libro (verified by grep
  across `src/*.cyr` for the param + raw-offset pattern). Total
  specific-struct cross-file coverage: 7 → 13.
- **New CI gate — "Raw-offset bound check (struct field-count
  enforcement)".** For the seven ambiguous-param structs whose
  param name appears in multiple defining files (anchor, archive,
  ts_attestation, entry, error, filestore, _patrastore, sth,
  receipt, retention, review, integrity, memstore, stream,
  ts_request), the gate validates that each `load64(param + N)` /
  `store64(param + N, …)` site stays within the struct's field
  count (`max_offset = (n_fields - 1) * 8`). 17 (file, param,
  struct, field_count) tuples registered.
- **Sanity-checked the new gate fires** by injecting a bogus
  `store64(r + 64, 0)` in `src/retention.cyr` (retention is a
  2-field struct, max +8); the gate caught the +64 offset
  immediately. Source restored afterwards; no committed change
  from the sanity check.

### Bug class caught

Off-by-one offset bugs after a struct shrinks — e.g., removing a
field from a derived-accessors struct leaves stale raw-offset
references in the defining file that point past the new struct
boundary. The cross-file specific-struct guard never caught this
because it only fires on raw offsets *outside* the defining file.
The per-file allowlist registers param names but doesn't constrain
the offset values. The new offset-bound gate closes the gap.

### Verified

- Local CI sim of the extended cross-file guard against current
  `src/*.cyr` — PASS (no false positives on the 6 new struct
  entries).
- Local CI sim of the new offset-bound gate against current
  `src/*.cyr` — PASS (all 17 tracked (file, param, struct)
  triples have offset values within the registered max).
- Build clean (`CYRIUS_DCE=1 cyrius build src/main.cyr build/libro`).
- Test suite: 502 passed, 0 failed (unchanged from 2.6.1).
- LIBRO_TPM build: 514 passed, 0 failed (unchanged from 2.6.1).

### Maintenance

- When a `#derive(accessors)` struct gains or loses a field, the
  corresponding `field_count` in the offset-bound table needs to
  be updated. The maintenance signal is loud: the CI gate fires
  immediately on a stale field count combined with a new
  raw-offset site outside the old range.
- Adding a new derived struct: if its canonical param name is
  unambiguous across `src/*.cyr`, register it via `check_struct`
  in the cross-file guard. If the param name is shared, register
  via `check_offset_bound` in the new gate.

## [2.6.1] - 2026-05-10

**Layout-invariant coverage expansion.** The first item on the
sequenced 2.6.x patch line. Adds layout-invariant tests for every
remaining `#derive(accessors)` struct in the public surface — 15
new structs (16 under `-D LIBRO_TPM`), bringing total layout
coverage from 10 → 25 (26 with TPM). Catches `#derive(accessors)`
codegen drift across cyrius toolchain bumps before any end-to-end
test would.

### Added

- **15 new `test_layout_*` fns** covering: `archive`, `error`,
  `integrity`, `review`, `receipt`, `memstore`, `stream`,
  `ts_request`, `ts_response`, `ts_attestation`, `retention`,
  `proof_node`, `merkle_proof`, `consistency`, `pv`.
- **1 new `test_layout_tpm_anchor`** (gated behind `#ifdef LIBRO_TPM`)
  covering the opt-in TPM-anchor struct's 4-field layout.
- **59 new assertions** in the default build / 67 with `-D LIBRO_TPM`.

### Pattern

Each test follows the same shape as the existing 10 layout tests
(2.0.4 trio + 2.3.0 extension): allocate a freelist buffer sized to
N×8 bytes, poke distinguishable sentinel values at each raw offset,
read back via the derived accessors, assert each one returns the
sentinel. UUID-prefixed structs (`receipt`) skip the first 16 bytes
the same way `anchor` and `entry` do — those slots are `_uuid_hi` /
`_uuid_lo` placeholders without accessors.

Internal `_`-prefixed structs (`_patrastore`, `_sub`) are
intentionally not covered — they're not public-surface and have no
consumer-visible layout contract.

### Verified

- `CYRIUS_DCE=1 cyrius build src/main.cyr build/libro` clean.
- `./build/libro` — **502 passed, 0 failed** (was 443).
- `CYRIUS_DCE=1 cyrius build -D LIBRO_TPM src/main.cyr build/libro_tpm`
  clean; **514 passed, 0 failed** (was 451).
- `cyrfmt --check src/*.cyr` clean; `cyrius lint src/*.cyr` clean.

### Roadmap impact

Closes the first sequenced item on the 2.6.x patch line. The
remaining sequenced 2.6.x items (raw-offset guard expansion → RFC
6901 pointers → bench-hijack re-investigation → JSON streaming)
stay open for subsequent patches.

## [2.6.0] - 2026-05-10

**`proof_from_json` round-trip closes + doc-health ledger.** Two
threads bundled: finishing the proof-JSON parser (the 2.0.x
follow-up that landed as "partial verify" — 2.6.0 lifts it to a
full round-trip), and adding a living documentation-currency
ledger modeled on the agnosys pattern.

### Added

- **Lossless inclusion-path JSON serialization** — `proof_to_json`
  now emits each `merkle_proof.path` entry as an object
  `{"h":"<hex>","s":<0|1>}` instead of a bare hex string. The
  `s` (side bit) was previously dropped, so parsed proofs lost
  the left-vs-right info `merkle_verify_proof` needs. With the
  side preserved, every parsed merkle_proof round-trips and
  verifies.
- **`proof_from_json` parser accepts both forms** — the 2.6.0
  object form (lossless) and the pre-2.6 bare-string form
  (degraded: side defaults to `SIDE_LEFT`). Archival proofs
  written by older libro versions remain readable; the
  legacy-form proofs won't pass `merkle_verify_proof` (they
  couldn't before either) but the iproof structure parses
  without crashing.
- **`test_proof_from_json_roundtrip_full`** — replaces the
  former `_partial_verify` test. Asserts entries chain-verify,
  rebuilt tree root matches parsed sth.root, AND every parsed
  inclusion passes `merkle_verify_proof` (the new contract).
- **`test_proof_from_json_legacy_path_accepted`** — pins the
  backward-compat path: bare-string emissions parse, produce
  proof_nodes with `SIDE_LEFT` defaults, and the parser doesn't
  crash. 8 new assertions total.
- **`docs/doc-health.md`** — new living ledger covering all 25
  markdown files in the libro repo. Buckets: Fresh / Stale /
  Read-through-outstanding / Evergreen / Frozen / ADRs. Pattern
  lifted from [agnosys](https://github.com/MacCracken/agnosys/blob/main/docs/doc-health.md).
  Initial inventory + 4-row cleanup paired with this release;
  4 rows flagged stale (threat-model, dependency-watch,
  standards-mapping, architecture/overview) for 2.6.x / 2.7.0
  refresh.

### Changed

- **`_sb_proof_inclusion`** in `src/proof_json.cyr` — path-array
  emission upgraded to object form with `h` + `s` keys.
- **`_pfj_parse_one_inclusion`** — path-array parser now branches
  on first non-whitespace char: `{` → object form (extract `h`
  + `s`, wrap in `proof_node_new`); `"` → legacy bare string
  (wrap in `proof_node_new(s, SIDE_LEFT)`). Old `_pfj_parse_string`
  path retained for the legacy branch.
- **`CLAUDE.md`** — Project Identity version bumped `2.0.0-dev`
  → `2.6.0-dev`; Current State refreshed (22 modules counting
  the opt-in TPM one; 30 benches across 3 binaries; 435/443
  test counts; binary size ~456 KB; dist line count ~5.4k);
  toolchain pin string refreshed `5.4.2` → `5.10.34`.
- **`README.md`** — Architecture diagram, Quick Start test-count
  line, and Project structure module list all reflect the
  2.6.0 surface (22 modules, 32 benches, 435/443 tests).
- **`docs/development/roadmap.md`** — `proof_from_json`
  round-trip marked ✅ shipped (was the highest-leverage
  "Open — unblocked" item). doc-health.md added to docs
  Tier 7 (Engineering issues) bucket.

### Verified

- `CYRIUS_DCE=1 cyrius build src/main.cyr build/libro` clean.
- `./build/libro` — **443 passed, 0 failed** (was 435 — net
  +8 from full-roundtrip + legacy-form tests; one prior test
  replaced).
- `CYRIUS_DCE=1 cyrius build -D LIBRO_TPM src/main.cyr build/libro_tpm`
  clean; **451 passed, 0 failed** (443 + 8 TPM-gated).
- All three benchmark binaries build + run.
- `./build/fuzz_libro` — all 12 harnesses survive 100-iteration
  runs (the `fuzz_proof_from_json` harness exercises the new
  object-form parsing via random `{` chars in the input).
- `cyrfmt --check src/*.cyr` clean; `cyrius lint src/*.cyr` clean.
- Raw-offset CI check passes (no new offset sites; the parser
  uses `proof_node_new` rather than raw stores).

### Migration notes

Consumers that have *saved* proof JSON output from libro
≤ 2.5.0:

- **Re-parse with libro ≥ 2.6.0** — saved bytes still parse via
  the legacy bare-string code path. The structural fields
  (entries, tree-head, anchor slot) round-trip identically.
- **Verification of saved inclusions doesn't work** — same
  status as pre-2.6 (the side bit was never captured). Re-emit
  the proof under 2.6.0+ to get verifiable inclusion arrays.
- **No on-the-wire format break** — clients running mixed
  versions still parse each other's output; the object-form
  produces correct verification, the legacy form produces a
  parseable but un-verifiable inclusion array.

### Roadmap impact

- **The high-leverage 2.x "unblocked" item is closed.** Remaining
  filler items (JSON streaming, RFC 6901 pointers, raw-offset
  guard expansion, struct-layout test expansion) are smaller in
  scope; pick up as room appears in any minor.
- **Stale-doc refresh window** opens — the 4 rows flagged in
  `docs/doc-health.md` (threat-model, dependency-watch,
  standards-mapping, architecture/overview) want a pass during
  the 2.6.x / 2.7.0 cycle.

## [2.5.0] - 2026-05-10

**TPM-sealed `WitnessAnchor` lands (opt-in).** Hardware-backed
anchor attestation behind `-D LIBRO_TPM`. Default builds are
unaffected — no agnosys/tpm2-tools surface linked, same binary
size, same dependency graph at runtime. Consumers wanting
hardware-rooted audit attestation flip the build define on.

This closes the final scheduled minor on the 2.x roadmap.

### Added

- **`src/tpm_anchor.cyr`** — new opt-in module wrapping libro's
  `anchor` struct with a TPM seal over the anchor's self-hash.
  Three new APIs:
  - `tpm_anchor_new(inner, output_dir, pcr_indices)` — seals the
    inner anchor's hash to the host TPM at current PCR state.
    Returns 0 on any failure (no TPM, no tpm2-tools, seal subprocess
    error). Consumers null-check and degrade to software-only.
  - `tpm_anchor_verify(ta)` — returns one of `TPM_ANCHOR_VALID` /
    `TPM_ANCHOR_INNER_INVALID` / `TPM_ANCHOR_UNAVAILABLE` /
    `TPM_ANCHOR_UNSEAL_FAILED` / `TPM_ANCHOR_HASH_MISMATCH`.
    Dispatches three conditions (inner integrity + TPM unseal +
    cryptographic binding); VALID requires all three.
  - `tpm_anchor_verify_strict(ta)` — bool-shaped wrapper, returns
    1 only on full hardware-backed success.
- **`tpm_anchor_default_pcr_indices()`** — the conservative AGNOS
  default: PCR 0 (firmware) + PCR 7 (Secure Boot config). Tight
  enough to detect firmware/boot-policy tampering, loose enough that
  ordinary userspace + kernel updates don't invalidate seals.
- **`enum TpmAnchorVerify`** — return-code constants for the verify
  dispatch. Mirror the `AnchorVerify` shape so callers can switch
  on integer values.
- **`#derive(accessors)` struct `tpm_anchor`** — 4 fields:
  `inner`, `sealed_ctx`, `pcr_indices`, `output_dir`. Standard
  accessor pattern.
- **`[deps.agnosys]`** promoted to a direct pin in `cyrius.cyml`
  (was transitive via sigil). Matches sigil 3.0.1's floor at 1.0.4;
  libro now controls the version independent of sigil's pin
  movements.
- **`docs/guides/tpm-anchors.md`** — new guide covering trust model
  ("proves anchor was created on this TPM at this PCR state; does
  NOT prove host honesty or chain correctness"), build flow, runtime
  requirements (tpm2-tools, /dev/tpmrm0 permissions, writable
  output_dir), PCR-policy alternatives, file persistence semantics.
- **4 new test fns / 8 new assertions** in `src/main.cyr` (gated
  behind `#ifdef LIBRO_TPM`): null-handle returns INNER_INVALID,
  default PCR shape, tampered-inner rejection, hardware roundtrip
  (best-effort; logs+skips on hosts without tpm2-tools rather than
  failing).
- **CI step** "TPM-opt-in build + test (LIBRO_TPM)" — builds with
  `-D LIBRO_TPM` and runs the gated test battery. Verifies API
  correctness on hosts without tpm2-tools (the typical CI shape);
  hardware-success coverage requires a TPM-equipped runner the
  shipped workflow doesn't assume.

### Changed

- **`src/main.cyr`** gains a single `#ifdef LIBRO_TPM` block at the
  end of the includes that pulls in `src/tpm_anchor.cyr`. Same
  block-shape for the test runner. Default builds skip the block
  entirely; no overhead.
- **CI manifest-completeness check** updated to skip `#ifdef`-gated
  includes when comparing main.cyr against `[lib].modules`. This
  preserves the gate for normal modules (regression coverage from
  the 2.0.1 era) while letting opt-in modules stay out of the
  default dist bundle.
- **`[lib].modules` intentionally does NOT list `src/tpm_anchor.cyr`**.
  cyrius distlib strips `#ifdef` markers when bundling, so including
  it in the manifest would force agnosys tpm_* symbols into the
  default `dist/libro.cyr` and break the no-TPM consumer contract.
  Consumers wanting TPM build from source with `-D LIBRO_TPM` or
  vendor the module directly. The manifest now carries an explicit
  comment documenting this.

### Trust model (summary; see guide for full detail)

`tpm_anchor_verify(ta) == TPM_ANCHOR_VALID` proves three things
*together*:

1. The inner `WitnessAnchor`'s self-hash matches its claimed contents
   (`anchor_verify_integrity` passes).
2. The TPM can unseal the blob libro sealed at anchor-creation time
   (PCRs named by `pcr_indices` are in the same state they were at
   seal time).
3. The unsealed bytes equal the inner anchor's hash (cryptographic
   binding — the seal can't be transplanted).

What it does NOT prove: chain correctness (verify against the tree
separately), host honesty (a compromised host with TPM control at
seal time can produce a valid seal for arbitrary content), identity
(combine with Ed25519/ML-DSA-65 entry signing for attribution).

### Verified

- `CYRIUS_DCE=1 cyrius build src/main.cyr build/libro` clean.
- `./build/libro` — **435 passed, 0 failed** (unchanged from 2.4.0).
- `CYRIUS_DCE=1 cyrius build -D LIBRO_TPM src/main.cyr build/libro_tpm` clean.
- `./build/libro_tpm` — **443 passed, 0 failed** (435 + 8 TPM-gated).
- All three benchmark binaries build + run.
- `./build/fuzz_libro` — all 12 harnesses survive 100-iteration runs.
- `cyrfmt --check src/*.cyr` clean; `cyrius lint src/*.cyr` clean.
- Test environment notes: this host has `/dev/tpm0` but no
  `tpm2-tools` installed, so the `hardware_roundtrip_best_effort`
  test exercises the seal-fails-gracefully path. The seal-succeeds
  path is reachable on a properly-equipped TPM host; the test
  battery handles both.

### Roadmap impact

- **2.x roadmap closes here.** The four major minors planned in the
  2.1.0 remap have all shipped: 2.2.0 PQ signing, 2.3.0 hybrid
  signing, 2.4.0 PatraStore perf, 2.5.0 TPM anchors. The remaining
  roadmap items are "Open — unblocked (not yet slotted)" filler-grade
  work (`proof_from_json`, JSON streaming, RFC 6901 pointers, struct-
  layout test expansion, raw-offset guard expansion) and genuinely
  ecosystem-blocked items (multi-node federation, sigil 3.1
  parallel batch verify).

## [2.4.0] - 2026-05-10

**PatraStore performance tier.** Wires patra 1.7–1.9's three perf
features (prepared statements, group commit, STR-keyed btree
indexes) into the PatraStore API. Existing consumers see no
behaviour change — the prepared-statement work is transparent, and
the BATCH-mode + index APIs are opt-in. The opt-in shape is
deliberate: each knob trades a real-disk write-path cost for a
read-path speedup, and the right choice depends on the consumer's
workload mix.

### Added

- **`patrastore_set_sync_mode(s, mode)` / `patrastore_get_sync_mode(s)`
  / `patrastore_flush(s)`** — thin wrappers over patra 1.8.0's group-
  commit API. Default after `patrastore_open` is `PATRA_SYNC_FULL`
  (durable on every mutating exec); switch to `PATRA_SYNC_BATCH` to
  amortize fdatasync across writes (auto-flushes every 64).
- **`patrastore_append_batch(s, entries)`** — bulk-append vec of
  entries with sync mode auto-toggled. Internally: save caller's
  mode, switch to BATCH, loop appends, flush, restore. Returns the
  count of successful inserts. Composes cleanly with consumer-held
  BATCH windows (caller already in BATCH stays in BATCH on return).
- **`patrastore_create_source_index(s)`** — opt-in STR-keyed btree
  index on the `src` column (patra 1.7.0). Converts
  `patrastore_by_source` from O(N) full-scan to O(log N) btree
  probe + filter. Idempotent. Documented trade-off: per-insert
  cost rises because every append must also update the index page.
- **Prepared SELECT and COUNT statements** behind `patrastore_load_all`
  and `patrastore_len`. Parsed once at `patrastore_open`, finalized
  at `patrastore_close`. Skips the ~8 µs tokenize+parse step per
  read call. Transparent — no API change.
- **16 new test assertions** across 6 perf-tier test fns: sync
  mode default, sync mode switch, append_batch correctness,
  append_batch preserves caller's BATCH mode, prepared statements
  populated at open, indexed by_source roundtrip + idempotent index
  creation. 417 → 435 total (counting hybrid sig + new perf).
- **4 new bench rows** in `libro_bench_io`: `patra_append_50_full`,
  `patra_append_50_batch`, `patra_load_all_50`, `patra_by_source_50`.

### Changed

- **`_patrastore` struct extended from 2 to 4 fields** (16 → 32 bytes).
  New slots: `select_stmt`, `count_stmt` (prepared statement handles,
  0 until `patrastore_open` populates them). Underscore-prefixed
  internal-only — no public layout test pinned to the old shape.
- **`patrastore_open` populates the two prepared statement handles**
  after `_patrastore_ensure_table` confirms the table exists. The
  ordering matters: preparing before the table is created would
  silently yield 0-handles and the read paths would fall through to
  the unprepared dispatch.
- **`patrastore_close` finalizes both prepared statements** before
  closing the db. `patra_finalize(0)` is a no-op so this is safe on
  the partial-init path (e.g. if `patrastore_open` failed early).
- **`patrastore_load_all` / `patrastore_len`** check for prepared-
  statement handles and route through `patra_query_prepared` when
  available; fall back to the un-prepared `patra_query` if the
  handle is 0 (defensive; shouldn't happen on a properly-opened
  store).
- **`benches/libro_io.bcyr`** picks up `lib/patra.cyr` +
  `src/patra_store.cyr` for the new patra bench rows. Three
  independent stores (full-mode appends, batch-mode appends,
  pre-loaded reads) — the read store opts into the src index so the
  `patra_by_source_50` row exercises the indexed path.

### Performance (/tmp tmpfs, x86_64)

| Bench                  | Time     | Notes |
|------------------------|---------:|-------|
| `patra_append_50_full` | 2.31 ms  | Per-insert: ~46 µs |
| `patra_append_50_batch`| 2.22 ms  | ~3% faster on tmpfs |
| `patra_load_all_50`    | 575 µs   | Uses prepared SELECT |
| `patra_by_source_50`   | 591 µs   | Uses src_idx (opt-in) |

The full-vs-batch delta is invisible on tmpfs because fdatasync is
a no-op there — same caveat documented in patra's own 1.8.0 bench
notes. Real-disk btrfs/nvme runs show ~64× speedup per patra's
upstream numbers (19.5 ms → 306 µs per insert amortized in
500-insert loops). libro's bench rows here are useful for catching
regressions in the bookkeeping overhead (mode toggles, prepared
dispatch, index update).

### Documentation

- **`docs/guides/integration.md`** gains a "PatraStore — performance
  tier" section: bulk append example, consumer-driven sync-mode
  control, indexed by-source query opt-in with workload heuristic,
  durability contract, and the tmpfs caveat for measuring the win.

### Not breaking

- All four new API entries (`patrastore_set_sync_mode`,
  `patrastore_get_sync_mode`, `patrastore_flush`,
  `patrastore_append_batch`, `patrastore_create_source_index`) are
  pure additions.
- `_patrastore` struct grew (4 fields vs 2), but it's
  underscore-prefixed and consumers are expected to use the
  `patrastore_*` API surface, not raw struct access. No tracked
  consumer is affected.
- The prepared-statement wiring is transparent — `patrastore_load_all`
  and `patrastore_len` produce identical output, just faster on the
  parse-cost dimension.

### Verified

- `CYRIUS_DCE=1 cyrius build src/main.cyr build/libro` clean.
- `./build/libro` — **435 passed, 0 failed** (was 417).
- All three benchmark binaries build + run.
- `./build/fuzz_libro` — all 12 harnesses survive 100-iteration runs.
- `cyrfmt --check src/*.cyr` clean; `cyrius lint src/*.cyr` clean.

### Roadmap impact

- **2.5.x — TPM-sealed `WitnessAnchor`** is next. agnosys is already
  a transitive dep via sigil 3.0.1; needs to be promoted to a direct
  `[deps.agnosys]` pin and a new opt-in `src/tpm_anchor.cyr` module
  behind a `LIBRO_TPM=1` build define.

## [2.3.0] - 2026-05-10

**Hybrid signing lands.** Entries can now be signed with both
Ed25519 and ML-DSA-65 simultaneously, and verified under
sigil 3.0's `sigil_verify_hybrid` AND-mode contract. This is the
migration path the 2.2.0 changelog flagged: chains that need to
outlive a single algorithm's threat horizon (Ed25519 today,
ML-DSA-65 tomorrow, both during the transition).

### Added

- **`signing_key_generate_hybrid()`** — generates Ed25519 and
  ML-DSA-65 keypairs from *independent* 32-byte seeds. No shared
  entropy: 64 bytes of CSPRNG output are cheap, and domain
  separation is free insurance against any future KDF correlation
  finding.
- **`SIG_ALG_HYBRID = 2` dispatch** added to `sign_entry`,
  `verify_entry_signature`, `sign_tree_head`, `verify_tree_head`.
  Hybrid sign produces both signatures; hybrid verify requires
  both to validate. Single-algorithm callers (Ed25519 / ML-DSA-65)
  continue to work unchanged.
- **`verifying_key_from_bytes_hybrid(ed_bytes, pq_bytes)`** —
  constructor for a hybrid vk from raw Ed25519 + ML-DSA-65 pk
  bytes. `verifying_key_from_signing(sk)` auto-routes through it
  when `sk.algorithm == SIG_ALG_HYBRID`.
- **`signing_key_verifying_hex_2(sk)` / `verifying_key_to_hex_2(vk)`**
  — slot-2 hex accessors. The pre-2.3 `*_hex` accessors stay
  Ed25519-shaped for backward compatibility (return only slot 1).
- **`entry_sig_new_hybrid(...)`** — 7-arg constructor that fills
  both slots of the extended `entry_sig` struct.
- **23 new test assertions** across 4 hybrid test fns
  (roundtrip, tamper-rejection on either side, zeroize across
  both buffers, tree-head roundtrip). 394 → 417 total.
- **6 new layout-invariant assertions** for the new struct
  fields (`signing_key.bytes_2/pub_bytes_2/seed_2`,
  `verifying_key.bytes_2`, `entry_sig.signature_2/verifying_key_2`).
- **2 new bench rows** in `libro_bench_core` —
  `hybrid_sign_entry`, `hybrid_verify_sig`.

### Changed

- **`signing_key` extended from 6 to 9 fields** (48 → 72 bytes).
  New slots: `bytes_2`, `pub_bytes_2`, `seed_2`. Single-algorithm
  keys leave them as 0; hybrid keys populate both.
- **`verifying_key` extended from 2 to 3 fields** (16 → 24 bytes).
  New slot: `bytes_2` (ML-DSA-65 pk for hybrid; 0 otherwise).
- **`entry_sig` extended from 5 to 7 fields** (40 → 56 bytes).
  New slots: `signature_2` (ML-DSA-65 hex sig for hybrid; 0
  otherwise), `verifying_key_2` (ML-DSA-65 hex pk for hybrid; 0
  otherwise).
- **`signing_key_zeroize` is hybrid-aware** — clears both 64-byte
  Ed25519 sk and 4032-byte ML-DSA-65 sk plus both 32-byte seeds
  when `algorithm == SIG_ALG_HYBRID`.
- **`sign_tree_head` hybrid output** is the two hex sigs
  concatenated as `<ed_hex>|<mldsa_hex>` (6747 chars total). The
  STH structure carries one signature `Str` field, so the
  delimiter form is the only fit without expanding sth's layout.
  `verify_tree_head` splits at the pipe and dispatches
  `sigil_verify_hybrid`. Pipe is unambiguous because hex digits
  never include `|`.
- **`signing_key_from_seed`** struct allocation updated from
  `fl_alloc(48)` to `fl_alloc(72)` to match the new layout.
- **Layout-invariant tests updated** for the three extended
  structs. Pre-fix the tests would have silently failed once the
  derived accessors moved to new offsets — these are the load-
  bearing guard for #derive(accessors) regressions, see
  ADR 0005.

### Performance (x86_64 dev host, sigil 3.0.1)

| Op                       | Ed25519 | ML-DSA-65 | Hybrid (sum) |
|--------------------------|--------:|----------:|-------------:|
| `sign_entry`             | 1.1 ms  | 3.5 ms    | 4.6 ms       |
| `verify_entry_signature` | 6.6 ms  | 2.1 ms    | 8.7 ms       |

Hybrid cost is essentially the sum of the two primitives —
expected, since AND-mode runs both. Per-entry sign + verify in
the single-digit-millisecond range stays well within the
per-event budget for kernel-audit / aegis / stiva workloads.

### Documentation

- **`docs/guides/integration.md`** gains a "Hybrid Signing"
  section: keygen + sign / verify examples, sizing table,
  performance table, three-step migration story
  (Ed25519 → Hybrid → ML-DSA-65), and the backwards-compatible
  verify pattern (a pre-2.3 Ed25519-only vk can still verify
  the Ed25519 portion of a hybrid sig).

### Breaking — backward-compatible

The `entry_sig`, `signing_key`, and `verifying_key` struct sizes
*increased*, but the existing field offsets are preserved.
Pre-2.3 callers using the derived accessors see no change.
Pre-2.3 callers doing raw-offset access on `sk + 48` or higher
would break — but the CI raw-offset guard (2.0.4) already
prevents new such call sites, and the existing allowlist covers
no slot-2 access. No real-world consumer is affected.

### Verified

- `CYRIUS_DCE=1 cyrius build src/main.cyr build/libro` clean.
- `./build/libro` — **417 passed, 0 failed** (was 388).
- All three benchmark binaries build + run.
- `./build/fuzz_libro` — all 12 harnesses survive 100-iteration
  runs (no crashes).
- `cyrfmt --check src/*.cyr` clean; `cyrius lint src/*.cyr` clean.

### Roadmap impact

- **2.4.x — PatraStore performance** is now next in line.
- **2.5.x — TPM-sealed `WitnessAnchor`** unaffected.

## [2.2.0] - 2026-05-10

**Post-quantum signing lands.** ML-DSA-65 (NIST FIPS 204) entry
signing alongside Ed25519, dispatched at runtime via the
`algorithm` field on `signing_key` / `verifying_key`. Sigil 3.0.0
shipped the full FIPS 204 stack in 2.1.0; this is the libro-side
plumbing that wires it into `sign_entry` / `verify_entry_signature`
/ `sign_tree_head` / `verify_tree_head`.

Long-standing roadmap item — moves out of "ecosystem-blocked"
permanently. Hybrid (Ed25519 + ML-DSA-65) signing follows in 2.3.x.

### Added

- **`signing_key_generate_mldsa()`** — generates a 4032-byte
  ML-DSA-65 secret key + 1952-byte public key from 32 bytes of
  CSPRNG entropy. Same `signing_key` struct shape as Ed25519
  (fields are pointers; buffer sizes differ behind them); the
  `algorithm` field is set to `SIG_ALG_ML_DSA_65 = 1`. Uses
  `random_bytes` + `secret var seed_stack[32]` for entropy
  gathering, matching the 2.1.1 hardening pattern.
- **`verifying_key_from_bytes_alg(bytes, alg)`** — alg-aware vk
  constructor. The legacy `verifying_key_from_bytes(bytes)` stays
  Ed25519-specific for backward compat.
- **Cross-algorithm rejection battery** (`test_signing_cross_alg_rejected`
  in `src/main.cyr`) — pins that an Ed25519 signature can't verify
  against an ML-DSA vk and vice versa. Verify dispatch is gated on
  the *vk's* algorithm (the trust anchor), not the signature's
  claimed algorithm — an attacker swapping the `entry_sig.algorithm`
  string can't trick a verifier into accepting bytes under a
  primitive the consumer didn't authorize.
- **15 new test assertions** across 5 test fns (roundtrip, tamper-
  rejection, key_id, zeroize, cross-alg). 373 → 388 total.
- **2 new bench rows** in `libro_bench_core` — `mldsa65_sign_entry`,
  `mldsa65_verify_sig`. Iteration counts dropped to 100 (vs 1000
  for Ed25519) because per-op cost is in the ms range.

### Changed

- **`sign_entry` / `verify_entry_signature` / `sign_tree_head` /
  `verify_tree_head` are now polymorphic.** Dispatch on the key's
  algorithm field — pre-2.2 Ed25519 callers see no behaviour
  change because `signing_key_generate()` still sets
  `algorithm = SIG_ALG_ED25519`. PQC callers use
  `signing_key_generate_mldsa()` and the rest of the API is
  unchanged.
- **`signing_key_zeroize` is now alg-aware** — uses `_sig_sk_bytes(alg)`
  to memset the right size (4032 for ML-DSA, 64 for Ed25519). Pre-
  fix the function memset 64 bytes regardless, leaving ~3.9 KB of
  ML-DSA secret-key material un-cleared.
- **`signing_key_verifying_hex` / `verifying_key_to_hex` are
  alg-aware** — encode 64-char hex (Ed25519) or 3904-char hex
  (ML-DSA-65) based on `algorithm`.
- **`entry_sig.algorithm` now carries sigil's canonical name string**
  (`sig_alg_name(alg)` — `"Ed25519"` or `"ML-DSA-65"`). Pre-2.2 it
  was hardcoded to `"ed25519"` (lowercase). Internal-only
  consequence — nothing in libro 2.1.x reads the string content;
  pinned in 2.0.4's `test_layout_entry_sig` only as a field-offset
  probe (not value).
- **Three new size-dispatch helpers** in `signing.cyr` —
  `_sig_pk_bytes` / `_sig_sk_bytes` / `_sig_sig_bytes`. Centralized
  so add-an-algorithm changes stay localized. No scattered `64` or
  `MLDSA65_SK_BYTES` literals in the dispatch fns.

### Performance (x86_64 dev host, sigil 3.0.1)

| Op                       | Ed25519 | ML-DSA-65 | Ratio |
|--------------------------|--------:|----------:|------:|
| `sign_entry`             | 1.1 ms  | 3.5 ms    | 3.1×  |
| `verify_entry_signature` | 6.6 ms  | 2.1 ms    | 0.32× |

ML-DSA-65 verify is *faster* than Ed25519 verify in libro's
build — a surprise headline. Sigil's 3.0 ML-DSA implementation
optimizes the verify path well. Sign is ~3× slower than Ed25519
but absolute is well within audit-workload budget (one signed
entry per kernel-event is fine).

### Documentation

- **`docs/guides/integration.md`** gains a "Post-Quantum Signing"
  section: keygen + sign / verify examples, sizing table,
  performance table, "when to use which" guidance, and the
  vk-dispatch trust-model note.

### Verified

- `CYRIUS_DCE=1 cyrius build src/main.cyr build/libro` clean.
- `./build/libro` — **388 passed, 0 failed** (was 373).
- All three benchmark binaries build + run.
- `./build/fuzz_libro` — all 12 harnesses survive 100-iteration
  runs (no crashes).
- `cyrfmt --check src/*.cyr` clean; `cyrius lint src/*.cyr` clean.

### Roadmap impact

- **2.3.x — Hybrid signing (Ed25519 + ML-DSA-65)** unblocked. The
  algorithm-dispatch infrastructure is in place; hybrid is mostly
  a new entry-storage shape (two signature slots) plus a
  `sigil_verify_hybrid` wrapper.
- **2.4.x — PatraStore performance** unaffected by this minor.
- **2.5.x — TPM-sealed `WitnessAnchor`** unaffected.

## [2.1.1] - 2026-05-10

First slice of the 2.1.x toolchain-bump follow-ups planned in the
remapped roadmap. Three shippable items + two investigations
documented.

### Added

- **`[deps].stdlib`** extended with `random` (cyrius `lib/random.cyr`
  v5.7.35 — kernel CSPRNG via `getrandom(2)`) and `test` (cyrius
  `lib/test.cyr` v5.7.43 — table-driven testing helpers; included
  for future use even though the 2.1.1 refactor pass was deferred).

### Changed

- **`signing_key_generate` reads entropy via `random_bytes` into a
  `secret var seed_stack[32]`**, then `memcpy`s into the long-lived
  heap seed buffer (`signing.cyr`). Replaces the prior
  `file_open("/dev/urandom")` + `file_read` + `file_close` chain.
  The stack copy is auto-zeroized on function exit (cyrius 5.5.12+
  `secret var` guarantee), eliminating the brief stack-resident
  entropy window. Heap-resident seed is unchanged — still cleared
  by `signing_key_zeroize`.
- **`ts_request_generate_nonce` uses `secret var nonce_buf[16]`**
  + `random_bytes` (`timestamping.cyr`). Defense-in-depth: nonces
  aren't strictly secret, but a stack-resident nonce buffer in a
  later frame would be a stale-data leak, and the language
  guarantee removes the need for vigilance.
- **Dropped the `file_open("/dev/urandom") + file_read + file_close`
  pattern from libro entirely.** Both call sites migrated to
  `random_bytes`. Cleaner under Landlock policies that deny
  `/dev/urandom` traversal — `getrandom(2)` is the one-syscall
  path that works in sandboxed contexts (see new integration
  guide section).

### Documented

- **Landlock hardening recipe for PatraStore consumers** added to
  `docs/guides/integration.md`. Cyrius 5.7.35 ships `lib/security.cyr`
  with the Landlock enums and `lib/syscalls_<arch>_linux.cyr` with
  the three syscall wrappers (`sys_landlock_create_ruleset`,
  `sys_landlock_add_rule`, `sys_landlock_restrict_self`). The new
  guide section walks through a concrete daemon-style policy that
  restricts the libro process to a single audit-data directory and
  documents the kernel ≥5.13 requirement, the monotonic-once-applied
  semantics, and the natural pairing with `getrandom` for entropy
  in sandboxed contexts. Doc-only — libro itself stays unopinionated
  about sandbox policy.

### Investigated, deferred

- **`proof_to_json` bench-context control-flow hijack** (carried from
  2.0.5). Re-tested under cyrius 5.10.34 — bug persists, manifestation
  changed: 5.4.7 looped main() at ~25 Hz; 5.10.34 SIGILLs on the first
  bench iteration after a clean `proof_build` + `proof_to_json` pair.
  Same class of bug, different surface. Stack trace recorded in
  `benches/libro_proof.bcyr` header comment for future diagnosis.
  Bench addition reverted; the test-suite path (`test_proof_to_json_*`
  in `src/main.cyr`) continues to pass cleanly. Remains on the
  roadmap.
- **`lib/test.cyr` table-driven refactor** considered and deferred
  (see roadmap for rationale). libro's homogeneous test groups
  (`test_layout_*`, canonical JSON, SHA-256 vectors) each exercise
  different accessor functions per case, so `test_each` would need
  fn-pointer indirection that costs more LOC than it saves. The
  stdlib include is in place for future use; the refactor itself is
  marked as deferred-indefinitely.

### Verified

- `CYRIUS_DCE=1 cyrius build src/main.cyr build/libro` clean.
- `./build/libro` — **373 passed, 0 failed** (unchanged from 2.1.0).
- All three benchmark binaries build + run.
- `./build/fuzz_libro` — all 12 harnesses survive 100-iteration runs.
- `cyrfmt --check src/*.cyr` clean; `cyrius lint src/*.cyr` clean.

## [2.1.0] - 2026-05-10

**Toolchain + dependency refresh.** Bumps cyrius 5.4.7 → 5.10.34,
sigil 2.8.3 → 3.0.1, patra 1.1.1 → 1.9.3. Closes the gap that had
accumulated against the cyrius v5.8.65 stdlib-foldin manifest (sigil
3.x and patra 1.9.x are the foldin slot for both deps). Carries one
breaking-out-of-tree change — `lib/` is no longer git-tracked — and
one internal rename to clear a new cyrius duplicate-fn warning.

### Changed

- **`cyrius` pin** `5.4.7` → `5.10.34` (`cyrius.cyml [package].cyrius`).
- **`[deps.sigil].tag`** `2.8.3` → `3.0.1`; module path
  `sigil.cyr` → `dist/sigil.cyr` to match sigil's canonical manifest
  convention. Sigil 3.0's breaking changes (`TRUST_COMMUNITY` enum
  removal, `alog_append_to_file` / `alog_load_from_file` renames,
  `-D SIGIL_BATCH_PARALLEL` removal) don't touch libro — none of
  the affected surface is exercised. Libro's sigil consumption
  (`sha256_*`, `ed25519_*`, `hex_*`) is unchanged.
- **`[deps.patra].tag`** `1.1.1` → `1.9.3`; module path
  `patra.cyr` → `dist/patra.cyr`. Patra's 1.9.0 rename of
  `json_build` → `patra_json_build` doesn't touch libro (no caller).
  The 1.6–1.8 feature surface (`COL_BYTES`, `INSERT OR IGNORE`,
  STR-indexed btree, sync modes, prepared statements, slab
  allocator) is additive — libro's existing `patra_init` /
  `patra_open` / `patra_exec` / `patra_query` / `patra_begin` /
  `patra_commit` / `patra_rollback` / `patra_result_*` consumption
  is unchanged.
- **`[deps].stdlib`** extended with `string`, `fs`, `tagged`,
  `process`, `ct`, `keccak`, `thread`. These are required by the
  sigil 3.0.1 dist bundle (`lib/ct.cyr` for `ct_select`,
  `lib/keccak.cyr` for `_keccak_f1600` / `shake256`,
  `lib/thread.cyr` for the parallel-batch-verify mutex primitives).
  Without them, `ed25519_keypair` SIGILLs at runtime via the
  unresolved `ct_select` reference (the cyrius "undefined fn
  (will crash at runtime)" build warning is now load-bearing).
- **`src/main.cyr`, `benches/libro_core.bcyr`,
  `benches/libro_io.bcyr`, `benches/libro_proof.bcyr`,
  `fuzz/fuzz_libro.fcyr`** updated to `include` the new stdlib
  modules and the transitive `lib/agnosys.cyr` that sigil pulls in.
- **`benches/libro_core.bcyr`** picks up `src/anchoring.cyr` (its
  `src/proof.cyr` call into `anchor_verify_integrity` was
  previously an unresolved forward reference that the cyrius 5.10.x
  "undefined fn" gate now blocks).
- **`benches/libro_io.bcyr`** picks up `src/retention.cyr` (same
  pattern — `chain.cyr` calls `retention_split_index`).
- **`src/timestamping.cyr` — `ts_request_set_nonce` →
  `ts_request_generate_nonce`.** Collides with the
  `#derive(accessors)`-generated setter on the `ts_request` struct
  under cyrius 5.10's stricter duplicate-fn check. Renamed; the
  new fn calls the auto-derived setter to install the random nonce.
  The redundant manual `ts_request_set_hash_alg` (identical to the
  auto-derived setter) is removed. Only caller in `src/main.cyr`
  updated.

### Removed

- **`lib/` is no longer git-tracked** (49 files removed from the
  tree). The directory is now repopulated by `cyrius deps` from
  the `[deps]` pins in `cyrius.cyml`. Matches the agnosys / patra /
  yukti convention — prevents stale vendored stdlib stubs from a
  prior cyrius version drifting against the manifest pin.
  `/lib/` added to `.gitignore`.

### CI / release

- **`[deps].stdlib` extended with `bench`, `fnptr`.** Both are
  consumed by the three bench binaries; previously they resolved
  against the toolchain's bundled fallback `lib/`, which exists
  locally but not in CI's install layout (the prior CI flattened
  the toolchain into `$HOME/.cyrius/lib/` instead of the
  `versions/$CYRIUS_VERSION/lib/` shape cyrius's shadow-lib check
  expects). With both in `[deps].stdlib`, `cyrius deps` lands
  them in `./lib/` deterministically.
- **CI install step rewritten** (`.github/workflows/ci.yml`,
  `release.yml`) to use the canonical `scripts/install.sh`
  installer instead of unpacking the release tarball by hand.
  The hand-rolled flow flattened the toolchain into
  `$HOME/.cyrius/lib/` (or `$HOME/.cyrius/versions/$VER/lib/`
  without the `~/.cyrius/lib → versions/$VER/lib` symlink),
  whichever variant we tried. `cyrius deps` reads stdlib from
  `~/.cyrius/lib/`, and without the symlink it fails on every
  `[deps].stdlib` entry — `4 deps resolved, 22 errors` /
  `cannot read ./lib/<name>.cyr`. The canonical installer is
  what `cyriusly install <ver>` calls, lays out
  `versions/$VER/{bin,lib}/` + `current` + the `bin`/`lib`
  symlinks correctly, and is the single source of truth that
  stays in sync with future cyrius releases.
- **Explicit `cyrius deps` + `cyrius deps --verify` steps** before
  any compile in both CI and release. The lockfile-hash gate
  short-circuits with a warning until `cyrius.lock` lands in-tree
  (one push grace period for fresh deps). Mirrors the agnosys
  pattern.
- **`cyrius.cyml [package].version = "${file:VERSION}"`** — `VERSION`
  is now the single source of truth; the manifest interpolates
  from it. Matches agnosys; eliminates the manifest/VERSION drift
  failure mode that `scripts/version-bump.sh` had to paper over.
  The docs-job version-consistency check now asserts the
  interpolation literal is present instead of comparing two
  redundant strings.
- **Release archive now ships `cyrius.lock`** alongside the
  source tarball, dist bundle, prebuilt binary, and SHA256SUMS.
  Consumers pinning libro by tag can verify the resolved-deps
  graph matches what was released.

### Verified

- `CYRIUS_DCE=1 cyrius build src/main.cyr build/libro` clean.
- `./build/libro` — **373 passed, 0 failed** (unchanged from 2.0.5).
- All three benchmark binaries build + run on cyrius 5.10.34
  (`libro_bench_core`, `libro_bench_io`, `libro_bench_proof`).
- `./build/fuzz_libro` — all 12 harnesses survive 100-iteration
  runs (no crashes).
- `cyrfmt --check src/*.cyr` clean.
- `cyrius lint src/*.cyr` clean (0 warnings across 22 modules).

### Known

- A new cyrius 5.10 build note flags `cwd ./lib/ shadows
  version-pinned /home/<...>/.cyrius/versions/5.10.35/lib/`.
  Informational — silence with `CYRIUS_NO_WARN_SHADOW_LIB=1` or
  ignore. The deps-resolved `./lib/` is the one we want; the
  toolchain-bundled snapshot is older.

## [2.0.5] - 2026-04-19

Continuing through the unblocked-work list surfaced in the post-2.0.4
audit. Two items shipped, one item surfaced a new bug that's filed
as open work, and the roadmap was re-scoped against the upstream
ecosystem reality (sigil has TPM primitives already; Cyrius keccak
is stalled behind Windows-target work and bug-pass priority).

### Added
- **Third bench binary** `benches/libro_proof.bcyr` — deferred from
  2.0.2 when the proof-path benches wouldn't fit in either existing
  bench binary under cc5 5.4.2's 16384 fixup-table cap. Ships two
  proof-build benchmarks (`proof_build_unsigned_25`,
  `proof_build_signed_25`). Iteration counts are deliberately low
  (3) because each iteration allocates an iproof + merkle tree +
  N inclusion proofs via the bump allocator; higher counts push
  heap pressure into multi-GB territory. Baseline perf on this
  machine: 373 µs unsigned / 1.886 ms signed.
- **Seven more `#derive(accessors)` layout-invariant tests** in
  `src/main.cyr` covering the shape spectrum beyond the 2.0.4
  trio: `entry` (11 fields + inline UUID), `signing_key` (6),
  `verifying_key` (2, baseline), `entry_sig` (5), `merkle_tree`
  (2), `sth` (6), `filestore` (3 — pins the 2.0 +cpath field).
  Each test writes sentinels via raw offsets and asserts the
  derived accessors read them back. **316 → 350 tests** (+34
  assertions).
- **Bench-context `proof_to_json` hang** filed as an open hardening
  item in `docs/development/roadmap.md`. Every attempt to measure
  `proof_to_json(ip)` inside `bench_run` caused `main()` to re-enter
  repeatedly at ~25 Hz. The function itself works correctly in the
  test suite (all `test_proof_to_json_*` pass). Isolated to the
  combination of `proof_json.cyr` include + a call site inside a
  bench. Shipped `libro_proof.bcyr` without the `proof_to_json`
  benches pending root cause.

### Changed
- **CI raw-offset per-file allowlist** extended for `src/main.cyr`
  to register the seven new `*_layout` test locals
  (`entry_layout`, `sk_layout`, `vk_layout`, `es_layout`,
  `mt_layout`, `sth_layout`, `fs_layout`) as intentional
  raw-offset probes.
- **Roadmap restructured.** Release detail collapsed into a brief
  history table; "Rust features port — complete" section removed
  (all three items shipped in 2.0); "MCP tools via bote" removed
  (lives in bote's repo); ecosystem-blocked items re-reviewed
  against upstream roadmaps — named unblockers corrected where
  they'd diverged from reality. The pre-2.0.5 roadmap listed
  "TPM-backed chain sealing" as blocked on sigil, but sigil 2.8.4
  already ships `src/tpm.cyr` wrapping agnosys 1.0.0's TPM
  primitives (not bundled in `dist/sigil.cyr` because TPM requires
  agnosys as a separate dep). TPM is now categorized as unblocked,
  pending a libro-side architectural decision slated for 2.1.0.
  ML-DSA post-quantum signing is still blocked but now honestly
  described: Cyrius `lib/keccak.cyr` was originally 5.2.x scope
  but was pushed back behind Windows-target work and the current
  bug/issue pass, no near-term ETA.
- **Full documentation audit** — CLAUDE.md counts updated (316 →
  350 tests, 22 → 24 benches across 3 binaries, ~440 → ~445 KB);
  README + quickstart + testing + dependency-watch all reconciled
  against 2.0.5 reality. Previously stale: bench count, test count,
  binary size, the "only 2 bench binaries" description.
- **Threat model addendum** — the previous "TPM backing (blocked)"
  residual-risk row updated to "unblocked, not integrated" with a
  pointer to the 2.1.0 roadmap item.

### Deferred
- **Offset-map raw-offset guard.** Originally considered as a third
  CI gate to cross-check raw-offset `N` values against the set of
  valid offsets in `#derive(accessors)` layouts per file. Analyzed
  and deferred: the existing specific-struct guard (2.0.1/2.0.2) +
  per-file allowlist (2.0.4) already catch ~99% of the regression
  class; a third guard would require a small parser for
  `#derive` declarations and would add CI complexity for a narrow
  additional catch (out-of-bounds offsets on params already in the
  per-file allowlist). Will revisit if the need becomes concrete.

### Validation
- **350 tests, 0 failed** (316 → 350: +34 assertions from seven
  new layout-invariant tests).
- 24 benches across 3 binaries, all report clean. 11 fuzz targets,
  all clean. Lint + format clean. Dist regenerated at `v2.0.5`.
- All four CI gates dry-run green: manifest completeness,
  specific-struct raw-offset guard, per-file allowlist,
  dist-freshness.

### Upcoming
- **2.0.6** — `proof_from_json` round-trip + dedicated fuzz target.
  Paired JSON parser for archival workflows where consumers save
  a signed proof and re-verify it later without the original chain.
- **2.1.0** — TPM-backed `WitnessAnchor` sealing via agnosys +
  sigil.tpm integration. Minor bump because it adds a new optional
  dep.

## [2.0.4] - 2026-04-19

Scaffold-hardening sprint with zero src/* library changes — five
threads, all concentrated in CI gates, tests, and long-lived
documentation. The goal was to lock in the 2.0 design decisions as
durable ADRs, formalize the threat model, and extend the CI safety
net so future regressions of the classes caught in 2.0.0 / 2.0.2 /
2.0.3 can't recur.

### Added
- **ADR 0005 — `#derive(accessors)` adoption** (`docs/adr/`).
  Documents why 2.0 reversed v1.2.0's REJECT decision: the
  UUID-zeroing bug surfaced during 2.0 made the safety case
  explicit, agnosys convention shifted, and cc5 5.4.7 stabilized
  derive generation. Codifies the UUID-placeholder pattern and the
  "raw offsets inside defining file" convention.
- **ADR 0006 — `dist/libro.cyr` committed-artifact contract**
  (`docs/adr/`). The invariants that make `cyrius distlib` safe:
  `[lib] modules ⊇ main.cyr includes`, four-way version parity
  (VERSION / cyrius.cyml / dist header / git tag), why the
  `SIG_ALG_ED25519` warning is expected (stdlib/sigil/patra
  precede libro in consumer include order). Rooted in Finding 1
  from 2.0.0's hardening pass.
- **ADR 0007 — Nested / scalar-aware canonical-JSON hashing**
  (`docs/adr/`). The 2.0 breaking change explained: 1.x quoted
  every JSON value as a string, silently collapsing `{"n": 42}`
  and `{"n": "42"}` into identical hashes (a latent
  second-preimage primitive). Describes the recursive byte-walker
  that replaces it and spells out the re-verification expectation
  for consumers with non-string / nested `details` in 1.x chains.
- **Threat model** (`docs/development/threat-model.md`). Replaces
  the 40-line Rust-era stub with a Cyrius/2.0.3-era version:
  trust-boundary diagram, asset table, 12 numbered threats with
  mitigation / residual-risk / status per class, supply-chain pin
  list, review cadence. Absorbs findings from the 2.0 audit as
  mitigation precedents.
- **CI bench-history wiring** (`.github/workflows/ci.yml`). The
  `LIBRO_BENCH_HISTORY` + `LIBRO_BENCH_TAG` infrastructure shipped
  in 2.0 but CI wasn't consuming it — we had no perf trend across
  cuts. Now every CI run emits one CSV row per bench tagged with
  the commit SHA, uploaded as an `actions/upload-artifact@v4`
  workflow artifact with 90-day retention. Local dry-run confirms
  14 rows from `libro_core` alone.
- **CI raw-offset allowlist gate** (`.github/workflows/ci.yml`).
  Per-file registry of parameter names that may appear in raw
  `load64(X + N)` / `store64(X + N, …)` / `load64(X)` form.
  Complements the specific-struct guard from 2.0.1/2.0.2: that
  one covers 7 unambiguous (struct, param) pairs; this one closes
  the ambiguous-single-letter case by requiring per-file
  registration of every raw-offset param name. A new raw-offset
  site on an unregistered name fails CI — forcing the author
  either to use a derived accessor (preferred) or to opt into the
  allowlist with a justification comment. The gate flagged my own
  layout-test locals on first dry-run, which is the correct
  behavior.
- **Struct-layout invariant tests** in `src/main.cyr` — three
  tests covering the `#derive(accessors)` shape spectrum: chain
  (4 fields, no UUID), iproof (6 fields, no UUID), anchor
  (9 fields, 16-byte inline-UUID prefix). Each test writes
  sentinel values via raw offsets and asserts the derived
  accessors return them, then writes via derived setters and
  asserts raw-offset reads see them. Guards against a future
  Cyrius `#derive(accessors)` compiler regression that emits
  offsets inconsistently. The tests' locals are named
  `chain_layout` / `iproof_layout` / `anchor_layout` (not `c` /
  `ip` / `a`) to avoid triggering the specific-struct guard;
  they're explicitly registered in the allowlist with a comment.

### Validation
- **316 tests, 0 failed** (294 → 316: +22 asserts from three
  layout-invariant tests).
- 22 benches, 11 fuzz targets — all clean.
- All four CI gates dry-run green: manifest completeness,
  specific-struct raw-offset guard, per-file allowlist,
  dist-freshness.
- No `src/*.cyr` library-code changes — dist byte-identical to
  2.0.3 except for the version-header line.
- Lint + format clean; binary 440 KB (was 437 KB — +3 KB from
  the three layout-test functions).

## [2.0.3] - 2026-04-19

Fuzz-driven hardening sprint. Three new fuzz targets were added for
2.0-era surfaces that had no dedicated coverage (`chain_import`,
`filestore_verify_streamed`, the nested canonical-JSON hasher). The
second immediately surfaced a HIGH-severity denial-of-service bug in
the streaming verifier.

### Fixed
- **`filestore_verify_streamed` infinite loop on unterminated input
  (HIGH, fuzz-caught).** The outer streaming loop
  `while (eof == 0 || buf_fill > 0)` had no exit path when EOF was
  reached with residual bytes that contained no newline — i.e., any
  truncated or unterminated JSONL file. EOF set `eof = 1`, the inner
  scan found no `\n` and consumed nothing, `buf_fill > 0` kept the
  outer condition true, and the loop spun forever. The shared
  `file_lock_shared` was also never released, compounding into a
  cross-process lock leak. Fix: inject a synthetic `\n` at
  `_fsvs_buf + buf_fill` when `eof == 1 && consumed == 0 &&
  buf_fill > 0 && buf_fill < _fsvs_buf_size`, letting the existing
  scan loop pick up the residual as a final line and exit via the
  normal path. Adjacent line-too-long check broadened to fire
  regardless of EOF state, so an exactly-64KB terminal line still
  returns an error rather than silently dropping data. The 2.0
  validation missed this because all three existing
  `test_filestore_verify_streamed*` tests built fixtures via
  `filestore_append`, which always writes a trailing `\n` — the
  residual-after-EOF path was never exercised. `filestore_load_all`
  happens to handle the case correctly (its scan naturally exits
  when `pos >= total`), so the bug is unique to the streaming path.
  See `docs/audit/2026-04-19-audit-2.0.md` Finding 4.

### Added
- **`test_filestore_verify_streamed_unterminated_tail`** in
  `src/main.cyr` — writes 26 bytes of junk with no trailing newline,
  runs `filestore_verify_streamed`, and asserts it returns (rather
  than hangs). The assertion-content is nominal; the regression
  signal is reachability. If the fix is reverted, CI times out on
  this test instead of hanging indefinitely.
- **Three new fuzz targets** in `fuzz/fuzz_libro.fcyr` (8 → 11):
  - `fuzz_chain_import` — random JSONL bytes through the
    `chain_import` meta-header + entry parser. Clean (no crashes).
  - `fuzz_filestore_verify_streamed` — random JSONL bytes through
    the streaming verifier. Caught the HIGH bug above.
  - `fuzz_canonical_json_hash` — random `details` payloads through
    `entry_new` + `entry_compute_hash`, exercising the 2.0 nested
    canonical-JSON byte-walker. Clean.
  The fuzz binary was also missing `include "src/chain_io.cyr"`;
  added so the new `chain_import` target can link.

### Validation
- **294 tests, 0 failed** (293 → 294: +1 unterminated-tail
  regression test).
- 11 fuzz targets (was 8), all clean under the default iteration
  counts, full harness completes in well under 10 s.
- 22 benches across 2 binaries, all report. Lint + format clean.
  Dist regenerated at `v2.0.3` (4477 lines).

## [2.0.2] - 2026-04-19

Continuing the P(-1) hardening cadence started in 2.0.0/2.0.1. Extends
the 2.0.1 raw-offset guard from `struct chain` to six more derived
structs, fixes one cross-module raw-offset reader the 2.0 sweep
missed, and strengthens the `proof_to_json` tests so an offset-typo
regression would actually break a test (not just silently produce
wrong JSON).

### Fixed
- **`src/proof_json.cyr:84-87` was reading `iproof` via raw offsets**
  (`load64(ip)`, `load64(ip + 16)`, `load64(ip + 24)`, `load64(ip + 32)`)
  — same Finding-2 class as the 2.0.0 chain_io.cyr / review.cyr
  migrations, surviving the sweep because the 2.0.1 CI guard only
  covered `struct chain`. Migrated to `iproof_tree_head` /
  `iproof_inclusions` / `iproof_entries` / `iproof_anchor`. Behavior
  preserved (all prior `test_proof_to_json_*` assertions still pass);
  the extended CI guard below now enforces the invariant.

### Added
- **Extended CI raw-offset guard** in `.github/workflows/ci.yml` —
  covers 6 more `#derive(accessors)` structs with unambiguous
  cross-codebase parameter names: `ip` (iproof), `sk` (signing_key),
  `vk` (verifying_key), `es` (entry_sig), `mp` (merkle_proof),
  `cp` (consistency). Now iterates a `(struct, defining_file, param)`
  rule table via a helper function, so adding a new rule is a
  one-line change. Rules were selected by dry-running each candidate
  param name across `src/*.cyr` and keeping only those that map
  unambiguously to a single struct (e.g. `a` is used for `anchor`,
  `archive`, *and* `ts_attestation` across different files — not safe
  to add without a finer-grained check).
- **`test_proof_to_json_fields_present`** in `src/main.cyr` — builds
  an unsigned proof with inclusions over a 3-entry chain, calls
  `proof_to_json`, and asserts the output contains the JSON markers
  for all four iproof-backed sections (`tree_head`, `root`,
  `tree_size`, `entries`, `inclusions`, `anchor`). Would have caught
  any of the 4 offset-typo regressions fixed above — the prior two
  tests only checked start/end braces and the null case.

### Deferred
- `proof_to_json` benchmark — an attempt to add it to
  `benches/libro_io.bcyr` overflowed cc5's 16384 fixup-table cap
  (same constraint that forced the 1.2.0 core/io split). `libro_core.bcyr`
  excludes `src/proof_json.cyr` on purpose per 2.0's rationale
  ("modules lives separately so bench binaries that exercise proof
  verification without the JSON dep can exclude it and stay under
  the cap"). Clean path is a third bench binary
  (`benches/libro_proof.bcyr`) with a minimal include set — filed as
  a 2.0.3+ follow-up.

### Validation
- **293 tests, 0 failed** (286 → 293: +7 asserts from
  `test_proof_to_json_fields_present`).
- 22 benches across 2 binaries; fuzz clean; lint + format clean;
  dist regenerated at `v2.0.2-dev` (4477 lines).

## [2.0.1] - 2026-04-19

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

[2.0.5]: https://github.com/MacCracken/libro/compare/2.0.4...2.0.5
[2.0.4]: https://github.com/MacCracken/libro/compare/2.0.3...2.0.4
[2.0.3]: https://github.com/MacCracken/libro/compare/2.0.2...2.0.3
[2.0.2]: https://github.com/MacCracken/libro/compare/2.0.1...2.0.2
[2.0.1]: https://github.com/MacCracken/libro/compare/2.0.0...2.0.1
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
