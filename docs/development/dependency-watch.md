# Dependency Watch

Tracked dependency version constraints, known incompatibilities, and upgrade paths.

## ed25519-dalek / rand_core

**Status:** Pinned to `ed25519-dalek` v2 which depends on `rand_core` 0.6

**Issue:** `ed25519-dalek` v2 is built against `rand_core` 0.6. The `rand` crate v0.9 upgraded to `rand_core` 0.9, making `rand` 0.9's `ThreadRng` incompatible with `ed25519-dalek`'s `CryptoRngCore` trait bound (trait mismatch across `rand_core` versions).

**Current workaround:** We use `rand_core::OsRng` directly (re-exported by `ed25519-dalek`) instead of depending on `rand` at all. This avoids the version conflict entirely.

**Upgrade path:** When `ed25519-dalek` releases a version built on `rand_core` 0.9+ (or `rand` 0.9+ compatibility), the `OsRng` import can optionally be replaced with `rand::thread_rng()` if the `rand` crate is desired for other use cases. Monitor:
- https://github.com/dalek-cryptography/curve25519-dalek/issues — upstream `rand_core` 0.9 migration
- https://crates.io/crates/ed25519-dalek — new releases

**Impact:** Low. `OsRng` is the correct choice for cryptographic key generation regardless — it sources entropy directly from the OS. `ThreadRng` internally uses `OsRng` for seeding anyway.

## rusqlite

**Status:** Pinned to `rusqlite` 0.34

**Note:** `rusqlite` 0.39+ is available but we pin to 0.34 for MSRV compatibility (Rust 1.89). The `bundled` feature compiles SQLite from source via `libsqlite3-sys`, so there is no system SQLite dependency. Monitor for security patches to `libsqlite3-sys`.

## chrono

**Status:** Using `chrono` 0.4 with `serde` feature

**Note:** `chrono` timestamps are serialized as RFC3339 strings. The `compute_hash` function feeds `timestamp.to_rfc3339()` into the hasher. Any future chrono version that changes RFC3339 formatting (e.g., subsecond precision) would break hash compatibility. This is unlikely but worth verifying on major upgrades.
