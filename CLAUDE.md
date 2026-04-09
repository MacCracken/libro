# Libro — Claude Code Instructions

## Project Identity

**Libro** (Italian: book) — Cryptographic audit chain — tamper-proof SHA-256 hash-linked event logging and verification

- **Type**: Cyrius library (single-file compilation via `include`)
- **License**: GPL-3.0-only
- **Version**: 1.0.0 (2026-04-09)
- **Language**: [Cyrius](https://github.com/MacCracken/cyrius) 2.7.2+
- **Genesis repo**: [agnosticos](https://github.com/MacCracken/agnosticos)
- **Philosophy**: [AGNOS Philosophy & Intention](https://github.com/MacCracken/agnosticos/blob/main/docs/philosophy.md)
- **Standards**: [First-Party Standards](https://github.com/MacCracken/agnosticos/blob/main/docs/development/applications/first-party-standards.md)
- **Recipes**: [zugot](https://github.com/MacCracken/zugot) — takumi build recipes

## Consumers

daimon (audit), aegis (security events), stiva (container lifecycle), sigil (trust), ark (package ops)

## Development Process

### Build & Test

```bash
# Compile
cyrius build src/main.cyr build/libro

# Run tests (193 tests)
./build/libro

# Run benchmarks (15 benchmarks)
cyrius build benches/libro.bcyr build/libro_bench && ./build/libro_bench

# Format check
cyrfmt --check src/*.cyr
```

### Development Loop (continuous)

1. Work phase — new features, roadmap items, bug fixes
2. Compile check: `cyrius build src/main.cyr build/libro`
3. Test: `./build/libro` — all 193 tests must pass
4. Format: `cyrfmt --check src/*.cyr`
5. Benchmark additions for new code
6. Run benchmarks: `cyrius build benches/libro.bcyr build/libro_bench && ./build/libro_bench`
7. Audit phase — review performance, memory, security, correctness
8. Deeper tests/benchmarks from audit observations
9. Run benchmarks again — prove the wins
10. If audit heavy → return to step 7
11. Documentation — update CHANGELOG, roadmap, docs
12. Version check — VERSION and cyrius.toml in sync (`scripts/version-bump.sh`)
13. Return to step 1

### Key Principles

- **Never skip benchmarks.** Numbers don't lie.
- **Tests + benchmarks are the way.** 193 tests, 15 benchmarks.
- **Own the stack.** Zero external dependencies — Cyrius stdlib only.
- **No magic.** Every operation is measurable, auditable, traceable.
- **Sakshi tracing.** All key operations instrumented via sakshi (stderr profile).
- **Globals for cross-call state.** Cyrius single-pass compiler clobbers locals across function calls — use globals when values must survive nested calls.
- **Raw bytes for CR LF.** Cyrius does not support `\r` escape — use `store8(buf, 13); store8(buf+1, 10)` for network protocols.
- **`fl_alloc` for structs, `alloc` for hashmaps.** Freelist supports individual free; bump allocator for long-lived collections.
- **Compiler fixup limit: 8192.** Split large programs across multiple compilation units.
- **`match` is a keyword** — do not use as a variable name.
- **`assert_eq` with values >127 corrupts counters** — use `assert(x == val, "name")` instead.

## Project Structure

```
src/main.cyr           Entry point + 193 inline tests
src/*.cyr              Library modules (18 files)
benches/libro.bcyr     15 benchmarks
lib/                   Vendored Cyrius stdlib
build/                 Compiled binaries (gitignored)
scripts/               version-bump.sh
docs/                  Architecture, guides, compliance, ADRs
```

## Documentation Structure

```
Root files (required):
  README.md, CHANGELOG.md, CLAUDE.md, CONTRIBUTING.md, SECURITY.md, CODE_OF_CONDUCT.md, LICENSE

docs/ (required):
  architecture/overview.md — module map, data flow, consumers
  development/roadmap.md — completed, backlog, future
  guides/quickstart.md — getting started
  guides/testing.md — test and benchmark guide
  guides/integration.md — consumer patterns

docs/ (reference):
  development/threat-model.md, dependency-watch.md
  compliance/standards-mapping.md
  adr/ — architecture decision records
```

## DO NOT
- **Do not commit or push** — the user handles all git operations (commit, push, tag)
- **NEVER use `gh` CLI** — use `curl` to GitHub API only
- Do not add unnecessary dependencies — Cyrius stdlib only
- Do not skip benchmarks before claiming performance improvements
- Do not commit `build/`

## Known Cyrius Compiler Issues

1. **Local variable clobbering** — function parameters and locals may be overwritten by nested function calls. Workaround: save critical values to globals before calling other functions.
2. **`map_get` after `map_set` in same call chain** — hashmap lookups may fail to find entries set in deeply nested call contexts. Workaround: restructure to minimize call depth between set and get.
3. **No `\r` escape sequence** — use raw byte 13 for carriage return in network protocols.
4. **Fixup table limit (8192)** — programs with more than ~8192 forward references fail to compile. Split into multiple compilation units.
