# ADR 0007: Nested / scalar-aware canonical-JSON hashing

**Date:** 2026-04-19
**Status:** Accepted (breaking change vs 1.x)

## Context

Every `AuditEntry` has a `details` field that stores JSON. Entry
integrity is defined by `entry_compute_hash(e)` — SHA-256 over a
canonical serialization of the entry's fields, including the
canonicalized `details`. Canonical here means "the same logical
value produces the same hash bytes regardless of incidental
formatting," which requires key ordering, whitespace normalization,
and type-faithful value emission.

The 1.x canonicalizer had two bugs that had been latent since the
Rust port:

1. **It quoted every value as a string, regardless of type.**
   `{"n": 42}` and `{"n": "42"}` hashed identically. So did
   `{"ok": true}` and `{"ok": "true"}`. This silently collapsed
   distinct logical entries into the same hash — a second-preimage
   primitive against an attacker who controlled value-type shape.
   In practice this was rare for libro's primary consumers (whose
   details are overwhelmingly string-keyed and string-valued), but
   it was a latent integrity bug nonetheless.

2. **It did not descend into nested objects or arrays.** Any
   `details` containing a nested `{…}` or `[…]` either hashed as
   the raw substring (order-dependent, non-canonical) or crashed on
   quote-balancing assumptions. Consumers avoiding nesting were
   safe by convention; consumers using it got undefined behavior.

The right semantic for canonical-JSON hashing, well-established in
the literature (RFC 8785 JCS, various ad-hoc "canonical JSON"
schemes), is:

- Objects sort keys lexicographically.
- Arrays preserve order.
- Scalars emit verbatim, trimmed of surrounding whitespace, with
  their native JSON type (number as number, bool as bool, null as
  null, string with JSON-quoted escapes).
- Recursion is depth-unlimited.

## Decision

2.0 replaces the 1.x canonicalizer with a recursive byte-walker in
`src/entry.cyr`. The walker:

- Parses the raw JSON bytes structurally.
- For objects: sorts keys lexicographically, emits
  `"key":<canonical-value>` pairs separated by `,`.
- For arrays: emits `<canonical-value>,<canonical-value>,…`
  preserving source order.
- For scalars: whitespace-trims and emits verbatim bytes. Numbers,
  booleans, and null are preserved as-is (no string-coercion).
- Recurses without depth bound.

The walker operates on raw bytes rather than building an AST to
avoid an allocation per scalar. The output bytes feed directly into
the SHA-256 hasher.

## Consequences

- **Breaking change.** Entries whose `details` contained any of
  the following hash to different values under 2.0:
  - Non-string JSON values (numbers, bools, null)
  - Arrays
  - Nested objects
- Entries with all-string flat objects **hash identically** to 1.x.
  The large majority of libro's primary consumers fall in this
  bucket; their chains re-verify without change on 2.0.
- **Re-verification expectation.** A consumer migrating a chain
  from 1.x to 2.0 where `details` used non-string or nested shapes
  should expect `verify_chain` to fail on those entries and
  should plan to re-hash (or to freeze the 1.x chain as archived
  evidence and start a fresh 2.0 chain).
- **Security posture.** The second-preimage primitive from bug (1)
  is eliminated: `{"n": 42}` and `{"n": "42"}` now hash to
  distinct values. Nested objects hash canonically and
  determinstically, so integrity is stable under re-serialization.
- **No new dependencies.** The walker is ~300 lines inside
  `src/entry.cyr` — it doesn't delegate to `lib/json.cyr`'s
  parser (which builds an AST vec of pairs) because the walker
  needs byte-level output control for deterministic emission.

## Related

- CHANGELOG v2.0.0 `Breaking` section documents this.
- 5 new tests in `src/main.cyr` (`test_canonical_json_*`) cover
  nested objects, arrays, scalar types, and mixed shapes.
- `fuzz_canonical_json_hash` (added 2.0.3) feeds random / malformed
  JSON bytes through the walker to assert no-crash on adversarial
  input. Clean.
- The walker design also eliminated the class of bug where
  `_cjh_is_ws(c)` was using `c` as a character parameter alongside
  `c = chain` uses elsewhere; resolved by the 2.0 accessor
  migration (ADR 0005).
