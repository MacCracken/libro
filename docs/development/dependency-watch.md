# Dependency Watch

## Zero External Dependencies

Libro uses Cyrius stdlib only. There are no external crate or package dependencies to track.

## Cyrius Standard Library Modules Used

| Module | Purpose |
|--------|---------|
| `string.cyr` | C-string operations (strlen, memcpy, memset) |
| `fmt.cyr` | Integer/hex formatting |
| `alloc.cyr` | Bump allocator for long-lived buffers |
| `freelist.cyr` | Segregated freelist for struct allocation/free |
| `vec.cyr` | Dynamic vector |
| `str.cyr` | Managed strings (fat pointer: data+len) |
| `hashmap.cyr` | String-keyed hash table (FNV-1a) |
| `json.cyr` | Flat JSON parsing/building |
| `io.cyr` | File I/O (open, read, write) |
| `assert.cyr` | Test assertions |
| `bench.cyr` | Nanosecond benchmarking |
| `fnptr.cyr` | Function pointer calls |
| `syscalls.cyr` | Linux x86_64 syscall wrappers |

## Upgrade Considerations

- **Cyrius compiler upgrades** may change codegen behavior. Re-run all 193 tests and 15 benchmarks after upgrading.
- **SHA-256 implementation** is hand-rolled in `src/sha256.cyr`. If Cyrius stdlib gains a crypto module, consider migrating.
- **Ed25519 signing** is deferred (using HMAC-SHA256). When Cyrius gains elliptic curve support, upgrade `src/signing.cyr`.
