use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_memory_allocation(c: &mut Criterion) {
    c.bench_function("alloc_10mb", |b| b.iter(|| {
        let data = vec![0u8; 10 * 1024 * 1024];
        black_box(data.len());
    }));

    c.bench_function("vec_reserve_100k", |b| b.iter(|| {
        let mut v = Vec::with_capacity(100_000);
        for i in 0..100_000 {
            v.push(i);
        }
        black_box(v.len());
    }));
}

criterion_group!(benches, bench_memory_allocation);
criterion_main!(benches);
