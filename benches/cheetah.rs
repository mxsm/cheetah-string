use cheetah_string::CheetahString;
use criterion::criterion_main;
use criterion::Criterion;
use criterion::{black_box, criterion_group};

fn criterion_benchmark(c: &mut Criterion) {
    let cs1 = CheetahString::new();
    c.bench_function("Empty CheetahString", |b| b.iter(|| black_box(cs1.clone())));
    let s1 = String::new();
    c.bench_function("empty string", |b| b.iter(|| black_box(s1.clone())));

    let cs2 = CheetahString::from_static_str("Hello, World!");
    c.bench_function("Static CheetahString", |b| {
        b.iter(|| black_box(cs2.clone()))
    });
    let s2 = String::from("Hello, World!");
    c.bench_function("Static string", |b| b.iter(|| black_box(s2.clone())));

    let s_ = String::from("Hello, World!");
    let cs3 = CheetahString::from_string(s_);
    c.bench_function("String CheetahString", |b| {
        b.iter(|| black_box(cs3.clone()))
    });
    let s3 = String::from("Hello, World!");
    c.bench_function("String", |b| b.iter(|| black_box(s3.clone())));

    for size in [
        64,
        128,
        512,
        4 * 1024,
        16 * 1024,
        64 * 1024,
        512 * 1024,
        1024 * 1024,
    ] {
        let cs4 = CheetahString::from_string(String::from("h".repeat(size)));

        c.bench_function(format!("{}B CheetahString", size).as_str(), |b| {
            b.iter(|| black_box(cs4.clone()))
        });

        let s4 = String::from("a".repeat(size));
        c.bench_function(format!("{}B std string", size).as_str(), |b| {
            b.iter(|| black_box(s4.clone()))
        });
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
