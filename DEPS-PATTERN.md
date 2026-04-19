# LIBRO IS A CYRIUS DEP — READ THIS BEFORE TOUCHING BUILD / RELEASE

**This file is non-negotiable. Do not invent an alternative
distribution mechanism. Do not ignore it because "it seems to
work without it". Patra is the reference. Copy patra.**

---

## Who consumes libro

libro is an **upstream Cyrius library**. Downstream Cyrius
projects pull libro into their `cyrius.cyml` as a git-tagged
dep. The current known-intended consumers per libro's own
`CLAUDE.md` are **daimon, aegis, stiva, sigil, ark**. Any of
them (or any future project) wires libro in like this:

```toml
[deps.libro]
git = "https://github.com/MacCracken/libro.git"
tag = "<libro version>"
modules = ["dist/libro.cyr"]
```

`cyrius deps` clones libro at the tag and copies `dist/libro.cyr`
into the consumer's `lib/libro_libro.cyr`. That's the entry
point they `include` from.

**libro is NOT currently a dep of the cyrius compiler itself.**
It does not need to appear in `cyrius/cyrius.cyml` to be
consumed — the dep relationship is downstream-to-libro, not
cyrius-to-libro. But the distribution contract (below) is
exactly the same either way.

## The contract

`dist/libro.cyr` is **the** distribution artifact. That's it.

- Every tagged release must have `dist/libro.cyr` committed.
- The bundle must be a self-contained, include-free single
  `.cyr` file containing every public function / struct /
  global libro exports.
- The file path and name are fixed: `dist/libro.cyr`. Not
  `dist/libro-1.2.0.cyr`. Not `build/libro.cyr`. Not
  `libro.cyr` at the repo root. **`dist/libro.cyr`.**

If `dist/libro.cyr` is missing at the tag, every downstream
consumer's `cyrius deps` step breaks at git-archive-fetch time.

## How to produce it

Use `cyrius distlib` — it reads `[build]` / `[lib]` from
`cyrius.cyml` and emits `dist/<name>.cyr` deterministically.

```sh
cyrius distlib
```

That command lands `dist/libro.cyr`. Run it:

1. **Locally** whenever `src/*.cyr` changes — verify the bundle
   is up to date, then commit it.
2. **In the release workflow** (`.github/workflows/release.yml`)
   before any `git archive` / asset-upload step, same as patra.

## Why not `scripts/bundle.sh`?

You may notice patra has a `scripts/bundle.sh` that concatenates
`src/*.cyr` with `grep -v "^include "`. That is the **legacy**
bundling pattern, pre-v5.2.0. Per the Cyrius v5.2.0 CHANGELOG:

> `cyrius build --distlib` (renamed to `cyrius distlib`) —
> Single-command library distribution: bundles `src/` modules
> into `dist/{name}.cyr`. Respects `[build] modules` ordering
> from manifest. Strips `include` directives. Reproducible.
> Replaces per-repo `scripts/bundle.sh` across all deps
> (sakshi, patra, sigil, yukti, mabda, sankoch).

Patra hasn't migrated yet. **libro should skip straight to
`cyrius distlib`.** Do not port `bundle.sh` from patra. Do not
write a new `scripts/bundle.sh` for libro. Use the tool that
already exists.

## The reference — patra

Even though patra still has the legacy `bundle.sh`, its
**structural** dep contract is correct and is the reference:

1. `ls ~/Repos/patra/dist/patra.cyr` — exists, committed.
2. `cat ~/Repos/patra/cyrius.cyml` — look at `[package]`,
   `[build]`, `[deps.*]` shape. Match it.
3. `cat ~/Repos/patra/.github/workflows/release.yml` — look at
   how the release asset upload references `dist/patra.cyr`.
   Match that flow (but invoke `cyrius distlib` instead of
   `sh scripts/bundle.sh`).

If libro's build/release deviates from this shape, either the
deviation is documented in libro's CHANGELOG with a clear reason,
or it is a bug.

## What will break if you ignore this file

- Any downstream consumer wiring `[deps.libro]` sees a 404 when
  `cyrius deps` tries to pull `dist/libro.cyr` from the tagged
  commit.
- Consumer CI turns red. The user tags a libro release, asks why
  daimon / aegis / stiva broke, and finds out `dist/libro.cyr`
  wasn't produced.
- This has happened before on other deps. It wastes a release
  cycle every time.

## If you think you have a reason to deviate

You don't. Ask first. The distribution contract is an ecosystem
concern, not a libro-local decision.

---

## Verification checklist before any libro release

- [ ] `dist/libro.cyr` exists, is non-empty.
- [ ] `dist/libro.cyr` is committed at the tagged commit
      (`git log --oneline dist/libro.cyr | head` shows a commit
      at or before the tag).
- [ ] `dist/libro.cyr` is up-to-date with `src/*.cyr` — regenerate
      with `cyrius distlib` and `git diff dist/libro.cyr` shows
      no delta.
- [ ] A simulated-consumer test: a tiny `.cyr` program that
      `include`s only `lib/libro_libro.cyr` (after copying
      `dist/libro.cyr` there) compiles clean.
- [ ] The release workflow (`.github/workflows/release.yml`)
      runs `cyrius distlib` before any asset-upload step, and
      uploads `dist/libro.cyr` as a release asset (match patra's
      upload shape).

If any box is unchecked, the release is not ready.
