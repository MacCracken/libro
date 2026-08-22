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

## Amendment (2.8.12) — the gap below is CLOSED

⭐ **The 2.8.12 amendment that follows described a gap; 2.8.12 then closed it.**
Canonical JSON now normalizes:

- **Strings** — decoded and re-emitted in RFC 8785 §3.2.2.2 minimal escaping,
  for values and for object keys, with keys sorted by their DECODED form. So
  `"A"` and `"\u0041"` hash identically, as key or as value, and the optional
  solidus escape is normalized away.
- **Numbers** — an exact plain decimal with no exponent, no leading zeros, no
  trailing fractional zeros and no `-0`, computed with digit-string arithmetic
  (no float parsing). `1` == `1.0` == `1e0` == `1.00E+00` == `10e-1`, and
  `1e50` == its 51-digit expansion.
- **Non-BMP characters** — an escaped surrogate pair decodes to real UTF-8
  rather than CESU-8, so a document hashes the same whether or not its producer
  escaped emoji. A lone surrogate is rejected.
- **Empty `details`** — no longer short-circuits to the literal bytes `null`,
  which collided with the valid document `null`. It takes the 0x00-tagged raw
  path like any other non-JSON input.

⚠ **One deliberate deviation from RFC 8785 remains:** JCS mandates ECMAScript
`Number::toString` (shortest round-trip); libro's form is exact. The two agree
on every value with a short decimal expansion. Carried on the roadmap with the
reasoning, because closing it means writing a shortest-round-trip float
formatter into a hash preimage.

⚠ **A number too extreme to expand exactly** (`1e100000`) is REJECTED by the
validator and takes the injective raw fallback — so the normal form is always
bounded, and there is no cliff where two spellings of one value straddle a cap.

## Superseded by the above — what "canonical" meant in 2.8.11

⚠ **The Context section above overclaims, and this amendment is the accurate
statement.** It says canonical means *"the same logical value produces the same
hash bytes regardless of incidental formatting"* and cites RFC 8785 (JCS). The
implementation does not do that, and measuring it is the only way to know:

```
{"a":"A"}          de425648c641cb17...
{"a":"\u0041"}     721c53020aeda4d7...   <- same logical value, different hash
{"a":1}            015abd7f5cc57a2d...
{"a":1.0}          c29a44abc114a1d7...   <- same logical value, different hash
{"a":1e0}          533e2d9500a389f8...   <- same logical value, different hash
```

**What IS normalized:** object key order (lexicographic), whitespace between
tokens, and structural form. Two documents differing only in those produce the
same digest — which is what libro's own consumers actually vary.

**What is NOT normalized:** string escape form (`"A"` vs `"\u0041"` vs `"\/"`)
and number representation (`1` / `1.0` / `1e0` / `1E+0`). Scalars and strings
are emitted as the source bytes, so the digest is byte-faithful within a token.
RFC 8785 normalizes both (ECMAScript `Number::toString` for numbers, minimal
escaping for strings); libro does not.

**This direction is safe.** These are *non-normalizations*, not collisions: two
encodings of one logical value give two different digests. Nothing distinct
shares a digest through this path, so it is not a forgery primitive — it is a
weaker equivalence than the word "canonical" implies. A consumer that
re-serializes `details` through a different JSON library before storing it will
get a different entry hash for what it considers the same content; a consumer
that stores the bytes it was given (all current ones) is unaffected.

Not fixed, deliberately: full JCS normalization would change every entry hash a
second time, one release after 2.8.11 already did. Documented instead, and
carried on the roadmap so the next preimage change can absorb it.

## Amendment (2.8.11) — recursion IS bounded, and malformed input is rejected

The Decision below says *"Recurses without depth bound"* and lists
*"Recursion is depth-unlimited"* as the target semantic. **Both are now false.**

- **Depth is capped at `CJH_MAX_DEPTH` (128).** The emitters are mutually
  recursive with one native frame per level, so an unbounded document was a
  remote stack-exhaustion crash — about 130 KB of nested array opens killed the
  process on append and on every subsequent verify.
- **Malformed input no longer takes the canonical path at all.** The walker
  used to skip bytes it did not recognise, which made `{"a":1}`,
  `{ZZZZ"a":1}`, `{"a":1 JUNK}`, `{"a" 1}` and `{"a":1}XXXXXXX` share one
  digest — a genuine second-preimage primitive, and the most serious defect in
  the 2026-08-22 audit. The document is now strictly validated first
  (`canonical_json_is_valid`); anything that fails takes a `0x00`-tagged,
  length-prefixed hash of the raw bytes, which cannot collide with a canonical
  emission or with another malformed input.

See CHANGELOG [2.8.11] and `docs/audit/2026-08-22-audit.md`.

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
