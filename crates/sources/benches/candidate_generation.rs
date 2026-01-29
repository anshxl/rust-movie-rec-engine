//! Benchmarks for candidate generation
//!
//! Run with: cargo bench --package sources
//!
//! This will benchmark Thunder and Phoenix sources on the full MovieLens dataset.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use data_loader::DataIndex;
use sources::{user_context::build_user_context, PhoenixSource, ThunderSource};
use std::path::Path;
use std::sync::Arc;

fn load_test_data() -> Arc<DataIndex> {
    let data_dir = Path::new("../../data/ml-1m");
    let index = DataIndex::load_from_files(data_dir).expect("Failed to load test data");
    Arc::new(index)
}

fn bench_thunder_candidates(c: &mut Criterion) {
    let data_index = load_test_data();
    let thunder = ThunderSource::new(data_index.clone());

    // Use user 1 as test user
    let context = build_user_context(&data_index, 1).expect("Failed to build user context");

    c.bench_function("thunder_get_candidates", |b| {
        b.iter(|| {
            let candidates = thunder.get_candidates(black_box(&context), black_box(300));
            black_box(candidates)
        })
    });
}

fn bench_phoenix_candidates(c: &mut Criterion) {
    let data_index = load_test_data();
    let phoenix = PhoenixSource::new(data_index.clone());

    // Use user 1 as test user
    let context = build_user_context(&data_index, 1).expect("Failed to build user context");

    c.bench_function("phoenix_get_candidates", |b| {
        b.iter(|| {
            let candidates = phoenix.get_candidates(black_box(&context), black_box(200));
            black_box(candidates)
        })
    });
}

fn bench_build_user_context(c: &mut Criterion) {
    let data_index = load_test_data();

    c.bench_function("build_user_context", |b| {
        b.iter(|| {
            let context = build_user_context(&data_index, black_box(1)).unwrap();
            black_box(context)
        })
    });
}

criterion_group!(
    benches,
    bench_thunder_candidates,
    bench_phoenix_candidates,
    bench_build_user_context
);
criterion_main!(benches);
