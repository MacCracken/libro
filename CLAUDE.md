# Libro — Claude Code Instructions

## Project Identity

**Libro** (Italian: book) — Cryptographic audit chain — tamper-proof SHA-256 hash-linked event logging and verification

- **Type**: Cyrius library (single-file compilation via `include`)
- **License**: GPL-3.0-only
- **Version**: SemVer 2.0.0
- **Language**: [Cyrius](https://github.com/MacCracken/cyrius) (ported from Rust v0.92.0)
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

# Run tests
cyrius test

# Run benchmarks (with history tracking)
cyrius bench

# Full audit: self-host, test, fmt, lint, vet, deny, bench
cyrius audit

# Policy enforcement
cyrius deny src/main.cyr
```

### Development Loop (continuous)

1. Work phase — new features, roadmap items, bug fixes
2. Compile check: `cyrius build src/main.cyr build/libro`
3. Test: `cyrius test` — all suites must pass
4. Lint + format: `cyrius fmt --check`, `cyrius lint`
5. Policy check: `cyrius deny src/main.cyr`
6. Benchmark additions for new code
7. Run benchmarks: `cyrius bench` (tracks history automatically)
8. Audit phase — review performance, memory, security, correctness
9. Deeper tests/benchmarks from audit observations
10. Run benchmarks again — prove the wins
11. If audit heavy → return to step 8
12. Full audit: `cyrius audit` — self-host, test, fmt, lint, vet, deny, bench
13. Documentation — update CHANGELOG, roadmap, docs
14. Version check — VERSION and cyrius.toml in sync (`scripts/version-bump.sh`)
15. Return to step 1

### Key Principles

- **Never skip benchmarks.** Numbers don't lie.
- **Tests + benchmarks are the way.** Target 80%+ coverage.
- **Own the stack.** Zero external dependencies — Cyrius stdlib only.
- **No magic.** Every operation is measurable, auditable, traceable.
- **Globals for cross-call state.** Cyrius single-pass compiler clobbers locals across function calls — use globals when values must survive nested calls.
- **Raw bytes for CR LF.** Cyrius does not support `\r` escape — use `store8(buf, 13); store8(buf+1, 10)` for network protocols.
- **`fl_alloc` for structs, `alloc` for hashmaps.** Freelist supports individual free; bump allocator for long-lived collections.
- **Compiler fixup limit: 8192.** Split large programs across multiple compilation units.
- **`match` is a keyword** — do not use as a variable name.
- **`assert_eq` with values >127 corrupts counters** — use `assert(x == val, "name")` instead.

## Project Structure

```
src/main.cyr           Entry point + core tests
src/*.cyr              Library modules
tests/*.tcyr           Test suites
benches/*.bcyr         Benchmarks
fuzz/*.fcyr            Fuzz harnesses
examples/              Usage examples
lib/                   Vendored Cyrius stdlib (28 modules)
build/                 Compiled binaries (gitignored)
```

## Documentation Structure

```
Root files (required):
  README.md, CHANGELOG.md, CLAUDE.md, CONTRIBUTING.md, SECURITY.md, CODE_OF_CONDUCT.md, LICENSE

docs/ (required):
  architecture/overview.md — module map, data flow, consumers
  development/roadmap.md — completed, backlog, future

docs/ (when earned):
  guides/ — usage guides, integration patterns
  development/ — semver, threat model
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
