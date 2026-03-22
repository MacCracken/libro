use criterion::{Criterion, criterion_group, criterion_main};
use libro::{AuditChain, EventSeverity};
use libro::merkle::MerkleTree;
use libro::export;
use libro::query::QueryFilter;
use libro::store::AuditStore;

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
    c.bench_function("chain_append_100", |b| {
        b.iter(|| make_chain(100))
    });
}

fn bench_append_1000(c: &mut Criterion) {
    c.bench_function("chain_append_1000", |b| {
        b.iter(|| make_chain(1000))
    });
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
    c.bench_function("merkle_proof_1000", |b| {
        b.iter(|| tree.proof(500).unwrap())
    });
}

fn bench_merkle_verify_proof(c: &mut Criterion) {
    let chain = make_chain(1000);
    let tree = MerkleTree::build(chain.entries()).unwrap();
    let proof = tree.proof(500).unwrap();
    c.bench_function("merkle_verify_proof", |b| {
        b.iter(|| libro::merkle::verify_proof(&proof))
    });
}

// --- Query ---

fn bench_query_1000(c: &mut Criterion) {
    let chain = make_chain(1000);
    let filter = QueryFilter::new().source("bench").action("event-500");
    c.bench_function("query_filter_1000", |b| {
        b.iter(|| chain.query(&filter))
    });
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
                .map(|i| (EventSeverity::Info, "bench".to_owned(), format!("event-{i}"), serde_json::json!({})))
                .collect();
            chain.append_batch(events);
            chain
        })
    });
}

fn bench_page_1000(c: &mut Criterion) {
    let chain = make_chain(1000);
    c.bench_function("chain_page_mid_100", |b| {
        b.iter(|| chain.page(450, 100))
    });
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

criterion_group!(
    benches,
    bench_append_100,
    bench_append_1000,
    bench_verify_100,
    bench_verify_1000,
    bench_merkle_build_1000,
    bench_merkle_proof,
    bench_merkle_verify_proof,
    bench_query_1000,
    bench_export_jsonl_1000,
    bench_export_csv_1000,
    bench_append_batch_1000,
    bench_page_1000,
    bench_file_store_append_100,
    bench_file_store_load_100,
);
criterion_main!(benches);
