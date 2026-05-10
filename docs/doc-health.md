---
name: Libro Documentation Health
description: Living state of doc currency in the libro repo — fresh / stale / archived / read-through-outstanding, refreshed as docs are touched
type: state
---

# Documentation Health — libro

> **Last refresh**: 2026-05-10 (initial audit at 2.6.0 + non-release docs cleanup pass — 4 stale rows → fresh, 2 read-through rows → fresh) | **Refresh cadence**: when docs are touched, update the affected row.
> **Scope**: This repo only (`libro`) — root-level files (README, CHANGELOG, CLAUDE.md, etc.) plus the entire `docs/` tree. Cross-repo dep pin drift lives in [`development/dependency-watch.md`](development/dependency-watch.md), not here.

This is a **ledger**, not a one-time audit. Rewrite-in-place as docs change. Libro is a cryptographic audit-chain library every Cyrius audit-event consumer (daimon, aegis, stiva, sigil, ark) depends on — stale API / trust-model / verification docs propagate downstream, so doc currency carries weight. The doc surface is moderate (~25 files) and most are load-bearing.

Pattern lifted from the agnosys ledger ([`agnosys/docs/doc-health.md`](https://github.com/MacCracken/agnosys/blob/main/docs/doc-health.md)) — same buckets, libro-shaped tiers.

---

## At a glance — 2026-05-10 inventory

**~25 markdown files** total (8 root + 17 under `docs/`). Bucket counts after the post-2.6.0 non-release docs cleanup:

| Bucket | Count | What it means |
|---|---|---|
| ✅ **Fresh — touched in 2.1.x → 2.6.0 cycle (incl. post-2.6.0 docs pass)** | 13 | CHANGELOG, roadmap, CLAUDE, README, doc-health, dependency-watch, threat-model, architecture/overview, standards-mapping, integration guide, tpm-anchors guide, quickstart, testing. The post-2.6.0 docs pass moved the 4 stale rows (threat-model, dependency-watch, standards-mapping, architecture/overview) and the 2 read-through rows (quickstart, testing) into fresh. |
| 🟡 **Stale — refresh in place** | 0 | All stale rows from the 2.6.0 release cleared in the non-release docs pass. |
| 🟠 **Read-through outstanding** | 0 | All cleared in the non-release docs pass. |
| 🔵 **No version-tied claims today** | 4 | `SECURITY.md`, `CODE_OF_CONDUCT.md`, `LICENSE`, `CONTRIBUTING.md`, `DEPS-PATTERN.md`. None reference current version numbers or moving APIs. |
| 📦 **Date-stamped historical record** | 2 | `docs/audit/2026-04-19-audit.md` (1.x P(-1) audit), `docs/audit/2026-04-19-audit-2.0.md` (2.0.0 audit). Point-in-time reports; the date is in the filename. |
| ❓ **Open strategic question** | 0 | None outstanding. |
| 📝 **ADRs** | 7 | All 7 ADRs (0001–0007) reflect decisions made in the 1.0 → 2.0 era. None retired; none currently superseded. |

**Doc cleanup completed in the 2.6.0 release pass:**

- ✅ `CLAUDE.md` — Project Identity version bumped 2.0.0-dev → 2.6.0-dev; Current State refreshed (module count, bench count, test count default vs LIBRO_TPM, binary size, dist line count); toolchain pin refreshed 5.4.2 → 5.10.34.
- ✅ `README.md` — Architecture diagram + Quick Start test-count line + Project structure module list all reflect 22 modules (21 default + 1 opt-in TPM), 435/443 tests, the new bench counts (18 + 12 + 2 = 32).
- ✅ `docs/development/roadmap.md` — Already current as of 2.5.0; this pass touches the "Open — unblocked (not yet slotted)" section to mark `proof_from_json` round-trip as ✅ completed in 2.6.0.
- ✅ `docs/guides/integration.md` — Three new sections added during 2.2 / 2.3 / 2.4: Post-Quantum Signing, Hybrid Signing, PatraStore performance tier. All carry sigil 3.0.1 / patra 1.9.3 pinning.
- ✅ `docs/guides/tpm-anchors.md` — New in 2.5.0; trust model + build flow + PCR-policy alternatives + persistence semantics. Frozen as the canonical TPM anchor doc.
- ✅ `docs/doc-health.md` — This file, initial audit at 2.6.0.

**Doc cleanup completed in the post-2.6.0 non-release docs pass (2026-05-10):**

- ✅ `docs/development/dependency-watch.md` — refreshed external-dep table (cyrius 5.10.34 / sigil 3.0.1 / patra 1.9.3 / agnosys 1.0.4 with the 2.5.0 direct-pin); stdlib table grew to 22 modules covering the 2.1.0 sigil-bundle expansion + 2.1.1 `random` + `test`; upgrade-path sections rewritten to reflect PQ stack, PatraStore perf, opt-in TPM, parallel-batch-verify status; crypto-primitives history extended through 2.5.0; watch-list items marked SHIPPED.
- ✅ `docs/architecture/overview.md` — module map gained `tpm_anchor`; signing module description gained polymorphic dispatch note; new "Signing Algorithm Dispatch (2.2.0+)" section explains the 3-way enum; new "PatraStore Persistence Surface (2.4.0+)" section enumerates the perf knobs; new "TPM-Sealed Anchors (2.5.0+, opt-in)" section with the trust-model summary; design principles list extended with polymorphic dispatch + opt-in hardware integration.
- ✅ `docs/development/threat-model.md` — header date bumped to post-2.6.0; trust boundaries gained agnosys; T2 (entry forgery) rewritten for the 3-way polymorphic dispatch; T6 (key material) upgraded to MITIGATED with the 2.1.1 secret-var + getrandom + alg-aware zeroize story; T7 cross-reference updated to TPM anchor; **new T13 (Quantum cryptanalysis)** mitigated via 2.2.0/2.3.0; **new T14 (Anchor tampering)** mitigated software, hardware-sealed opt-in; residual-risk table refreshed; supply-chain block refreshed with all four pins; review cadence rewritten.
- ✅ `docs/compliance/standards-mapping.md` — coverage matrix gained FIPS 204 + CNSA 2.0 + AU-9(3) hardware-attestation rows; cryptographic-guarantees table extended with PQ-readiness / migration path / lossless proof RT / hardware-rooted anchor; algorithms table extended with ML-DSA-65 / hybrid / TPM seal / getrandom / ct_eq_bytes; post-quantum-migration section rewritten to reflect shipped 2.2/2.3; new Hardware-Rooted Anchor Attestation section; supply-chain block refreshed with 4 pins; industry-comparison table gained PQ-signing + hardware-attestation rows.
- ✅ `docs/guides/quickstart.md` — "21 modules" line now notes the 2.5.0 opt-in module.
- ✅ `docs/guides/testing.md` — assertion count refreshed (350 → 443 default / 451 LIBRO_TPM); fuzz count 11 → 12; test-categories list gained PatraStore perf tier, ML-DSA-65 + Hybrid signing groups, proof_from_json round-trip, TPM-sealed anchors (opt-in), extended struct-layout coverage; bench counts 24 → 32 across the three binaries; libro_core gained mldsa65_sign/verify + hybrid_sign/verify rows; libro_io gained the 4 patra perf rows; quickstart block gained the `-D LIBRO_TPM` build line; CI-gates table extended with TPM-opt-in build check + 2.5.0 refinements to manifest + per-file allowlist.

---

## Tier 1 — Root files

| File | Last touched | Status | Notes |
|---|---|---|---|
| `README.md` | 2026-05-10 | ✅ Fresh | Top-line refreshed to 2.6.0 — 22 modules / 435 (or 443 LIBRO_TPM) tests / new bench counts / dist line count. Quality-gates list still mentions raw-offset allowlist + dist freshness + version parity (all current). |
| `CHANGELOG.md` | 2026-05-10 | ✅ Fresh | Source of truth for shipped work. Entries through 2.6.0 (proof_from_json full round-trip + doc-health.md initial audit). |
| `CLAUDE.md` | 2026-05-10 | ✅ Fresh | Durable rules. Project Identity version + Current State counts + toolchain pin all refreshed in 2.6.0 pass. |
| `CONTRIBUTING.md` | 2026-04-19 | 🔵 No version-tied claims | Process doc. |
| `SECURITY.md` | 2026-04-19 | 🔵 No version-tied claims | Reporting policy + scope. |
| `CODE_OF_CONDUCT.md` | 2026-03-21 | 🔵 No version-tied claims | Standard. |
| `DEPS-PATTERN.md` | 2026-04-19 | 🔵 No version-tied claims | The `dist/libro.cyr` distribution contract. Patra is the reference. |
| `VERSION` | 2026-05-10 | ✅ Fresh | `2.6.0` — single source of truth, read into `cyrius.cyml` via `${file:VERSION}`. |
| `LICENSE` | (initial commit) | 🔵 Evergreen | GPL-3.0-only. |

---

## Tier 2 — Project state (`docs/development/`)

| File | Last touched | Status | Notes |
|---|---|---|---|
| `roadmap.md` | 2026-05-10 | ✅ Fresh | 2.x roadmap closed at 2.5.0 (PQ + hybrid + PatraStore perf + TPM all shipped). 2.6.0 closes `proof_from_json` round-trip. "Open — unblocked (not yet slotted)" still lists JSON streaming, RFC 6901 pointers, struct-layout test expansion, raw-offset guard expansion. |
| `threat-model.md` | 2026-05-10 | ✅ Fresh | Refreshed in post-2.6.0 docs pass: added T13 (PQ cryptanalysis, MITIGATED via 2.2/2.3) and T14 (anchor tampering, software-mitigated + hardware-sealed opt-in via 2.5.0); upgraded T2 (entry forgery) for polymorphic 3-way dispatch; upgraded T6 (key material) for `secret var` + `getrandom` + alg-aware zeroize; refreshed supply-chain block + residual-risk table. |
| `dependency-watch.md` | 2026-05-10 | ✅ Fresh | Refreshed in post-2.6.0 docs pass: external-dep table now reflects current pins (cyrius 5.10.34 / sigil 3.0.1 / patra 1.9.3 / agnosys 1.0.4); stdlib table grew to 22 modules; PQ stack + PatraStore perf + opt-in TPM tracked; crypto-primitives history extended through 2.5.0; watch-list items marked SHIPPED. |

---

## Tier 3 — Architecture (`docs/architecture/`)

| File | Last touched | Status | Notes |
|---|---|---|---|
| `overview.md` | 2026-05-10 | ✅ Fresh | Refreshed in post-2.6.0 docs pass: module map now lists 22 modules (21 default + 1 opt-in TPM); new Signing Algorithm Dispatch section covers the 3-way enum; new PatraStore Persistence Surface section covers the 2.4.0 perf knobs; new TPM-Sealed Anchors section covers the 2.5.0 opt-in. Design-principles list gained polymorphic dispatch + opt-in hardware integration. |

---

## Tier 4 — ADRs (`docs/adr/`)

| File | Last touched | Status | Notes |
|---|---|---|---|
| `0001-cyrius-port.md` | 2026-04-19 | ✅ Fresh | Accepted (1.0.0). Rust → Cyrius port rationale. Historical decision, holds. |
| `0002-sha256-only.md` | 2026-04-19 | ✅ Fresh | Accepted. Hash algorithm choice. 2.2.0 added PQ signing but kept SHA-256 for entry hashing — the ADR's reasoning carries. |
| `0003-hmac-signing.md` | 2026-04-19 | ✅ Fresh | Accepted. Signing-key derivation pre-Ed25519. Superseded in spirit by the 1.0.2 sigil-Ed25519 migration but the ADR is the historical record. |
| `0004-memorystore-primary.md` | 2026-04-19 | ✅ Fresh | Accepted. MemoryStore vs FileStore vs PatraStore design split. Holds. |
| `0005-derive-accessors.md` | 2026-04-19 | ✅ Fresh | Accepted (2.0.0). `#derive(accessors)` adoption + layout-invariant tests + raw-offset guard. 2.5.0's CI raw-offset allowlist refinement (skip `#ifdef`-gated includes) extends this without superseding it. |
| `0006-dist-artifact-contract.md` | 2026-04-19 | ✅ Fresh | Accepted (2.0.0). `dist/libro.cyr` as the consumer contract. 2.5.0's deliberate exclusion of `src/tpm_anchor.cyr` from `[lib].modules` (distlib strips #ifdef) extends this with the opt-in pattern. |
| `0007-canonical-json-hashing.md` | 2026-04-19 | ✅ Fresh | Accepted (2.0.0). Nested scalar-aware canonical-JSON hashing. Holds — 2.6.0's proof_from_json work touches JSON shape (lossless inclusion paths) but the canonical-hashing rule for entry `details` is untouched. |

**ADR posture today**: low decision-velocity, like agnosys. Architecturally significant calls earn an ADR; minor decisions ride CHANGELOG + design comments. Notable judgement calls during the 2.x cycle that landed in CHANGELOG / module headers rather than ADRs: the polymorphic signing dispatch (2.2.0 ML-DSA + 2.3.0 hybrid) and the opt-in module pattern (2.5.0 TPM anchor).

---

## Tier 5 — Audit reports (`docs/audit/`)

Date-stamped point-in-time reports. Each P(-1) hardening pass per CLAUDE.md cadence lands a new report; existing reports are not edited (the date is in the filename).

| File | Date | Status | Notes |
|---|---|---|---|
| `2026-04-19-audit.md` | 2026-04-19 | 📦 Historical record | Pre-2.0.0 P(-1) hardening audit. |
| `2026-04-19-audit-2.0.md` | 2026-04-19 | 📦 Historical record | 2.0.0 follow-up audit; F-1..F-4 closed in 2.0.1–2.0.3. |

---

## Tier 6 — Guides (`docs/guides/`)

| File | Last touched | Status | Notes |
|---|---|---|---|
| `quickstart.md` | 2026-05-10 | ✅ Fresh | Refreshed in post-2.6.0 docs pass: "21 modules" line now notes the 2.5.0 opt-in `src/tpm_anchor.cyr` (build with `-D LIBRO_TPM`). |
| `testing.md` | 2026-05-10 | ✅ Fresh | Refreshed in post-2.6.0 docs pass: assertion count 350 → 443 default / 451 LIBRO_TPM; fuzz target count 11 → 12; bench count 24 → 32 across 3 binaries; test-categories list gained PatraStore perf tier, ML-DSA-65 + Hybrid signing groups, proof_from_json round-trip, TPM-sealed anchors (opt-in), extended struct-layout coverage; CI-gates table extended. |
| `integration.md` | 2026-05-10 | ✅ Fresh | Consumer integration patterns. Three new sections during 2.2/2.3/2.4: Post-Quantum Signing, Hybrid Signing, PatraStore performance tier. Landlock hardening section landed in 2.1.1. |
| `tpm-anchors.md` | 2026-05-10 | ✅ Fresh | New in 2.5.0. TPM trust model + build flow + PCR-policy alternatives + persistence semantics. |

---

## Tier 7 — Compliance (`docs/compliance/`)

| File | Last touched | Status | Notes |
|---|---|---|---|
| `standards-mapping.md` | 2026-05-10 | ✅ Fresh | Refreshed in post-2.6.0 docs pass: coverage matrix gained FIPS 204 / CNSA 2.0 / NIST AU-9(3) hardware-attestation rows; cryptographic-guarantees table extended with PQ readiness / migration path / lossless proof round-trip / hardware-rooted anchor attestation; algorithms table extended with ML-DSA-65 / hybrid / TPM seal / getrandom; post-quantum-migration section rewritten to "shipped"; new Hardware-Rooted Anchor Attestation section; supply-chain block + industry-comparison table refreshed. |

---

## Open strategic questions

None outstanding for the 2.6.0 cut. This section will repopulate when:

- A new doc category appears that doesn't fit an existing tier (e.g. a `docs/development/issues/` ledger if libro starts tracking upstream-blocked items the way agnosys does — currently no agnosys-style upstream-blocked surface).
- The audit / review cadence shifts (current pattern: P(-1) at minor cuts per CLAUDE.md, last full audit at 2.0.0). If 3.0.x adopts a different rhythm, this file's tiers may need restructuring.
- An ADR needs to be retired or formally superseded — would force a posture call (close-in-place vs. write a successor ADR).

---

## Open items currently on the roadmap

State summary for cross-reference with `docs/development/roadmap.md`:

- **`lib/test.cyr` table-driven refactor** — investigated in 2.1.1, not pursued for that release. libro's current homogeneous test groups exercise different accessor fns per case, so `test_each` adds indirection without LOC savings.
- **`proof_to_json` bench-context control-flow hijack** — still open. Re-tested in 2.1.1 + 2.2.0 + 2.5.0 against cyrius 5.10.34; bug persists, manifestation changed. Sequenced as 4th of 5 items on the 2.6.x line.
- **Raw-offset guard expansion to ambiguous-param structs / RFC 6901 JSON Pointer queries / JSON streaming** — sequenced as 2.6.2 / 2.6.3 / tail-end items on the 2.6.x patch line per `roadmap.md`.
- **Struct-layout invariant tests for the remaining structs** — ✅ shipped in 2.6.1. 15 new test_layout_* fns + 1 TPM-gated, total layout coverage 10 → 25 (26 with TPM).

---

## Refresh procedure

When docs are touched:

1. Find the affected row in the relevant tier table.
2. Update **Last touched** column to the new date.
3. Update **Status** column if the bucket changed.
4. Update **Notes** column if the next step changed.
5. If a doc moved or was archived, update its row to reflect the new home.
6. Re-anchor "Last refresh" date in the header.

When the bucket counts at the top drift, refresh the at-a-glance table.

---

## What this file is NOT

- Not a substitute for [`development/roadmap.md`](development/roadmap.md) (which holds the forward plan).
- Not a CHANGELOG (which records what shipped, not what's stale).
- Not a dependency-watch (cross-repo pin drift lives in [`development/dependency-watch.md`](development/dependency-watch.md)).
- Not a per-doc review log (we record the result of an audit pass, not the per-doc reasoning).

---

*Last refresh: 2026-05-10 (initial audit at 2.6.0 + non-release docs cleanup pass — all 6 stale/read-through rows closed). Refresh in place when docs are touched.*
