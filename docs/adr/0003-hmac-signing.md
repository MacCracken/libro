# ADR 0003: HMAC-SHA256 Signing (Ed25519 Deferred)

**Date:** 2026-04-09
**Status:** Accepted (interim)

## Context

The Rust version used Ed25519 (via ed25519-dalek) for asymmetric entry signing. Ed25519 requires elliptic curve arithmetic over Curve25519, which involves modular exponentiation, field arithmetic, and point multiplication — approximately 3,000-5,000 lines of careful implementation not available in the Cyrius stdlib.

## Decision

Use HMAC-SHA256 (keyed hash) as the signing primitive. Maintain the same API surface (SigningKey, VerifyingKey, EntrySignature, sign_entry, verify_entry_signature) so the upgrade to Ed25519 is a drop-in replacement.

## Consequences

- **Symmetric, not asymmetric** — the verifying key must contain the secret key material (cannot be shared publicly without compromising signing capability)
- **Same security guarantees** for single-party verification (the signer verifies their own chain)
- **Not suitable** for third-party verification without key disclosure
- **API-compatible** — upgrading to Ed25519 changes only the internal crypto, not the function signatures
- **Planned upgrade:** blocked on sigil converting to Cyrius. Sigil will provide Ed25519 (and eventually ML-DSA) as the ecosystem crypto primitive source. Do NOT attempt Ed25519 in libro — wait for sigil. See roadmap.
