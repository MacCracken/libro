# ADR 0004: MemoryStore as Primary Backend

**Date:** 2026-04-09
**Status:** Superseded by FileStore (1.0.1) and PatraStore (1.0.2)

## Context

The Rust version had three storage backends: MemoryStore (in-memory), FileStore (JSON Lines with flock), and SqliteStore (indexed SQL queries). FileStore depends on advisory file locking (fs2 crate), and SqliteStore depends on rusqlite with bundled libsqlite3. Neither has a Cyrius equivalent.

## Decision

Ship v1.0 with MemoryStore as the sole backend. Provide export functions (JSON Lines, CSV) for persistence. Defer FileStore and SqliteStore to v1.1+.

## Consequences

- **In-process only** — chain state lives in memory, lost on process exit
- **Persistence via export** — consumers can write chains to disk using `export_jsonl()` or `export_csv()` at checkpoints
- **No concurrent access** — single process owns the chain (no flock needed)
- **Streaming verification** still works — `memstore_verify_streamed()` provides O(chunk_size) memory verification
- **FileStore** shipped in v1.0.1 — JSON Lines append with flock locking
- **SQL storage** blocked on patra + sigil integration. Patra bundles its own SHA-256 which conflicts with libro's. Once both use sigil's SHA-256, patra can be included as the SQL backend. Do NOT include patra.cyr directly until then. See roadmap.

## Superseded (1.0.1, 1.0.2)

FileStore (JSON Lines with `flock`) shipped in 1.0.1 and PatraStore (patra SQL)
in 1.0.2. The SHA-256 conflict above went away in 1.0.2, when the vendored patra
stopped bundling its own SHA-256 and libro moved to sigil's. PatraStore's tests
were gated until 1.1.0 fixed a use-after-free on patra result-set pointers
(`docs/audit/2026-04-19-audit.md`, Finding 1). libro ships three `AuditStore`
backends today, MemoryStore, FileStore and PatraStore; the quickstart's "Storage
Backends" table compares them. The text above records the 1.0.0 decision.
