---
name: Libro Documentation Health
description: Living state of doc currency in the libro repo — fresh / stale / archived / read-through-outstanding, refreshed as docs are touched
type: state
---

# Documentation Health — libro

> **Last refresh**: 2026-10-28 (libro 2.11.3, at cyrius 6.7.18 / sigil 3.13.15 / patra 1.16.6). The full re-check was 2.11.2's documentation pass (CHANGELOG `[2.11.2]` *Docs*); 2.11.3 updated the docs that state test counts, sizes or what the JSONL readers return.
> **Scope**: This repo only (`libro`) — root-level files (README, CHANGELOG, CLAUDE.md, etc.) plus the entire `docs/` tree. Cross-repo dep pin drift lives in [`development/dependency-watch.md`](development/dependency-watch.md), not here.

This is a **ledger**, not a one-time audit. Rewrite-in-place as docs change. Libro is a cryptographic audit-chain library that ten Cyrius repos pin (agnostic, argonaut, bote, cyrius-yeomans-descent, daimon, kybernet, nein, phylax, stiva, t-ron; checked 2026-10-28) — stale API / trust-model / verification docs propagate downstream, so doc currency carries weight. Most of the ~28 files are load-bearing.

Pattern lifted from the agnosys ledger ([`agnosys/docs/doc-health.md`](https://github.com/MacCracken/agnosys/blob/main/docs/doc-health.md)) — same buckets, libro-shaped tiers.

---

## At a glance — 2026-10-28 inventory

**28 markdown files** (7 root + 21 under `docs/`), plus `VERSION` and `LICENSE`.

| Bucket | Count | What it means |
|---|---|---|
| ✅ **Fresh** | 16 | Re-checked against the 2.11.2 tree: README, CHANGELOG, CLAUDE, CONTRIBUTING, SECURITY, doc-health, roadmap, threat-model, dependency-watch, overview, quickstart, testing, integration, tpm-anchors, standards-mapping, plus `VERSION`. |
| 🟡 **Stale — refresh in place** | 0 | — |
| 🟠 **Known gap, left as is** | 4 items | In four of the fresh docs; each listed under *Known gaps* below, with why it was not changed. |
| 🔵 **No version-tied claims today** | 3 | `CODE_OF_CONDUCT.md`, `LICENSE`, `DEPS-PATTERN.md`. |
| 📦 **Date-stamped historical record** | 4 | The four `docs/audit/` reports. |
| 📝 **ADRs** | 7 | 0001, 0002, 0005, 0006, 0007 Accepted; 0003 and 0004 Superseded (2.11.2 status call). |

---

## Tier 1 — Root files

| File | Last touched | Status | Notes |
|---|---|---|---|
| `README.md` | 2026-10-28 | ✅ Fresh | Test counts 848 / 860; ADR list marks 0003 and 0004 superseded. |
| `CHANGELOG.md` | 2026-10-28 | ✅ Fresh | Source of truth for shipped work, through **2.11.3**. |
| `CLAUDE.md` | 2026-10-28 | ✅ Fresh | Version 2.11.3, tests 848 / 860, binary 994,328 B; consumer list re-checked 2026-10-28 (agnostic in, aegis out); Project Structure block corrected to 21 modules + `chain_io`; fixup-table cap noted as growable since cyrius 6.2.0; quirks header (6.7.18). |
| `CONTRIBUTING.md` | 2026-10-28 | ✅ Fresh | Toolchain pin 6.7.18, expected 848 passed. |
| `SECURITY.md` | 2026-10-28 | ✅ Fresh, one gap | Attack-surface table covers ML-DSA-65 / hybrid, the 2.8.11 / 2.8.12 canonical-form and Merkle classes, `lib/ct.cyr`, `getrandom` UUIDs and the unverified RFC 3161 token; the 2.11.3 malformed-record refusal; fuzz 12 targets; all four audits listed. The *Supported Versions* table still reads 2.0.x — see *Known gaps*. |
| `CODE_OF_CONDUCT.md` | 2026-03-21 | 🔵 No version-tied claims | Standard. |
| `DEPS-PATTERN.md` | 2026-10-14 | 🔵 No version-tied claims | The `dist/libro.cyr` distribution contract. Patra is the reference. |
| `VERSION` | 2026-10-28 | ✅ Fresh | `2.11.3` — single source of truth, read into `cyrius.cyml` via `${file:VERSION}`. |
| `LICENSE` | 2026-04-09 | 🔵 No version-tied claims | GPL-3.0-only. |

---

## Tier 2 — Project state (`docs/development/`)

| File | Last touched | Status | Notes |
|---|---|---|---|
| `roadmap.md` | 2026-10-28 | ✅ Fresh | Forward-facing only. The 2.11.2 section is done and removed; the bayan Unicode-table item moved to *2.x.x — Blocked upstream*; the closed patra-issues line dropped; the 2.11.x JSONL-readers item added, then closed by 2.11.3; line references re-pointed after this pass. |
| `threat-model.md` | 2026-10-28 | ✅ Fresh, one gap | TPM boundary is sigil-tpm; supply chain at 6.7.18 / 3.13.15 / 1.16.6; new T15 (canonical form), T16 (Merkle and proof binding), T17 (unverified RFC 3161 token); T1 residual records the JSONL readers' malformed-last-record gap and its 2.11.3 fix (verifiers and import refuse; loaders still skip). The residual table and T6 disagree on pre-zeroize reads — see *Known gaps*. |
| `dependency-watch.md` | 2026-10-28 | ✅ Fresh, one gap | Counts 848 / 860 and capacity at 2.11.3; bayan Unicode-table section; `sv_verify_batch` "inherits" claim corrected; agnosys retirement dated 2.7.5. Its *Crypto primitives history* stops at 2.8.3 — see *Known gaps*. |

---

## Tier 3 — Architecture (`docs/architecture/`)

| File | Last touched | Status | Notes |
|---|---|---|---|
| `overview.md` | 2026-10-28 | ✅ Fresh | Preimage diagram includes `hash_algorithm`; canonical-JSON rules at 2.8.12; RFC 9162 domain separation; `ct_eq*` from stdlib `lib/ct.cyr`; agnosys removed; consumer list re-checked. |

---

## Tier 4 — ADRs (`docs/adr/`)

| File | Last touched | Status | Notes |
|---|---|---|---|
| `0001-cyrius-port.md` | 2026-10-28 | ✅ Accepted | Status note: Ed25519 (1.0.2), FileStore (1.0.1), PatraStore (1.0.2) and sakshi tracing made up the losses it listed. |
| `0002-sha256-only.md` | 2026-10-28 | ✅ Accepted | Status note: the decision stands; the implementation moved to sigil's SHA-256 (with SHA-NI dispatch) in 1.0.2. |
| `0003-hmac-signing.md` | 2026-10-28 | ✅ Superseded | By Ed25519 through sigil in 1.0.2; ML-DSA-65 2.2.0, hybrid 2.3.0. |
| `0004-memorystore-primary.md` | 2026-10-28 | ✅ Superseded | By FileStore (1.0.1) and PatraStore (1.0.2; tests ungated in 1.1.0). |
| `0005-derive-accessors.md` | 2026-04-19 | ✅ Accepted | `#derive(accessors)` + layout-invariant tests + raw-offset gates. |
| `0006-dist-artifact-contract.md` | 2026-10-28 | ✅ Accepted | Correction appended: the Context section's consumer list was wrong. |
| `0007-canonical-json-hashing.md` | 2026-08-22 | ✅ Accepted | Carries its 2.8.11 and 2.8.12 amendments. |

**ADR posture today**: low decision-velocity, like agnosys. Architecturally significant calls earn an ADR; minor decisions ride CHANGELOG + design comments. Notable judgement calls during the 2.x cycle that landed in CHANGELOG / module headers rather than ADRs: the polymorphic signing dispatch (2.2.0 ML-DSA + 2.3.0 hybrid) and the opt-in module pattern (2.5.0 TPM anchor). A superseded ADR is closed in place: its status line names what replaced it, and a section at the end says when.

---

## Tier 5 — Audit reports (`docs/audit/`)

Date-stamped point-in-time reports; existing reports are not edited (the date is in the filename).

| File | Date | Status | Notes |
|---|---|---|---|
| `2026-04-19-audit.md` | 2026-04-19 | 📦 Historical record | Pre-1.1.0 P(-1) hardening audit. |
| `2026-04-19-audit-2.0.md` | 2026-04-19 | 📦 Historical record | 2.0.0 follow-up audit; F-1..F-4 closed in 2.0.1–2.0.3. |
| `2026-08-22-audit.md` | 2026-08-22 | 📦 Historical record | 2.8.10 → 2.8.11: 26 defects, all fixed in 2.8.11. |
| `2026-08-22-audit-2.md` | 2026-08-22 | 📦 Historical record | 2.8.11 → 2.8.12 round 2: 33 defects, all fixed in 2.8.12. |

---

## Tier 6 — Guides (`docs/guides/`)

| File | Last touched | Status | Notes |
|---|---|---|---|
| `quickstart.md` | 2026-10-28 | ✅ Fresh | Expected 848 passed; `filestore_verify_streamed` comment names the malformed-record error; in-repo include list mirrors `src/main.cyr` (thin sigil auto-included, `thread_local` before it, no `lib/sigil.cyr`); every snippet compiled and run at 2.11.2. |
| `testing.md` | 2026-10-28 | ✅ Fresh | Counts 848 / 860; the pinned ML-DSA bench input documented beside the bench table. |
| `integration.md` | 2026-10-28 | ✅ Fresh, one gap | `filestore_load_and_verify` example names the malformed-record error (2.11.3); perf tables re-measured at 2.11.2; ML-DSA per-(key, message) cost explained; hybrid shipped 2.3.0; the "prepared SELECT / COUNT" section rewritten to what `patra_store.cyr` does since 2.8.9. The Landlock example does not compile — see *Known gaps*. |
| `tpm-anchors.md` | 2026-10-28 | ✅ Fresh | Counts 848 / 860; full re-sync to restore the default lib; tpm2-tools by absolute path; PCR index bounds; fixed sealed-file names; its *Roadmap* section replaced by a pointer to roadmap 2.16.0. |

---

## Tier 7 — Compliance (`docs/compliance/`)

| File | Last touched | Status | Notes |
|---|---|---|---|
| `standards-mapping.md` | 2026-10-28 | ✅ Fresh | TPM via sigil-tpm; canonical JSON at 2.8.12 (the "scalars emit verbatim" line was false); stdlib list from `cyrius.cyml` with the bayan carve; Timestamp Integrity states what libro does not verify. |

---

## Known gaps

Present-state drift found in the 2.11.2 pass and deliberately not changed:

- **`SECURITY.md` *Supported Versions*** lists 2.0.x as supported and 1.x as critical-fixes-only. Which versions are supported is a policy call for the maintainer, not a doc-currency fix.
- **`threat-model.md`**: the residual-risk table marks "Pre-zeroize memory read" as Mitigated (2.1.1), while T6's residual says libro does not defend against pre-zeroize reads. Resolving it is a call on security status.
- **`integration.md` Landlock example** uses `O_PATH`, which no stdlib file defines, so the snippet does not compile as written.
- **`dependency-watch.md` *Crypto primitives history*** ends at 2.8.3; the 2.8.11 / 2.8.12 preimage and Merkle changes and the sigil 3.13.x bumps are not in it. Incomplete rather than false; the CHANGELOG has them.

---

## Open items currently on the roadmap

See [`development/roadmap.md`](development/roadmap.md) for the live forward-facing list.

---

## Refresh procedure

When docs are touched:

1. Find the affected row in the relevant tier table.
2. Update **Last touched** column to the new date.
3. Update **Status** column if the bucket changed.
4. Update **Notes** column if the next step changed.
5. If a doc moved or was archived, update its row to reflect the new home.
6. Re-anchor "Last refresh" date in the header.

When the bucket counts at the top drift, refresh the at-a-glance table. Record what changed in the CHANGELOG, not here.

---

## What this file is NOT

- Not a substitute for [`development/roadmap.md`](development/roadmap.md) (which holds the forward plan).
- Not a CHANGELOG (which records what shipped, not what's stale).
- Not a dependency-watch (cross-repo pin drift lives in [`development/dependency-watch.md`](development/dependency-watch.md)).
- Not a per-doc review log (we record the result of an audit pass, not the per-doc reasoning).

---

*Last refresh: 2026-10-28 (libro 2.11.3). Refresh in place when docs are touched.*
