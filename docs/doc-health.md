---
name: Libro Documentation Health
description: Living state of doc currency in the libro repo — fresh / stale / archived / read-through-outstanding, refreshed as docs are touched
type: state
---

# Documentation Health — libro

> **Last refresh**: 2026-07-28 (2.8.3 toolchain refresh — cyrius 6.4.66 → **6.4.83**, 17 patch releases, **zero source changes**; sigil **3.12.1** / patra **1.12.12** re-confirmed as the newest published tags, so dep pins are unchanged. 502/514 tests, 33 benches, fuzz clean, 22 modules lint 0-warning. **This pass also cleared a three-release secondary-docs backlog** — the ledger had not been refreshed since 2.7.3, so 2.8.0's two structural changes (agnosys dropped, TPM re-sourced from sigil; sigil surface thinned to sub-bundles behind an optional `tpm` feature) had never reached the secondary docs. Refreshed to the 6.4.83 / 3.12.1 / 1.12.12 stack: `dependency-watch` (dep table rebuilt — agnosys row retired, `sigil_tpm` row added, 4 missing stdlib rows added, upgrade-path prose rewritten), `threat-model` + `standards-mapping` (supply-chain blocks), `integration` (perf table re-measured — ML-DSA sign moved 14.3 ms → 3.4 ms across the sigil pins, and the dangling "roadmap §2.x" parallel-verify reference is fixed), `tpm-anchors` (agnosys → sigil backend, plus the `--features tpm` build flow that the old commands omitted), `quickstart` + `CONTRIBUTING` (both still claimed cyrius **5.4.7**; quickstart still said "373 passed"), `README` (patra 1.11.0 → 1.12.12), `CLAUDE.md` (version/pin/quirk-header/binary sizes + a TPM build recipe + two new resolved-quirk entries). `testing.md` was checked and deliberately **not** touched — its 502/514, 33-bench and 12-fuzz counts are still correct and its 6.1.23 mentions are accurate history. Prior: 2026-06-11 (2.7.3 toolchain refresh + stdlib `bayan` carve — cyrius 6.1.23 → **6.1.35**, sigil 3.7.8 → **3.7.10** (**required** — 6.1.35 hard-errors on a missing `include`; sigil 3.7.10 `#ifndef`-guards the dist's *unguarded* opt-in `src/sha_ni.cyr`/`src/aes_ni.cyr` includes — the intended source-tree-consumer path), patra **1.11.0** / agnosys **1.4.1** already latest; 502/514 tests, 33 benches, fuzz clean. Migrated the stdlib `json`/`bigint` includes to the bundled **`bayan`** dist (cyrius 6.1.25 carve) across `[deps] stdlib`, `src/main.cyr`, all three benches, the fuzz harness, and `tests/` repros — no call-site change (back-compat shim forwards legacy names). Root docs (VERSION/CLAUDE/CHANGELOG) + the live-pin secondary docs (`dependency-watch`, `threat-model`, `standards-mapping`, `integration`) refreshed to the 6.1.35 / 3.7.10 stack. Prior: 2026-06-10 (2.7.2 toolchain + dep bump — cyrius 6.0.53 → **6.1.23** (6.0 → 6.1 minor-line crossing, zero source migrations), sigil 3.6.0 → **3.7.8**, patra 1.10.3 → **1.11.0**, agnosys 1.3.2 → **1.4.1**; 502/514 tests. Shipped the long-deferred `proof_to_json_25` bench (benches 32 → **33**) after re-testing confirmed cyrius 6.1.23 cleared the carried-open bench-context hijack/SIGILL. Roadmap P2 (`ct_eq` → `ct_eq_bytes_lens`) closed as already-complete (source + dist already migrated). **Cleared the entire "Outstanding after 2.7.1" secondary-docs backlog** — `dependency-watch` / `testing` / `threat-model` / `standards-mapping` / `integration` all refreshed off pre-2.6.3 pins to the 6.1.23 / 3.7.8 / 1.11.0 / 1.4.1 stack, with the now-shipped parallel-batch-verify (sigil 3.6.0) corrected in `dependency-watch`. Prior: 2026-06-03 (2.7.1 toolchain + dep bump — cyrius 6.0.51 → **6.0.53**, sigil 3.5.7 → **3.6.0**, agnosys 1.2.8 → **1.3.2** (patra 1.10.3 already latest); 502/514 tests. 6.0.53 raised the per-file `#derive` cap 64 → 512, so `tpm_anchor` is back to `#derive(accessors)` and the 2.6.5 hand-written-accessor workaround is removed. sigil 3.6.0 requires `lib/thread_local.cyr` included before it — added to `[deps] stdlib` + `src/main.cyr` — else the binary links but SIGILLs. Prior: 2026-06-03 (2.7.0 toolchain bump — cyrius pin 6.0.14 → **6.0.51** (dep pins unchanged: sigil 3.5.7 / patra 1.10.3 / agnosys 1.2.8); 502/514 tests. No source logic changed — the lone `src/*.cyr` edit is a corrected comment in `tpm_anchor.cyr`: the `-D LIBRO_TPM` build blocker is the **per-file `#derive` cap (max 64)**, not the 256-entry type-table cap recorded in 2.6.5 (which 6.0.51 separately raised to 1024). 6.0.51 now diagnoses the derive cap explicitly. Hand-written accessors stay. The secondary docs `dependency-watch` / `testing` / `threat-model` / `standards-mapping` / `integration` remain 🟡 stale on pre-2.6.3 pins.) Prior: 2026-05-28 (2.6.5 toolchain bump — root docs README / CLAUDE / CHANGELOG / VERSION refreshed to cyrius 6.0.14, sigil 3.5.7, patra 1.10.3, agnosys 1.2.8; 502/514 tests. One targeted source change: `src/tpm_anchor.cyr` swapped `#derive(accessors)` for hand-written accessors to dodge cyrius's silent 256-entry type-table cap, which was breaking the `-D LIBRO_TPM` build under 6.0.14 (upstream issue filed). `cyrius.lock` is now gitignored (patra/sigil convention). The 2.6.3 / 2.6.4 / 2.6.5 dep-pin bumps still haven't been propagated to the secondary docs, so `dependency-watch` / `testing` / `threat-model` / `standards-mapping` / `integration` remain flagged 🟡 stale below.) Prior: 2026-05-25 (2.6.4 toolchain bump to 6.0.1); 2026-05-10 (initial audit at 2.6.0 + non-release docs cleanup + 2.6.2 wrap-up). | **Refresh cadence**: when docs are touched, update the affected row.
> **Scope**: This repo only (`libro`) — root-level files (README, CHANGELOG, CLAUDE.md, etc.) plus the entire `docs/` tree. Cross-repo dep pin drift lives in [`development/dependency-watch.md`](development/dependency-watch.md), not here.

This is a **ledger**, not a one-time audit. Rewrite-in-place as docs change. Libro is a cryptographic audit-chain library every Cyrius audit-event consumer (daimon, aegis, stiva, sigil, ark) depends on — stale API / trust-model / verification docs propagate downstream, so doc currency carries weight. The doc surface is moderate (~25 files) and most are load-bearing.

Pattern lifted from the agnosys ledger ([`agnosys/docs/doc-health.md`](https://github.com/MacCracken/agnosys/blob/main/docs/doc-health.md)) — same buckets, libro-shaped tiers.

---

## At a glance — 2026-05-10 inventory

**~25 markdown files** total (8 root + 17 under `docs/`). Bucket counts after the post-2.6.0 non-release docs cleanup:

| Bucket | Count | What it means |
|---|---|---|
| ✅ **Fresh** | 13 | CHANGELOG, README, CLAUDE, VERSION, doc-health, roadmap, architecture/overview, quickstart, tpm-anchors, plus the five formerly-stale secondary docs refreshed to the 6.1.23 stack in the 2.7.2 pass: `dependency-watch`, `testing`, `threat-model`, `standards-mapping`, `integration`. |
| 🟡 **Stale — refresh in place** | 0 | Cleared in the 2.7.2 pass — all five secondary docs refreshed off their pre-2.6.3 pins (cyrius 5.10.34 / sigil 3.0.1 / patra 1.9.3 / agnosys 1.0.4) to cyrius 6.1.23 / sigil 3.7.8 / patra 1.11.0 / agnosys 1.4.1 and 502/514 tests / 33 benches. |
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

- ✅ `README.md` — bundled patra `v1.9.5` → **v1.10.3**. Test/fuzz/bench counts unchanged (502/514).
- ✅ `CLAUDE.md` — Project Identity version 2.6.4 → **2.6.5 (2026-05-28)**, language pin 6.0.1 → **6.0.14**; Dependencies + Project Structure bundled patra `v1.9.5` → **v1.10.3**; CI/Release toolchain pin `6.0.1` → **6.0.14**; added Known-Quirk #4 (256-entry type-table cap).
- ✅ `CHANGELOG.md` — `[2.6.5]` entry: cyrius 6.0.1 → 6.0.14; sigil 3.4.3 → 3.5.7, patra 1.9.5 → 1.10.3, agnosys 1.2.7 → 1.2.8. **Fixed**: `-D LIBRO_TPM` build (cyrius 256-type-cap — `tpm_anchor` now uses hand-written accessors). **Changed**: `cyrius.lock` gitignored (not committed/verified/shipped, matching patra/sigil); dist regenerated.
- ✅ `src/tpm_anchor.cyr` — dropped `#derive(accessors)` for four hand-written `load64`/`store64` getters + setters (`ta`, already allowlisted). The only `src/*.cyr` change in 2.6.5.
- ✅ `.github/workflows/ci.yml` + `release.yml` — removed the committed-lock `sha256sum -c` verify step and the lock from release assets/SHA256SUMS (matches patra/sigil).
- ✅ cyrius upstream — filed `docs/development/issues/2026-05-28-type-table-256-cap-silent-fail.md` + repro.
- ✅ `docs/doc-health.md` — this pass: header + bucket table + Tier 1 rows refreshed; stale-doc targets re-pinned to the 6.0.14 stack.
- ⚠️ **Known structural drift left untouched** (pre-existing, not version-tied): CLAUDE.md "Project Structure" still says "20 files" / "263 inline tests + 20 module includes" and omits `chain_io` from its module list. Carried over from the 2.6.4 note; left scoped out of the toolchain bump.

**Doc cleanup completed in the 2.7.0 release pass (2026-06-03):**

- ✅ `CLAUDE.md` — Project Identity version 2.6.5 → **2.7.0 (2026-06-03)**, language pin 6.0.14 → **6.0.51**; CI/Release toolchain pin `6.0.14` → **6.0.51**; Known-Quirk #4 rewritten from "256-entry type-table cap" to the **per-file `#derive` cap (max 64)** with the corrected root-cause; Resolved list gained "256 type/struct table cap raised to 1024 in 6.0.51".
- ✅ `CHANGELOG.md` — `[2.7.0]` entry: cyrius 6.0.14 → 6.0.51 (dep pins unchanged). **Fixed**: corrected the `-D LIBRO_TPM` root-cause attribution (per-file `#derive` cap, not the type-table cap); hand-written accessors stay. **Notes**: 256 type-table cap raised to 1024 upstream (not the TPM blocker).
- ✅ `src/tpm_anchor.cyr` — corrected the accessor-block comment to name the real cap (per-file `#derive`, max 64) and 6.0.51's explicit diagnostic. No code change; hand-written accessors unchanged.
- ✅ `dist/libro.cyr` — regenerated; only the version header changed (`tpm_anchor` isn't bundled).
- ✅ `docs/doc-health.md` — this pass: header refreshed to the 6.0.51 stack.

**Doc cleanup completed in the 2.7.1 release pass (2026-06-03):**

- ✅ `CLAUDE.md` — version 2.7.0 → **2.7.1**, language/CI pin 6.0.51 → **6.0.53**; BIG NOTE flipped from "keep the TPM workaround" to the **sigil 3.6.0 `thread_local`-before-sigil-or-SIGILL** gotcha; quirk #4 rewritten (per-file `#derive` cap 64 → **512** in 6.0.53, workaround removed); new quirk #5 (TLS-backed modules precede consumers); Resolved list gained the 64 → 512 raise.
- ✅ `CHANGELOG.md` — `[2.7.1]` entry: cyrius 6.0.51 → 6.0.53, sigil 3.5.7 → 3.6.0, agnosys 1.2.8 → 1.3.2. **Removed**: hand-written `tpm_anchor` accessors (back to `#derive`). **Added**: `thread_local` stdlib include. Notes sigil 3.6.0's parallel batch verify retiring the roadmap ecosystem-blocked item.
- ✅ `src/tpm_anchor.cyr` — `#derive(accessors)` restored; comment rewritten to the 64 → 512 history. `src/main.cyr` + `cyrius.cyml [deps] stdlib` — `thread_local` added before sigil.
- ✅ `dist/libro.cyr` — regenerated; only the version header changed.
- ✅ `docs/doc-health.md` — this pass: header refreshed to the 6.0.53 stack.

**Doc cleanup completed in the 2.7.2 release pass (2026-06-10):**

- ✅ `VERSION` → **2.7.2**; `cyrius.cyml` cyrius pin 6.0.53 → **6.1.23** + dep pins sigil **3.7.8** / patra **1.11.0** / agnosys **1.4.1** (version field stays `${file:VERSION}`).
- ✅ `CLAUDE.md` — version 2.7.1 → **2.7.2 (2026-06-10)**, language/CI pin 6.0.53 → **6.1.23**, quirks header → (6.1.23); Benchmarks 32 → **33** (libro_proof 2 → 3, `proof_to_json` note); Dependencies bundled patra → **v1.11.0**; Project Structure bench counts corrected (14/8 → 18/12, added libro_proof line).
- ✅ `CHANGELOG.md` — `[2.7.2]` entry: cyrius 6.0.53 → 6.1.23, sigil 3.6.0 → 3.7.8, patra 1.10.3 → 1.11.0, agnosys 1.3.2 → 1.4.1. **Added**: `proof_to_json_25` bench (resolved bench-context hijack). **Notes**: roadmap P2 (`ct_eq`) already complete; secondary-docs refresh.
- ✅ `README.md` — bundled patra v1.10.3 → **v1.11.0**; bench line "24 (14+8+2)" → **33 (18+12+3)**; `proof_json` module note + Project Structure bench counts updated.
- ✅ `docs/development/roadmap.md` — header "Active patch line — 2.6.x" → **2.7.x**; removed the two now-closed items (`ct_eq` migration; `proof_to_json` bench re-investigation).
- ✅ `benches/libro_proof.bcyr` — added `proof_to_json_25` (+ `store`/`export`/`file_store`/`proof_json` include closure); header note rewritten RESOLVED.
- ✅ `dist/libro.cyr` — regenerated (5481 lines); only the version header moved.
- ✅ **All five "Outstanding after 2.7.1" secondary docs refreshed** (see Tier rows below): `dependency-watch` (table + counts + parallel-batch-verify now-shipped + agnosys floor), `testing` (counts 350/443/451 → 502/514, 33 benches, `proof_to_json` RESOLVED), `threat-model` + `standards-mapping` (supply-chain blocks → 6.1.23 stack), `integration` (perf table re-measured under sigil 3.7.8).

---

## Tier 1 — Root files

| File | Last touched | Status | Notes |
|---|---|---|---|
| `README.md` | 2026-06-10 | ✅ Fresh | 2.7.2 pass: bundled patra → v1.11.0; bench line corrected 24 (14+8+2) → **33 (18+12+3)**; `proof_json` module note + Project Structure bench counts updated. Test counts (502/514), fuzz (12), architecture diagram (21 + 1 opt-in) still current. |
| `CHANGELOG.md` | 2026-06-10 | ✅ Fresh | Source of truth for shipped work. Entries through **2.7.2** (cyrius 6.1.23 / sigil 3.7.8 / patra 1.11.0 / agnosys 1.4.1; `proof_to_json` bench shipped; roadmap P2 closed already-done). |
| `CLAUDE.md` | 2026-06-10 | ✅ Fresh | Durable rules. 2.7.2 pass: version → 2.7.2, language/CI pin → 6.1.23, quirks header → (6.1.23), Benchmarks 32 → 33, bundled patra → v1.11.0, Project Structure bench counts corrected (14/8 → 18/12 + libro_proof line). Pre-existing "20 vs 21 modules / 263 inline tests" drift in Project Structure still untouched. |
| `CONTRIBUTING.md` | 2026-04-19 | 🔵 No version-tied claims | Process doc. |
| `SECURITY.md` | 2026-04-19 | 🔵 No version-tied claims | Reporting policy + scope. |
| `CODE_OF_CONDUCT.md` | 2026-03-21 | 🔵 No version-tied claims | Standard. |
| `DEPS-PATTERN.md` | 2026-04-19 | 🔵 No version-tied claims | The `dist/libro.cyr` distribution contract. Patra is the reference. |
| `VERSION` | 2026-06-10 | ✅ Fresh | `2.7.2` — single source of truth, read into `cyrius.cyml` via `${file:VERSION}`. |
| `LICENSE` | (initial commit) | 🔵 No version-tied claims | GPL-3.0-only. |

---

## Tier 2 — Project state (`docs/development/`)

| File | Last touched | Status | Notes |
|---|---|---|---|
| `roadmap.md` | 2026-06-10 | ✅ Fresh | Forward-facing only. 2.7.2 pass: header → "Active patch line — 2.7.x"; the two closed items removed (`ct_eq` migration — already done; `proof_to_json` bench re-investigation — resolved + bench shipped). Remaining open: RFC 6901 pointers / JSON streaming, ecosystem-blocked threads, ideas. |
| `threat-model.md` | 2026-06-10 | ✅ Fresh | 2.7.2 pass: supply-chain block refreshed to cyrius 6.1.23 / sigil 3.7.8 / patra 1.11.0 / agnosys 1.4.1. Threat content (T2/T6/T13/T14, residual-risk table) unchanged — still holds. |
| `dependency-watch.md` | 2026-06-10 | ✅ Fresh | 2.7.2 pass: external-dep table → 6.1.23 / 3.7.8 / 1.11.0 / 1.4.1; toolchain count line → 502/514, 33 benches; parallel-batch-verify corrected to ✅ shipped (sigil 3.6.0); agnosys floor note updated (direct pin advanced to 1.4.1); crypto-history gained a v2.7.x entry. |

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
| `testing.md` | 2026-06-10 | ✅ Fresh | 2.7.2 pass: assertion counts → **502 / 514**, fuzz 11 → 12 targets, benches 32 → 33; `proof_to_json` section rewritten RESOLVED + new `proof_to_json_25` row; libro_proof header 2 → 3 benches; CI-gate TPM count → 502 → 514. |
| `integration.md` | 2026-06-10 | ✅ Fresh | 2.7.2 pass: PQ/ML-DSA perf table re-measured under sigil 3.7.8 / cyrius 6.1.23 (Ed25519 1.1/6.6 ms; ML-DSA-65 14.3/2.2 ms; hybrid reference added). Historical "introduced in patra 1.x" notes left as-is (accurate). Consumer patterns unchanged. |
| `tpm-anchors.md` | 2026-05-10 | ✅ Fresh | New in 2.5.0. TPM trust model + build flow + PCR-policy alternatives + persistence semantics. |

---

## Tier 7 — Compliance (`docs/compliance/`)

| File | Last touched | Status | Notes |
|---|---|---|---|
| `standards-mapping.md` | 2026-06-10 | ✅ Fresh | 2.7.2 pass: supply-chain block refreshed to cyrius 6.1.23 / sigil 3.7.8 / patra 1.11.0 / agnosys 1.4.1. Coverage matrix + crypto-guarantees + algorithms tables unchanged — still hold. |

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
