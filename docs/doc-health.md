---
name: Libro Documentation Health
description: Living state of doc currency in the libro repo — fresh / stale / archived / read-through-outstanding, refreshed as docs are touched
type: state
---

# Documentation Health — libro

> **Last refresh**: 2026-05-10 (initial audit + 4-row cleanup, paired with the 2.6.0 release work) | **Refresh cadence**: when docs are touched, update the affected row.
> **Scope**: This repo only (`libro`) — root-level files (README, CHANGELOG, CLAUDE.md, etc.) plus the entire `docs/` tree. Cross-repo dep pin drift lives in [`development/dependency-watch.md`](development/dependency-watch.md), not here.

This is a **ledger**, not a one-time audit. Rewrite-in-place as docs change. Libro is a cryptographic audit-chain library every Cyrius audit-event consumer (daimon, aegis, stiva, sigil, ark) depends on — stale API / trust-model / verification docs propagate downstream, so doc currency carries weight. The doc surface is moderate (~25 files) and most are load-bearing.

Pattern lifted from the agnosys ledger ([`agnosys/docs/doc-health.md`](https://github.com/MacCracken/agnosys/blob/main/docs/doc-health.md)) — same buckets, libro-shaped tiers.

---

## At a glance — 2026-05-10 inventory

**~25 markdown files** total (8 root + 17 under `docs/`). Bucket counts after the 2.6.0 release-paired cleanup:

| Bucket | Count | What it means |
|---|---|---|
| ✅ **Fresh — touched in 2.1.x → 2.6.0 cycle** | 7 | CHANGELOG, roadmap, integration guide (3 new sections: PQ/hybrid/PatraStore-perf), tpm-anchors guide (new in 2.5.0), this doc-health.md (new in 2.6.0). CLAUDE.md + README.md top-line facts refreshed in the 2.6.0 pass (module count, test count, version, cyrius pin). |
| 🟡 **Stale — refresh in place** | 4 | `docs/development/threat-model.md`, `docs/development/dependency-watch.md`, `docs/compliance/standards-mapping.md`, `docs/architecture/overview.md`. All last touched 2026-04-19 (2.0.4); each needs a pass against the 2.1–2.6 surface (PQ sigs, hybrid, TPM, PatraStore perf knobs, new deps). |
| 🟠 **Read-through outstanding** | 2 | `docs/guides/quickstart.md`, `docs/guides/testing.md` — last touched 2026-04-19. No version refs surfaced as obviously broken, but a re-read against the 2.6.0 surface would confirm the examples still work and the test count claims are current. |
| 🔵 **Probably evergreen** | 4 | `SECURITY.md`, `CODE_OF_CONDUCT.md`, `LICENSE`, `CONTRIBUTING.md`, `DEPS-PATTERN.md`. No version-tied claims; re-read annually. (DEPS-PATTERN.md is the dist artifact contract — invariant by design across the 2.x line.) |
| 📦 **Archive / frozen by design** | 2 | `docs/audit/2026-04-19-audit.md` (1.x P(-1) audit), `docs/audit/2026-04-19-audit-2.0.md` (2.0.0 audit). Both are date-stamped point-in-time reports; kept verbatim as historical record. |
| ❓ **Open strategic question** | 0 | None outstanding — see [Open questions](#open-strategic-questions) for the empty list and what would re-open it. |
| 📝 **ADRs** | 7 | All 7 ADRs (0001–0007) reflect decisions made in the 1.0 → 2.0 era. Re-evaluate posture at v3.0.0 cut. None retired; none currently superseded. |

**Doc cleanup completed in the 2.6.0 release pass:**

- ✅ `CLAUDE.md` — Project Identity version bumped 2.0.0-dev → 2.6.0-dev; Current State refreshed (module count, bench count, test count default vs LIBRO_TPM, binary size, dist line count); toolchain pin refreshed 5.4.2 → 5.10.34.
- ✅ `README.md` — Architecture diagram + Quick Start test-count line + Project structure module list all reflect 22 modules (21 default + 1 opt-in TPM), 435/443 tests, the new bench counts (18 + 12 + 2 = 32).
- ✅ `docs/development/roadmap.md` — Already current as of 2.5.0; this pass touches the "Open — unblocked (not yet slotted)" section to mark `proof_from_json` round-trip as ✅ completed in 2.6.0.
- ✅ `docs/guides/integration.md` — Three new sections added during 2.2 / 2.3 / 2.4: Post-Quantum Signing, Hybrid Signing, PatraStore performance tier. All carry sigil 3.0.1 / patra 1.9.3 pinning.
- ✅ `docs/guides/tpm-anchors.md` — New in 2.5.0; trust model + build flow + PCR-policy alternatives + persistence semantics. Frozen as the canonical TPM anchor doc.
- ✅ `docs/doc-health.md` — This file, initial audit at 2.6.0.

---

## Tier 1 — Root files

| File | Last touched | Status | Notes |
|---|---|---|---|
| `README.md` | 2026-05-10 | ✅ Fresh | Top-line refreshed to 2.6.0 — 22 modules / 435 (or 443 LIBRO_TPM) tests / new bench counts / dist line count. Quality-gates list still mentions raw-offset allowlist + dist freshness + version parity (all current). |
| `CHANGELOG.md` | 2026-05-10 | ✅ Fresh | Source of truth for shipped work. Entries through 2.6.0 (proof_from_json full round-trip + doc-health.md initial audit). |
| `CLAUDE.md` | 2026-05-10 | ✅ Fresh | Durable rules. Project Identity version + Current State counts + toolchain pin all refreshed in 2.6.0 pass. |
| `CONTRIBUTING.md` | 2026-04-19 | 🔵 Evergreen | Process doc; no version-tied claims. Re-read annually. |
| `SECURITY.md` | 2026-04-19 | 🔵 Evergreen | Reporting policy + scope. No version-tied claims. |
| `CODE_OF_CONDUCT.md` | 2026-03-21 | 🔵 Evergreen | Standard. |
| `DEPS-PATTERN.md` | 2026-04-19 | 🔵 Evergreen | The `dist/libro.cyr` distribution contract. Invariant by design across the 2.x line — patra remains the reference. Re-read at v3.0.0 cut to confirm the pattern still holds. |
| `VERSION` | 2026-05-10 | ✅ Fresh | `2.6.0` — single source of truth, read into `cyrius.cyml` via `${file:VERSION}`. |
| `LICENSE` | (initial commit) | 🔵 Evergreen | GPL-3.0-only. |

---

## Tier 2 — Project state (`docs/development/`)

| File | Last touched | Status | Notes |
|---|---|---|---|
| `roadmap.md` | 2026-05-10 | ✅ Fresh | 2.x roadmap closed at 2.5.0 (PQ + hybrid + PatraStore perf + TPM all shipped). 2.6.0 closes `proof_from_json` round-trip. "Open — unblocked (not yet slotted)" still lists JSON streaming, RFC 6901 pointers, struct-layout test expansion, raw-offset guard expansion. |
| `threat-model.md` | 2026-04-19 | 🟡 Stale | Last touched at 2.0.4 — pre-dates PQ signing (2.2.0), hybrid (2.3.0), and TPM anchors (2.5.0). Refresh would add: (a) Ed25519-only chains' threat horizon vs quantum, (b) hybrid migration path as a threat-model mitigation, (c) TPM-sealed anchor trust model (already lives in tpm-anchors.md but the threat model should cross-reference). |
| `dependency-watch.md` | 2026-04-19 | 🟡 Stale | Last touched at 2.0.4 when pins were cyrius 5.4.2 / sigil 2.8.4 / patra 1.1.1. Refresh to reflect 2.6.0 pins: cyrius 5.10.34 / sigil 3.0.1 / patra 1.9.3 / agnosys 1.0.4. May also document the 2.2.0 ML-DSA-65 dep on sigil's FIPS 204 stack as a new tracked surface. |

---

## Tier 3 — Architecture (`docs/architecture/`)

| File | Last touched | Status | Notes |
|---|---|---|---|
| `overview.md` | 2026-04-19 | 🟡 Stale | Module map + data flow at 2.0.4. Refresh would add: `src/tpm_anchor.cyr` (opt-in 2.5.0), the dispatch tables for SIG_ALG_ED25519 / SIG_ALG_ML_DSA_65 / SIG_ALG_HYBRID, and the PatraStore prepared-statement + sync-mode + index slots. ~22 modules now vs 20 documented. |

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

**ADR posture**: low decision-velocity, like agnosys. Only architecturally significant calls earn an ADR — minor decisions ride CHANGELOG + design comments. 2.2.0 (PQ signing) was a candidate but the dispatch design lives in the CHANGELOG + signing.cyr comments and didn't warrant an ADR (no architectural reversal). 2.3.0 (hybrid) similarly. 2.5.0 (opt-in TPM) is the closest call — could earn an "opt-in module pattern" ADR if a second opt-in module lands, otherwise the precedent stays in the CHANGELOG + tpm_anchor.cyr header.

---

## Tier 5 — Audit reports (`docs/audit/`)

Date-stamped, frozen by design. Each P(-1) hardening pass per CLAUDE.md cadence lands a new report — old reports stay verbatim as the historical record.

| File | Date | Status | Notes |
|---|---|---|---|
| `2026-04-19-audit.md` | 2026-04-19 | 📦 Frozen | Pre-2.0.0 P(-1) hardening audit. Historical record. |
| `2026-04-19-audit-2.0.md` | 2026-04-19 | 📦 Frozen | 2.0.0 follow-up audit; F-1..F-4 closed in 2.0.1–2.0.3. Historical record. |

Next audit slot: at v3.0.0 cut (or sooner if a CVE pattern surfaces against libro's parser surfaces — canonical-JSON hasher, proof_from_json, PatraStore SQL builder — or against sigil's PQ stack).

---

## Tier 6 — Guides (`docs/guides/`)

| File | Last touched | Status | Notes |
|---|---|---|---|
| `quickstart.md` | 2026-04-19 | 🟠 Read-through | Getting-started. No obvious version drift but worth confirming examples + test-count claims still match 2.6.0. |
| `testing.md` | 2026-04-19 | 🟠 Read-through | Test + benchmark guide. May reference old test count (350) or old bench layout (24 benches across 3 binaries → now 32). Read-through to confirm. |
| `integration.md` | 2026-05-10 | ✅ Fresh | Consumer integration patterns. Three new sections during 2.2/2.3/2.4: Post-Quantum Signing, Hybrid Signing, PatraStore performance tier. Landlock hardening section landed in 2.1.1. |
| `tpm-anchors.md` | 2026-05-10 | ✅ Fresh | New in 2.5.0. TPM trust model + build flow + PCR-policy alternatives + persistence semantics. |

---

## Tier 7 — Compliance (`docs/compliance/`)

| File | Last touched | Status | Notes |
|---|---|---|---|
| `standards-mapping.md` | 2026-04-19 | 🟡 Stale | Compliance mappings at 2.0.4. Refresh to add: NIST FIPS 204 ML-DSA-65 readiness (2.2.0), NSA CNSA 2.0 PQ-signing alignment (2.3.0 hybrid), the FedRAMP / NSA-style hardware-attestation hook (2.5.0 TPM seal). These are sellable claims for the audit-chain product and should be reflected in the compliance doc. |

---

## Open strategic questions

None outstanding for the 2.6.0 cut. This section will repopulate when:

- A new doc category appears that doesn't fit an existing tier (e.g. a `docs/development/issues/` ledger if libro starts tracking upstream-blocked items the way agnosys does — currently no agnosys-style upstream-blocked surface).
- The audit / review cadence shifts (current pattern: P(-1) at minor cuts per CLAUDE.md, last full audit at 2.0.0). If 3.0.x adopts a different rhythm, this file's tiers may need restructuring.
- An ADR needs to be retired or formally superseded — would force a posture call (close-in-place vs. write a successor ADR).

---

## In-flight (deferred, not stale)

Roadmap items that are deliberately deferred rather than stale:

- **`lib/test.cyr` table-driven refactor** — deferred indefinitely per the 2.1.1 investigation note in `roadmap.md`. libro's homogeneous test groups exercise different accessor fns per case, so `test_each` adds indirection without LOC savings.
- **`proof_to_json` bench-context control-flow hijack** — still open per `roadmap.md`. Re-tested in 2.1.1 + 2.2.0 + 2.5.0 against cyrius 5.10.34; bug persists. Filed for future cyrius bug-pass cycles to close incidentally.
- **JSON streaming / RFC 6901 pointers / raw-offset guard expansion / layout-test expansion** — `Open — unblocked (not yet slotted)` per `roadmap.md`. Filler-grade; pick up when room appears in any minor.

---

## Forward doc-policy commitments

| # | Commitment | Trigger | Source | Notes |
|---|---|---|---|---|
| 1 | **Audit report retention** — keep all `docs/audit/YYYY-MM-DD-audit.md` reports verbatim through at least v3.0.0; re-evaluate at the major cut whether pre-2.0 reports get folded into a single historical summary. | v3.0.0 cut | This file | Today's surface is 2 reports — purge pressure is zero. |
| 2 | **ADR posture** — Re-evaluate the 7 ADRs at v3.0.0 cut: confirm none have been silently superseded by code changes; consider whether to elevate the 2.5.0 opt-in-module pattern to its own ADR if a second opt-in module lands. | v3.0.0 cut | This file | All 7 current ADRs hold as of 2.6.0. |
| 3 | **Stale-row turnaround** — the 4 stale rows in this audit (threat-model, dependency-watch, standards-mapping, architecture/overview) should be refreshed during the 2.6.x / 2.7.0 doc-cleanup window. None are critical, but all carry consumer-facing claims that drift fast. | 2.7.0 cut | This file | Doc-cleanup pass at minor cuts, agnosys-style. |
| 4 | **doc-health refresh on touch** — rewrite-in-place when docs are touched. The 2.6.0 cleanup pass exercises the pattern; subsequent passes update this file alongside CHANGELOG + roadmap. | Each minor cut's closeout | This file | Pattern proven by agnosys (`docs/doc-health.md` refreshed across 1.1.13 → 1.2.1). |

---

## Refresh procedure

When docs are touched:

1. Find the affected row in the relevant tier table.
2. Update **Last touched** column to the new date.
3. Update **Status** column if the bucket changed.
4. Update **Notes** column if the next step changed.
5. If a doc moved or was archived, update its row to reflect the new home.
6. Re-anchor "Last refresh" date in the header.

When the bucket counts at the top drift by more than ~3 in any cell, refresh the at-a-glance table.

This file's refresh cadence is **opportunistic** (touched when other docs are touched), not periodic. The 2.6.0 release establishes the baseline; each minor cut's doc-sync step updates this file alongside CHANGELOG + roadmap.

---

## What this file is NOT

- Not a substitute for [`development/roadmap.md`](development/roadmap.md) (which holds the forward plan).
- Not a CHANGELOG (which records what shipped, not what's stale).
- Not a dependency-watch (cross-repo pin drift lives in [`development/dependency-watch.md`](development/dependency-watch.md)).
- Not a per-doc review log (we record the result of an audit pass, not the per-doc reasoning).

---

*Last refresh: 2026-05-10 (initial audit, paired with the 2.6.0 release pass). Refresh in place when docs are touched.*
