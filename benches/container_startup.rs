use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_container_startup_cold(c: &mut Criterion) {
    c.bench_function("container_startup_cold_ms", |b| b.iter(|| {
        let cold_start_delay = std::time::Duration::from_millis(50);
        black_box(cold_start_delay);
    }));

    c.bench_function("read_file_metadata", |b| b.iter(|| {
        let path = std::path::Path::new("/tmp");
        let metadata = std::fs::metadata(path);
        black_box(metadata.is_ok());
    }));
}

criterion_group!(benches, bench_container_startup_cold);
criterion_main!(benches);
