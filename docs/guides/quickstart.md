# Quick Start

## Install Cyrius

```bash
cyriusup install 2.7.2 && cyriusup use 2.7.2
```

## Build & Test

```bash
git clone https://github.com/MacCracken/libro.git
cd libro
cyrius build src/main.cyr build/libro
./build/libro  # 193 tests, 0 failures
```

## Basic Usage

Include libro modules in your Cyrius program via single-file compilation:

```cyrius
include "lib/alloc.cyr"
include "lib/vec.cyr"
include "lib/str.cyr"
# ... (other stdlib as needed)

# Include libro modules (dependency order matters)
include "src/error.cyr"
include "src/sha256.cyr"
include "src/hasher.cyr"
include "src/entry.cyr"
include "src/verify.cyr"
include "src/query.cyr"
include "src/chain.cyr"
```

### Create a Chain and Append Entries

```cyrius
alloc_init();
fl_init();

var c = chain_new();

# Append an audit entry
chain_append(c, SEV_INFO, str_from("auth"), str_from("login"),
    str_from("{\"user\":\"alice\",\"ip\":\"10.0.0.1\"}"));

chain_append(c, SEV_SECURITY, str_from("aegis"), str_from("intrusion.detected"),
    str_from("{\"source\":\"10.0.0.5\",\"port\":22}"));

# Chain auto-links entries via SHA-256 hashes
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

### Query Entries

```cyrius
# Filter by source
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

# Inclusion proof for entry 0
var proof = merkle_proof(tree, 0);
var valid = merkle_verify_proof(proof);  # returns 1 if valid
```

### Export

```cyrius
# Export to CSV
var fd = file_open("audit.csv", O_WRONLY | O_CREAT | O_TRUNC, 0x1A4);
export_csv(chain_entries(c), fd);
file_close(fd);

# Export to JSON Lines
fd = file_open("audit.jsonl", O_WRONLY | O_CREAT | O_TRUNC, 0x1A4);
export_jsonl(chain_entries(c), fd);
file_close(fd);
```

### Sign Entries

```cyrius
var sk = signing_key_generate();
var e = vec_get(chain_entries(c), 0);
var sig = sign_entry(sk, e);

# Verify with derived key
var vk = verifying_key_from_signing(sk);
var valid = verify_entry_signature(vk, e, sig);  # returns 1

# Zeroize key material when done
signing_key_zeroize(sk);
```

### Streaming (Pub/Sub)

```cyrius
var stream = stream_new();
var sub = stream_subscribe(stream, "libro/auth/#");

# Publish entries — delivered to matching subscribers
stream_publish(stream, entry);

# Receive
var received = stream_recv(sub);
```

### Chain Review

```cyrius
var r = chain_review(c);
review_print(r);
# Prints: entry count, integrity status, time range, source/severity/agent distributions
```

## Retention Policies

```cyrius
# Keep last 1000 entries
var policy = retention_keep_count(1000);
var archive = chain_apply_retention(c, policy);

# Compliance presets
var pci = retention_pci_dss();       # 1 year
var hipaa = retention_hipaa();       # 6 years
var sox = retention_sox();           # 7 years
```
