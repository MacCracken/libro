---
name: Libro Documentation Health
description: Living state of doc currency in the libro repo — fresh / stale / archived / read-through-outstanding, refreshed as docs are touched
type: state
---

# Documentation Health — libro

> **Last refresh**: 2026-05-28 (2.6.5 toolchain bump — root docs README / CLAUDE / CHANGELOG / VERSION refreshed to cyrius 6.0.14, sigil 3.5.7, patra 1.10.3, agnosys 1.2.8; 502/514 tests unchanged. Zero source migrations; the `-D LIBRO_TPM` build that broke under 6.0.1 builds clean again under 6.0.14. The 2.6.3 / 2.6.4 / 2.6.5 dep-pin bumps still haven't been propagated to the secondary docs, so `dependency-watch` / `testing` / `threat-model` / `standards-mapping` / `integration` remain flagged 🟡 stale below.) Prior: 2026-05-25 (2.6.4 toolchain bump to 6.0.1); 2026-05-10 (initial audit at 2.6.0 + non-release docs cleanup + 2.6.2 wrap-up). | **Refresh cadence**: when docs are touched, update the affected row.
> **Scope**: This repo only (`libro`) — root-level files (README, CHANGELOG, CLAUDE.md, etc.) plus the entire `docs/` tree. Cross-repo dep pin drift lives in [`development/dependency-watch.md`](development/dependency-watch.md), not here.

This is a **ledger**, not a one-time audit. Rewrite-in-place as docs change. Libro is a cryptographic audit-chain library every Cyrius audit-event consumer (daimon, aegis, stiva, sigil, ark) depends on — stale API / trust-model / verification docs propagate downstream, so doc currency carries weight. The doc surface is moderate (~25 files) and most are load-bearing.

Pattern lifted from the agnosys ledger ([`agnosys/docs/doc-health.md`](https://github.com/MacCracken/agnosys/blob/main/docs/doc-health.md)) — same buckets, libro-shaped tiers.

---

## At a glance — 2026-05-10 inventory

**~25 markdown files** total (8 root + 17 under `docs/`). Bucket counts after the post-2.6.0 non-release docs cleanup:

| Bucket | Count | What it means |
|---|---|---|
| ✅ **Fresh** | 8 | CHANGELOG, README, CLAUDE, VERSION, doc-health (refreshed in the 2.6.4 pass); roadmap, architecture/overview, quickstart, tpm-anchors (no current-pin claims, still accurate). |
| 🟡 **Stale — refresh in place** | 5 | `dependency-watch`, `testing`, `threat-model`, `standards-mapping`, `integration` — all cite pre-2.6.3 pins (cyrius 5.10.34 / sigil 3.0.1 / patra 1.9.3 / agnosys 1.0.4) and, for `testing`, pre-2.6.4 assertion counts (443/451). Need a pass to cyrius 6.0.14 / sigil 3.5.7 / patra 1.10.3 / agnosys 1.2.8 and 502/514 tests. The 2.6.3 dep bump never propagated here; 2.6.4 and 2.6.5 widened the gap. |
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

**Doc cleanup completed in the 2.6.4 release pass (2026-05-25):**

- ✅ `README.md` — test counts 435/443 → **502/514**; fuzz targets 11 → **12**; bundled patra `v1.1.1` → **v1.9.5**. Architecture diagram (21 + 1 opt-in) and bench counts (18/12/2) were already correct.
- ✅ `CLAUDE.md` — Project Identity version 2.6.0-dev → **2.6.4 (2026-05-25)**, language pin 5.4.2+ → **6.0.1**; Current State tests 435/443 → **502/514**, benches 30 → **32**, binary `~456 KB` → **~1.1 MB** (with a note that cyrius 6.0.x DCE NOPs but no longer shrinks — DCE/non-DCE builds are byte-identical), dist `~5.4k` → **~5.5k** lines; Dependencies patra `v1.1.1` → **v1.9.5**; Build & Test expected `373 passed` → **502 passed**; CI/Release toolchain pin `5.10.34` → **6.0.1**.
- ✅ `CHANGELOG.md` — `[2.6.4]` entry (cyrius 5.10.44 → 6.0.1 major-line crossing with zero source migrations; sigil/patra/agnosys bumps; lock + dist regenerated).
- ✅ `docs/doc-health.md` — this pass: header + bucket table + Tier 1 rows refreshed; the 5 secondary docs that never absorbed the 2.6.3/2.6.4 dep bumps flagged 🟡 stale.
- ⚠️ **Known structural drift left untouched** (pre-existing, not version-tied): CLAUDE.md "Project Structure" still says "20 files" / "263 inline tests + 20 module includes" and omits `chain_io` from its module list — actual is 21 library modules + `tpm_anchor` (22 includes) and 172 `test_` fns. Not corrected here to keep the pass scoped to the toolchain bump.

**Doc cleanup completed in the 2.6.5 release pass (2026-05-28):**

- ✅ `README.md` — bundled patra `v1.9.5` → **v1.10.3**. Test/fuzz/bench counts unchanged (no source changes).
- ✅ `CLAUDE.md` — Project Identity version 2.6.4 → **2.6.5 (2026-05-28)**, language pin 6.0.1 → **6.0.14**; Dependencies + Project Structure bundled patra `v1.9.5` → **v1.10.3**; CI/Release toolchain pin `6.0.1` → **6.0.14**.
- ✅ `CHANGELOG.md` — `[2.6.5]` entry (cyrius 6.0.1 → 6.0.14 within the 6.0 line, zero source migrations; sigil 3.4.3 → 3.5.7, patra 1.9.5 → 1.10.3, agnosys 1.2.7 → 1.2.8; LIBRO_TPM build recovered; lock + dist regenerated).
- ✅ `docs/doc-health.md` — this pass: header + bucket table + Tier 1 rows refreshed; stale-doc targets re-pinned to the 6.0.14 stack.
- ⚠️ **Known structural drift left untouched** (pre-existing, not version-tied): CLAUDE.md "Project Structure" still says "20 files" / "263 inline tests + 20 module includes" and omits `chain_io` from its module list. Carried over from the 2.6.4 note; left scoped out of the toolchain bump.

**Outstanding after 2.6.5 (next docs pass):**

- 🟡 `docs/development/dependency-watch.md` — external-dep table still shows the **pre-2.6.3** pins; this is the canonical pin-tracking doc, so it carries top priority. Target: cyrius 6.0.14 / sigil 3.5.7 / patra 1.10.3 / agnosys 1.2.8.
- 🟡 `docs/guides/testing.md` — assertion counts (350/443/451) and the cyrius 5.10.34 reference are stale.
- 🟡 `docs/development/threat-model.md`, `docs/compliance/standards-mapping.md` — supply-chain blocks cite `cyrius = "5.10.34"`.
- 🟡 `docs/guides/integration.md` — Post-Quantum / Hybrid / PatraStore sections pin sigil 3.0.1 / patra 1.9.3.

---

## Tier 1 — Root files

| File | Last touched | Status | Notes |
|---|---|---|---|
| `README.md` | 2026-05-28 | ✅ Fresh | 2.6.5 pass: bundled patra → v1.10.3. Test counts (502/514), fuzz (12 targets), architecture diagram (21 + 1 opt-in), bench counts (18/12/2), quality-gates list all still current. |
| `CHANGELOG.md` | 2026-05-28 | ✅ Fresh | Source of truth for shipped work. Entries through 2.6.5 (cyrius 6.0.14 toolchain bump, zero source migrations). |
| `CLAUDE.md` | 2026-05-28 | ✅ Fresh | Durable rules. Project Identity + Dependencies + CI/Release toolchain pin refreshed in 2.6.5 pass (6.0.14, patra v1.10.3). Pre-existing structural drift in "Project Structure" (20 vs 21 modules, "263 inline tests") left untouched — see the 2.6.5 cleanup note above. |
| `CONTRIBUTING.md` | 2026-04-19 | 🔵 No version-tied claims | Process doc. |
| `SECURITY.md` | 2026-04-19 | 🔵 No version-tied claims | Reporting policy + scope. |
| `CODE_OF_CONDUCT.md` | 2026-03-21 | 🔵 No version-tied claims | Standard. |
| `DEPS-PATTERN.md` | 2026-04-19 | 🔵 No version-tied claims | The `dist/libro.cyr` distribution contract. Patra is the reference. |
| `VERSION` | 2026-05-28 | ✅ Fresh | `2.6.5` — single source of truth, read into `cyrius.cyml` via `${file:VERSION}`. |
| `LICENSE` | (initial commit) | 🔵 No version-tied claims | GPL-3.0-only. |

---

## Tier 2 — Project state (`docs/development/`)

| File | Last touched | Status | Notes |
|---|---|---|---|
| `roadmap.md` | 2026-05-10 | ✅ Fresh | Forward-facing only as of 2.6.2: open 2.6.x items (RFC 6901 pointers / proof_to_json bench re-investigation / JSON streaming), ecosystem-blocked threads, items tracked in other repos, and ideas. Release history was moved out — CHANGELOG owns release detail. |
| `threat-model.md` | 2026-05-10 | 🟡 Stale | Threat content (T2/T6/T13/T14, residual-risk table) still holds, but the supply-chain block pins `cyrius = "5.10.34"`. Refresh to 6.0.14. |
| `dependency-watch.md` | 2026-05-10 | 🟡 Stale | **Top priority** — this is the canonical pin-tracking doc and its external-dep table still shows the pre-2.6.3 pins (cyrius 5.10.34 / sigil 3.0.1 / patra 1.9.3 / agnosys 1.0.4). Needs cyrius 6.0.14 / sigil 3.5.7 / patra 1.10.3 / agnosys 1.2.8, plus a note on the 5 → 6 major-line crossing. |

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
| `testing.md` | 2026-05-10 | 🟡 Stale | Assertion counts (350 / 443 default / 451 LIBRO_TPM) are pre-2.6.4 — actual is **502 / 514**. Also references cyrius 5.10.34. Bench/fuzz/category structure still accurate. |
| `integration.md` | 2026-05-10 | 🟡 Stale | Consumer patterns still valid, but the Post-Quantum / Hybrid / PatraStore sections pin sigil 3.0.1 / patra 1.9.3. Refresh to sigil 3.5.7 / patra 1.10.3. |
| `tpm-anchors.md` | 2026-05-10 | ✅ Fresh | New in 2.5.0. TPM trust model + build flow + PCR-policy alternatives + persistence semantics. |

---

## Tier 7 — Compliance (`docs/compliance/`)

| File | Last touched | Status | Notes |
|---|---|---|---|
| `standards-mapping.md` | 2026-05-10 | 🟡 Stale | Coverage matrix + crypto-guarantees + algorithms tables still hold, but the supply-chain block pins `cyrius = "5.10.34"`. Refresh to 6.0.14. |

---

## Open strategic questions

None outstanding for the 2.6.0 cut. This section will repopulate when:

- A new doc category appears that doesn't fit an existing tier (e.g. a `docs/development/issues/` ledger if libro starts tracking upstream-blocked items the way agnosys does — currently no agnosys-style upstream-blocked surface).
- The audit / review cadence shifts (current pattern: P(-1) at minor cuts per CLAUDE.md, last full audit at 2.0.0). If 3.0.x adopts a different rhythm, this file's tiers may need restructuring.
- An ADR needs to be retired or formally superseded — would force a posture call (close-in-place vs. write a successor ADR).

---

## Open items currently on the roadmap

See `docs/development/roadmap.md` for the live forward-facing list.
Open as of 2.6.2: RFC 6901 JSON Pointer queries / `proof_to_json`
bench-context re-investigation / JSON streaming.

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

*Last refresh: 2026-05-10 (initial audit at 2.6.0 + non-release docs cleanup pass + 2.6.2 wrap-up — roadmap pared to forward-facing only). Refresh in place when docs are touched.*
