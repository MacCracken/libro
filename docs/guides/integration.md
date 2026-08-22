# Integration Patterns

How AGNOS ecosystem consumers use libro for audit logging.

## Daimon — Agent Lifecycle

```cyrius
var audit = chain_new();

# Agent registration
chain_append(audit, SEV_INFO, str_from("daimon"), str_from("agent.register"),
    str_from("{\"agent_id\":\"web-01\",\"sandbox\":\"landlock\"}"));

# Agent sandbox applied
chain_append(audit, SEV_INFO, str_from("daimon"), str_from("agent.sandbox"),
    str_from("{\"agent_id\":\"web-01\",\"policy\":\"network-only\"}"));

# Agent deregistration
chain_append(audit, SEV_INFO, str_from("daimon"), str_from("agent.deregister"),
    str_from("{\"agent_id\":\"web-01\",\"reason\":\"shutdown\"}"));
```

## Aegis — Security Events

```cyrius
var audit = chain_new();

# Policy violation
chain_append(audit, SEV_WARNING, str_from("aegis"), str_from("policy.violation"),
    str_from("{\"agent\":\"web-01\",\"policy\":\"fs-read\",\"path\":\"/etc/passwd\"}"));

# Intrusion attempt
chain_append(audit, SEV_SECURITY, str_from("aegis"), str_from("intrusion.detected"),
    str_from("{\"source\":\"10.0.0.5\",\"port\":22,\"attempts\":5}"));

# Query security events for incident response
var q = query_new();
query_min_severity(q, SEV_SECURITY);
var incidents = chain_query(audit, q);
```

## Stiva — Container Lifecycle

```cyrius
var audit = chain_new();

chain_append(audit, SEV_INFO, str_from("stiva"), str_from("container.create"),
    str_from("{\"id\":\"ctr-abc\",\"image\":\"nginx:1.25\"}"));

chain_append(audit, SEV_INFO, str_from("stiva"), str_from("container.start"),
    str_from("{\"id\":\"ctr-abc\",\"pid\":12345}"));

chain_append(audit, SEV_WARNING, str_from("stiva"), str_from("container.oom"),
    str_from("{\"id\":\"ctr-abc\",\"rss_mb\":512}"));

chain_append(audit, SEV_INFO, str_from("stiva"), str_from("container.stop"),
    str_from("{\"id\":\"ctr-abc\",\"exit_code\":137}"));
```

## Sigil — Trust Decisions

```cyrius
var audit = chain_new();

# Sign and anchor the chain for third-party verification
var sk = signing_key_generate();

chain_append(audit, SEV_INFO, str_from("sigil"), str_from("key.rotate"),
    str_from("{\"old_key\":\"abc...\",\"new_key\":\"def...\"}"));

chain_append(audit, SEV_INFO, str_from("sigil"), str_from("signature.verify"),
    str_from("{\"package\":\"daimon-1.2.0\",\"result\":\"valid\"}"));

# Build integrity proof for external audit
var ip = proof_build_signed(audit, sk);
proof_with_all_inclusions(ip);
var tree = iproof_tree(ip);
var anchor = anchor_new(tree, audit);
proof_with_anchor(ip, anchor);

# Verify proof independently
var vk = verifying_key_from_signing(sk);
var pv = proof_verify_signed(ip, vk);
# pv_is_valid(pv) == 1
```

## Ark — Package Operations

```cyrius
var audit = chain_new();

chain_append(audit, SEV_INFO, str_from("ark"), str_from("package.install"),
    str_from("{\"name\":\"nginx\",\"version\":\"1.25.3\",\"hash\":\"a1b2c3...\"}"));

chain_append(audit, SEV_INFO, str_from("ark"), str_from("package.update"),
    str_from("{\"name\":\"nginx\",\"from\":\"1.25.3\",\"to\":\"1.25.4\"}"));

chain_append(audit, SEV_WARNING, str_from("ark"), str_from("package.vulnerability"),
    str_from("{\"name\":\"openssl\",\"cve\":\"CVE-2026-1234\",\"severity\":\"high\"}"));

# Export for compliance audit
var fd = file_open("package-audit.csv", O_WRONLY | O_CREAT | O_TRUNC, 0x1A4);
export_csv(chain_entries(audit), fd);
file_close(fd);
```

## Streaming Pattern — Cross-Module

```cyrius
var stream = stream_new();

# Each consumer subscribes to its topic hierarchy
var daimon_sub = stream_subscribe(stream, "libro/daimon/#");
var security_sub = stream_subscribe(stream, "libro/aegis/#");
var all_sub = stream_subscribe(stream, "libro/#");

# Any append publishes to matching subscribers
chain_append(audit, SEV_SECURITY, str_from("aegis"), str_from("alert"),
    str_from("{\"type\":\"brute-force\"}"));
var e = vec_get(chain_entries(audit), chain_len(audit) - 1);
stream_publish(stream, e);

# security_sub and all_sub receive it; daimon_sub does not
```

## Portable Chain Snapshot — `chain_export` / `chain_import`

```cyrius
var audit = chain_new();
chain_append(audit, SEV_INFO, str_from("daimon"), str_from("boot"),
    str_from("{\"host\":\"node-01\"}"));
chain_append(audit, SEV_WARNING, str_from("daimon"), str_from("quota"),
    str_from("{\"agent\":\"web-01\",\"used_mb\":512}"));

# Round-trip to a JSON Lines snapshot. Line 0 is the chain meta
# (prev_chain_hash + max_capacity); lines 1+ are entries in the
# same shape FileStore emits.
chain_export(audit, str_from("audit.jsonl"));

# Reload on a different host, or after a restart.
var restored = chain_import(str_from("audit.jsonl"));

# The restored chain hash-verifies end-to-end without the original
# memory. Consumers forwarding snapshots between nodes should verify
# before trusting the contents.
var verify_err = chain_verify(restored);
# verify_err == 0 on success; non-zero is an error object.
# ⚠ chain_import itself returns 0 for a missing or invalid header (since
# 2.8.11 it actually honours that contract — before, a headerless JSONL
# imported minus its genesis entry and still looked valid). Check it:
if (restored == 0) { /* not a libro chain export */ }

# Overflow archives (from capacity-capped rotation) are NOT part of
# the snapshot — drive a FileStore or PatraStore directly if you
# need full-history persistence across rotations.
```

## Telling an error from a result (2.8.11+)

libro's pervasive convention is "return the value on success, an error struct
pointer on failure". **Both are non-zero pointers**, so `if (r != 0)` reads a
failure as a success — which for an integrity library is the worst possible
direction. Since 2.8.11 every error object carries a magic word and there is a
predicate for it:

```cyrius
var entries = filestore_load_and_verify(fs);
if (libro_is_error(entries) == 1) {
    # An integrity violation OR an unreadable log. Before 2.8.11 both of
    # these were indistinguishable from success — a DELETED audit log
    # verified clean, because load_all returned an empty vec.
    println(error_msg(entries));
} else {
    # entries is the vec
}
```

Same for `patrastore_load_and_verify` and `entry_new_validated`:

```cyrius
var e = entry_new_validated(SEV_INFO, source, action, details, prev);
if (libro_is_error(e) == 1) { /* field too long — error_code(e) says which */ }
```

⚠ **Count-returning APIs are different.** `filestore_verify_streamed` and
`memstore_verify_streamed` return the NUMBER of entries verified on success —
and `0` is a legitimate success for an empty log. Use `libro_is_error(r) == 0`,
never `libro_is_ok(r)`:

```cyrius
var n = filestore_verify_streamed(fs, 8);
if (libro_is_error(n) == 1) { /* I/O failure, or a record over 64KB */ }
else { /* n entries verified; n may legitimately be 0 */ }
```

`libro_is_error` is safe to call on any of these — it rejects small integers
and misaligned values before dereferencing.

## Retention Pattern — Compliance

```cyrius
var audit = chain_new();
# ... append entries over time ...

# Apply PCI DSS retention (1 year)
var policy = retention_pci_dss();
var archive = chain_apply_retention(audit, policy);

if (archive != 0) {
    # Export archived entries before discarding
    var fd = file_open("archive.jsonl", O_WRONLY | O_CREAT | O_TRUNC, 0x1A4);
    export_jsonl(archive_entries(archive), fd);
    file_close(fd);
}
```

## Post-Quantum Signing — ML-DSA-65 (NIST FIPS 204)

libro 2.2.0 adds ML-DSA-65 entry signing alongside Ed25519. The
two algorithms share `sign_entry` / `verify_entry_signature` /
`sign_tree_head` / `verify_tree_head` — dispatch happens via
the `algorithm` field on the signing / verifying key, set at
keygen time.

```cyrius
# Ed25519 (default — pre-2.2 callers unchanged)
var sk_ed = signing_key_generate();
var sig   = sign_entry(sk_ed, entry);     # 64-byte signature
var vk_ed = verifying_key_from_signing(sk_ed);
verify_entry_signature(vk_ed, entry, sig);

# ML-DSA-65 (FIPS 204, 2.2.0+)
var sk_pq = signing_key_generate_mldsa();
var sig   = sign_entry(sk_pq, entry);     # 3309-byte signature
var vk_pq = verifying_key_from_signing(sk_pq);
verify_entry_signature(vk_pq, entry, sig);
```

### Sizing

| Field | Ed25519 | ML-DSA-65 | Ratio |
|-------|---------|-----------|------:|
| Public key (`sk.pub_bytes`) | 32 B | 1952 B | 61× |
| Secret key (`sk.bytes`)     | 64 B | 4032 B | 63× |
| Signature (raw)             | 64 B | 3309 B | 51× |
| Signature (hex in `entry_sig.signature`) | 128 chars | 6618 chars | 51× |

The `signing_key` and `verifying_key` struct *layouts* don't
change — those fields are pointers, and the buffers behind them
just allocate to the right size for the chosen algorithm.

### Performance (sigil 3.12.1 / cyrius 6.4.83, x86_64 dev host)

| Op | Ed25519 | ML-DSA-65 | Notes |
|----|--------:|----------:|-------|
| `sign_entry`         | 1.1 ms | 3.4 ms | ~3× Ed25519 sign — down from ~14 ms at sigil 3.7.10, the rejection-loop cost having largely gone in the 3.8–3.12 line |
| `verify_entry_signature` | 6.6 ms | 2.1 ms | ML-DSA verify is ~3× faster than Ed25519 verify in this build |

(For reference, hybrid Ed25519+ML-DSA-65 on the same host: `sign_entry`
3.9 ms, `verify_entry_signature` 8.9 ms — both keys exercised.)

Figures are the `libro_core` bench means at 2.8.3; re-measure with
`CYRIUS_DCE=1 cyrius build benches/libro_core.bcyr build/libro_bench_core`
rather than trusting them across a dep bump — the ML-DSA sign figure
moved 4× between sigil pins.

Per-entry signing in the millisecond range is fine for audit
workloads (one entry per kernel-audit / aegis-event / stiva-state-
change). For batch signing of many entries the cost is linear.
Libro's own verify path is one entry at a time; sigil ships a
parallel `sv_verify_batch` (truly parallel since 3.6.0), but libro
does not currently wire it up — a consumer verifying large chains
can parallelize across entries itself.

### When to use which

- **Ed25519** is the default — smaller signatures, well-understood
  threat model, and the right choice for chains that don't anchor
  long-lived audit trails into a post-quantum world.
- **ML-DSA-65** is the right choice when your audit retention
  window meaningfully overlaps with cryptographically-relevant
  quantum computers — i.e. PCI / SOX / HIPAA workloads with
  multi-year retention, or compliance regimes that explicitly
  flag PQ readiness (NIST CNSA 2.0, federal post-quantum mandates).
- **Hybrid (Ed25519 + ML-DSA-65)** is on the 2.3.x roadmap — it
  produces both signatures per entry, lets verifiers accept the
  policy-required subset, and is the migration path for chains
  that need to outlive a single algorithm's threat horizon. Until
  it lands, opt new chains directly into ML-DSA-65 if PQ is the
  goal.

### Algorithm dispatch

`verify_entry_signature(vk, e, es)` dispatches on `vk.algorithm`,
not on the signature's claimed algorithm. This is intentional:
the verifying key is the trust anchor, and an attacker who could
swap the algorithm string in `entry_sig` shouldn't be able to
trick the verifier into accepting a signature under a primitive
the consumer didn't authorize.

A signature produced under algorithm A and verified against a vk
configured for algorithm B is rejected: B's primitive will fail
to decode the bytes (different layout) or fail to verify (wrong
math). The `test_signing_cross_alg_rejected` battery in
`src/main.cyr` pins this.

## Hybrid Signing — Ed25519 + ML-DSA-65 (2.3.0)

libro 2.3.0 adds hybrid signing as the migration path for chains
that need to outlive a single algorithm's threat horizon. A hybrid
signing key carries *both* an Ed25519 keypair and an ML-DSA-65
keypair (independent seeds, no shared entropy). Each sign call
produces both signatures; verify requires *both* to validate
(AND-mode, matching sigil's `sigil_verify_hybrid` contract).

```cyrius
# Hybrid keygen — two independent 32-byte seeds, two keypairs.
var sk = signing_key_generate_hybrid();

# sign_entry produces an entry_sig carrying both signatures.
var sig = sign_entry(sk, entry);
# sig.signature       = Ed25519 hex   (128 chars)
# sig.signature_2     = ML-DSA-65 hex (6618 chars)
# sig.verifying_key   = Ed25519 pk hex
# sig.verifying_key_2 = ML-DSA-65 pk hex
# sig.algorithm       = "Hybrid(Ed25519+ML-DSA-65)"

# Verify dispatches on vk.algorithm == SIG_ALG_HYBRID and gates
# both primitives — Ed25519 verify AND ML-DSA-65 verify must pass.
var vk = verifying_key_from_signing(sk);
verify_entry_signature(vk, entry, sig);
```

### Why hybrid

A hybrid signature is a single audit-chain assertion under two
distinct cryptographic assumptions. If either algorithm is broken
in the future — Ed25519 by some discrete-log advance, ML-DSA-65
by a lattice-cryptanalysis breakthrough — the *other* still
witnesses the entry's authenticity. For audit chains with
multi-year retention windows, hybrid is the conservative choice
during the transition era.

The cost is roughly additive — both primitives run on every sign
and every verify. Sigil's published numbers + libro's measured
2.3.0 baseline:

| Op | Ed25519 | ML-DSA-65 | Hybrid (sum) |
|----|--------:|----------:|-------------:|
| `sign_entry`             | 1.1 ms | 3.5 ms | 4.6 ms |
| `verify_entry_signature` | 6.6 ms | 2.1 ms | 8.7 ms |

Per-entry sign + verify in the millisecond range stays well
within the per-event budget for kernel-audit / aegis / stiva
workloads.

### Migration story — Ed25519 → Hybrid → ML-DSA-65

A consumer worried about post-quantum threats but not ready to
abandon Ed25519 today follows this rotation:

1. **2.x today** — chain begins Ed25519-only. Consumers verify
   with Ed25519 vks. Existing code path.
2. **Rotate to hybrid** — at a chain boundary (e.g., a chain
   rotation under retention policy, or a fresh log file), switch
   to `signing_key_generate_hybrid()`. New entries carry both
   signatures; verifiers either upgrade to a hybrid vk (validates
   both) or keep using their Ed25519-only vk (validates the
   Ed25519 portion only — see "Backwards-compatible verify"
   below).
3. **Eventually rotate to ML-DSA-65 only** — once the consumer's
   threat model considers Ed25519 retired (a multi-year horizon),
   another chain boundary moves to `signing_key_generate_mldsa()`.
   Hybrid entries from step 2 remain valid under their hybrid vks
   indefinitely.

### Backwards-compatible verify

A hybrid `entry_sig` is structurally a superset of an Ed25519
`entry_sig`: slot-1 carries the same Ed25519 hex sig + pk a
pre-2.3 verifier expected. A consumer who has *not* upgraded to
a hybrid vk can still verify the Ed25519 portion of a hybrid
signature by constructing an Ed25519-only vk:

```cyrius
# Read just the Ed25519 portion of a hybrid sig with a 2.x vk.
var vk_ed = verifying_key_from_bytes(
    hex_decode_str(entry_sig_verifying_key(sig)));
# vk_ed.algorithm == SIG_ALG_ED25519 → verify dispatches to ed25519_verify.
verify_entry_signature(vk_ed, entry, sig);
```

This is the explicit single-algorithm fallback — useful while a
fleet rolls out the hybrid-aware verifier. It loses the PQ
guarantee for that consumer; the chain itself still has the
ML-DSA-65 signatures recorded for any verifier who *does* upgrade.

### Tree-head signature shape

`sign_tree_head` returns a single `Str` for backward compatibility
with pre-2.3 callers (proof.cyr's `iproof.tree_head.signature`
field is one Str, not two). For hybrid mode the two hex sigs are
concatenated with a `|` delimiter:

```
ed25519_hex_sig | mldsa65_hex_sig
```

`verify_tree_head` finds the pipe, splits, and dispatches
`sigil_verify_hybrid`. The pipe character is unambiguous because
hex digits never include `|`. Length: 128 + 1 + 6618 = 6747 chars.

## PatraStore — performance tier (2.4.0)

libro 2.4.0 wires patra 1.7–1.9's perf surface (prepared statements,
group commit, STR-keyed btree indexes) into the PatraStore API
without changing the existing call shape. Existing consumers see no
behaviour change; opt-in knobs unlock real-disk speedups on bulk
write and indexed read paths.

### Bulk append with batched fsync

```cyrius
# Replaces N individual patrastore_append calls in a loop:
var ok = patrastore_append_batch(ps, entries_vec);
# `ok` is the count of successful inserts. The fn internally toggles
# patra to SYNC_BATCH mode for the loop and flushes + restores the
# caller's prior sync mode before returning.
```

Patra's `SYNC_BATCH` mode amortizes fdatasync across multiple
mutating writes — auto-flushes every 64 writes (the `PATRA_BATCH_FLUSH_N`
threshold), on `patra_flush`, on `patra_close`, or on switch back to
`SYNC_FULL`. Documented speedup on real-disk btrfs/nvme is ~64×
(per the patra 1.8.0 changelog: 19.5 ms/insert SYNC_FULL → 306 µs/insert
SYNC_BATCH amortized for 500-insert bulk loops).

**Durability contract**: a successful `patrastore_append_batch` means
all entries are in OS page cache and visible to the same process. The
final `patrastore_flush` ensures they survive a crash; without that
flush, up to 63 entries could be lost (the auto-flush window).

**Tmpfs caveat**: on `/tmp` (tmpfs) fdatasync is a no-op and the
SYNC_FULL vs SYNC_BATCH delta vanishes. libro's bench rows
(`patra_append_50_full` / `patra_append_50_batch` in
`libro_bench_io`) run on `/tmp` and show ~3% delta — that's
bookkeeping overhead, not the real-disk win. To measure the real
win, point a bench at a btrfs/nvme path.

### Consumer-driven sync-mode control

For workloads that bulk-import across multiple `patrastore_append_batch`
calls (e.g. a full-chain replay from an archive), the consumer can
hold the BATCH window open across calls:

```cyrius
patrastore_set_sync_mode(ps, PATRA_SYNC_BATCH);
patrastore_append_batch(ps, batch_1);    # preserves caller's BATCH
patrastore_append_batch(ps, batch_2);    # still in BATCH
patrastore_flush(ps);                    # one fdatasync for both
patrastore_set_sync_mode(ps, PATRA_SYNC_FULL);
```

`patrastore_append_batch` checks the caller's sync mode at entry and
restores it before returning, so it composes cleanly with this
pattern.

### Indexed by-source queries

By default PatraStore has no secondary indexes — every
`patrastore_by_source` call scans the table. For consumers that
filter by source frequently and have many entries, opt in to a
STR-keyed btree index:

```cyrius
var ps = patrastore_open(path);
patrastore_create_source_index(ps);  # one-time setup; idempotent
# Subsequent by_source calls take the O(log N) btree path.
var kernel_rows = patrastore_by_source(ps, str_from("kernel"));
```

**Trade-off**: the index makes every subsequent
`patrastore_append` slower — every insert must also update the
btree page. Patra's published numbers (1.7.1 changelog) show ~21%
speedup on STR-equality SELECT at 500 entries; the per-insert
overhead is in the single-digit-µs range on real disk.

Heuristic for whether to opt in:

- Query-heavy + selective (`vec_len(by_source(...)) << total`): yes.
- Write-heavy + rare source filters: no — the index slows the
  write hot path without payoff.
- Mixed: bench both with your actual workload shape before deciding.

### Prepared SELECT and COUNT statements (transparent)

`patrastore_load_all` and `patrastore_len` use patra prepared
statements internally — parsed once at `patrastore_open`, finalized
at `patrastore_close`. This skips the ~8 µs tokenize+parse step per
call. No API change; the gain is automatic for existing consumers.

`patrastore_append` and `patrastore_by_source` stay un-prepared
because their SQL is value-templated per call (patra has no bind-
parameter API as of 1.9.3).

## Hardening — Landlock sandbox for PatraStore

PatraStore opens `.patra` files at consumer-supplied paths. A
defense-in-depth measure for daemon-style deployments is to apply
a Linux Landlock policy that restricts the libro process to only
the audit-data directory tree, eliminating arbitrary file-read /
file-traversal as a post-compromise primitive.

Cyrius 5.7.35+ ships `lib/security.cyr` with the Landlock enums
and `lib/syscalls_<arch>_linux.cyr` with the three syscall
wrappers. This is consumer-side glue — libro itself stays
unopinionated about sandbox policy, since the right deny-list
depends on the consumer's deployment shape.

```cyrius
include "lib/security.cyr"
include "lib/syscalls.cyr"

# Build a ruleset that gates filesystem writes + reads.
var attr[16];
store64(&attr,
    LANDLOCK_ACCESS_FS_WRITE_FILE
  | LANDLOCK_ACCESS_FS_READ_FILE
  | LANDLOCK_ACCESS_FS_READ_DIR
  | LANDLOCK_ACCESS_FS_MAKE_REG);
var ruleset_fd = sys_landlock_create_ruleset(&attr, 8, 0);
if (ruleset_fd < 0) { /* kernel < 5.13 or syscall blocked */ }

# Allow read+write+create only beneath /var/lib/audit/.
var path_fd = sys_open("/var/lib/audit", O_PATH | O_CLOEXEC, 0);
var rule_attr[16];
store64(&rule_attr,
    LANDLOCK_ACCESS_FS_WRITE_FILE
  | LANDLOCK_ACCESS_FS_READ_FILE
  | LANDLOCK_ACCESS_FS_READ_DIR
  | LANDLOCK_ACCESS_FS_MAKE_REG);
store64(&rule_attr + 8, path_fd);
sys_landlock_add_rule(ruleset_fd, LANDLOCK_RULE_PATH_BENEATH,
    &rule_attr, 0);
sys_close(path_fd);

# Apply. The calling thread (and any clone()'d children) can no
# longer touch files outside /var/lib/audit. This cannot be relaxed.
sys_landlock_restrict_self(ruleset_fd, 0);

# After this point, patrastore_open(...) inside /var/lib/audit/ works.
# patrastore_open(...) anywhere else fails with EACCES, and so does
# any incidental dlopen / config-file read attempt by transitively
# linked code.
```

Notes:

- Landlock restricts only the calling thread + descendants. Don't
  apply it before forking off helper processes that need broader
  access.
- The ruleset is monotonic — once applied, it cannot be widened.
  Apply it once at process start, after libro's `patra_init()` has
  done any setup that needs unsandboxed fs access.
- Kernel 5.13+ required (`sys_landlock_create_ruleset` returns
  `-ENOSYS` on older kernels). Detect at runtime and degrade to
  unsandboxed operation if the consumer's deployment matrix
  spans older kernels.
- Pairs naturally with `getrandom` for entropy (already used by
  libro 2.1.1's `signing_key_generate` and
  `ts_request_generate_nonce`) — both work inside a sandboxed
  process where `/dev/urandom` might be denied by the ruleset.
