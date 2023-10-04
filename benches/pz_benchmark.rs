#![cfg(unix)]
use criterion::{criterion_group, criterion_main, Criterion};

pub fn criterion_benchmark(c: &mut Criterion) {
    use std::path::PathBuf;

    use partialzip::partzip::PartialZip;

    c.bench_function("local file benchmark", |b| {
        b.iter(|| {
            let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            d.push("testdata/test.zip");
            let mut pz = PartialZip::new(&format!("file://localhost{}", d.display()))
                .expect("cannot create PartialZip in benchmark");
            let _list = pz.list_detailed();
            let _download = pz.download("1.txt");
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
