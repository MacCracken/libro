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
# verify_err == 0 on success; non-zero is an error object

# Overflow archives (from capacity-capped rotation) are NOT part of
# the snapshot — drive a FileStore or PatraStore directly if you
# need full-history persistence across rotations.
```

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

### Performance (sigil 3.0.1, x86_64 dev host)

| Op | Ed25519 | ML-DSA-65 | Notes |
|----|--------:|----------:|-------|
| `sign_entry`         | 1.1 ms | 3.5 ms | PQ sign rejection-loops average 4–5 iterations |
| `verify_entry_signature` | 6.6 ms | 2.1 ms | ML-DSA verify is faster than Ed25519 verify in this build |

Per-entry signing in the millisecond range is fine for audit
workloads (one entry per kernel-audit / aegis-event / stiva-state-
change). For batch signing of many entries, the cost is linear
and verifiers can parallelize across entries (single-threaded
today, see roadmap §2.x).

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
