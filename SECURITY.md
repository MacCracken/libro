# Security Policy

## Scope

Libro is a cryptographic audit chain library. It provides tamper-proof
event logging using SHA-256 hash linking (FIPS 180-4), Ed25519 (RFC 8032),
ML-DSA-65 (FIPS 204) and hybrid Ed25519 + ML-DSA-65 signatures, and Merkle
tree verification. Crypto primitives
are delegated to [sigil](https://github.com/MacCracken/sigil); SQL
storage to [patra](https://github.com/MacCracken/patra).

See [docs/development/threat-model.md](docs/development/threat-model.md)
for the full trust-boundary and threats-vs-mitigations analysis.

## Attack Surface Summary

Condensed from the threat model. Full rationale lives there.

| Area | Risk | Mitigation |
|------|------|------------|
| Hash computation | Second-preimage via field boundary shifting | Length-prefixed fields (LE u64) before each variable-length input |
| Canonical JSON hashing | Value-type coercion / nested-object collapse; malformed or differently-spelled `details` colliding | 2.0 nested scalar-aware byte-walker; malformed input rejected (2.8.11); strings and numbers normalized (2.8.12); ADR 0007 |
| Hash / signature compare | Timing side-channel | stdlib `lib/ct.cyr`'s branchless `ct_eq_bytes_lens` via `constant_time_eq_str` |
| CSV export | Field injection | All user-provided fields routed through `csv_escape()` |
| Merkle tree | Proof forgery; leaf/node confusion; odd-node duplication | SHA-256 binary Merkle tree with RFC 9162 `0x00`/`0x01` domain separation and odd-node promotion (2.8.11); inclusion + consistency verified against root |
| Signatures | Forgery without key; algorithm downgrade | Ed25519 / ML-DSA-65 / hybrid AND-mode via sigil; verify dispatches on the verifying key's algorithm, not the signature's claim; tree heads sign `tree_head_digest(root, tree_size, timestamp, algorithm)` (2.8.12) |
| Key material | Memory exposure | `signing_key_zeroize` overwrites bytes + seed; heap-only storage |
| UUID generation | Entropy | 122 random bits from `getrandom` (stdlib `random_bytes`), RFC 4122 version/variant bits; fail-closed on entropy failure |
| DER encoding (RFC 3161) | Malformed input; unverified token | Bounds-checked TLV parsing with tag checks (2.8.11). The token is **not** verified: no TSA CMS-signature check, no TSTInfo `messageImprint` / nonce binding (roadmap 2.12.0, 2.x.x) |
| FileStore | Concurrent write corruption | Advisory `flock` on append/load |
| FileStore streaming verify | DoS via malformed input | Bounded 64KB buffer; unterminated-tail hang fixed in 2.0.3 (Finding 4) |
| `chain_import` / PatraStore | Parser crash on malformed input | Fuzz-covered (12 targets); parse-failure returns gracefully |
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
- **Constant-time security-critical compares** via stdlib `lib/ct.cyr`.
- **Length-prefixed field hashing** prevents second-preimage via
  boundary shifting.
- **Nested scalar-aware canonical JSON** (2.0; ADR 0007), strict since
  2.8.11 and normalizing since 2.8.12. 1.x's string-quoted canonicalizer
  is superseded — it had a latent second-preimage primitive.
- **Structured tracing** via sakshi on all key operations.
- **Minimal attack surface** — no network I/O in core library.

## Fuzz coverage

The fuzz harness (`fuzz/fuzz_libro.fcyr`) ships 12 no-crash targets
covering every parser and verifier entry point: SHA-256, hex decode,
DER TLV, entry creation, chain ops, signature verify, JSON parse,
topic match, chain_import, filestore_verify_streamed, canonical JSON
hash, proof_from_json. Any future parser / verifier addition should come with a
matching fuzz target — see `docs/guides/testing.md`.

## Audit history

- `docs/audit/2026-04-19-audit.md` — pre-1.1.0 audit (PatraStore UAF,
  FileStore silent corruption across loads).
- `docs/audit/2026-04-19-audit-2.0.md` — 2.0 hardening audit and
  post-release addenda (manifest completeness, accessor-migration
  tail, `filestore_verify_streamed` DoS).
- `docs/audit/2026-08-22-audit.md` — 2.8.10 → 2.8.11 audit: 26 defects,
  including the canonical-JSON collision on malformed `details` and the
  Merkle domain-separation gap.
- `docs/audit/2026-08-22-audit-2.md` — 2.8.11 → 2.8.12 round 2: 33
  defects, most introduced by round 1's own fixes.
