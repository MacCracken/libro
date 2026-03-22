# Security Policy

## Scope

Libro is a cryptographic audit chain library. It provides tamper-proof event logging using SHA-256 hash linking, Ed25519 digital signatures, and Merkle tree verification. It is a pure Rust library with no `unsafe` code.

## Attack Surface

| Area | Risk | Mitigation |
|------|------|------------|
| Hash computation | Second-preimage via field boundary shifting | Length-prefixed fields (LE u64) before each variable-length input |
| JSON canonicalization | Non-deterministic key order | Sorted-key canonical JSON writer; hash includes structural delimiters |
| Entry deserialization | Crafted entries bypass integrity | Fields are private; `Deserialize` produces unverified entries; `verify()` / `load_and_verify()` must be called |
| File store concurrency | Interleaved writes from multiple processes | Advisory file locking (`flock`) on append and load |
| SQLite store | SQL injection | All queries use parameterized placeholders (`?N`) |
| CSV export | Field injection via crafted agent_id/source/action | All user-provided fields passed through `csv_escape()` |
| Merkle tree | Proof forgery | Standard binary Merkle tree with SHA-256; proofs verified against root hash |
| Ed25519 signatures | Key compromise | Library does not store keys; consumer manages key lifecycle |
| Streaming (majra) | Unbounded subscriber backlog | Majra's `TypedPubSub` uses bounded broadcast channels (default 256) |
| Canonical JSON recursion | Stack overflow via deeply nested JSON | Bounded by `serde_json::Value` construction; no user-controlled recursion depth |

## Supported Versions

| Version | Supported |
|---------|-----------|
| 0.21.x | Yes |
| < 0.21 | No |

## Reporting a Vulnerability

Please report security issues to **security@agnos.dev**.

- You will receive acknowledgement within 48 hours
- We follow a 90-day coordinated disclosure timeline
- Please do not open public issues for security vulnerabilities

## Design Principles

- Zero `unsafe` code
- All public types are `Send + Sync` where applicable
- Compile-time thread safety via Rust's type system
- Parameterized queries for all SQL
- No network I/O in core library (streaming is opt-in via feature flag)
- Minimal dependency surface
