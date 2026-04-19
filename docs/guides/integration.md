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
