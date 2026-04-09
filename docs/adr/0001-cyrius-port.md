# ADR 0001: Port from Rust to Cyrius

**Date:** 2026-04-09
**Status:** Accepted

## Context

Libro was originally written in Rust (8,513 lines, 262 tests). The AGNOS ecosystem is standardizing on Cyrius as its systems programming language. All first-party projects (vidya, agnosys, daimon, aegis, stiva, sigil, ark) are being ported to Cyrius for consistency, reduced build complexity, and zero external dependency guarantees.

## Decision

Port libro from Rust to Cyrius, targeting feature parity with Rust v0.92.0.

## Consequences

**Gained:**
- Zero external dependencies (Cyrius stdlib only)
- 141KB static ELF binary (vs ~800KB Rust release)
- 122ms build time (vs ~30s Rust release build)
- Consistent toolchain with AGNOS ecosystem
- Single-file compilation simplifies integration

**Lost:**
- Ed25519 signatures (requires elliptic curve math not in stdlib — using HMAC-SHA256)
- Async streaming (no tokio — using synchronous function pointer callbacks)
- FileStore / SqliteStore (deferred — MemoryStore primary)
- Serde derives (custom JSON export instead)
- Trait-based polymorphism (Cyrius has no traits — using naming conventions)

**Mitigations:**
- HMAC-SHA256 provides the same API surface; Ed25519 upgrade planned for v1.1
- Synchronous pub/sub is simpler and sufficient for in-process audit
- Export functions (JSONL/CSV) cover persistence needs until FileStore is ported
