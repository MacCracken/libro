# Security Policy

## Scope

Libro is a cryptographic audit chain library. It provides tamper-proof event logging using SHA-256 hash linking, HMAC-SHA256 digital signatures, and Merkle tree verification. It is a pure Cyrius library with zero external dependencies.

## Attack Surface

| Area | Risk | Mitigation |
|------|------|------------|
| Hash computation | Second-preimage via field boundary shifting | Length-prefixed fields (LE u64) before each variable-length input |
| JSON canonicalization | Non-deterministic key order | Sorted-key canonical JSON writer; hash includes structural delimiters |
| Hash comparison | Timing side-channel | Constant-time comparison via bitwise OR accumulation (no early exit) |
| CSV export | Field injection via crafted agent_id/source/action | All user-provided fields passed through `csv_escape()` |
| Merkle tree | Proof forgery | Standard binary Merkle tree with SHA-256; proofs verified against root hash |
| RFC 9162 consistency | Append-only violation | Full RFC 9162 consistency proof verification algorithm |
| HMAC-SHA256 signatures | Key compromise | Library does not persist keys; consumer manages key lifecycle |
| Key material | Memory exposure | `signing_key_zeroize()` overwrites key bytes with zeros |
| UUID generation | Entropy | 128 bits from `/dev/urandom` with RFC 4122 version/variant bits |
| DER encoding | Malformed input | Bounds-checked TLV parsing with explicit length validation |
| Streaming pub/sub | Unbounded subscriber backlog | In-process vec queue; consumers should drain via `stream_recv()` |
| Kernel audit | Privilege escalation | Read-only access to `/proc/agnos/audit`; write requires AGNOS privileges |

## Supported Versions

| Version | Supported |
|---------|-----------|
| 1.0.x   | Yes       |
| < 1.0   | No        |

## Reporting a Vulnerability

Please report security issues to **security@agnos.dev**.

- You will receive acknowledgement within 48 hours
- We follow a 90-day coordinated disclosure timeline
- Please do not open public issues for security vulnerabilities

## Design Principles

- Zero external dependencies — Cyrius stdlib only
- No `unsafe` equivalent — pure Cyrius with explicit memory operations
- Constant-time hash comparisons throughout
- Length-prefixed field hashing prevents second-preimage attacks
- Canonical JSON with sorted keys for deterministic hashing
- Structured tracing via sakshi on all key operations
- Minimal attack surface — no network I/O in core library
