# Quick Start

## Install Cyrius

Libro pins the Cyrius version in `cyrius.cyml` (field `cyrius = "…"`). CI reads
this field and installs exactly that toolchain, so the authoritative way to
install is:

```bash
cyriusup install "$(grep -E '^cyrius[[:space:]]*=' cyrius.cyml | sed -E 's/.*"([^"]+)".*/\1/')"
```

As of this release the pin is **6.4.83**. If you've just cloned libro fresh,
you may also need `cyrius deps` once to populate the `lib/` symlinks. Libro
resolves a **thin** sigil surface (`lib/sigil-mldsa.cyr` plus
`lib/sigil_{sha_ni,sha256,hex}.cyr`) rather than the monolithic
`lib/sigil.cyr` — pulling the full bundle re-inlines an x509/RSA fold libro
never calls and adds ~13 MB of static `.bss` to every binary.

## Build & Test

```bash
git clone https://github.com/MacCracken/libro.git
cd libro

# Build (CYRIUS_DCE=1 matches CI/release)
CYRIUS_DCE=1 cyrius build src/main.cyr build/libro

# Run tests — expect "502 passed, 0 failed"
./build/libro
```

## Basic Usage

Libro's 21 library modules (+1 opt-in `src/tpm_anchor.cyr` behind `-D LIBRO_TPM`) compile as a single unit. In a consumer you can
either include them individually (useful inside the libro repo) or pull the
committed `dist/libro.cyr` bundle via `cyrius deps` (what downstream projects
actually do — see [DEPS-PATTERN.md](../../DEPS-PATTERN.md)).

For in-repo exploration, include the stdlib, sigil, patra, then the libro
modules in the order `src/main.cyr` uses:

```cyrius
include "lib/assert.cyr"
include "lib/alloc.cyr"
include "lib/vec.cyr"
include "lib/str.cyr"
include "lib/fmt.cyr"
include "lib/syscalls.cyr"
include "lib/io.cyr"
include "lib/freelist.cyr"
include "lib/hashmap.cyr"
include "lib/bayan.cyr"       # json + bigint (cyrius 6.1.25 carve)
include "lib/sakshi.cyr"
include "lib/chrono.cyr"
include "lib/sigil.cyr"       # SHA-256 + Ed25519 + ct_eq
include "lib/patra.cyr"       # SQL storage (only if you need PatraStore)

# Libro modules — order matches src/main.cyr's include list.
include "src/error.cyr"
include "src/hasher.cyr"
include "src/entry.cyr"
include "src/verify.cyr"
include "src/query.cyr"
include "src/retention.cyr"
include "src/chain.cyr"
include "src/store.cyr"
include "src/export.cyr"
include "src/review.cyr"
include "src/merkle.cyr"
include "src/signing.cyr"
include "src/anchoring.cyr"
include "src/timestamping.cyr"
include "src/proof.cyr"
include "src/kernel_audit.cyr"
include "src/file_store.cyr"
include "src/chain_io.cyr"
include "src/patra_store.cyr"
include "src/streaming.cyr"
include "src/proof_json.cyr"
```

For downstream consumers, the above collapses to one line once
`cyrius deps` has pulled the dist:

```cyrius
include "lib/libro_libro.cyr"    # after stdlib + sigil + patra
```

### Create a Chain and Append Entries

```cyrius
alloc_init();
fl_init();
ed25519_init();    # only if you will use signing

var c = chain_new();

chain_append(c, SEV_INFO, str_from("auth"), str_from("login"),
    str_from("{\"user\":\"alice\",\"ip\":\"10.0.0.1\"}"));

chain_append(c, SEV_SECURITY, str_from("aegis"), str_from("intrusion.detected"),
    str_from("{\"source\":\"10.0.0.5\",\"port\":22}"));
```

### Verify Integrity

```cyrius
var err = chain_verify(c);
if (err == 0) {
    println("chain integrity: VALID");
} else {
    println("chain integrity: BROKEN");
}
```

### Batch Append

```cyrius
# Four parallel vecs — severities / sources / actions / details.
var sevs = vec_new(); vec_push(sevs, SEV_INFO); vec_push(sevs, SEV_WARNING);
var srcs = vec_new(); vec_push(srcs, str_from("auth")); vec_push(srcs, str_from("auth"));
var acts = vec_new(); vec_push(acts, str_from("login")); vec_push(acts, str_from("fail"));
var dets = vec_new(); vec_push(dets, str_from("{}")); vec_push(dets, str_from("{\"attempts\":3}"));

var created = chain_append_batch(c, sevs, srcs, acts, dets);
# `created` is a vec of the new entry pointers. One rotation check for the
# whole batch (over-capacity is tolerated for the call's duration).
```

### Query Entries

```cyrius
# Single-field convenience
var auth_entries = chain_by_source(c, str_from("auth"));

# Composable filters (ANDed)
var q = query_new();
query_min_severity(q, SEV_WARNING);
query_source(q, str_from("aegis"));
var alerts = chain_query(c, q);
```

### Build Merkle Proofs

```cyrius
var tree = merkle_build(chain_entries(c));
var root = merkle_tree_root(tree);

# Inclusion proof for entry 0.
# Note: the function is `merkle_inclusion_proof` — the plain
# `merkle_proof` identifier is reserved by the struct type.
var proof = merkle_inclusion_proof(tree, 0);
var valid = merkle_verify_proof(proof);  # 1 if valid
```

### Export & Round-trip

```cyrius
# Stream-friendly exporters to a file descriptor.
var fd = file_open("audit.csv", O_WRONLY | O_CREAT | O_TRUNC, 0x1A4);
export_csv(chain_entries(c), fd);
file_close(fd);

fd = file_open("audit.jsonl", O_WRONLY | O_CREAT | O_TRUNC, 0x1A4);
export_jsonl(chain_entries(c), fd);
file_close(fd);

# Portable chain snapshot — meta header + entries, round-trips through chain_verify.
chain_export(c, str_from("snapshot.jsonl"));
var restored = chain_import(str_from("snapshot.jsonl"));
assert(chain_verify(restored) == 0, "snapshot round-trip preserves integrity");
```

### Sign Entries

```cyrius
var sk = signing_key_generate();
var e = vec_get(chain_entries(c), 0);
var sig = sign_entry(sk, e);

# Verify with derived verifying key.
var vk = verifying_key_from_signing(sk);
var valid = verify_entry_signature(vk, e, sig);  # 1 on success

# Zeroize key material when done.
signing_key_zeroize(sk);
```

### Integrity Proofs

```cyrius
var ip = proof_build_signed(c, sk);
proof_with_all_inclusions(ip);

var tree2 = iproof_tree(ip);
var anchor = anchor_new(tree2, c);
proof_with_anchor(ip, anchor);

# Pretty-printed JSON emission (ports Rust `to_proof_json`).
var json = proof_to_json(ip);

# Verify end-to-end.
var pv = proof_verify_signed(ip, vk);
assert(pv_is_valid(pv) == 1, "signed proof valid");
```

### Streaming (Pub/Sub)

```cyrius
var stream = stream_new();
var sub = stream_subscribe(stream, "libro/auth/#");

stream_publish(stream, entry);
var received = stream_recv(sub);
```

### Chain Review

```cyrius
var r = chain_review(c);
review_print(r);
# Prints entry count, integrity status, time range, source/severity/agent distributions.
```

## Retention Policies

```cyrius
var policy = retention_keep_count(1000);
var archive = chain_apply_retention(c, policy);

# Compliance presets
var pci = retention_pci_dss();   # 1 year  (PCI DSS 4.0 §10.7)
var hipaa = retention_hipaa();   # 6 years (HIPAA 45 CFR §164.530(j))
var sox = retention_sox();       # 7 years (SOX §802)
```

## Storage Backends

Libro ships three `AuditStore` implementations. Pick by persistence needs:

| Backend      | Persistence | Concurrent access | Query shape       |
|--------------|-------------|-------------------|-------------------|
| MemoryStore  | None        | Single process    | Load + in-memory filter |
| FileStore    | JSONL file  | Advisory flock    | Load + in-memory filter |
| PatraStore   | SQL (patra) | patra's locking   | Indexed SQL WHERE |

```cyrius
# FileStore — append-only JSON Lines with flock
var fs = filestore_open(str_from("audit.jsonl"));
filestore_append(fs, entry);
var verified = filestore_verify_streamed(fs, 8);    # bounded-memory

# PatraStore — patra-backed SQL
var ps = patrastore_open(str_from("audit.db"));
patrastore_append(ps, entry);
var rows = patrastore_query(ps, q);                 # uses SQL WHERE
```
