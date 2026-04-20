# Architecture Overview

## Module Map

```
libro (Cyrius library, single-file compilation)
├── error          — Structured error types (field / index / expected-vs-actual)
├── hasher         — SHA-256 wrapper, hex encode/decode, length-prefixed hashing
│                    (delegates to sigil for SHA-256 + hex + ct_eq)
├── entry          — AuditEntry, UUID v4, RFC 3339 timestamps, canonical JSON
├── verify         — Standalone verify_chain(entries, base_index) for loose entries
├── query          — QueryFilter: composable multi-field filtering (ANDed)
├── retention      — RetentionPolicy: count / duration / absolute; compliance presets
├── chain          — AuditChain: append, batch append, rotate, auto-rotate, verify,
│                    query, pagination
├── store          — MemoryStore: in-memory backend with streaming verification
├── export         — JSON Lines and CSV export with field escaping
├── review         — ChainReview: structured summary with integrity status
├── merkle         — MerkleTree: inclusion proofs, RFC 9162 consistency proofs,
│                    canonical roots
├── signing        — Ed25519 signing via sigil, key generation, key_id rotation
├── anchoring      — WitnessAnchor: self-hashing snapshots, anchor meta-chain
├── timestamping   — RFC 3161 DER encoding/decoding, timestamp attestations
├── proof          — IntegrityProof: signed tree heads, inclusion/consistency bundles
├── kernel_audit   — AGNOS /proc/agnos/audit interface
├── file_store     — Append-only JSON Lines backend with flock, streaming verify
├── chain_io       — chain_export / chain_import portable JSONL round-trip
├── patra_store    — SQL-backed backend via patra with indexed queries
├── streaming      — MQTT-style pub/sub with `*` and `#` wildcards
└── proof_json     — Pretty-printed JSON emitter for IntegrityProof
```

21 library modules. Authoritative list in `cyrius.cyml` `[lib] modules`;
CI enforces it matches the include list in `src/main.cyr`.

## Design Principles

- **Append-only** — entries cannot be modified or deleted after creation
- **Integrity by construction** — `entry_new` computes the hash at
  construction time; it cannot be skipped
- **Own the stack** — Cyrius stdlib + sigil (crypto) + patra (SQL); no
  third-party crates
- **Deterministic hashing** — nested scalar-aware canonical JSON
  (ADR 0007) + length-prefixed variable fields
- **Constant-time security-critical compares** — via sigil's branchless
  `ct_eq`, routed through `constant_time_eq_str`
- **Declarative struct layout** — every `struct` uses
  `#derive(accessors)` (ADR 0005); cross-module raw offsets are
  CI-forbidden
- **Structured tracing** — sakshi instrumentation on all key operations

## Data Flow

```
Event → entry_new() → hash computed → chain_append() / chain_append_batch()
                                              │
                                              ▼
                              ┌────────────────────────────────┐
                              │ AuditStore implementation       │
                              │ ├── MemoryStore (in-process)   │
                              │ ├── FileStore  (JSONL + flock) │
                              │ └── PatraStore (SQL via patra) │
                              └────────────────┬───────────────┘
                                               │
        ┌──────────────────────────────────────┼──────────────────────────────┐
        ▼                ▼                     ▼                      ▼
   verify_chain     chain_query           merkle_build            chain_review
   filestore_       chain_by_source      (inclusion +             (structured
    verify_streamed (range / severity /   consistency proofs)      summary)
                    agent filters)
                                               │
                                               ▼
                                   proof_build_unsigned /
                                   proof_build_signed (Ed25519)
                                               │
                                               ▼
                                        IntegrityProof
                                   (signed tree head + entries +
                                    inclusion/consistency proofs +
                                    optional WitnessAnchor)
                                               │
                                               ├── proof_to_json (pretty JSON)
                                               └── proof_verify_* (end-to-end)
```

Side channels not shown: `chain_export` / `chain_import` for portable
snapshots; `src/streaming.cyr` for in-process pub/sub; kernel_audit for
AGNOS `/proc/agnos/audit` ingestion.

## Hash Algorithm

SHA-256 (FIPS 180-4) via sigil, with length-prefixed variable fields
and nested scalar-aware canonical JSON for `details`:

```
entry_hash = SHA-256(
    id (16 bytes, fixed UUID, no length prefix)
    || LE_u64(len(timestamp_rfc3339)) || timestamp_rfc3339
    || LE_u64(len(severity_str))      || severity_str
    || LE_u64(len(source))            || source
    || LE_u64(len(action))            || action
    || canonical_json(details)                       ← recursive byte-walker
    || LE_u64(len(agent_id))          || agent_id
    || LE_u64(len(prev_hash))         || prev_hash
)
```

**Length prefixes** (LE u64 before each variable field) prevent
second-preimage via field-boundary shifting.

**Canonical JSON** for `details`, added in 2.0 (ADR 0007), is a
depth-unlimited recursive byte-walker:
- Objects sort keys lexicographically, emit `"key":<value>` separated
  by `,`
- Arrays preserve source order
- Scalars emit verbatim with native JSON type (number / bool / null /
  string) — no type-coercion to string
- The 1.x string-quoting canonicalizer is superseded; it had a latent
  second-preimage primitive via value-type coercion (`{"n": 42}` and
  `{"n": "42"}` hashed identically). Flat all-string objects hash the
  same under 1.x and 2.x; everything else changes.

## Verification Layering

Two primitives, used together:

- **`verify_chain(entries, base_index)`** — loose-entries primitive;
  takes a vec of entries and an index offset. Checks self-hashes and
  prev_hash linkage. No notion of a chain's `prev_chain_hash` — the
  caller provides `base_index` so error reports can point at absolute
  positions. Used by FileStore, streams, archive-verify, and the
  streaming `filestore_verify_streamed`.
- **`chain_verify(c)`** — wraps `verify_chain` with the AuditChain's
  `prev_chain_hash` check (genesis entry must link to the chain's
  previous head, handling the empty-string case for a fresh chain).
  This is the primitive consumers usually call.

## Struct Layout (post-2.0)

Every libro struct is declared with `#derive(accessors)`:

```cyrius
#derive(accessors)
struct chain { entries; prev_hash; max_capacity; overflow; }
```

The compiler emits `chain_entries` / `chain_prev_hash` /
`chain_max_capacity` / `chain_overflow` getters and `chain_set_*`
setters. Inline-UUID structs (`entry`, `anchor`, `receipt`) reserve
the first 16 bytes with `_uuid_hi` / `_uuid_lo` placeholders; their
`*_id(x)` pointer-returning accessors stay hand-written.

**Cross-module readers must use the generated accessors.** Raw
`load64(x + N)` / `store64(x + N, …)` is convention-allowed only
inside the defining file. Two CI gates enforce this: a specific-
struct guard (registers 7 unambiguous param names: `c`, `ip`, `sk`,
`vk`, `es`, `mp`, `cp`) and a per-file allowlist (every raw-offset
param name must be registered per file). See ADR 0005.

## Consumers

| Project | Audit Domain |
|---------|-------------|
| daimon  | Agent lifecycle (register, sandbox, deregister) |
| aegis   | Security events (policy violations, intrusions) |
| stiva   | Container lifecycle (create, start, stop, kill) |
| sigil   | Trust decisions (signatures, key rotation) |
| ark     | Package events (install, update, remove) |

Downstream consumers pull libro via `[deps.libro]` in their own
`cyrius.cyml`; `cyrius deps` fetches `dist/libro.cyr` at the pinned
tag. See [DEPS-PATTERN.md](../../DEPS-PATTERN.md) and
[ADR 0006](../adr/0006-dist-artifact-contract.md) for the
distribution contract.
