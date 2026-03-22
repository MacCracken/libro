# Threat Model

## Trust Boundaries

Libro operates at the **library boundary**. It trusts the calling application to:
- Provide meaningful source/action/details values
- Manage signing keys securely (if using `signing` feature)
- Call `verify()` or `load_and_verify()` after loading from untrusted sources
- Configure appropriate retention policies

Libro does NOT trust:
- Serialized data from disk/network (entries must be verified after deserialization)
- JSON key ordering (canonical sorted-key hashing)
- Field boundaries (length-prefixed to prevent second-preimage)

## Attack Surface

| Module | Risk | Mitigation |
|--------|------|------------|
| `entry` (hash) | Field boundary collision | Length-prefixed (LE u64) variable fields |
| `entry` (hash) | Non-deterministic JSON | Canonical JSON with sorted keys |
| `entry` (serde) | Crafted deserialized entry bypasses integrity | Fields private; `verify()` required post-deserialization |
| `file_store` | Concurrent write interleaving | Advisory flock on append |
| `file_store` | Malformed lines | Parse error with line number; logged via `tracing::error!` |
| `sqlite_store` | SQL injection | Parameterized queries only |
| `export` (CSV) | Field injection | All user-provided fields escaped via `csv_escape()` |
| `merkle` | Proof forgery | SHA-256 binary Merkle tree; proof verified against root |
| `signing` | Key compromise | Library doesn't store keys; consumer responsibility |
| `streaming` | Subscriber backlog | Bounded broadcast channels (majra default: 256) |

## Unsafe Code

None. The crate contains zero `unsafe` blocks.

## Supply Chain

- `cargo-deny` enforces license allowlist, bans wildcards, denies unknown registries
- Minimal direct dependencies (core: sha2, serde, chrono, uuid, thiserror, tracing, fs2)
- Heavy dependencies gated behind features (rusqlite, ed25519-dalek, majra/tokio)
