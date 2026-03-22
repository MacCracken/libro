# Testing Guide

## Running Tests

```bash
# Core (no optional features)
cargo test

# With SQLite store
cargo test --features sqlite

# With signing
cargo test --features signing

# With streaming
cargo test --features streaming

# All features
cargo test --features sqlite,signing,streaming
```

## Test Categories

| Category | Count | Location |
|----------|-------|----------|
| Unit tests | ~85 | Colocated in each module |
| Integration tests | ~15 | `src/tests/mod.rs` |
| Doc tests | ~2 | `query.rs`, `signing.rs` |
| Feature-gated | ~25 | `sqlite_store`, `signing`, `streaming` modules |

## Coverage

Target: 90%+ line coverage (currently 95%+).

```bash
# Generate coverage report
cargo tarpaulin --features sqlite,signing,streaming --skip-clean

# HTML report
cargo tarpaulin --features sqlite,signing,streaming --out html
```

Coverage configuration is in `codecov.yml` (90% project target, 75% patch target).

## Benchmarks

```bash
# Run benchmarks with auto-appending history
make bench

# Or directly
./scripts/run-benchmarks.sh

# Just criterion (no history)
cargo bench --bench chain
```

Results are saved to `benchmark-results/`:
- `latest.json` / `latest.md` — most recent run
- `history.json` / `history.md` — all runs appended

## Testing Patterns

### Tamper Detection

Use the `#[cfg(test)]` helpers on `AuditEntry`:

```rust
entry.corrupt_action("hacked");  // Modify action without recomputing hash
entry.corrupt_hash("bad");       // Replace hash directly
```

### Store Testing

All stores implement `AuditStore`. Test via the trait for consistency:

```rust
let mut store = MemoryStore::new();  // or FileStore, SqliteStore
store.append(&entry).unwrap();
let loaded = store.load_and_verify().unwrap();
```

### Async Tests (streaming)

Use `#[tokio::test]`:

```rust
#[tokio::test]
async fn stream_test() {
    let stream = AuditStream::new();
    let mut rx = stream.subscribe("libro/#");
    stream.publish(&entry);
    let msg = rx.recv().await.unwrap();
}
```

## Local CI

```bash
make check   # fmt + clippy + test + audit
```
