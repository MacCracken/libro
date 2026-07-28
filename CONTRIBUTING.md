# Contributing to Libro

## Prerequisites

- [Cyrius](https://github.com/MacCracken/cyrius) compiler — version is
  pinned in `cyrius.cyml` (`cyrius = "…"` field; currently **6.4.83**).
  Install with:
  ```bash
  cyriusup install "$(grep -E '^cyrius[[:space:]]*=' cyrius.cyml | sed -E 's/.*"([^"]+)".*/\1/')"
  ```
- Linux x86_64 (Cyrius compiles to static ELF; libro is ELF-only).
- Git.

Run `cyrius deps` once after cloning if the `lib/` symlinks don't resolve —
those pull from `[deps.sigil]` / `[deps.patra]` in `cyrius.cyml`. Note that
libro resolves a **thin** sigil surface (`lib/sigil-mldsa.cyr` plus
`lib/sigil_{sha_ni,sha256,hex}.cyr`), not a single `lib/sigil.cyr`; the TPM
fold is optional and only appears under `cyrius deps --features tpm`.

## Development Workflow

```bash
# Build (CYRIUS_DCE=1 matches CI/release)
CYRIUS_DCE=1 cyrius build src/main.cyr build/libro

# Run tests — expect "502 passed, 0 failed"
./build/libro

# Fuzz (12 targets, no-crash asserts)
CYRIUS_DCE=1 cyrius build fuzz/fuzz_libro.fcyr build/fuzz_libro && \
    timeout 30 ./build/fuzz_libro

# Bench
CYRIUS_DCE=1 cyrius build benches/libro_core.bcyr build/libro_bench_core && \
    ./build/libro_bench_core
CYRIUS_DCE=1 cyrius build benches/libro_io.bcyr   build/libro_bench_io   && \
    ./build/libro_bench_io

# Format + lint
cyrfmt --check src/*.cyr fuzz/*.fcyr
for f in src/*.cyr; do cyrius lint "$f"; done

# Regenerate the consumer-distribution artifact if src/* changed
cyrius distlib
# → commit dist/libro.cyr alongside your src/ changes
```

CLAUDE.md at the repo root describes the full P(-1) + sprint workflow
the project follows.

## Code Style

- Format with `cyrfmt` before committing.
- Crypto primitives come from sigil — don't reintroduce per-crate
  SHA-256 / Ed25519. The `hasher` module delegates; use it.
- Use `fl_alloc` for structs with individual lifetimes,
  `alloc` for long-lived buffers / collections.
- Use `#derive(accessors)` on every new `struct`. Cross-module readers
  **must** use the generated accessors; raw `load64(X + N)` /
  `store64(X + N, …)` is OK only inside the defining file (see ADR 0005
  and the CI raw-offset guards).
- Wrap pointers from ephemeral buffers (patra result sets, file-read
  buffers) before stashing on structs — `str_from` / `str_new` **share**
  ownership, they don't copy. See `_ps_copy_cstr` in
  `src/patra_store.cyr`.
- Don't use `match` as a variable name (reserved keyword).
- `assert(x == val, "name")` rather than `assert_eq` for values > 127.

## Adding a Module

1. Create `src/module_name.cyr`.
2. Add `include "src/module_name.cyr"` to `src/main.cyr` (respect
   dependency order).
3. **Add it to `[lib] modules` in `cyrius.cyml`.** The CI manifest-
   completeness gate fails if `src/main.cyr` includes a file not in
   the manifest (drift caught 2.0.0's missing-dist incident — see
   `docs/audit/2026-04-19-audit-2.0.md` Finding 1).
4. Add tests in `src/main.cyr`.
5. Add benchmarks if performance-sensitive (`libro_core.bcyr` for
   crypto / chain / merkle / sign, `libro_io.bcyr` for
   export / review / anchor / stream / filestore). Watch the cc5
   16384 fixup-table cap.
6. Regenerate `dist/libro.cyr` via `cyrius distlib` and commit it.
7. Update documentation (README module table, ADR if a design
   decision warrants one).

## Adding a Raw-Offset Site

The CI guards will reject new `load64(X + N)` / `store64(X + N, …)` /
`load64(X)` sites unless either:

- The pair `(param_name, file)` is already in the per-file allowlist
  in `.github/workflows/ci.yml`, **or**
- The file is the defining file for the struct whose layout `X` uses,
  and the param name matches the specific-struct guard's registered
  name for that struct.

If you need a new raw-offset site outside these, the first choice is
**to use a derived accessor instead** — that's the whole point of ADR
0005. If you genuinely need raw access (e.g., a layout-invariant test),
extend the allowlist with a comment explaining why.

## Testing

All tests live inline in `src/main.cyr`. Pattern:

```cyrius
fn test_my_feature() {
    var c = chain_new();
    chain_append(c, SEV_INFO, str_from("src"), str_from("act"),
        str_from("{}"));
    assert(chain_len(c) == 1, "my feature works");
}
```

Then register in `main()`:

```cyrius
test_group("My Feature");
test_my_feature();
```

Any PR that changes behavior must also add a test for it. See
`docs/guides/testing.md` for the full category list.

## Tracing

Use sakshi for structured logging on key operations. The byte-length
argument is the ASCII length of the literal — keep it accurate so the
stderr profile is parseable:

```cyrius
sakshi_info("module: operation",  17);
sakshi_debug("module: detail",    14);
sakshi_error("module: failure",   15);
sakshi_warn("module: warning",    15);
```

## Pull Requests

- One logical change per PR.
- All tests pass (`0 failed`). Fuzz must be clean; lint 0 warnings.
- Benchmarks should not regress without justification. Attach the
  `bench-history.csv` artifact from CI if a perf impact is
  deliberate.
- Update CHANGELOG.md under `[<next-version>-dev] - unreleased`
  (or the next pending release heading) **only if the change will
  ship across multiple commits**. For single-commit changes that
  ship in one tag cut, the CHANGELOG entry can wait until the cut
  itself — see the cadence in recent releases.
- Update docs if the public API changes.
- Regenerate `dist/libro.cyr` if `src/*` changed.

## Release Cadence

Releases are cut by the maintainer. VERSION stays at the last shipped
value during work; when the batch is ready, bump to the new release
number, add the CHANGELOG heading, regenerate the dist, tag, push.
The `-dev` suffix is only used if mid-work commits to `main` are
required before the full batch is ready. See the memory/preference
in `CLAUDE.md` for details.

## License

By contributing, you agree that your contributions will be licensed
under GPL-3.0-only.
