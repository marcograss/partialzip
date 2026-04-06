#![cfg(unix)]
use criterion::{criterion_group, criterion_main, Criterion};

use std::io::Write;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

fn setup_large_zip() -> tempfile::NamedTempFile {
    let file = tempfile::NamedTempFile::new().unwrap();
    let mut zip = ZipWriter::new(file.reopen().unwrap());
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    zip.start_file("large.bin", options).unwrap();

    // Generate 10 MB of high-entropy data to ensure the compressed file is large
    let mut data = vec![0u8; 1024 * 1024]; // 1 MB buffer
    let mut seed = 0x12345678u32;
    for _ in 0..10 {
        for byte in &mut data {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            *byte = (seed >> 24) as u8;
        }
        zip.write_all(&data).unwrap();
    }
    zip.finish().unwrap();

    file
}

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

    c.bench_function("local file benchmark large download", |b| {
        let large_zip_path = setup_large_zip();
        b.iter(|| {
            let pz = PartialZip::new(&format!("file://localhost{}", large_zip_path.path().display()))
                .expect("cannot create PartialZip in benchmark download");
            let _download = pz.download("large.bin");
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
