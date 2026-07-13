# Libro — Claude Code Instructions

> **READ `DEPS-PATTERN.md` AT REPO ROOT BEFORE TOUCHING BUILD OR
> RELEASE.** libro ships to downstream Cyrius projects via a
> committed `dist/libro.cyr` produced by `cyrius distlib`. That
> is the only distribution contract. Patra is the reference.
> Do not invent alternatives.

## Project Identity

**Libro** (Italian: book) — Cryptographic audit chain — tamper-proof SHA-256 hash-linked event logging and verification

- **Type**: Cyrius library (single-file compilation via `include`)
- **License**: GPL-3.0-only
- **Version**: 2.8.0 (2026-07-13)
- **Language**: [Cyrius](https://github.com/MacCracken/cyrius) 6.4.62 (pin in `cyrius.cyml` `cyrius = "..."` field)
- **Genesis repo**: [agnosticos](https://github.com/MacCracken/agnosticos)
- **Philosophy**: [AGNOS Philosophy & Intention](https://github.com/MacCracken/agnosticos/blob/main/docs/philosophy.md)
- **Standards**: [First-Party Standards](https://github.com/MacCracken/agnosticos/blob/main/docs/development/applications/first-party-standards.md)
- **Recipes**: [zugot](https://github.com/MacCracken/zugot) — takumi build recipes

## Consumers

daimon (audit), aegis (security events), stiva (container lifecycle), sigil (trust), ark (package ops)

## Current State

- **Source**: 21 library modules in `[lib] modules` + 1 opt-in module (`src/tpm_anchor.cyr` behind `-D LIBRO_TPM`); `cyrius deps` resolves stdlib + sigil + patra (agnosys dropped at the agnosys → agnodrm decomposition — TPM now sourced from sigil ≥ 3.9.0)
- **Benchmarks**: 33 across three binaries (`libro_core.bcyr` 18 + `libro_io.bcyr` 12 + `libro_proof.bcyr` 3 — split because cc5 5.4.2's 16384 fixup-table cap; `libro_proof` gained `proof_to_json_25` in 2.7.2 once cyrius 6.1.23 cleared the long-standing bench-context hijack)
- **Fuzz**: 1 harness (`fuzz/fuzz_libro.fcyr`, 12 targets)
- **Tests**: 502 default / 514 with `-D LIBRO_TPM` (all pass)
- **Binary**: ~14 MB (default; LIBRO_TPM similar) **as of cyrius 6.4.x** — but only ~1 MB is `.text`; the ~13 MB is zero-init `.bss` from 6.4.x promoting sigil's banked `crypto_scratch` array locals off the stack into a shared global (see quirk #9). Under 6.0.x–6.3.x it was ~1.1 MB. The `.bss` is lazily mapped (negligible RSS), and `dist/libro.cyr` is source not a binary — size is a `build/libro` metric only. DCE still NOPs dead code (~540 KB NOPed) but doesn't shrink the file; `CYRIUS_DCE=1` and a plain build are byte-identical.
- **Distribution artifact**: committed `dist/libro.cyr` — produced by `cyrius distlib`, ~5.5k lines. See `DEPS-PATTERN.md` for the contract.

## Dependencies

- **sigil** — SHA-256, Ed25519, HMAC, hex, constant-time compare (Cyrius stdlib `lib/sigil.cyr`)
- **patra** — SQL-backed storage (pinned v1.12.9 via `[deps.patra]` tag; resolved into `lib/patra.cyr` by `cyrius deps` from upstream `dist/patra.cyr` — `lib/` is gitignored, the tag pin is the contract)
- **sakshi** — structured tracing (Cyrius stdlib)

No external deps beyond the Cyrius toolchain.

## Build & Test

```bash
# Build (DCE matches CI/release)
CYRIUS_DCE=1 cyrius build src/main.cyr build/libro

# Run tests — expect "502 passed, 0 failed"
./build/libro

# Benchmarks (three binaries — cc5 fixup table limit forced the core/io split
# in 1.2.0; libro_proof.bcyr was added to host proof-path benches without
# pushing the others over the cap)
CYRIUS_DCE=1 cyrius build benches/libro_core.bcyr  build/libro_bench_core  && ./build/libro_bench_core
CYRIUS_DCE=1 cyrius build benches/libro_io.bcyr    build/libro_bench_io    && ./build/libro_bench_io
CYRIUS_DCE=1 cyrius build benches/libro_proof.bcyr build/libro_bench_proof && ./build/libro_bench_proof

# Fuzz
CYRIUS_DCE=1 cyrius build fuzz/fuzz_libro.fcyr build/fuzz_libro && timeout 10 ./build/fuzz_libro

# Format + lint
cyrfmt --check src/*.cyr
for f in src/*.cyr; do cyrius lint "$f"; done
```

## Development Process

### P(-1): Scaffold Hardening (before any new feature work)

Defined in the first-party standards
[example_claude.md §P(-1)](https://github.com/MacCracken/agnosticos/blob/main/docs/development/applications/example_claude.md).
TL;DR: read roadmap + CHANGELOG + open issues → clean build + lint + tests
→ baseline benchmarks → internal deep review → external research → security
audit (file findings in `docs/audit/YYYY-MM-DD-audit.md`) → additional tests
from findings → post-review benchmarks → docs audit → repeat if heavy.

### Work Loop (continuous)

1. Work phase — roadmap item, feature, bug fix
2. Build check: `CYRIUS_DCE=1 cyrius build src/main.cyr build/libro`
3. Test: `./build/libro` — must report `0 failed`
4. Lint + format: `cyrius lint src/*.cyr`, `cyrfmt --check src/*.cyr`
5. Bench additions for new code
6. Run benchmarks, compare to baseline
7. Audit — perf, memory, security, correctness
8. Docs — CHANGELOG, roadmap, ADRs if applicable
9. Version sync — `VERSION`, `cyrius.cyml`, CHANGELOG header (`scripts/version-bump.sh`)
10. Back to step 1

### Task Sizing

- **Low/Medium**: batch freely — multiple items per cycle
- **Large**: one change at a time, verify each
- **If unsure**: treat as large

### Key Principles

- **Tests + benchmarks + fuzz are the way.** Numbers or it didn't happen.
- **Own the stack.** No deps beyond the Cyrius toolchain.
- **No magic.** Every op is measurable, auditable, traceable.
- **Sakshi tracing.** All key operations instrumented via sakshi (stderr profile).
- **`fl_alloc` for structs with individual lifetimes; `alloc` for long-lived collections.**
- **`str_from` / `str_new` wrap pointers, they don't copy.** If the cstr comes from an ephemeral buffer (patra result set, file-read buffer), copy before wrapping. See `_ps_copy_cstr` in `src/patra_store.cyr` and Finding 1 of `docs/audit/2026-04-19-audit.md`.
- **`match` is a reserved keyword** — don't use as a variable name.
- **Globals for workaround state.** cc3 clobbered locals across calls; cc5 is better but not perfect. When in doubt, global.

## Project Structure

```
src/main.cyr            Entry point — 263 inline tests + 20 module includes
src/*.cyr               Library modules (20 files: error, hasher, entry, verify,
                        query, retention, chain, store, export, review, merkle,
                        signing, anchoring, timestamping, proof, kernel_audit,
                        file_store, patra_store, streaming, proof_json)
benches/libro_core.bcyr 18 core benchmarks (crypto/chain/merkle/sign/batch)
benches/libro_io.bcyr   12 i/o benchmarks (export/review/anchor/stream/filestore)
benches/libro_proof.bcyr 3 proof-path benchmarks (build unsigned/signed + to_json)
dist/libro.cyr          Consumer distribution artifact (cyrius distlib)
fuzz/fuzz_libro.fcyr    Fuzz harnesses (no-crash assertions)
tests/                  Standalone repros (patra_standalone.cyr, etc.)
lib/                    Vendored Cyrius stdlib + patra v1.11.2 bundle
build/                  Compiled binaries (gitignored)
scripts/version-bump.sh Syncs VERSION + cyrius.cyml
docs/                   Architecture, guides, compliance, ADRs, audit reports
```

## Documentation Structure

```
Root files (required):
  README.md, CHANGELOG.md, CLAUDE.md, CONTRIBUTING.md,
  SECURITY.md, CODE_OF_CONDUCT.md, LICENSE, VERSION,
  cyrius.cyml

docs/ (required):
  architecture/overview.md — module map, data flow, consumers
  development/roadmap.md — completed, backlog, future
  guides/quickstart.md — getting started
  guides/testing.md — test + benchmark guide
  guides/integration.md — consumer patterns

docs/ (when earned):
  audit/YYYY-MM-DD-audit.md — security audit reports
  adr/ — architecture decision records
  development/threat-model.md, dependency-watch.md
  compliance/standards-mapping.md
```

## CI / Release

- **Toolchain pin**: `cyrius` field inside `cyrius.cyml` (currently `cyrius = "6.4.62"`). CI and release workflows extract it via `grep -E '^cyrius[[:space:]]*=' cyrius.cyml | sed ...` — no separate toolchain file, no hardcoded version strings in YAML.
- **Manifest**: `cyrius.cyml` (was `cyrius.toml` through v1.0.4; renamed in 1.1.0 to match first-party convention).
- **DCE**: every `cyrius build` in CI and release runs with `CYRIUS_DCE=1`. Binary size is a release metric.
- **Tag filter**: release workflow triggers on `tags: ['[0-9]*']` — semver-only.
- **Version-verify gate**: release asserts `VERSION == cyrius.cyml version == git tag` before building.
- **Docs gate**: CI verifies VERSION matches cyrius.cyml and appears as `[x.y.z]` heading in CHANGELOG.

## DO NOT

- **Do not commit or push** — the user handles all git operations (commit, push, tag)
- **NEVER use `gh` CLI** — use `curl` to GitHub API only
- Do not add dependencies beyond the Cyrius toolchain
- Do not skip benchmarks before claiming performance improvements
- Do not commit `build/`
- Do not hardcode Cyrius version in CI YAML — read the `cyrius = "..."` field from `cyrius.cyml`

## Known Cyrius Compiler Quirks (6.4.62)

> ## ⚠️ BIG NOTE — sigil 3.6.0 needs `lib/thread_local.cyr` before it (or SIGILL)
>
> Since 2.7.1 (sigil 3.6.0), `lib/thread_local.cyr` **must be included
> before `lib/sigil.cyr`** (it is, in `src/main.cyr`, right after
> `thread.cyr`; and listed in `[deps] stdlib`). sigil 3.6.0's
> `crypto_scratch` banks per-thread crypto working arrays over cyrius
> 6.0.52 TLS and calls `thread_local_init/get/set`. **Omit it and the
> binary LINKS FINE but SIGILLs at runtime** (exit 132, zero output) —
> it does NOT fail to compile, so a build-only check won't catch it.
> Always run the suite, not just build. If a future stdlib/sigil bump
> reintroduces a bare SIGILL on startup, a missing TLS-prerequisite
> include is the first suspect.
>
> **This applies to EVERY harness that includes `lib/sigil.cyr`, not
> just `src/main.cyr`.** The three benches (`benches/libro_*.bcyr`),
> the fuzz harness (`fuzz/fuzz_libro.fcyr`), and the sigil-using
> standalone repros (`tests/patra.cyr`, `tests/patra_standalone.cyr`,
> `tests/fixup_limit_repro.cyr`) each need their own
> `include "lib/thread_local.cyr"` before `lib/sigil.cyr` — they don't
> inherit main.cyr's. They went un-updated from 2.7.1 until **2.7.4**,
> when sigil 3.7.14 first exercised the TLS `crypto_scratch` path those
> harnesses hit and CI's bench/fuzz **run** step started core-dumping
> (build was always clean — the tell is `warning: undefined function
> 'thread_local_init/set/get'` in the build log just before the SIGILL).
> When adding a new sigil-using harness, copy main.cyr's
> `thread.cyr → thread_local.cyr → … → sigil.cyr` ordering.
>
> *(Resolved, for history: the `-D LIBRO_TPM` per-file `#derive` cap —
> see quirk #4. cyrius 6.0.53 raised it 64 → 512, so `tpm_anchor` is
> back to `#derive(accessors)` and the 2.6.5 hand-written-accessor
> workaround is gone as of 2.7.1.)*

Most cc3-era workarounds documented in earlier libro versions are now resolved.
Quirks still worth knowing:

1. **Local variable clobbering** — still possible across deeply nested call chains. Not a guaranteed bug, but if a local's value looks wrong after a function call, try promoting it to a global as a workaround. Several `_ps_*` globals in `src/patra_store.cyr` exist for this reason.
2. **Freelist vs bump allocator discipline** — `fl_alloc` + `fl_free` for individually-freed structs; `alloc()` for long-lived collections. Mixing them is correct but easy to reason about wrong.
3. **Single-pass compiler** — forward references across function boundaries work via fixups (cap 16384 in 5.4.2, up from 8192), but include order still matters for type/struct visibility.
4. **Per-file `#derive` struct cap — now 512 (was 64; raised in 6.0.53).** A single compilation unit (main.cyr + all its `include`s, flattened) may carry at most **512** `#derive(...)` structs. libro is nowhere near that. *History:* through 6.0.51 the cap was **64**, and libro's `-D LIBRO_TPM` build (agnosys's 39 `#derive` + libro's 27 + a derived `tpm_anchor`) tripped it, so 2.6.5 hand-wrote `tpm_anchor`'s accessors. 6.0.53 raised the cap to 512 (verified: 512 builds, 513 fails `error: too many #derive structs in one file (max 512)`), and 2.7.1 restored `#derive(accessors)` on `tpm_anchor` and dropped the workaround. *Footnote:* 2.6.5 originally mis-attributed the 64-cap failure to the separate 256-entry type/struct *table* cap (6.0.51 raised that one to 1024); the `#derive` cap was always the real blocker. Upstream record: cyrius `docs/development/issues/2026-06-03-derive-struct-cap-64-is-real-tpm-blocker.md` (resolved by the 64 → 512 raise); original mis-attribution archived at `archived/2026-05-28-type-table-256-cap-silent-fail.md`.
5. **TLS-backed stdlib modules must precede their consumers.** `lib/thread_local.cyr` (cyrius ≥ 6.0.52) installs per-thread storage via the CPU thread-pointer register; modules that bank state over it (sigil 3.6.0's `crypto_scratch`) must be `include`d *after* it. Wrong order links cleanly but SIGILLs at first use. See the BIG NOTE above.
6. **A missing `include` is now a HARD ERROR (cyrius ≥ 6.1.35).** Earlier toolchains soft-skipped an `include` whose file was absent; 6.1.35 aborts the build with `error: cannot open include file: <path>`. This surfaced in the 2.7.3 bump. sigil's `dist/sigil.cyr` *inlines* the sha_ni / aes_ni modules but **intentionally** retains an opt-in `include "src/sha_ni.cyr"` / `include "src/aes_ni.cyr"` — libs are opt-in: a source-tree consumer that includes only `src/sha256.cyr` relies on that line to pull in the hardware-dispatch infra. In sigil 3.7.8 those opt-in includes were **unguarded**, so inside the bundle (where the file is absent from the fold) they soft-skipped on 6.1.23 but hard-error on 6.1.35. **Fix is the sigil 3.7.10 bump** — it `#ifndef`-guards them (`_SIGIL_SHA_NI_INCLUDED` / aes_ni marker) and `#define`s the marker where the bundle inlines the module, so the redundant include self-skips. This is the correct fix for the dual consumption model, not a workaround — there is no distlib bug. If a future dep bump reintroduces `cannot open include file: src/*.cyr`, the dep shipped an *unguarded* opt-in include — bump the dep to a guarded release, don't vendor the missing file.
7. **stdlib `bayan` / `ganita` carves (cyrius 6.1.25+).** The 6.1.x line consolidated standalone stdlib modules into bundled dists: **`bayan`** absorbs `json` / `bigint` / `base64` / `csv` / `toml` / `cyml` / `u128`; **`ganita`** absorbs `matrix` / `linalg` / `math_advanced`. The old single-module files (`lib/json.cyr`, `lib/bigint.cyr`, …) no longer ship in the snapshot, so `include "lib/json.cyr"` fails (see quirk 6). libro `include`s `lib/bayan.cyr` and lists `"bayan"` in `[deps] stdlib` (2.7.3). Each bundle carries a `_compat.cyr` shim forwarding the **legacy `json_*` / `bigint_*` names**, so call sites need no change — but the shim is a deprecation-window courtesy; prefer the canonical `bayan_*` prefix for new code.
8. **Bare `ERR_*` error-enum names — duplicate-symbol warning (6.2.11) → namespace lint note (6.4.x).** libro's `src/error.cyr` enum uses bare `ERR_*` names (`ERR_IO`, `ERR_UNKNOWN`, …). (a) **6.2.11 linker** emitted `warning: duplicate symbol '<NAME>' redefined with conflicting value (last definition wins)` when the ported **agnosys** error enum defined the same names — benign; each resolved consistently within a build. **agnosys was dropped at the agnosys → agnodrm decomposition (TPM now sourced from sigil), so that specific collision is gone** and the warning no longer fires. (b) **6.4.x lint** now emits a `note` on `src/error.cyr:5-6` proposing leaf libs prefix their enum (`LIBRO_ERR_*`) to avoid the flat enum-const namespace reserved for the sakshi base logger (proposal `2026-07-11-error-enum-namespace-lint-gate`). Still **informational** — a `note`, not a warning/error; CI's lint step is non-fatal (`cyrius lint … || true`) and the suite passes 502 / 514. Surfaced in the 2.8.0 (6.4.62) refresh. Only act (rename the enum to `LIBRO_ERR_*` across all call sites + the CI raw-offset allowlist) if it graduates from note → enforced error, or a value-dependent test breaks.
9. **Oversized array locals promoted to zero-init `.bss` (cyrius 6.4.x).** 6.4.x no longer keeps a large fixed-size `var X[N]` array local on the stack when it exceeds the per-fn stack budget — it puts it in a **shared global** (`.bss`), emitting `note: oversized array local kept in shared global (not per-thread) - exceeds per-fn stack budget; use alloc()` plus `warning: large static data (N bytes)`. In libro's fold the trigger is **sigil's banked `crypto_scratch`** (`var X[N * SIGIL_CRYPTO_BANKS]`, 64 banks), so the default build's binary jumped **~1.1 MB → ~14 MB** at the 6.4.62 bump — **all in `.bss`** (`.text` stays ~1 MB; `size build/libro` shows `bss ≈ 13046672`). The `.bss` is zero-filled and lazily mapped by the OS (negligible resident memory unless touched). Benign: it is a codegen policy, not a libro bug — sigil's scratch sizes are byte-identical 3.9.8 → 3.11.1 — and the shipped artifact `dist/libro.cyr` is source, not a binary. Only the `build/libro` size metric moved. Surfaced in 2.8.0.

### Resolved (stop treating as bugs in 5.4.2)

- `\r` escape sequence — **works** since 4.x. Don't hand-emit byte 13.
- Negative literals `-1`, `-N` — **work** since 3.10.3. No need for `(0 - N)`.
- Compound assignment `+=`, `-=`, `*=`, etc. — **work** since 3.10.3.
- Undefined functions — now a **compile-time error**, not a silent NULL stub (was Bug #26 source).
- 256-initialized-global cap — **removed**.
- Fixup table cap — **raised to 16384** (was 8192).
- 256-entry type/struct table cap — **raised to 1024** in 6.0.51 (was 256). Distinct from the per-file `#derive` cap in quirk 4; was never the TPM-build blocker.
- Per-file `#derive` cap — **raised to 512** in 6.0.53 (was 64). This *was* the real `-D LIBRO_TPM` blocker; the 2.6.5 `tpm_anchor` hand-written-accessor workaround was removed in 2.7.1. See quirk 4.
