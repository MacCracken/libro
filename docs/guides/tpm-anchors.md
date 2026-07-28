# TPM-Sealed Anchors (LIBRO_TPM, opt-in)

libro 2.5.0 adds optional hardware-backed `WitnessAnchor` sealing
via TPM 2.0. A sealed anchor proves that the anchor was created on a
specific TPM at a specific PCR state — a strictly stronger attestation
than software-only `anchor_verify_integrity`.

The feature is **opt-in** behind a build define so that default libro
builds don't link tpm2-tools-dependent code and stay deployable in
sandboxed / rootless / non-Linux environments.

## When to use TPM-sealed anchors

Use TPM sealing when your threat model includes a *host-level
attacker* with read/write access to the audit chain, but **not** kernel-
level access to the TPM. Concrete scenarios:

- Compliance regimes that require hardware-rooted audit attestation
  (FedRAMP, NSA CNSA 2.0, certain HIPAA / SOX deployments).
- Multi-tenant environments where an admin on one tenant could
  modify another tenant's chain on disk but cannot influence the
  TPM's PCR state without rebooting.
- Forensic workflows where "this anchor was created on this exact
  hardware at this boot session" is the load-bearing claim.

**Don't** use TPM sealing if:

- Your attacker model includes kernel-level access — they can replay
  unseal under a controlled PCR state.
- You run in a container without `/dev/tpm*` access, or rootless
  without TPM ACLs configured.
- Your retention window crosses planned firmware/Secure-Boot rotations
  — re-sealing is your responsibility.

## What TPM sealing proves

A successful `tpm_anchor_verify(ta) == TPM_ANCHOR_VALID` proves
**three** things, all together:

1. The inner `WitnessAnchor`'s self-hash matches its claimed contents
   (`anchor_verify_integrity` passes).
2. The host TPM can unseal the blob libro sealed at anchor-creation
   time — i.e. the PCRs named by `pcr_indices` are in the *same state*
   they were in when the anchor was sealed.
3. The unsealed bytes equal the inner anchor's hash (cryptographic
   binding — the seal can't be transplanted from another anchor).

In English: "This anchor was created on this TPM at PCR state X, and
the anchor data hasn't been tampered with since."

## What TPM sealing does *not* prove

- **Chain correctness.** A sealed anchor with wrong merkle root /
  entry count still verifies against the TPM. Call
  `anchor_verify_against_tree(inner, tree)` separately.
- **Host honesty.** A compromised host that controls the TPM at
  seal time can produce a fully-valid sealed anchor for arbitrary
  content. TPM sealing protects against *after-the-fact*
  tampering, not creation-time fraud.
- **Identity / authorship.** A TPM seal binds to a PCR state, not to
  a signer. Combine with Ed25519/ML-DSA-65 entry signing for
  attribution.

## Building with TPM support

Since 2.8.0 the TPM backend is sigil's, resolved through an **optional**
dep (`[deps.sigil_tpm]`) behind the `tpm` feature. The feature has to be
passed to **both** `cyrius deps` and `cyrius build` — `deps` decides
whether the fold is resolved into `lib/` at all, `build` decides whether
it is compiled in. Passing it to only `build` fails with
`undefined variable 'TPM_SHA256'`.

```sh
# Default build — no TPM surface resolved or linked at all, smaller binary.
CYRIUS_DCE=1 cyrius build src/main.cyr build/libro

# TPM-opt-in build — resolves sigil's tpm_* fold, then compiles
# src/tpm_anchor.cyr against it. One benign
# `duplicate fn '_sigil_random_fill'` warning is expected (sigil-tpm and
# sigil-mldsa both carry random).
cyrius deps --features tpm
CYRIUS_DCE=1 cyrius build --features tpm -D LIBRO_TPM src/main.cyr build/libro_tpm

# Restore the thin, tpm-free default resolution afterwards.
cyrius deps
```

Runtime requirements for the TPM-opt-in build to actually *seal*:

- `/dev/tpmrm0` or `/dev/tpm0` exists and is accessible to the libro
  process (`tss` group on most distros).
- `tpm2-tools` package installed (`tpm2_create`, `tpm2_load`,
  `tpm2_unseal` on `$PATH`).
- A directory the libro process can write to, for sigil's
  `tpm_seal` to drop `sealed.ctx` / `sealed.pub` / `sealed.priv`.

If any of those is missing, `tpm_anchor_new` returns 0 and the
consumer falls through to plain `anchor_*` API. libro's contract is
that the opt-in compile path never crashes on partial-environment
hosts — the test battery pins this.

## Usage

```cyrius
# Create a normal libro anchor first.
var tree = merkle_build(chain_entries(c));
var inner = anchor_new(tree, c);

# Seal it against the conservative AGNOS PCR policy (PCR 0 +
# PCR 7 — firmware + Secure Boot configuration).
var pcrs = tpm_anchor_default_pcr_indices();
var ta = tpm_anchor_new(inner, str_from("/var/lib/libro/seals"), pcrs);
if (ta == 0) {
    # No TPM, or tpm2-tools missing, or seal failed for some other
    # environmental reason. Consumer policy: degrade to software-only
    # anchor, OR refuse to continue. The choice belongs to the consumer.
}

# Verify. TPM_ANCHOR_VALID requires all three conditions in §
# "What TPM sealing proves" above.
var r = tpm_anchor_verify(ta);
if (r == TPM_ANCHOR_VALID) {
    # Hardware-backed.
} elif (r == TPM_ANCHOR_UNAVAILABLE) {
    # TPM was sealed-with but is now missing — likely a different
    # host than the one that created the anchor. Consumer policy.
} elif (r == TPM_ANCHOR_UNSEAL_FAILED) {
    # PCR state has changed since sealing. Either an attacker, or a
    # legitimate firmware/Secure-Boot update — consumer must
    # distinguish (compare against a known-good PCR baseline).
} elif (r == TPM_ANCHOR_HASH_MISMATCH) {
    # Cryptographic binding broken — the unsealed bytes don't match
    # the anchor's claimed hash. Strong tamper signal; reject.
} elif (r == TPM_ANCHOR_INNER_INVALID) {
    # Inner anchor self-hash check failed. Reject regardless of TPM.
}

# Strict bool form for policies that require hardware attestation:
if (tpm_anchor_verify_strict(ta) == 1) {
    # Accept.
}
```

## PCR selection

The default `tpm_anchor_default_pcr_indices()` selects **PCR 0 +
PCR 7**:

- **PCR 0** measures firmware (UEFI / coreboot). Detects firmware
  rollback or unauthorized image swaps.
- **PCR 7** measures the Secure Boot configuration (keys + policy).
  Detects unauthorized boot-chain key changes.

Both PCRs are stable across legitimate userspace and kernel updates;
they only change on firmware flashes or Secure Boot reconfiguration.
This is the AGNOS-aligned conservative default — tight enough to
detect attacks libro cares about, loose enough that ordinary system
maintenance doesn't invalidate seals.

Consumers with different threat models can pass their own
`pcr_indices` vec to `tpm_anchor_new`. Common alternatives:

- **PCR 0, 2, 7** — add firmware boot extras.
- **PCR 0, 4, 7** — also bind to the bootloader (PCR 4). Seals are
  invalidated on every kernel update — useful for "this anchor was
  created on this exact kernel image".
- **PCR 0, 7, 11** — bind to a measured initramfs. Tight; same
  re-seal cadence as bootloader binding.

Wider PCR selections give stronger attestation but force re-sealing
on every legitimate state change in the included PCRs. There's no
universally right choice — pick based on your maintenance cadence
and the cost of re-seal.

## Persistence

`tpm_seal` writes three files to the `output_dir` you supply:

- `sealed.ctx` — TPM-handle context. The opaque token unseal needs.
- `sealed.pub` — public area (TPM 2.0 sealed-object schema).
- `sealed.priv` — private area (encrypted under TPM SRK).

The `tpm_anchor` struct's `sealed_ctx` field is an opaque pointer to
sigil's `tpm_sealed` struct, which owns the `sealed.ctx` path. The
consumer is responsible for:

1. Persisting all three files alongside the chain.
2. Restoring them to the same path before calling `tpm_anchor_verify`
   (e.g. if the chain moves hosts within the same TPM).
3. Cleaning up on chain rotation (the next `tpm_anchor_new` produces
   a fresh seal, but the old files don't auto-delete).

There is no key-rotation API in 2.5.0. Re-sealing under a different
PCR policy is achieved by calling `tpm_anchor_new` again with the
new policy and discarding the old sealed files.

## CI / build matrix

libro's CI runs both builds:

- **Default build**: 502 tests pass, no TPM surface resolved or linked.
- **`-D LIBRO_TPM` build**: 514 tests pass, exercises the API
  surface on hosts without tpm2-tools. The hardware-roundtrip test
  is best-effort: it logs a skip if the host can't actually seal,
  otherwise it pins the full success path.

Adding a `LIBRO_TPM=1` build to CI for a real TPM-equipped runner
gives full functional coverage of the seal/unseal roundtrip. The
shipped CI workflow only covers the API-correctness contract.

## Roadmap

- **Re-seal API** (deferred — not in 2.5.0) — explicit
  `tpm_anchor_reseal(ta, new_pcrs)` for the firmware-update flow.
  Currently consumers handle this by `tpm_anchor_new` against the
  same inner anchor.
- **PCR baseline integration** — pair the seal with sigil's
  `pcr_measurement_new` baselines so a verifier can distinguish
  "PCR changed legitimately" from "PCR changed unexpectedly".
  Tracked as a 2.x.x candidate once a consumer asks.
- **Sealed-anchor JSON serialization** — currently `tpm_anchor`
  carries opaque sigil handles, which don't serialize. A
  consumer that wants to ship sealed anchors across hosts within
  the same TPM would need a serialization layer.
