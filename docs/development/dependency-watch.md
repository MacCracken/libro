# Dependency Watch

Libro's external surface is intentionally small: the Cyrius toolchain,
sigil (crypto), patra (SQL), and the stdlib modules bundled with the
toolchain release. This document tracks what we pull in, how it pins,
and what to watch when upgrading.

## External dependencies

| Dep | Pin field | Current | Resolved by | Purpose |
|-----|-----------|---------|-------------|---------|
| Cyrius toolchain | `cyrius.cyml` `cyrius = "…"` | **5.4.7** | `cyriusup install …` | Compiler + bundled stdlib |
| sigil            | `cyrius.cyml` `[deps.sigil] tag = "…"`    | **2.8.3** | `cyrius deps` → `lib/sigil.cyr` | SHA-256, Ed25519, HMAC, hex, constant-time compare |
| patra            | `cyrius.cyml` `[deps.patra] tag = "…"`    | **1.1.1** | `cyrius deps` → `lib/patra.cyr` | SQL storage backend for `PatraStore` |

Zero third-party crates. No transitive graph to audit.

## Cyrius stdlib modules used

These ship with the Cyrius toolchain and land in `~/.cyrius/versions/<ver>/lib/`.
`cyrius.cyml` `[deps] stdlib = […]` mirrors what `src/main.cyr`
includes so the manifest stays load-bearing.

| Module        | Purpose |
|---------------|---------|
| `alloc`       | Bump allocator for long-lived buffers |
| `assert`      | Test assertions |
| `bigint`      | Arbitrary-precision integer ops (used by timestamping DER) |
| `chrono`      | Civil date arithmetic, epoch-to-RFC-3339 conversion |
| `fmt`         | Integer / hex formatting |
| `fnptr`       | Function-pointer calls (bench harness) |
| `freelist`    | Segregated freelist for struct allocation / free |
| `hashmap`     | String-keyed hash table (FNV-1a) |
| `io`          | File I/O (`file_open`, `read`, `write`, `close`, flock) |
| `json`        | JSON parse/build (used alongside libro's nested byte-walker) |
| `sakshi`      | Structured tracing — stderr profile |
| `str`         | Managed strings (fat pointer: data + len) |
| `syscalls`    | Linux x86_64 syscall wrappers |
| `vec`         | Dynamic vector |

Fuzz/bench binaries additionally pull `bench.cyr` (nanosecond benchmarking).

## Upgrade considerations

### Cyrius toolchain
- CI reads the pin from `cyrius.cyml`, so bumping the field + running
  `cyriusup install` locally is enough to change the toolchain.
- Every toolchain bump needs a full test + fuzz + bench pass (316
  tests, 11 fuzz targets, 26 benches) because codegen changes can
  surface subtle behavioral deltas. The 2.0 sprint bumped 5.4.2 →
  5.4.7 specifically for `#derive(accessors)` stability.
- Watch the **fixup-table cap** — cc5 5.4.2 raised it to 16384 (from
  8192 in cc3). Both bench binaries sit near the cap; a toolchain
  regression or new large module could overflow them. The bench
  binaries are deliberately split (ADR 0006 notes the constraint).

### sigil
- Provides all cryptographic primitives libro uses. A sigil CVE
  propagates directly.
- Upgrade path: bump `[deps.sigil] tag` in `cyrius.cyml`, run
  `cyrius deps` to resync `lib/sigil.cyr`, rebuild + retest + rerun
  fuzz. The `fuzz_sig_verify` and `fuzz_sha256` targets exercise the
  sigil surface libro relies on.
- sigil's own SHA-256 and Ed25519 are pure-Cyrius implementations;
  neither is FIPS 140-3 validated (see
  `docs/compliance/standards-mapping.md` §FIPS 140-3).

### patra
- Provides SQL storage for `PatraStore` (`src/patra_store.cyr`).
- Upgrade path: bump `[deps.patra] tag`, `cyrius deps`, rebuild +
  retest. PatraStore has its own test group in `src/main.cyr`
  covering append / load / verify / query / transaction /
  persistence; these should all pass before a patra upgrade is
  considered clean.
- patra's own fuzz harness + test suite run in the patra repo at the
  `patra-core` level; libro's pin gives you a known-stable baseline.

### stdlib modules
- Shipped atomically with the toolchain; a stdlib behavior change
  comes with a toolchain bump and is covered by the toolchain
  upgrade process above.
- If a stdlib module's API changes across a toolchain bump (rare —
  stdlib is stable post-1.0), `CYRIUS_DCE=1 cyrius build` surfaces
  it at compile time.

## Crypto primitives history

- **v1.0.0 → v1.0.1**: libro shipped its own SHA-256 (`src/sha256.cyr`)
  and HMAC-SHA256 as a signing-key placeholder. Zero crypto deps.
- **v1.0.2**: migrated SHA-256, Ed25519, hex, and `ct_eq` to sigil.
  `src/sha256.cyr` deleted; `src/hasher.cyr` became a thin delegator.
  Real Ed25519 signing replaced the HMAC placeholder.
- **v1.1.0+**: patra (SQL) joined as a second external dep for
  `PatraStore`. Full heap-based key zeroization landed.
- **v2.0**: nested scalar-aware canonical JSON landed (ADR 0007).
  No new crypto deps; just a corrected deterministic serializer
  feeding sigil's SHA-256.

## Watch list (ecosystem)

Items the roadmap tracks as blocked on upstream capability (from
`docs/development/roadmap.md`):

- **Post-quantum signatures (ML-DSA-65, FIPS 204)** — unblocker chain
  is Cyrius stdlib `lib/keccak.cyr` (SHAKE-128/256) → sigil 3.0
  `src/mldsa.cyr` → libro. Sigil 2.8.4 already defines
  `SIG_ALG_ML_DSA_65` / `SIG_ALG_HYBRID` enum placeholders. Cyrius
  keccak was originally slated for 5.2.x but has been pushed back
  behind Windows-target support and an ongoing bug/issue pass; no
  near-term ETA.
- **Hybrid signing (Ed25519 + ML-DSA-65)** — same chain as above;
  sigil has a `TrustPolicy.required_signature_algorithms` shape
  staged for 3.0.
- **TPM-backed chain sealing** — **unblocked** as of sigil 2.8.4 +
  agnosys 1.0.0. Sigil ships `src/tpm.cyr` (thin wrapper over
  agnosys TPM primitives); agnosys ships the tpm2-tools backend.
  Remaining work is a libro-side design decision: opt-in
  `src/tpm_anchor.cyr` module vs. consumer-side composition. See
  `docs/development/roadmap.md` "Open — unblocked".
- **Multi-node chain sync** — blocked on an AGNOS-level federation
  protocol; libro would gain a second meta-chain layer over the
  existing WitnessAnchor primitive.
