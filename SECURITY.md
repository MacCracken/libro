# Security Policy

## Scope

Libro is a cryptographic audit chain library. It provides tamper-proof
event logging using SHA-256 hash linking (FIPS 180-4), Ed25519 digital
signatures (RFC 8032), and Merkle tree verification. Crypto primitives
are delegated to [sigil](https://github.com/MacCracken/sigil); SQL
storage to [patra](https://github.com/MacCracken/patra).

See [docs/development/threat-model.md](docs/development/threat-model.md)
for the full trust-boundary and threats-vs-mitigations analysis.

## Attack Surface Summary

Condensed from the threat model. Full rationale lives there.

| Area | Risk | Mitigation |
|------|------|------------|
| Hash computation | Second-preimage via field boundary shifting | Length-prefixed fields (LE u64) before each variable-length input |
| Canonical JSON hashing | Value-type coercion / nested-object collapse | 2.0 nested scalar-aware byte-walker; ADR 0007 |
| Hash / signature compare | Timing side-channel | sigil's `ct_eq` (branchless constant-time) via `constant_time_eq_str` |
| CSV export | Field injection | All user-provided fields routed through `csv_escape()` |
| Merkle tree | Proof forgery | SHA-256 binary Merkle tree; inclusion + RFC 9162 consistency verified against root |
| Ed25519 signatures | Forgery without key | 128-bit security; `sign_entry` → `verify_entry_signature` via sigil |
| Key material | Memory exposure | `signing_key_zeroize` overwrites bytes + seed; heap-only storage |
| UUID generation | Entropy | 128 bits from `/dev/urandom`, RFC 4122 version/variant bits |
| DER encoding (RFC 3161) | Malformed input | Bounds-checked TLV parsing; `_der_parse_tlv` multi-return total/value |
| FileStore | Concurrent write corruption | Advisory `flock` on append/load |
| FileStore streaming verify | DoS via malformed input | Bounded 64KB buffer; unterminated-tail hang fixed in 2.0.3 (Finding 4) |
| `chain_import` / PatraStore | Parser crash on malformed input | Fuzz-covered (11 targets); parse-failure returns gracefully |
| Streaming pub/sub | Subscriber backlog | In-process vec queue; consumers drain via `stream_recv` |
| Kernel audit | Privilege escalation | Read-only access to `/proc/agnos/audit`; kernel is trusted |

## Supported Versions

| Version | Supported |
|---------|-----------|
| 2.0.x   | Yes       |
| 1.x     | Critical fixes only |
| < 1.0   | No        |

## Reporting a Vulnerability

Please report security issues to **security@agnos.dev**.

- You will receive acknowledgement within 48 hours.
- We follow a 90-day coordinated disclosure timeline.
- Please do not open public issues for security vulnerabilities.

## Design Principles

- **Own the stack.** Crypto comes from sigil; SQL from patra. No
  third-party crates.
- **No `unsafe` equivalent.** Cyrius doesn't have Rust's `unsafe`;
  the discipline is enforced via `#derive(accessors)` across every
  struct (ADR 0005) plus CI raw-offset guards.
- **Constant-time security-critical compares** via sigil's `ct_eq`.
- **Length-prefixed field hashing** prevents second-preimage via
  boundary shifting.
- **Nested scalar-aware canonical JSON** (2.0; ADR 0007). 1.x's
  string-quoted canonicalizer is superseded — it had a latent
  second-preimage primitive.
- **Structured tracing** via sakshi on all key operations.
- **Minimal attack surface** — no network I/O in core library.

## Fuzz coverage

The fuzz harness (`fuzz/fuzz_libro.fcyr`) ships 11 no-crash targets
covering every parser and verifier entry point: SHA-256, hex decode,
DER TLV, entry creation, chain ops, signature verify, JSON parse,
topic match, chain_import, filestore_verify_streamed, canonical JSON
hash. Any future parser / verifier addition should come with a
matching fuzz target — see `docs/guides/testing.md`.

## Audit history

- `docs/audit/2026-04-19-audit.md` — pre-1.1.0 audit (PatraStore UAF,
  FileStore silent corruption across loads).
- `docs/audit/2026-04-19-audit-2.0.md` — 2.0 hardening audit and
  post-release addenda (manifest completeness, accessor-migration
  tail, `filestore_verify_streamed` DoS).
