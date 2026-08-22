# Testing Guide

## Running the Full Suite

```bash
# Build and run all tests (661 assertions expected, 0 failures)
CYRIUS_DCE=1 cyrius build src/main.cyr build/libro && ./build/libro

# Fuzz harness (12 targets, no-crash asserts, ~10 s)
CYRIUS_DCE=1 cyrius build fuzz/fuzz_libro.fcyr build/fuzz_libro && \
    timeout 30 ./build/fuzz_libro

# Optional: opt-in TPM build (default skips sigil's tpm_seal surface).
# `--features tpm` is required on BOTH commands: on `deps` to resolve the
# optional [deps.sigil_tpm] fold, and on `build` to compile it in. Omit it
# from `deps` and the build fails with `undefined variable 'TPM_SHA256'`.
# A benign `duplicate fn '_sigil_random_fill'` warning is expected here.
cyrius deps --features tpm
CYRIUS_DCE=1 cyrius build --features tpm -D LIBRO_TPM src/main.cyr build/libro_tpm && \
    ./build/libro_tpm   # 673 assertions: 661 default + 12 TPM-gated

# Restore the thin, tpm-free default afterwards — a bare `cyrius deps` does
# NOT undo the above (it leaves one extra lock entry: 113 vs the honest 112).
rm -rf lib && cyrius lib sync --full && cyrius deps

# Benchmarks (three binaries — cc5 5.4.2 fixup-table cap forced the core/io
# split in 1.2.0; libro_proof.bcyr added later for proof-path coverage)
CYRIUS_DCE=1 cyrius build benches/libro_core.bcyr  build/libro_bench_core  && \
    ./build/libro_bench_core
CYRIUS_DCE=1 cyrius build benches/libro_io.bcyr    build/libro_bench_io    && \
    ./build/libro_bench_io
CYRIUS_DCE=1 cyrius build benches/libro_proof.bcyr build/libro_bench_proof && \
    ./build/libro_bench_proof

# Bench history (opt-in: LIBRO_BENCH_HISTORY=<path>, LIBRO_BENCH_TAG=<label>)
LIBRO_BENCH_HISTORY=/tmp/libro_bench.csv LIBRO_BENCH_TAG=dev \
    ./build/libro_bench_core
```

CI runs all of these on every push/PR; it additionally enforces dist
freshness, manifest completeness, and raw-offset discipline via grep
gates (see `.github/workflows/ci.yml`).

## Test Categories

All tests are inline in `src/main.cyr` and grouped via `test_group("…")`.
The group list, in declaration order:

- SHA-256 + hasher
- Entry + severity + UUID
- Canonical JSON hashing (nested + scalar-aware)
- Chain append / verify / tamper detection
- Chain rotation + auto-rotation
- Chain batch append
- Chain query / filter / retention / pagination
- MemoryStore
- FileStore (append / load / verify / streamed verify — including the
  2.0.3 regression test for unterminated-tail input)
- PatraStore (SQL-backed storage, ungated in 1.1.0 after the UAF fix)
- PatraStore (perf tier — 2.4.0): sync-mode round-trip, append_batch
  correctness, prepared statements survive open, indexed by_source
- ChainIO (chain_export / chain_import round-trip)
- Export (JSONL + CSV with escaping)
- Review + integrity
- Merkle tree + inclusion / consistency proofs
- Signing (Ed25519 via sigil, key rotation via key_id)
- Signing (ML-DSA-65) — 2.2.0 NIST FIPS 204 entry signing battery
- Signing (Hybrid Ed25519+ML-DSA-65) — 2.3.0 AND-mode hybrid battery
- Anchoring (WitnessAnchor + meta-chain)
- Timestamping (RFC 3161 DER encode/decode)
- Integrity proof (signed tree heads, inclusion/consistency bundles,
  anchor bundles, proof_to_json + 2.6.0 proof_from_json round-trip
  including legacy bare-string path acceptance)
- Streaming (pub/sub with MQTT wildcards)
- Kernel audit (AGNOS /proc interface)
- Struct layout (#derive accessors) — 2.0.4 invariant tests for chain,
  iproof, anchor; extended in 2.3.0 to signing_key / verifying_key /
  entry_sig with the new slot-2 fields
- TPM-sealed anchors (LIBRO_TPM, opt-in) — 2.5.0 sealed-anchor battery
  (only built when `-D LIBRO_TPM` is set)
- Gap coverage (retention / query / CSV / compliance presets / merkle
  16-leaf / stream recv-drain / filestore multi-append)

**Total: 661 assertions default / 673 with `-D LIBRO_TPM`** across
these groups. Count moves with every sprint; the source of truth is
the output of `./build/libro`, not this document.

## Benchmarks

Three bench binaries ship 33 benchmarks total. The original core/io
split landed in 1.2.0 because cc5 5.4.2's 16384 fixup-table cap
couldn't hold a single combined binary after the 2.0 canonical-JSON
walker landed. A third binary (`libro_proof.bcyr`) was added for
proof-path benches. The 2.2 / 2.3 / 2.4 cycle added rows for PQ /
hybrid signing + PatraStore perf knobs; 2.7.2 added `proof_to_json_25`
once cyrius 6.1.23 cleared the long-standing bench-context hijack.

### libro_core (18 benchmarks)

| Benchmark | Iterations | Target |
|-----------|------------|--------|
| `sha256_64b` | 10000 | raw SHA-256 on 64 bytes |
| `entry_hash` | 1000 | `entry_compute_hash` full path |
| `chain_append_100` | 10 | 100 appends |
| `chain_append_batch_100` | 10 | batch version of above |
| `chain_verify_100` | 100 | full-chain verify over 100 entries |
| `merkle_build_100` | 100 | merkle tree build over 100 entries |
| `merkle_proof` | 1000 | inclusion proof generation |
| `merkle_verify` | 1000 | inclusion proof verification |
| `merkle_consistency` | 100 | RFC 9162 consistency proof |
| `sign_entry` | 1000 | Ed25519 sign |
| `verify_sig` | 1000 | Ed25519 verify |
| `mldsa65_sign_entry` | 100 | ML-DSA-65 sign (FIPS 204, sigil 3.0) |
| `mldsa65_verify_sig` | 100 | ML-DSA-65 verify (~2.2 ms — faster than Ed25519 verify; ML-DSA shipped via sigil 3.0) |
| `hybrid_sign_entry` | 100 | Ed25519 + ML-DSA-65 sign (sum-of-two) |
| `hybrid_verify_sig` | 100 | Ed25519 + ML-DSA-65 verify AND-mode |
| `query_filter_100` | 1000 | `chain_query` over 100 entries |
| `proof_unsigned_100` | 10 | unsigned integrity-proof build |
| `hex_encode_32b` | 10000 | 32-byte hex encode |

### libro_proof (3 benchmarks)

| Benchmark | Iterations | Target |
|-----------|------------|--------|
| `proof_build_unsigned_25` | 3 | `proof_build_unsigned` + `proof_with_all_inclusions` over 25 entries |
| `proof_build_signed_25`   | 3 | signed variant (Ed25519 tree head) |
| `proof_to_json_25`        | 3 | canonical-JSON serialization of a fully-inclusioned proof |

Iteration counts are deliberately low — each proof-build iteration
allocates an iproof + merkle tree + N inclusion proofs via the bump
allocator. At 100 entries × higher iterations the bump allocator
grows without bound (there's no `alloc_reset` mid-bench), so heap
pressure pushes into multi-GB territory. 25 entries × 3 iters keeps
the run bounded while still exercising the O(N log N) path.

**`proof_to_json_25` shipped in 2.7.2.** For years, measuring
`proof_to_json(ip)` inside `bench_run` triggered a control-flow
hijack: cc5 5.4.x manifested as ~25 Hz main() re-entry; cyrius
5.10.34 (re-tested in 2.1.1 + 2.2.0 + 2.5.0) manifested as SIGILL on
the first bench iteration. The same call always passed in the test
suite (`test_proof_to_json_*` + `test_proof_from_json_roundtrip_full`),
so the bug was bench-context-specific to `proof_json.cyr` rather than
`proof_to_json` itself. Re-tested against cyrius 6.1.23: **resolved.**
The bench now ships (with `proof_json.cyr` + its `store`/`export`/
`file_store` include closure) and runs clean (`proof_to_json_25:
~218 µs avg`).

### libro_io (12 benchmarks)

| Benchmark | Iterations | Target |
|-----------|------------|--------|
| `export_jsonl_100` | 100 | JSON Lines export of 100 entries |
| `export_csv_100` | 100 | CSV export of 100 entries |
| `chain_review_100` | 100 | chain review / summary |
| `anchor_create` | 100 | WitnessAnchor construction |
| `anchor_verify` | 100 | anchor integrity verification |
| `stream_publish` | 1000 | pub/sub publish |
| `streamed_verify_100` | 10 | `filestore_verify_streamed` over 100 entries |
| `filestore_load_10` | 100 | FileStore load+parse of 10 entries |
| `patra_append_50_full` | 5 | PatraStore append with SYNC_FULL (per-call fdatasync) |
| `patra_append_50_batch` | 5 | PatraStore append with SYNC_BATCH (group commit; real-disk ~64× faster than FULL per patra 1.8.0) |
| `patra_load_all_50` | 50 | PatraStore load_all via prepared SELECT |
| `patra_by_source_50` | 50 | PatraStore by_source via opt-in STR src_idx |

### Bench history

When `LIBRO_BENCH_HISTORY=<path>` is set, each bench emits one CSV row
(`epoch,binary,name,avg_ns,min_ns,max_ns,iterations,tag`). Unset → no-op.
`LIBRO_BENCH_TAG=<label>` adds a free-form label (e.g., commit SHA). CI
sets both, tags with `$GITHUB_SHA`, and uploads `bench-history.csv` as a
workflow artifact with 90-day retention — so perf trends accumulate
across PRs/releases without any local-machine dependency.

## Fuzz Harness

Single binary (`fuzz/fuzz_libro.fcyr`), 12 targets. All targets assert
no-crash on random input; a target that returns normally is a pass.

| Target | Input shape | Target surface |
|--------|-------------|----------------|
| `fuzz_sha256` | random bytes | SHA-256 primitive |
| `fuzz_hex_decode` | random ASCII | hex parser (odd lengths, non-hex chars) |
| `fuzz_der_parse` | random bytes biased toward DER SEQUENCE | `_der_parse_tlv` |
| `fuzz_entry_create` | random strings | `entry_new` + `entry_verify` |
| `fuzz_chain_ops` | random op selection | append / verify / rotate / query interleavings |
| `fuzz_sig_verify` | random 64-byte sigs | `verify_entry_signature` rejects garbage |
| `fuzz_json_parse` | random bytes | `json_parse` (parser boundary) |
| `fuzz_topic_match` | random topic patterns | `stream_subscribe` / `stream_publish` |
| `fuzz_chain_import` | random JSONL bytes via tempfile | `chain_import` parser |
| `fuzz_filestore_verify_streamed` | random JSONL bytes via tempfile | 64KB-streaming verify |
| `fuzz_canonical_json_hash` | random `details` payloads | 2.0 nested byte-walker in `entry_compute_hash` |
| `fuzz_proof_from_json` | random bytes | 2.0.6 proof-JSON byte-walker in `proof_from_json` |

Three fuzz targets were added in 2.0.3, one more in 2.0.6. `fuzz_filestore_verify_streamed`
caught a HIGH-severity infinite-loop bug on its first run
(see `docs/audit/2026-04-19-audit-2.0.md` Finding 4). The
`fuzz_proof_from_json` target exercises both the 2.6.0 object-form
parser (random `{` chars in input) and the legacy bare-string
path; both must survive without crash.

## Writing Tests

Add test functions inline to `src/main.cyr`. Match the local convention:

```cyrius
fn test_my_feature() {
    # Setup
    var c = chain_new();
    chain_append(c, SEV_INFO, str_from("src"), str_from("act"),
        str_from("{}"));

    # Assert
    assert(chain_len(c) == 1, "my feature works");
}
```

Register the test in the appropriate group under `main()`:

```cyrius
test_group("My Feature");
test_my_feature();
```

## Testing Patterns

### Tamper Detection

Bypass the hash-recomputation in entry construction by writing a field
directly, then call `entry_verify`:

```cyrius
# Tamper with severity — test locals named to avoid tripping the
# raw-offset CI guard (which forbids c+N / e+N etc. outside defining
# files). Tests under src/main.cyr may reach into entries via raw
# offsets for this kind of tamper-detection exercise; the allowlist
# in .github/workflows/ci.yml pre-registers the names.
store64(entry + 24, SEV_CRITICAL);
assert(entry_verify(entry) == 0, "tamper detected");
```

### Struct-Layout Invariants

The 2.0.4 layout-invariant tests (`test_layout_chain` /
`test_layout_iproof` / `test_layout_anchor`) are templates for
writing one per struct. 2.3.0 extended the trio to 7 layout
tests covering `signing_key`, `verifying_key`, `entry_sig`,
`merkle_tree`, `sth`, `filestore`, and the original chain /
iproof / anchor. Write sentinel values via raw offsets and assert
the derived accessors return them, then the reverse. Would catch
a Cyrius `#derive(accessors)` compiler regression before any
end-to-end test would notice.

### Streaming

```cyrius
var s = stream_new();
var sub = stream_subscribe(s, "libro/#");
stream_publish(s, entry);
assert(stream_pending(sub) == 1, "entry delivered");
var received = stream_recv(sub);
```

### FileStore Round-trip

```cyrius
var path = str_from("/tmp/libro_test.jsonl");
sys_unlink("/tmp/libro_test.jsonl");
var fs = filestore_open(path);
filestore_append(fs, entry);
var loaded = filestore_load_all(fs);
var verified = filestore_verify_streamed(fs, 8);
```

## CI Gates (grep-based)

CI runs these after format/lint/build/test and fails on any violation.
They exist to prevent regression of classes already caught in audits:

| Gate | Added | Catches |
|------|-------|---------|
| Manifest completeness | 2.0.1 (refined 2.5.0) | `[lib] modules` drifting from `src/main.cyr` includes; 2.5.0 skip `#ifdef`-gated includes for opt-in modules |
| Specific-struct raw-offset guard | 2.0.1 + 2.0.2 | `load64(c+N)`, `load64(ip+N)`, etc. outside defining file |
| Per-file allowlist | 2.0.4 (extended 2.5.0) | new raw-offset param names appearing in unregistered files; 2.5.0 registers `ta` for `src/tpm_anchor.cyr` |
| TPM-opt-in build check | 2.5.0 | `-D LIBRO_TPM` build + tests pass (661 → 673 assertions) |
| Dist freshness | 1.1.1 | `dist/libro.cyr` missing or stale vs `src/` |
| Version parity (release only) | 1.1.1 | VERSION / cyrius.cyml / dist header / git tag disagreement |

See `.github/workflows/ci.yml` for the exact shell snippets. Each gate
is a small grep + comm dance — reproducible locally.
