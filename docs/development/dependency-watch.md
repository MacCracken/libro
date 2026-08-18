# Dependency Watch

Libro's external surface is intentionally small: the Cyrius toolchain,
sigil (crypto), patra (SQL), and the stdlib modules bundled with the
toolchain release. This document tracks what we pull in, how it pins,
and what to watch when upgrading.

## External dependencies

| Dep | Pin field | Current | Resolved by | Purpose |
|-----|-----------|---------|-------------|---------|
| Cyrius toolchain | `cyrius.cyml` `cyrius = "…"` | **6.5.27** | `~/.cyrius/bin/cyriusly install …` (canonical `scripts/install.sh`) | Compiler + bundled stdlib |
| sigil            | `cyrius.cyml` `[deps.sigil] tag = "…"`     | **3.12.9** | `cyrius deps` → `lib/sigil-mldsa.cyr` + `lib/sigil_{sha_ni,sha256,hex}.cyr` | SHA-256, Ed25519, ML-DSA-65, hybrid verify, hex. **Thin sub-surface, not the monolithic `dist/sigil.cyr`** (see below) |
| sigil (tpm)      | `cyrius.cyml` `[deps.sigil_tpm] tag = "…"` | **3.12.9** | `cyrius deps --features tpm` → `lib/sigil_tpm_sigil-tpm.cyr` | TPM 2.0 primitives (`tpm_seal` / `tpm_unseal` / `tpm_detect`). **Optional** — activated only by the `tpm` feature for the `-D LIBRO_TPM` build |
| patra            | `cyrius.cyml` `[deps.patra] tag = "…"`     | **1.13.8** | `cyrius deps` → `lib/patra.cyr` | SQL storage + prepared statements + group commit + STR btree indexes |

Zero third-party crates. No transitive graph to audit.

**Two structural changes since the 2.7.x line, both landed in 2.8.0:**

1. **`agnosys` is gone.** libro's only agnosys surface was the TPM primitives used by `src/tpm_anchor.cyr`. The agnosys → agnodrm decomposition (2026-06-19) moved the trust stack into sigil, which promoted TPM to first-class at 3.9.0 — so TPM now resolves from the sigil pin and the separate `[deps.agnosys]` entry was removed. `src/tpm_anchor.cyr` is unchanged: same symbol names, different source.
2. **The sigil surface is thin, and that is load-bearing.** `cyrius build` auto-includes every *active* `[deps.*]` module, so a fat dep dist lands in every binary whether or not `src/*.cyr` includes it. The monolithic `dist/sigil.cyr` inlines an x509/RSA/authenticode fold carrying ~13 MB of static `.bss` libro never calls — it put `build/libro` at ~14 MB. `[deps.sigil]` therefore pulls only `dist/sigil-mldsa.cyr` + `src/{sha_ni,sha256,hex}.cyr`, and TPM sits behind the optional `tpm` feature so default builds never link it. `.bss` is **80,152 B** at 2.8.3. Do not "simplify" this back to the monolith. See CLAUDE.md quirk #9 — including the trap that any isolation test must be built **outside** the project dir, or the manifest auto-include silently pulls the full surface into your control build too.

## Cyrius stdlib modules used

These ship with the Cyrius toolchain and land in `~/.cyrius/versions/<ver>/lib/`.
`cyrius.cyml` `[deps] stdlib = […]` mirrors what `src/main.cyr`
includes so the manifest stays load-bearing.

| Module        | Purpose |
|---------------|---------|
| `alloc`       | Bump allocator for long-lived buffers |
| `assert`      | Test assertions |
| `atomic`      | Atomic primitives — sigil / patra dep |
| `bayan`       | Bundled data-format dist (cyrius 6.1.25 carve) — supplies `json` parse/build (canonical-JSON hashing) and `bigint` (timestamping DER); also bundles base64/csv/toml/cyml/u128. Back-compat shim forwards legacy `json_*`/`bigint_*` names. |
| `bench`       | Nanosecond benchmarking harness |
| `chrono`      | Civil date arithmetic, epoch-to-RFC-3339 conversion |
| `ct`          | Constant-time helpers (`ct_eq_bytes`, `ct_select`) — sigil dep |
| `fmt`         | Integer / hex formatting |
| `fnptr`       | Function-pointer calls (`fncall0`–`fncall8`) — bench dispatch |
| `freelist`    | Segregated freelist for struct allocation / free |
| `fs`          | Filesystem helpers (path ops, stat) — sigil/patra dep |
| `hashmap`     | String-keyed hash table (FNV-1a) |
| `io`          | File I/O (`file_open`, `read`, `write`, `close`, flock) |
| `keccak`      | SHA-3 / SHAKE-128/256 — sigil ML-DSA-65 dep (2.2.0) |
| `process`     | Subprocess helpers — sigil TPM dep (`tpm2-tools` shell-out) |
| `random`      | `getrandom(2)` wrapper (2.1.1 hardening: replaces `/dev/urandom` reads) |
| `sakshi`      | Structured tracing — stderr profile |
| `slice`       | Byte-slice views — sigil / patra dep |
| `str`         | Managed strings (fat pointer: data + len) |
| `string`      | C-string helpers (`strlen`, `memcpy`, `memset`) |
| `sync`        | Synchronisation primitives — sigil / patra dep |
| `syscalls`    | Linux syscall wrappers (per-arch dispatched) |
| `tagged`      | Tagged unions (Result, Option) — sigil dep |
| `test`        | `test_each` table-driven harness (held in deps but not yet exercised) |
| `thread`      | Mutex / spawn primitives — sigil parallel-batch-verify dep |
| `thread_local`| Per-thread storage (cyrius ≥ 6.0.52). **Must precede the sigil folds** — sigil's `crypto_scratch` banks over it, and wrong order links clean but SIGILLs at first crypto use (exit 132, no output). Also supplies `thread_local_alloc`, which sigil ≥ 3.12.1 and patra ≥ 1.12.12 both call — hence the **cyrius ≥ 6.4.65 floor** those deps impose |
| `vec`         | Dynamic vector |

The dep list grew significantly in 2.1.0 to satisfy sigil 3.0.1's bundle requirements (`ct`, `keccak`, `thread`, `tagged`, `process`, `fs`, `string`); 2.1.1 added `random` (getrandom wrapper) and `test` (table-driven test helpers).

## Upgrade considerations

### Cyrius toolchain
- CI reads the pin from `cyrius.cyml`, so bumping the field + running
  the canonical installer (`scripts/install.sh` via curl, or
  `cyriusly install <ver>` which calls it) is enough to change the
  toolchain.
- Every toolchain bump needs a full test + fuzz + bench pass (518
  default / 530 with `-D LIBRO_TPM`, 12 fuzz targets, 33 benches
  across three binaries) because codegen changes can surface subtle
  behavioral deltas. 2.1.0 jumped 5.4.7 → 5.10.34 in one step —
  pulled `secret var`, `getrandom`, `lib/ct.cyr`, `lib/keccak.cyr`,
  `lib/random.cyr`, and the canonical installer flow. 2.7.2 crossed
  the 6.0 → 6.1 minor line (6.0.53 → 6.1.23) with zero source
  migrations; 2.8.3 jumped 17 patch releases (6.4.66 → 6.4.83),
  also with zero source migrations.
- **A green suite is not sufficient evidence for a toolchain bump.**
  cyrius 6.4.80 fixed a CRITICAL constant-fold defect that had been
  live since well before libro's 6.4.66 pin: a PEXPR-tier constant
  expression silently dropped its left operand when a literal
  subtraction went negative (`1 - 2 + 3` → `5`), miscompiling 10 % of
  systematic 3-term expressions while every upstream gate stayed
  green. Libro's 502/514 passed identically either side of the fix.
  The bump was cleared by scanning `src/`, `benches/` and `fuzz/` for
  the failing expression **shape** (comments and string literals
  stripped) and finding zero occurrences — not by the suite. Do the
  same for any future bump that names a silent-wrong-value fix.
- **Check capacity headroom, don't assume it.** `CYRIUS_STATS=1
  CYRIUS_DCE=1 cyrius build src/main.cyr <out>` reports `fn_table`,
  `identifiers` and `code_size` against their ceilings. At 2.8.3 libro
  is at `fn_table 2167 / 32768` and `identifiers 58749 / 524288`. This
  matters because 6.4.75 fixed a P0 where `fn_table` growth past 8192
  silently corrupted six fn-indexed side tables and the DCE `live[]`
  bitmap cleared only 1/4 its size — libro was never in range, but a
  harness that grows past 8192 fns on an older toolchain would be.
- Watch the **fixup-table cap** — cc5 5.4.2 raised it to 16384 (from
  8192 in cc3); 5.10.x preserved it. All three bench binaries sit
  comfortably under the cap (the 2.0.5 split into core/io/proof gave
  enough headroom for the 2.2/2.3 PQ + hybrid additions). The bench
  binaries are deliberately split (ADR 0006 notes the constraint).

### sigil
- Provides all cryptographic primitives libro uses. A sigil CVE
  propagates directly.
- 3.0.0 (May 2026) shipped the full FIPS 204 ML-DSA-65 stack
  (`src/mldsa*.cyr`, 8 modules) plus `sigil_verify_hybrid` —
  unblocked the libro 2.2 / 2.3 PQ work. The dist bundle expects the
  consumer to supply the stdlib surface (ct / keccak / thread /
  tagged / process / fs / string), which is why libro's `[deps]
  stdlib` list grew in 2.1.0.
- Upgrade path: bump the tag in **both** `[deps.sigil]` and
  `[deps.sigil_tpm]` (they must stay in lockstep — same repo, same
  tag), run `cyrius deps` to resync the thin folds, rebuild + retest +
  rerun fuzz. The `fuzz_sig_verify` and `fuzz_sha256` targets exercise
  the Ed25519 + SHA-256 surface; ML-DSA / hybrid coverage lives in
  `src/main.cyr` test groups "Signing (ML-DSA-65)" and
  "Signing (Hybrid Ed25519+ML-DSA-65)". Verify the TPM path separately
  with `cyrius deps --features tpm` + `cyrius build --features tpm -D
  LIBRO_TPM`, then restore the default resolution with a bare
  `cyrius deps`.
- **Watch the module list, not just the tag.** The pin names specific
  files (`dist/sigil-mldsa.cyr`, `src/{sha_ni,sha256,hex}.cyr`,
  `dist/sigil-tpm.cyr`). A sigil release that renames or re-folds any
  of them breaks resolution even though the tag bump looks routine —
  confirm the paths exist at the new tag before pinning.
- sigil ≥ 3.12.1 calls stdlib `thread_local_alloc`, so it imposes a
  **cyrius ≥ 6.4.65 floor**; on an older snapshot the build fails at
  link with `undefined function 'thread_local_alloc'`. patra ≥ 1.12.12
  imposes the identical floor. Bump the toolchain and these deps
  together.
- sigil's SHA-256, Ed25519, and ML-DSA-65 are pure-Cyrius
  implementations; none is FIPS 140-3 validated (see
  `docs/compliance/standards-mapping.md` §FIPS 140-3).
- Sigil 3.0.0 shipped parallel-batch-verify infrastructure
  (`sv_verify_batch`) but the workers serialized on a full-call mutex
  through the 3.0–3.5 line (correctness-only, ~1× serial throughput).
  **sigil 3.6.0 (libro 2.7.1) made it truly parallel** — dropped the
  per-call mutex over cyrius 6.0.52 thread-local storage (~3.42× at
  64 artifacts / 4 workers). Libro inherits the speedup on its
  batch-verify path with no API change; the former roadmap
  ecosystem-blocked item is retired.

### patra
- Provides SQL storage for `PatraStore` (`src/patra_store.cyr`).
- 1.7–1.9 shipped the perf surface libro 2.4.0 wired up:
  STR-keyed btree indexes (1.7.0), group-commit sync modes (1.8.0),
  prepared statements (1.8.2), aarch64-portable syscall wrappers
  (1.9.1–1.9.3). PatraStore's `patrastore_append_batch`,
  `patrastore_set_sync_mode`, `patrastore_create_source_index`, and
  the transparent prepared-SELECT/COUNT wiring all depend on these.
- Upgrade path: bump `[deps.patra] tag`, `cyrius deps`, rebuild +
  retest. PatraStore has two test groups in `src/main.cyr` ("PatraStore"
  + "PatraStore (perf tier — 2.4.0)") covering append / load /
  verify / query / transaction / persistence / sync-mode /
  batch-correctness / prepared statements / indexed by_source —
  these should all pass before a patra upgrade is considered clean.
- patra's own fuzz harness + test suite run in the patra repo at the
  `patra-core` level; libro's pin gives you a known-stable baseline.

### agnosys — RETIRED (2.8.0)

No longer a libro dependency. Libro's only agnosys surface was the TPM
2.0 primitives (`tpm_detect`, `tpm_seal`, `tpm_unseal`) used by the
opt-in `src/tpm_anchor.cyr`. The agnosys → agnodrm decomposition
(2026-06-19) moved the trust stack into sigil, which promoted TPM to
first-class at 3.9.0 — so `[deps.agnosys]` was removed and TPM now
resolves from `[deps.sigil_tpm]`. `src/tpm_anchor.cyr` needed no
change: the symbol names are identical, only the source moved.

Two consequences worth keeping straight:

- The old "default builds still pull `lib/agnosys.cyr` and rely on DCE
  to strip it" behaviour is **gone**. TPM is now an *optional* dep
  behind the `tpm` feature, so with the feature off it is not cloned,
  not copied, and not auto-included — the default binary carries no
  TPM surface at all rather than a DCE-stripped one.
- The bare `ERR_*` duplicate-symbol collision between libro's error
  enum and agnosys's went away with the dep. Libro's enum was
  independently namespaced to `LIBRO_ERR_*` in 2.8.2 for the separate
  6.4.x lint note; see CLAUDE.md quirk #8.

### stdlib modules
- Shipped atomically with the toolchain; a stdlib behavior change
  comes with a toolchain bump and is covered by the toolchain
  upgrade process above.
- If a stdlib module's API changes across a toolchain bump (rare —
  stdlib is stable post-1.0), `CYRIUS_DCE=1 cyrius build` surfaces
  it at compile time.

## Crypto primitives history

- **v1.0.0 → v1.0.1**: libro shipped its own SHA-256 (`src/sha256.cyr`)
  and HMAC-SHA256 as a signing-key placeholder. Zero crypto deps.
- **v1.0.2**: migrated SHA-256, Ed25519, hex, and `ct_eq` to sigil.
  `src/sha256.cyr` deleted; `src/hasher.cyr` became a thin delegator.
  Real Ed25519 signing replaced the HMAC placeholder.
- **v1.1.0+**: patra (SQL) joined as a second external dep for
  `PatraStore`. Full heap-based key zeroization landed.
- **v2.0**: nested scalar-aware canonical JSON landed (ADR 0007).
  No new crypto deps; just a corrected deterministic serializer
  feeding sigil's SHA-256.
- **v2.1.1**: signing key generation + timestamp nonce gathering
  switched from `/dev/urandom` to `getrandom(2)` via `lib/random.cyr`;
  the entropy buffer migrated to `secret var` for compiler-guaranteed
  zeroize-on-return.
- **v2.2.0**: ML-DSA-65 entry signing (NIST FIPS 204) landed via
  sigil 3.0.0. `EntrySignature.algorithm` dispatches between
  Ed25519 (slot 0) and ML-DSA-65 (slot 1). Sign + verify path
  polymorphic over the algorithm field.
- **v2.3.0**: Hybrid Ed25519 + ML-DSA-65 entry signing. `signing_key`
  / `verifying_key` / `entry_sig` structs gained slot-2 fields for
  the second algorithm; verify wraps `sigil_verify_hybrid` in
  AND-mode.
- **v2.5.0**: opt-in TPM-sealed `WitnessAnchor` via
  `src/tpm_anchor.cyr` (build with `-D LIBRO_TPM`). agnosys's
  `tpm_seal`/`tpm_unseal` primitives were the backend at the time;
  default builds don't link this surface. (Backend re-sourced to
  sigil in 2.8.0 — see below.)
- **v2.7.1 → 2.7.2**: no new primitives — toolchain/dependency
  refresh. sigil 3.6.0 made `sv_verify_batch` truly parallel (lock-
  free over cyrius 6.0.52 TLS); sigil 3.6.0+ requires
  `lib/thread_local.cyr` included before `lib/sigil.cyr` (else the
  binary links but SIGILLs at first crypto use). 2.7.2 advanced the
  stack to cyrius 6.1.23 / sigil 3.7.8 / patra 1.11.0 / agnosys
  1.4.1 with no source-logic change.
- **v2.7.3**: no new primitives — toolchain refresh to cyrius
  **6.1.35** + sigil **3.7.10** (patra 1.11.0 / agnosys 1.4.1
  already latest). The sigil bump is **required**: cyrius 6.1.35
  hard-errors on a missing `include`, and sigil 3.7.8's dist carried
  *unguarded* opt-in `include "src/sha_ni.cyr"` / `src/aes_ni.cyr`
  lines (the intended path for source-tree consumers) that 3.7.10
  `#ifndef`-guards so the bundle's redundant include self-skips. Also
  migrated the stdlib `json`/`bigint`
  includes to the bundled **`bayan`** dist (6.1.25 carve); the
  back-compat shim keeps `json_*`/`bigint_*` call sites unchanged.
- **v2.8.0**: no new primitives — a **dependency-shape** change. TPM
  re-sourced from agnosys to sigil (3.9.0 promoted it to first-class)
  and `[deps.agnosys]` dropped; the sigil surface thinned from the
  monolithic `dist/sigil.cyr` to `dist/sigil-mldsa.cyr` +
  `src/{sha_ni,sha256,hex}.cyr`, with TPM moved behind an optional
  `tpm` feature. Cut `build/libro` ~14 MB → ~724 KB by dropping the
  x509/RSA fold's ~13 MB of static `.bss` that libro never called.
- **v2.8.2**: toolchain 6.4.62 → **6.4.66**, sigil 3.11.1 → **3.12.1**,
  patra 1.12.9 → **1.12.12** — a *coupled* bump: both deps call stdlib
  `thread_local_alloc`, which first ships in cyrius 6.4.63. Libro's
  error enum namespaced `ERR_*` → `LIBRO_ERR_*`.
- **v2.8.3**: no new primitives and **no source change** — toolchain
  6.4.66 → **6.4.83** (17 patch releases). sigil **3.12.1** / patra
  **1.12.12** re-confirmed as the newest published tags, so the dep
  pins are unchanged. The bump's value is upstream correctness, chiefly
  the `_cfo` constant-fold class fixed across 6.4.74 / 6.4.80 / 6.4.81
  (see the toolchain notes above) and the 6.4.75 `fn_table` / DCE P0 —
  neither of which libro was in range of, both verified rather than
  assumed.

## Watch list (ecosystem)

Items the roadmap tracks as blocked on upstream capability (from
`docs/development/roadmap.md`):

- **Post-quantum signatures (ML-DSA-65, FIPS 204)** — ✅ **shipped
  in libro 2.2.0** via sigil 3.0.0. Unblocker chain was Cyrius
  stdlib `lib/keccak.cyr` (delivered v5.7.x) → sigil 3.0
  `src/mldsa*.cyr` (8 modules, May 2026) → libro 2.2.0 dispatch.
- **Hybrid signing (Ed25519 + ML-DSA-65)** — ✅ **shipped in libro
  2.3.0** via sigil 3.0.0's `sigil_verify_hybrid`. AND-mode
  verification gates both primitives.
- **TPM-backed chain sealing** — ✅ **shipped in libro 2.5.0** via
  agnosys 1.0.4's `tpm_seal`/`tpm_unseal`. Opt-in
  `src/tpm_anchor.cyr` behind `-D LIBRO_TPM`; default builds keep
  no agnosys-TPM surface linked.
- **Parallel batch verify** — ✅ **shipped via sigil 3.6.0 (libro
  2.7.1)**. The 3.0 `sv_verify_batch` infrastructure serialized on a
  full-call mutex; 3.6.0 dropped it over cyrius 6.0.52 thread-local
  storage for true parallelism (~3.42× at 64 artifacts / 4 workers).
  Libro inherits the speedup with no API change.
- **Multi-node chain sync** — blocked on an AGNOS-level federation
  protocol; libro would gain a second meta-chain layer over the
  existing WitnessAnchor primitive.
