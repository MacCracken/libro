# Libro — Claude Code Instructions

## Project Identity

**Libro** (Italian: book) — Cryptographic audit chain — tamper-proof SHA-256 hash-linked event logging and verification

- **Type**: Cyrius library (single-file compilation via `include`)
- **License**: GPL-3.0-only
- **Version**: 1.1.1 (2026-04-19)
- **Language**: [Cyrius](https://github.com/MacCracken/cyrius) 5.4.2+ (pin in `cyrius.cyml` `cyrius = "..."` field)
- **Genesis repo**: [agnosticos](https://github.com/MacCracken/agnosticos)
- **Philosophy**: [AGNOS Philosophy & Intention](https://github.com/MacCracken/agnosticos/blob/main/docs/philosophy.md)
- **Standards**: [First-Party Standards](https://github.com/MacCracken/agnosticos/blob/main/docs/development/applications/first-party-standards.md)
- **Recipes**: [zugot](https://github.com/MacCracken/zugot) — takumi build recipes

## Consumers

daimon (audit), aegis (security events), stiva (container lifecycle), sigil (trust), ark (package ops)

## Current State

- **Source**: 18 modules under `src/`, plus vendored stdlib + patra bundle under `lib/`
- **Tests**: 251 assertions (all pass — 19 PatraStore + Gap-coverage tests relanded in 1.1.0)
- **Benchmarks**: 21
- **Fuzz**: 1 harness (`fuzz/fuzz_libro.fcyr`, 8 targets)
- **Binary**: ~384 KB (DCE-built)

## Dependencies

- **sigil** — SHA-256, Ed25519, HMAC, hex, constant-time compare (Cyrius stdlib `lib/sigil.cyr`)
- **patra** — SQL-backed storage (bundled v1.1.1 at `lib/patra.cyr`, resynced from upstream `dist/patra.cyr`)
- **sakshi** — structured tracing (Cyrius stdlib)

No external deps beyond the Cyrius toolchain.

## Build & Test

```bash
# Build (DCE matches CI/release)
CYRIUS_DCE=1 cyrius build src/main.cyr build/libro

# Run tests — expect "251 passed, 0 failed"
./build/libro

# Benchmarks
CYRIUS_DCE=1 cyrius build benches/libro.bcyr build/libro_bench && ./build/libro_bench

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
src/main.cyr            Entry point — 251 inline tests + 18 module includes
src/*.cyr               Library modules (18 files: error, hasher, entry, verify,
                        query, retention, chain, store, export, review, merkle,
                        signing, anchoring, timestamping, proof, kernel_audit,
                        file_store, patra_store, streaming)
benches/libro.bcyr      21 benchmarks
fuzz/fuzz_libro.fcyr    Fuzz harnesses (no-crash assertions)
tests/                  Standalone repros (patra_standalone.cyr, etc.)
lib/                    Vendored Cyrius stdlib + patra v1.1.1 bundle
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

- **Toolchain pin**: `cyrius` field inside `cyrius.cyml` (currently `cyrius = "5.4.2"`). CI and release workflows extract it via `grep -E '^cyrius[[:space:]]*=' cyrius.cyml | sed ...` — no separate toolchain file, no hardcoded version strings in YAML.
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

## Known Cyrius Compiler Quirks (5.4.2)

Most cc3-era workarounds documented in earlier libro versions are now resolved.
Quirks still worth knowing:

1. **Local variable clobbering** — still possible across deeply nested call chains. Not a guaranteed bug, but if a local's value looks wrong after a function call, try promoting it to a global as a workaround. Several `_ps_*` globals in `src/patra_store.cyr` exist for this reason.
2. **Freelist vs bump allocator discipline** — `fl_alloc` + `fl_free` for individually-freed structs; `alloc()` for long-lived collections. Mixing them is correct but easy to reason about wrong.
3. **Single-pass compiler** — forward references across function boundaries work via fixups (cap 16384 in 5.4.2, up from 8192), but include order still matters for type/struct visibility.

### Resolved (stop treating as bugs in 5.4.2)

- `\r` escape sequence — **works** since 4.x. Don't hand-emit byte 13.
- Negative literals `-1`, `-N` — **work** since 3.10.3. No need for `(0 - N)`.
- Compound assignment `+=`, `-=`, `*=`, etc. — **work** since 3.10.3.
- Undefined functions — now a **compile-time error**, not a silent NULL stub (was Bug #26 source).
- 256-initialized-global cap — **removed**.
- Fixup table cap — **raised to 16384** (was 8192).
