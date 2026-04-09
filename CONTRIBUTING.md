# Contributing to Libro

## Prerequisites

- [Cyrius](https://github.com/MacCracken/cyrius) compiler 2.7.2+
- Linux x86_64 (Cyrius compiles to static ELF)
- Git

## Development Workflow

```bash
# Build
cyrius build src/main.cyr build/libro

# Run tests (193 tests, must all pass)
./build/libro

# Run benchmarks
cyrius build benches/libro.bcyr build/libro_bench && ./build/libro_bench

# Format check
cyrfmt --check src/*.cyr

# Full audit cycle
cyrius build src/main.cyr build/libro && ./build/libro
```

## Code Style

- Format with `cyrfmt` before committing
- No external dependencies — Cyrius stdlib only
- Use `fl_alloc` for structs (supports individual free), `alloc` for buffers
- Use globals for values that must survive nested function calls (compiler constraint)
- Do not use `match` as a variable name (reserved keyword)
- Use `assert(x == val, "name")` instead of `assert_eq` for values >127

## Adding a Module

1. Create `src/module_name.cyr`
2. Add `include "src/module_name.cyr"` to `src/main.cyr` (respect dependency order)
3. Add tests in `src/main.cyr` (test functions + entries in `main()`)
4. Add benchmarks in `benches/libro.bcyr` if performance-sensitive
5. Update documentation

## Testing

All tests are inline in `src/main.cyr`. Add test functions following the pattern:

```cyrius
fn test_my_feature() {
    # Setup
    var c = chain_new();
    chain_append(c, SEV_INFO, str_from("src"), str_from("act"), str_from("{}"));

    # Assert
    assert(chain_len(c) == 1, "my feature works");
}
```

Then add to `main()`:

```cyrius
test_group("My Feature");
test_my_feature();
```

## Tracing

Use sakshi for structured logging on key operations:

```cyrius
sakshi_info("module: operation", 17);   # INFO level
sakshi_debug("module: detail", 14);     # DEBUG level
sakshi_error("module: failure", 15);    # ERROR level
sakshi_warn("module: warning", 15);     # WARN level
```

## Pull Requests

- One logical change per PR
- All 193+ tests must pass
- Benchmarks should not regress without justification
- Update CHANGELOG.md under `[Unreleased]`
- Update docs if public API changes

## License

By contributing, you agree that your contributions will be licensed under GPL-3.0-only.
