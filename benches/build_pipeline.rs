use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_sha256_hashing(c: &mut Criterion) {
    c.bench_function("sha256_hash_1mb", |b| b.iter(|| {
        use sha2::{Sha256, Digest};
        let data = vec![0xABu8; 1024 * 1024];
        let mut hasher = Sha256::new();
        hasher.update(&data);
        let result = hasher.finalize();
        black_box(result);
    }));

    c.bench_function("tar_creation_small", |b| b.iter(|| {
        let mut tar_builder = tar::Builder::new(Vec::new());
        for i in 0..100 {
            let mut header = tar::Header::new_gnu();
            let content = format!("file_{}.txt", i);
            header.set_size(content.len() as u64);
            header.set_mode(0o644);
            tar_builder.append_data(&mut header, content.clone(), content.as_bytes()).unwrap();
        }
        let archive = tar_builder.into_inner().unwrap();
        black_box(archive.len());
    }));
}

criterion_group!(benches, bench_sha256_hashing);
criterion_main!(benches);
