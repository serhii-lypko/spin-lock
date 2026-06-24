use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

use spin_lock::{mutexed_buffer, spin_lock_buffer};

fn benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("buffer");
    group.bench_function("mutexed_buffer", |b| {
        b.iter(|| black_box(mutexed_buffer()))
    });
    group.bench_function("spin_lock_buffer", |b| {
        b.iter(|| black_box(spin_lock_buffer()))
    });
    group.finish();
}

criterion_group!(benches, benchmarks);
criterion_main!(benches);
