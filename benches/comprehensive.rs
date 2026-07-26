use cheetah_string::{CheetahBuilder, CheetahString};
use criterion::{black_box, criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};
use std::sync::Arc;

// Benchmark: String creation from various sources
fn bench_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("creation");

    // Empty string
    group.bench_function("CheetahString::new", |b| {
        b.iter_batched(
            || (),
            |()| black_box(CheetahString::new()),
            BatchSize::SmallInput,
        )
    });

    group.bench_function("String::new", |b| {
        b.iter_batched(|| (), |()| black_box(String::new()), BatchSize::SmallInput)
    });

    group.bench_function("Arc<String>::new", |b| {
        b.iter_batched(
            || (),
            |()| black_box(Arc::new(String::new())),
            BatchSize::SmallInput,
        )
    });

    // Short string (SSO optimized)
    let short = "hello";
    group.bench_function("CheetahString::from(short)", |b| {
        b.iter_batched(
            || short,
            |value| black_box(CheetahString::from(black_box(value))),
            BatchSize::SmallInput,
        )
    });

    group.bench_function("String::from(short)", |b| {
        b.iter_batched(
            || short,
            |value| black_box(String::from(black_box(value))),
            BatchSize::SmallInput,
        )
    });

    group.bench_function("Arc<String>::from(short)", |b| {
        b.iter_batched(
            || short,
            |value| black_box(Arc::new(String::from(black_box(value)))),
            BatchSize::SmallInput,
        )
    });

    // Medium string (23 bytes - SSO boundary)
    let medium = "12345678901234567890123"; // exactly 23 bytes
    group.bench_function("CheetahString::from(23B)", |b| {
        b.iter_batched(
            || medium,
            |value| black_box(CheetahString::from(black_box(value))),
            BatchSize::SmallInput,
        )
    });

    group.bench_function("String::from(23B)", |b| {
        b.iter_batched(
            || medium,
            |value| black_box(String::from(black_box(value))),
            BatchSize::SmallInput,
        )
    });

    // Long string (>SSO capacity)
    let long = "This is a longer string that exceeds SSO capacity";
    group.bench_function("CheetahString::from(long)", |b| {
        b.iter_batched(
            || long,
            |value| black_box(CheetahString::from(black_box(value))),
            BatchSize::SmallInput,
        )
    });

    group.bench_function("String::from(long)", |b| {
        b.iter_batched(
            || long,
            |value| black_box(String::from(black_box(value))),
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

// Benchmark: Cloning strings
fn bench_clone(c: &mut Criterion) {
    let mut group = c.benchmark_group("clone");

    // Empty
    let cs_empty = CheetahString::new();
    let s_empty = String::new();
    let arc_empty = Arc::new(String::new());

    group.bench_function("CheetahString::clone(empty)", |b| {
        b.iter_batched(
            || &cs_empty,
            |value| black_box(value.clone()),
            BatchSize::SmallInput,
        )
    });

    group.bench_function("String::clone(empty)", |b| {
        b.iter_batched(
            || &s_empty,
            |value| black_box(value.clone()),
            BatchSize::SmallInput,
        )
    });

    group.bench_function("Arc<String>::clone(empty)", |b| {
        b.iter_batched(
            || &arc_empty,
            |value| black_box(value.clone()),
            BatchSize::SmallInput,
        )
    });

    // Short (SSO)
    let cs_short = CheetahString::from("hello");
    let s_short = String::from("hello");
    let arc_short = Arc::new(String::from("hello"));

    group.bench_function("CheetahString::clone(short)", |b| {
        b.iter_batched(
            || &cs_short,
            |value| black_box(value.clone()),
            BatchSize::SmallInput,
        )
    });

    group.bench_function("String::clone(short)", |b| {
        b.iter_batched(
            || &s_short,
            |value| black_box(value.clone()),
            BatchSize::SmallInput,
        )
    });

    group.bench_function("Arc<String>::clone(short)", |b| {
        b.iter_batched(
            || &arc_short,
            |value| black_box(value.clone()),
            BatchSize::SmallInput,
        )
    });

    // Long
    let long_text = "a".repeat(1000);
    let cs_long = CheetahString::from(long_text.as_str());
    let s_long = String::from(long_text.as_str());
    let arc_long = Arc::new(String::from(long_text.as_str()));

    group.bench_function("CheetahString::clone(1KB)", |b| {
        b.iter_batched(
            || &cs_long,
            |value| black_box(value.clone()),
            BatchSize::SmallInput,
        )
    });

    group.bench_function("String::clone(1KB)", |b| {
        b.iter_batched(
            || &s_long,
            |value| black_box(value.clone()),
            BatchSize::SmallInput,
        )
    });

    group.bench_function("Arc<String>::clone(1KB)", |b| {
        b.iter_batched(
            || &arc_long,
            |value| black_box(value.clone()),
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

// Benchmark: Query operations
fn bench_query(c: &mut Criterion) {
    let mut group = c.benchmark_group("query");

    let cs = CheetahString::from("hello world, this is a test string");
    let s = String::from("hello world, this is a test string");

    group.bench_function("CheetahString::starts_with", |b| {
        b.iter(|| black_box(black_box(&cs).starts_with(black_box("hello"))))
    });

    group.bench_function("String::starts_with", |b| {
        b.iter(|| black_box(black_box(&s).starts_with(black_box("hello"))))
    });

    group.bench_function("CheetahString::ends_with", |b| {
        b.iter(|| black_box(black_box(&cs).ends_with(black_box("string"))))
    });

    group.bench_function("String::ends_with", |b| {
        b.iter(|| black_box(black_box(&s).ends_with(black_box("string"))))
    });

    group.bench_function("CheetahString::contains", |b| {
        b.iter(|| black_box(black_box(&cs).contains(black_box("test"))))
    });

    group.bench_function("String::contains", |b| {
        b.iter(|| black_box(black_box(&s).contains(black_box("test"))))
    });

    group.bench_function("CheetahString::find", |b| {
        b.iter(|| black_box(black_box(&cs).find(black_box("test"))))
    });

    group.bench_function("String::find", |b| {
        b.iter(|| black_box(black_box(&s).find(black_box("test"))))
    });

    group.finish();
}

// Benchmark: Transformation operations
fn bench_transform(c: &mut Criterion) {
    let mut group = c.benchmark_group("transform");

    // to_uppercase
    let cs_lower = CheetahString::from("hello world");
    let s_lower = String::from("hello world");

    group.bench_function("CheetahString::to_uppercase", |b| {
        b.iter(|| black_box(cs_lower.to_uppercase()))
    });

    group.bench_function("String::to_uppercase", |b| {
        b.iter(|| black_box(s_lower.to_uppercase()))
    });

    // replace
    let cs_replace = CheetahString::from("hello world hello");
    let s_replace = String::from("hello world hello");

    group.bench_function("CheetahString::replace", |b| {
        b.iter(|| black_box(cs_replace.replace("hello", "hi")))
    });

    group.bench_function("String::replace", |b| {
        b.iter(|| black_box(s_replace.replace("hello", "hi")))
    });

    // substring (CheetahString specific)
    group.bench_function("CheetahString::substring(short)", |b| {
        b.iter(|| black_box(cs_replace.substring(0, 5)))
    });

    group.bench_function("CheetahString::substring(long)", |b| {
        b.iter(|| black_box(cs_replace.substring(0, 15)))
    });

    group.finish();
}

// Benchmark: Concatenation
fn bench_concat(c: &mut Criterion) {
    let mut group = c.benchmark_group("concatenation");

    // Short + short (both fit in SSO)
    let cs1 = CheetahString::from("hello");
    let cs2 = CheetahString::from(" world");
    let s1 = String::from("hello");

    group.bench_function("CheetahString + &str", |b| {
        b.iter(|| black_box(cs1.clone() + " world"))
    });

    group.bench_function("String + &str", |b| {
        b.iter(|| black_box(s1.clone() + " world"))
    });

    group.bench_function("CheetahString + &CheetahString", |b| {
        b.iter(|| black_box(cs1.clone() + &cs2))
    });

    // Long strings
    let cs_long1 = CheetahString::from("This is a longer string");
    let cs_long2 = CheetahString::from(" that will not fit in SSO");

    group.bench_function("CheetahString + &str (long)", |b| {
        b.iter(|| black_box(cs_long1.clone() + " that will not fit in SSO"))
    });

    group.bench_function("CheetahString + &CheetahString (long)", |b| {
        b.iter(|| black_box(cs_long1.clone() + &cs_long2))
    });

    group.finish();
}

// Benchmark: Iteration
fn bench_iteration(c: &mut Criterion) {
    let mut group = c.benchmark_group("iteration");

    let cs = CheetahString::from("hello world test string");
    let s = String::from("hello world test string");

    group.bench_function("CheetahString::chars", |b| {
        b.iter(|| {
            for ch in cs.chars() {
                black_box(ch);
            }
        })
    });

    group.bench_function("String::chars", |b| {
        b.iter(|| {
            for ch in s.chars() {
                black_box(ch);
            }
        })
    });

    group.bench_function("CheetahString::split", |b| {
        b.iter(|| {
            for part in cs.split_str(" ") {
                black_box(part);
            }
        })
    });

    group.bench_function("String::split", |b| {
        b.iter(|| {
            for part in s.split(" ") {
                black_box(part);
            }
        })
    });

    group.finish();
}

// Benchmark: Size scaling
fn bench_size_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("size_scaling");

    for size in [10, 22, 23, 24, 25, 50, 100, 500, 1000].iter() {
        let text = "a".repeat(*size);

        group.bench_with_input(
            BenchmarkId::new("CheetahString::from", size),
            &text,
            |b, text| {
                b.iter_batched(
                    || text.as_str(),
                    |value| black_box(CheetahString::from(black_box(value))),
                    BatchSize::SmallInput,
                )
            },
        );

        group.bench_with_input(BenchmarkId::new("String::from", size), &text, |b, text| {
            b.iter_batched(
                || text.as_str(),
                |value| black_box(String::from(black_box(value))),
                BatchSize::SmallInput,
            )
        });

        let cs = CheetahString::from(text.as_str());
        let s = String::from(text.as_str());

        group.bench_with_input(
            BenchmarkId::new("CheetahString::clone", size),
            &cs,
            |b, cs| {
                b.iter_batched(
                    || cs,
                    |value| black_box(value.clone()),
                    BatchSize::SmallInput,
                )
            },
        );

        group.bench_with_input(BenchmarkId::new("String::clone", size), &s, |b, s| {
            b.iter_batched(
                || s,
                |value| black_box(value.clone()),
                BatchSize::SmallInput,
            )
        });
    }

    group.finish();
}

// Benchmark: construction and builder hot paths that depend on internal storage choices
fn bench_internal_hot_paths(c: &mut Criterion) {
    let mut group = c.benchmark_group("internal_hot_paths");

    let segments = ["alpha", "-", "beta", "-", "gamma", "-", "delta"];
    group.bench_function("CheetahBuilder::with_capacity+finish", |b| {
        b.iter(|| {
            let mut value = CheetahBuilder::with_capacity(64);
            for segment in segments {
                value.push_str(segment);
            }
            black_box(value.finish())
        })
    });

    group.bench_function("CheetahBuilder::new+finish", |b| {
        b.iter(|| {
            let mut value = CheetahBuilder::new();
            for segment in segments {
                value.push_str(segment);
            }
            black_box(value.finish())
        })
    });

    group.bench_function("String::with_capacity+push_str", |b| {
        b.iter(|| {
            let mut value = String::with_capacity(64);
            for segment in segments {
                value.push_str(segment);
            }
            black_box(value)
        })
    });

    let long = "a".repeat(256);
    group.bench_function("CheetahString::from(&str 256B)", |b| {
        b.iter(|| black_box(CheetahString::from(long.as_str())))
    });

    group.bench_function("CheetahString::from_string(256B)", |b| {
        b.iter(|| black_box(CheetahString::from_string(long.clone())))
    });

    group.bench_function("String::from(CheetahString::from(Arc<String> 256B))", |b| {
        b.iter(|| {
            let value = CheetahString::from(Arc::new(long.clone()));
            black_box(String::from(value))
        })
    });

    let short_bytes = b"hello".to_vec();
    group.bench_function("CheetahString::try_from_vec(5B)", |b| {
        b.iter(|| black_box(CheetahString::try_from_vec(short_bytes.clone()).unwrap()))
    });

    let long_bytes = vec![b'a'; 256];
    group.bench_function("CheetahString::try_from_vec(256B)", |b| {
        b.iter(|| black_box(CheetahString::try_from_vec(long_bytes.clone()).unwrap()))
    });

    group.bench_function("String::from(CheetahString::try_from_vec(256B))", |b| {
        b.iter(|| {
            let value = CheetahString::try_from_vec(long_bytes.clone()).unwrap();
            black_box(String::from(value))
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_creation,
    bench_clone,
    bench_query,
    bench_transform,
    bench_concat,
    bench_iteration,
    bench_size_scaling,
    bench_internal_hot_paths
);
criterion_main!(benches);
