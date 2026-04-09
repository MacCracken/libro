# Testing Guide

## Running Tests

```bash
# Build and run all tests
cyrius build src/main.cyr build/libro && ./build/libro

# Quick recompile + test
cyrius build src/main.cyr build/libro && ./build/libro
```

## Test Categories

| Category | Count | Location |
|----------|-------|----------|
| SHA-256 + crypto | 9 | `src/main.cyr` |
| Entry + severity | 10 | `src/main.cyr` |
| Chain + verify | 16 | `src/main.cyr` |
| Query + retention | 6 | `src/main.cyr` |
| Store (MemoryStore) | 5 | `src/main.cyr` |
| Export (JSONL, CSV) | 2 | `src/main.cyr` |
| Review + integrity | 3 | `src/main.cyr` |
| Merkle tree + proofs | 7 | `src/main.cyr` |
| Signing | 4 | `src/main.cyr` |
| Anchoring | 4 | `src/main.cyr` |
| Timestamping (RFC 3161) | 3 | `src/main.cyr` |
| Integrity proof | 5 | `src/main.cyr` |
| Kernel audit | 1 | `src/main.cyr` |
| Streaming (pub/sub) | 6 | `src/main.cyr` |
| **Total** | **193** | |

## Benchmarks

```bash
# Build and run benchmarks
cyrius build benches/libro.bcyr build/libro_bench && ./build/libro_bench
```

15 benchmarks covering: SHA-256 hashing, entry hash computation, chain append/verify,
Merkle tree build/proof/verify/consistency, signing/verification, query filtering,
JSONL/CSV export, chain review, and integrity proof generation.

## Testing Patterns

### Tamper Detection

Modify an entry field directly to bypass hash recomputation:

```
# Tamper with severity (offset +24 in entry struct)
store64(entry + 24, SEV_CRITICAL);
assert(entry_verify(entry) == 0, "tamper detected");
```

### Chain Verification

Build a chain and verify integrity:

```
var c = chain_new();
chain_append(c, SEV_INFO, str_from("src"), str_from("act"), str_from("{}"));
chain_append(c, SEV_INFO, str_from("src"), str_from("act"), str_from("{}"));
var err = chain_verify(c);
assert(err == 0, "chain valid");
```

### Store Testing

MemoryStore provides append, load, verify, query, and pagination:

```
var s = memstore_new();
memstore_append(s, entry);
var entries = memstore_load_and_verify(s);
```

### Streaming Tests

Publish entries and verify subscription delivery:

```
var s = stream_new();
var sub = stream_subscribe(s, "libro/#");
stream_publish(s, entry);
assert(stream_pending(sub) == 1, "entry delivered");
var received = stream_recv(sub);
```
