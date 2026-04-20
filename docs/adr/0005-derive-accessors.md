# ADR 0005: Adopt `#derive(accessors)` across all struct modules

**Date:** 2026-04-19
**Status:** Accepted (supersedes the v1.2.0 REJECT decision)

## Context

libro allocates structs as raw byte blocks via `fl_alloc(size)` and
accesses fields through hand-written `load64(x + OFFSET)` /
`store64(x + OFFSET, v)` calls. Each struct module hosts both the
allocator (which writes fields at known offsets) and a set of
hand-written accessor functions that duplicate those offsets in
getter form. Historically libro had ~108 such accessors across 15
struct modules. Every one was a site where a typo in an offset
constant — or a field insertion without updating every reader —
silently produced either garbage or a pointer-to-wrong-field,
bypassing the type system.

The v1.2.0 sprint documented this class but REJECTED the Cyrius
`#derive(accessors)` feature on two grounds: (1) AGNOS-wide
convention at the time was raw-offset accessors across libro, patra,
sigil, ark; (2) the ~30-line boilerplate saving was judged not worth
the consistency break. The reality turned out differently on both
counts:

1. **AGNOS convention shifted.** `agnosys` and peer crates began
   flagging derive adoption as a deliberate post-1.0 follow-up.
   libro was not holding a shared line; it was holding a stale one.
2. **The "boilerplate saving" framing undersold the safety angle.**
   The 2.0 nested-canonical-JSON sprint surfaced a UUID-zeroing bug
   where `store64(probe, 0)` zeroed only 8 of 16 bytes because the
   author assumed UUID was one 8-byte field. A declarative struct
   layout would have made that impossible: the accessor and the
   allocator read the same declaration, so they can't diverge.

Two ancillary reasons compounded:

- **Cyrius 5.4.7** (bumped from 5.4.2 for this migration) stabilized
  `#derive(accessors)` generation enough to use across all libro
  structs without hitting compiler quirks.
- **Cross-module readers** (e.g., `src/proof_json.cyr` reading
  `iproof`, `src/chain_io.cyr` reading `chain`) had no principled
  way to stay in sync with the owning module's layout. Every such
  reader was a latent coupling that the type system couldn't
  enforce.

## Decision

Adopt `#derive(accessors)` across all `struct` declarations in
`src/*.cyr`. The ~108 hand-written accessors are replaced by
declarative struct layouts; the compiler emits getters and `_set_`
setters. Inline-UUID structs (`entry`, `anchor`, `receipt`) keep
manual `_uuid_hi` / `_uuid_lo` placeholders to reserve the first
16 bytes; their `*_id(x)` pointer-returning accessors remain hand-
written (the generated getter would return a uint64, not a pointer).

Hot-path raw-offset accesses inside the defining file are allowed
by convention, documented in `src/chain.cyr`:

> Raw `load64(c + N)` / `store64(c + N, v)` forms still live inside
> this file for the few call sites that outpace the cost of a
> function call, but cross-module readers use the generated
> accessors so offset typos can't pass review.

## Consequences

- **27 derived structs** across 14 files now have compiler-generated
  accessors; ~108 hand-written accessors deleted.
- **One name collision** — `merkle_proof(tree, idx)` function
  renamed to `merkle_inclusion_proof(tree, idx)` because
  `struct merkle_proof` reserves the identifier as a type.
- **Inline-UUID migration pattern** — first 16 bytes reserved as
  `_uuid_hi` / `_uuid_lo`; ID accessor stays manual and returns a
  pointer to the struct head.
- **Toolchain pin bumped** — Cyrius 5.4.2 → 5.4.7 specifically for
  `#derive(accessors)` stability.
- **Cross-module raw-offset access is now a CI-enforced error.**
  See ADR 0006's sibling CI gates (raw-offset guard added 2.0.1,
  extended 2.0.2). Two post-2.0 sprints caught three missed
  migration sites (proof_json.cyr reading iproof; chain_io.cyr and
  review.cyr reading chain) — each would have been a latent "wrong
  field on future struct change" bug without the accessors.
- **Supersedes the v1.2.0 "Decisions (no code change)" entry**
  that marked this rejected. The original "shallow reject" is
  preserved in CHANGELOG v1.2.0 for history.

## Related

- CHANGELOG v2.0.0 `Changed` section lists every migrated struct.
- CI raw-offset guard (`.github/workflows/ci.yml`) enforces the
  cross-module invariant.
- `docs/audit/2026-04-19-audit-2.0.md` Finding 2 and Finding 4
  documented cross-module raw-offset survivors caught post-release.
