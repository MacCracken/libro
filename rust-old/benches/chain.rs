use criterion::{Criterion, criterion_group, criterion_main};
#[cfg(feature = "sqlite")]
use libro::SqliteStore;
use libro::export;
use libro::merkle::MerkleTree;
use libro::query::QueryFilter;
#[cfg(feature = "signing")]
use libro::signing::SigningKey;
use libro::store::AuditStore;
use libro::{AuditChain, EventSeverity};

fn make_chain(n: usize) -> AuditChain {
    let mut chain = AuditChain::new();
    for i in 0..n {
        chain.append(
            EventSeverity::Info,
            "bench",
            format!("event-{i}"),
            serde_json::json!({"i": i}),
        );
    }
    chain
}

// --- Chain operations ---

fn bench_append_100(c: &mut Criterion) {
    c.bench_function("chain_append_100", |b| b.iter(|| make_chain(100)));
}

fn bench_append_1000(c: &mut Criterion) {
    c.bench_function("chain_append_1000", |b| b.iter(|| make_chain(1000)));
}

fn bench_verify_100(c: &mut Criterion) {
    let chain = make_chain(100);
    c.bench_function("chain_verify_100", |b| b.iter(|| chain.verify().unwrap()));
}

fn bench_verify_1000(c: &mut Criterion) {
    let chain = make_chain(1000);
    c.bench_function("chain_verify_1000", |b| b.iter(|| chain.verify().unwrap()));
}

// --- Merkle tree ---

fn bench_merkle_build_1000(c: &mut Criterion) {
    let chain = make_chain(1000);
    let entries = chain.entries();
    c.bench_function("merkle_build_1000", |b| {
        b.iter(|| MerkleTree::build(entries).unwrap())
    });
}

fn bench_merkle_proof(c: &mut Criterion) {
    let chain = make_chain(1000);
    let tree = MerkleTree::build(chain.entries()).unwrap();
    c.bench_function("merkle_proof_1000", |b| b.iter(|| tree.proof(500).unwrap()));
}

fn bench_merkle_verify_proof(c: &mut Criterion) {
    let chain = make_chain(1000);
    let tree = MerkleTree::build(chain.entries()).unwrap();
    let proof = tree.proof(500).unwrap();
    c.bench_function("merkle_verify_proof", |b| {
        b.iter(|| libro::merkle::verify_proof(&proof))
    });
}

fn bench_merkle_consistency_proof(c: &mut Criterion) {
    let chain = make_chain(1000);
    let tree = MerkleTree::build(chain.entries()).unwrap();
    c.bench_function("merkle_consistency_1000", |b| {
        b.iter(|| tree.consistency_proof(500).unwrap())
    });
}

fn bench_merkle_verify_consistency(c: &mut Criterion) {
    let chain = make_chain(1000);
    let tree = MerkleTree::build(chain.entries()).unwrap();
    let proof = tree.consistency_proof(500).unwrap();
    c.bench_function("merkle_verify_consistency", |b| {
        b.iter(|| libro::merkle::verify_consistency(&proof))
    });
}

// --- Query ---

fn bench_query_1000(c: &mut Criterion) {
    let chain = make_chain(1000);
    let filter = QueryFilter::new().source("bench").action("event-500");
    c.bench_function("query_filter_1000", |b| b.iter(|| chain.query(&filter)));
}

// --- Export ---

fn bench_export_jsonl_1000(c: &mut Criterion) {
    let chain = make_chain(1000);
    let entries = chain.entries();
    c.bench_function("export_jsonl_1000", |b| {
        b.iter(|| {
            let mut buf = Vec::with_capacity(1024 * 1024);
            export::to_jsonl(entries, &mut buf).unwrap();
            buf
        })
    });
}

fn bench_export_csv_1000(c: &mut Criterion) {
    let chain = make_chain(1000);
    let entries = chain.entries();
    c.bench_function("export_csv_1000", |b| {
        b.iter(|| {
            let mut buf = Vec::with_capacity(1024 * 1024);
            export::to_csv(entries, &mut buf).unwrap();
            buf
        })
    });
}

// --- File store ---

fn bench_file_store_append_100(c: &mut Criterion) {
    let entries: Vec<_> = {
        let chain = make_chain(100);
        chain.entries().to_vec()
    };
    c.bench_function("file_store_append_100", |b| {
        b.iter(|| {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("bench.jsonl");
            let mut store = libro::FileStore::open(&path).unwrap();
            for e in &entries {
                store.append(e).unwrap();
            }
        })
    });
}

fn bench_append_batch_1000(c: &mut Criterion) {
    c.bench_function("chain_append_batch_1000", |b| {
        b.iter(|| {
            let mut chain = AuditChain::new();
            let events: Vec<_> = (0..1000)
                .map(|i| {
                    (
                        EventSeverity::Info,
                        "bench".to_owned(),
                        format!("event-{i}"),
                        serde_json::json!({}),
                    )
                })
                .collect();
            chain.append_batch(events);
            chain
        })
    });
}

fn bench_page_1000(c: &mut Criterion) {
    let chain = make_chain(1000);
    c.bench_function("chain_page_mid_100", |b| b.iter(|| chain.page(450, 100)));
}

fn bench_file_store_load_100(c: &mut Criterion) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bench.jsonl");
    let mut store = libro::FileStore::open(&path).unwrap();
    let chain = make_chain(100);
    for e in chain.entries() {
        store.append(e).unwrap();
    }
    c.bench_function("file_store_load_100", |b| {
        b.iter(|| store.load_all().unwrap())
    });
}

// --- Signing ---

#[cfg(feature = "signing")]
fn bench_sign_entry(c: &mut Criterion) {
    let key = SigningKey::generate();
    let chain = make_chain(1);
    let entry = &chain.entries()[0];
    c.bench_function("sign_entry", |b| b.iter(|| key.sign(entry)));
}

#[cfg(feature = "signing")]
fn bench_verify_signature(c: &mut Criterion) {
    let key = SigningKey::generate();
    let chain = make_chain(1);
    let entry = &chain.entries()[0];
    let sig = key.sign(entry);
    let vk = key.verifying_key();
    c.bench_function("verify_signature", |b| b.iter(|| sig.verify(entry, &vk)));
}

// --- SQLite store ---

#[cfg(feature = "sqlite")]
fn bench_sqlite_append_100(c: &mut Criterion) {
    let entries: Vec<_> = {
        let chain = make_chain(100);
        chain.entries().to_vec()
    };
    c.bench_function("sqlite_append_100", |b| {
        b.iter(|| {
            let mut store = SqliteStore::in_memory().unwrap();
            for e in &entries {
                store.append(e).unwrap();
            }
        })
    });
}

#[cfg(feature = "sqlite")]
fn bench_sqlite_query_100(c: &mut Criterion) {
    let mut store = SqliteStore::in_memory().unwrap();
    let chain = make_chain(100);
    for e in chain.entries() {
        store.append(e).unwrap();
    }
    let filter = QueryFilter::new().source("bench");
    c.bench_function("sqlite_query_100", |b| {
        b.iter(|| store.query(&filter).unwrap())
    });
}

criterion_group!(
    benches,
    bench_append_100,
    bench_append_1000,
    bench_verify_100,
    bench_verify_1000,
    bench_merkle_build_1000,
    bench_merkle_proof,
    bench_merkle_verify_proof,
    bench_merkle_consistency_proof,
    bench_merkle_verify_consistency,
    bench_query_1000,
    bench_export_jsonl_1000,
    bench_export_csv_1000,
    bench_append_batch_1000,
    bench_page_1000,
    bench_file_store_append_100,
    bench_file_store_load_100,
);

#[cfg(feature = "signing")]
criterion_group!(signing_benches, bench_sign_entry, bench_verify_signature,);

#[cfg(feature = "sqlite")]
criterion_group!(
    sqlite_benches,
    bench_sqlite_append_100,
    bench_sqlite_query_100,
);

#[cfg(all(feature = "signing", feature = "sqlite"))]
criterion_main!(benches, signing_benches, sqlite_benches);

#[cfg(all(feature = "signing", not(feature = "sqlite")))]
criterion_main!(benches, signing_benches);

#[cfg(all(not(feature = "signing"), feature = "sqlite"))]
criterion_main!(benches, sqlite_benches);

#[cfg(not(any(feature = "signing", feature = "sqlite")))]
criterion_main!(benches);
