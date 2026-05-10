# Dependency Watch

Libro's external surface is intentionally small: the Cyrius toolchain,
sigil (crypto), patra (SQL), and the stdlib modules bundled with the
toolchain release. This document tracks what we pull in, how it pins,
and what to watch when upgrading.

## External dependencies

| Dep | Pin field | Current | Resolved by | Purpose |
|-----|-----------|---------|-------------|---------|
| Cyrius toolchain | `cyrius.cyml` `cyrius = "…"` | **5.10.34** | `~/.cyrius/bin/cyriusly install …` (canonical `scripts/install.sh`) | Compiler + bundled stdlib |
| sigil            | `cyrius.cyml` `[deps.sigil] tag = "…"`    | **3.0.1** | `cyrius deps` → `lib/sigil.cyr` | SHA-256, Ed25519, ML-DSA-65, hybrid verify, HMAC, HKDF, AES-GCM, hex, constant-time compare |
| patra            | `cyrius.cyml` `[deps.patra] tag = "…"`    | **1.9.3** | `cyrius deps` → `lib/patra.cyr` | SQL storage + prepared statements + group commit + STR btree indexes |
| agnosys          | `cyrius.cyml` `[deps.agnosys] tag = "…"`  | **1.0.4** | `cyrius deps` → `lib/agnosys.cyr` | TPM 2.0 primitives + Landlock syscall wrappers (opt-in via `-D LIBRO_TPM`) |

Zero third-party crates. No transitive graph to audit. Agnosys was promoted from transitive (via sigil 3.0.1) to a direct pin in 2.5.0 so libro controls the version independently of sigil's pin movements.

## Cyrius stdlib modules used

These ship with the Cyrius toolchain and land in `~/.cyrius/versions/<ver>/lib/`.
`cyrius.cyml` `[deps] stdlib = […]` mirrors what `src/main.cyr`
includes so the manifest stays load-bearing.

| Module        | Purpose |
|---------------|---------|
| `alloc`       | Bump allocator for long-lived buffers |
| `assert`      | Test assertions |
| `bench`       | Nanosecond benchmarking harness |
| `bigint`      | Arbitrary-precision integer ops (used by timestamping DER) |
| `chrono`      | Civil date arithmetic, epoch-to-RFC-3339 conversion |
| `ct`          | Constant-time helpers (`ct_eq_bytes`, `ct_select`) — sigil dep |
| `fmt`         | Integer / hex formatting |
| `fnptr`       | Function-pointer calls (`fncall0`–`fncall8`) — bench dispatch |
| `freelist`    | Segregated freelist for struct allocation / free |
| `fs`          | Filesystem helpers (path ops, stat) — sigil/patra dep |
| `hashmap`     | String-keyed hash table (FNV-1a) |
| `io`          | File I/O (`file_open`, `read`, `write`, `close`, flock) |
| `json`        | JSON parse/build (used alongside libro's nested byte-walker) |
| `keccak`      | SHA-3 / SHAKE-128/256 — sigil ML-DSA-65 dep (2.2.0) |
| `process`     | Subprocess helpers — agnosys TPM dep |
| `random`      | `getrandom(2)` wrapper (2.1.1 hardening: replaces `/dev/urandom` reads) |
| `sakshi`      | Structured tracing — stderr profile |
| `str`         | Managed strings (fat pointer: data + len) |
| `string`      | C-string helpers (`strlen`, `memcpy`, `memset`) |
| `syscalls`    | Linux syscall wrappers (per-arch dispatched) |
| `tagged`      | Tagged unions (Result, Option) — sigil dep |
| `test`        | `test_each` table-driven harness (held in deps but not yet exercised) |
| `thread`      | Mutex / spawn primitives — sigil parallel-batch-verify dep |
| `vec`         | Dynamic vector |

The dep list grew significantly in 2.1.0 to satisfy sigil 3.0.1's bundle requirements (`ct`, `keccak`, `thread`, `tagged`, `process`, `fs`, `string`); 2.1.1 added `random` (getrandom wrapper) and `test` (table-driven test helpers).

## Upgrade considerations

### Cyrius toolchain
- CI reads the pin from `cyrius.cyml`, so bumping the field + running
  the canonical installer (`scripts/install.sh` via curl, or
  `cyriusly install <ver>` which calls it) is enough to change the
  toolchain.
- Every toolchain bump needs a full test + fuzz + bench pass (435
  default / 443 with `-D LIBRO_TPM`, 12 fuzz targets, 32 benches
  across three binaries) because codegen changes can surface subtle
  behavioral deltas. 2.1.0 jumped 5.4.7 → 5.10.34 in one step —
  pulled `secret var`, `getrandom`, `lib/ct.cyr`, `lib/keccak.cyr`,
  `lib/random.cyr`, and the canonical installer flow.
- Watch the **fixup-table cap** — cc5 5.4.2 raised it to 16384 (from
  8192 in cc3); 5.10.x preserved it. All three bench binaries sit
  comfortably under the cap (the 2.0.5 split into core/io/proof gave
  enough headroom for the 2.2/2.3 PQ + hybrid additions). The bench
  binaries are deliberately split (ADR 0006 notes the constraint).

### sigil
- Provides all cryptographic primitives libro uses. A sigil CVE
  propagates directly.
- 3.0.0 (May 2026) shipped the full FIPS 204 ML-DSA-65 stack
  (`src/mldsa*.cyr`, 8 modules) plus `sigil_verify_hybrid` —
  unblocked the libro 2.2 / 2.3 PQ work. The dist bundle expects the
  consumer to supply the stdlib surface (ct / keccak / thread /
  tagged / process / fs / string), which is why libro's `[deps]
  stdlib` list grew in 2.1.0.
- Upgrade path: bump `[deps.sigil] tag` in `cyrius.cyml`, run
  `cyrius deps` to resync `lib/sigil.cyr`, rebuild + retest + rerun
  fuzz. The `fuzz_sig_verify` and `fuzz_sha256` targets exercise the
  Ed25519 + SHA-256 surface; ML-DSA / hybrid coverage lives in
  `src/main.cyr` test groups "Signing (ML-DSA-65)" and
  "Signing (Hybrid Ed25519+ML-DSA-65)".
- sigil's SHA-256, Ed25519, and ML-DSA-65 are pure-Cyrius
  implementations; none is FIPS 140-3 validated (see
  `docs/compliance/standards-mapping.md` §FIPS 140-3).
- Sigil 3.0.0 shipped parallel-batch-verify infrastructure
  (`sv_verify_batch`) but the workers serialize on a full-call mutex
  in 3.0 (correctness-only, ~1× serial throughput). Libro stays
  serial until sigil 3.1's alloc-free verify-hot-path rewrite —
  tracked on the roadmap as ecosystem-blocked.

### patra
- Provides SQL storage for `PatraStore` (`src/patra_store.cyr`).
- 1.7–1.9 shipped the perf surface libro 2.4.0 wired up:
  STR-keyed btree indexes (1.7.0), group-commit sync modes (1.8.0),
  prepared statements (1.8.2), aarch64-portable syscall wrappers
  (1.9.1–1.9.3). PatraStore's `patrastore_append_batch`,
  `patrastore_set_sync_mode`, `patrastore_create_source_index`, and
  the transparent prepared-SELECT/COUNT wiring all depend on these.
- Upgrade path: bump `[deps.patra] tag`, `cyrius deps`, rebuild +
  retest. PatraStore has two test groups in `src/main.cyr` ("PatraStore"
  + "PatraStore (perf tier — 2.4.0)") covering append / load /
  verify / query / transaction / persistence / sync-mode /
  batch-correctness / prepared statements / indexed by_source —
  these should all pass before a patra upgrade is considered clean.
- patra's own fuzz harness + test suite run in the patra repo at the
  `patra-core` level; libro's pin gives you a known-stable baseline.

### agnosys
- Provides TPM 2.0 primitives (`tpm_detect`, `tpm_seal`, `tpm_unseal`,
  `tpm_get_random`) and Landlock syscall wrappers used by the
  opt-in `src/tpm_anchor.cyr` module (2.5.0+) and the Landlock
  hardening recipe in `docs/guides/integration.md`.
- Promoted from transitive-via-sigil to a direct pin in 2.5.0 so
  libro controls the agnosys version independently of sigil's
  pin movements. Matches sigil 3.0.1's floor at 1.0.4.
- Default builds (no `-D LIBRO_TPM`) still pull `lib/agnosys.cyr`
  into the include set because sigil 3.0.1 bundle references
  agnosys-side symbols (TPM-adjacent and Landlock enums). DCE
  strips the unused TPM functions in the default build.

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
- **v2.1.1**: signing key generation + timestamp nonce gathering
  switched from `/dev/urandom` to `getrandom(2)` via `lib/random.cyr`;
  the entropy buffer migrated to `secret var` for compiler-guaranteed
  zeroize-on-return.
- **v2.2.0**: ML-DSA-65 entry signing (NIST FIPS 204) landed via
  sigil 3.0.0. `EntrySignature.algorithm` dispatches between
  Ed25519 (slot 0) and ML-DSA-65 (slot 1). Sign + verify path
  polymorphic over the algorithm field.
- **v2.3.0**: Hybrid Ed25519 + ML-DSA-65 entry signing. `signing_key`
  / `verifying_key` / `entry_sig` structs gained slot-2 fields for
  the second algorithm; verify wraps `sigil_verify_hybrid` in
  AND-mode.
- **v2.5.0**: opt-in TPM-sealed `WitnessAnchor` via
  `src/tpm_anchor.cyr` (build with `-D LIBRO_TPM`). agnosys's
  `tpm_seal`/`tpm_unseal` primitives are the backend; default
  builds don't link this surface.

## Watch list (ecosystem)

Items the roadmap tracks as blocked on upstream capability (from
`docs/development/roadmap.md`):

- **Post-quantum signatures (ML-DSA-65, FIPS 204)** — ✅ **shipped
  in libro 2.2.0** via sigil 3.0.0. Unblocker chain was Cyrius
  stdlib `lib/keccak.cyr` (delivered v5.7.x) → sigil 3.0
  `src/mldsa*.cyr` (8 modules, May 2026) → libro 2.2.0 dispatch.
- **Hybrid signing (Ed25519 + ML-DSA-65)** — ✅ **shipped in libro
  2.3.0** via sigil 3.0.0's `sigil_verify_hybrid`. AND-mode
  verification gates both primitives.
- **TPM-backed chain sealing** — ✅ **shipped in libro 2.5.0** via
  agnosys 1.0.4's `tpm_seal`/`tpm_unseal`. Opt-in
  `src/tpm_anchor.cyr` behind `-D LIBRO_TPM`; default builds keep
  no agnosys-TPM surface linked.
- **Parallel batch verify** — sigil 3.0.0 shipped the
  infrastructure (`sv_verify_batch`) but the workers serialize on
  a full-call mutex (correctness-only, ~1× serial throughput in
  3.0). Sigil 3.1's alloc-free verify-hot-path rewrite is the
  actual unblocker; libro stays serial until then.
- **Multi-node chain sync** — blocked on an AGNOS-level federation
  protocol; libro would gain a second meta-chain layer over the
  existing WitnessAnchor primitive.
