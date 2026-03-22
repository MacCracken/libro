use criterion::{Criterion, criterion_group, criterion_main};
use libro::{AuditChain, entry::EventSeverity};

fn bench_append_1000(c: &mut Criterion) {
    c.bench_function("chain_append_1000", |b| {
        b.iter(|| {
            let mut chain = AuditChain::new();
            for i in 0..1000 {
                chain.append(
                    EventSeverity::Info,
                    "bench",
                    format!("event-{i}"),
                    serde_json::json!({}),
                );
            }
            chain
        })
    });
}

fn bench_verify_1000(c: &mut Criterion) {
    let mut chain = AuditChain::new();
    for i in 0..1000 {
        chain.append(
            EventSeverity::Info,
            "bench",
            format!("event-{i}"),
            serde_json::json!({}),
        );
    }
    c.bench_function("chain_verify_1000", |b| b.iter(|| chain.verify().unwrap()));
}

criterion_group!(benches, bench_append_1000, bench_verify_1000);
criterion_main!(benches);
