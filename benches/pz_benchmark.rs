#![cfg(unix)]
use criterion::{criterion_group, criterion_main, Criterion};

/// # Panics
/// Can panic while creating the `PartialZip` archive
pub fn criterion_benchmark(c: &mut Criterion) {
    use std::path::PathBuf;

    use partialzip::partzip::PartialZip;

    c.bench_function("local file benchmark detailed list", |b| {
        b.iter(|| {
            let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            d.push("testdata/test.zip");
            let pz = PartialZip::new(&format!("file://localhost{}", d.display()))
                .expect("cannot create PartialZip in benchmark detailed");
            let _list = pz.list_detailed();
        });
    });

    c.bench_function("local file benchmark list names", |b| {
        b.iter(|| {
            let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            d.push("testdata/test.zip");
            let pz = PartialZip::new(&format!("file://localhost{}", d.display()))
                .expect("cannot create PartialZip in benchmark");
            let _list = pz.list_names();
        });
    });

    c.bench_function("local file download", |b| {
        b.iter(|| {
            let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            d.push("testdata/test.zip");
            let pz = PartialZip::new(&format!("file://localhost{}", d.display()))
                .expect("cannot create PartialZip in benchmark download");
            let _download = pz.download("1.txt");
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
