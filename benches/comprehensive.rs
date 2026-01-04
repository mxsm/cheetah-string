use cheetah_string::CheetahString;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::sync::Arc;

// Benchmark: String creation from various sources
fn bench_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("creation");

    // Empty string
    group.bench_function("CheetahString::new", |b| {
        b.iter(|| black_box(CheetahString::new()))
    });

    group.bench_function("String::new", |b| b.iter(|| black_box(String::new())));

    group.bench_function("Arc<String>::new", |b| {
        b.iter(|| black_box(Arc::new(String::new())))
    });

    // Short string (SSO optimized)
    let short = "hello";
    group.bench_function("CheetahString::from(short)", |b| {
        b.iter(|| black_box(CheetahString::from(short)))
    });

    group.bench_function("String::from(short)", |b| {
        b.iter(|| black_box(String::from(short)))
    });

    group.bench_function("Arc<String>::from(short)", |b| {
        b.iter(|| black_box(Arc::new(String::from(short))))
    });

    // Medium string (23 bytes - SSO boundary)
    let medium = "12345678901234567890123"; // exactly 23 bytes
    group.bench_function("CheetahString::from(23B)", |b| {
        b.iter(|| black_box(CheetahString::from(medium)))
    });

    group.bench_function("String::from(23B)", |b| {
        b.iter(|| black_box(String::from(medium)))
    });

    // Long string (>SSO capacity)
    let long = "This is a longer string that exceeds SSO capacity";
    group.bench_function("CheetahString::from(long)", |b| {
        b.iter(|| black_box(CheetahString::from(long)))
    });

    group.bench_function("String::from(long)", |b| {
        b.iter(|| black_box(String::from(long)))
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
        b.iter(|| black_box(cs_empty.clone()))
    });

    group.bench_function("String::clone(empty)", |b| {
        b.iter(|| black_box(s_empty.clone()))
    });

    group.bench_function("Arc<String>::clone(empty)", |b| {
        b.iter(|| black_box(arc_empty.clone()))
    });

    // Short (SSO)
    let cs_short = CheetahString::from("hello");
    let s_short = String::from("hello");
    let arc_short = Arc::new(String::from("hello"));

    group.bench_function("CheetahString::clone(short)", |b| {
        b.iter(|| black_box(cs_short.clone()))
    });

    group.bench_function("String::clone(short)", |b| {
        b.iter(|| black_box(s_short.clone()))
    });

    group.bench_function("Arc<String>::clone(short)", |b| {
        b.iter(|| black_box(arc_short.clone()))
    });

    // Long
    let long_text = "a".repeat(1000);
    let cs_long = CheetahString::from(long_text.as_str());
    let s_long = String::from(long_text.as_str());
    let arc_long = Arc::new(String::from(long_text.as_str()));

    group.bench_function("CheetahString::clone(1KB)", |b| {
        b.iter(|| black_box(cs_long.clone()))
    });

    group.bench_function("String::clone(1KB)", |b| {
        b.iter(|| black_box(s_long.clone()))
    });

    group.bench_function("Arc<String>::clone(1KB)", |b| {
        b.iter(|| black_box(arc_long.clone()))
    });

    group.finish();
}

// Benchmark: Query operations
fn bench_query(c: &mut Criterion) {
    let mut group = c.benchmark_group("query");

    let cs = CheetahString::from("hello world, this is a test string");
    let s = String::from("hello world, this is a test string");

    group.bench_function("CheetahString::starts_with", |b| {
        b.iter(|| black_box(cs.starts_with("hello")))
    });

    group.bench_function("String::starts_with", |b| {
        b.iter(|| black_box(s.starts_with("hello")))
    });

    group.bench_function("CheetahString::ends_with", |b| {
        b.iter(|| black_box(cs.ends_with("string")))
    });

    group.bench_function("String::ends_with", |b| {
        b.iter(|| black_box(s.ends_with("string")))
    });

    group.bench_function("CheetahString::contains", |b| {
        b.iter(|| black_box(cs.contains("test")))
    });

    group.bench_function("String::contains", |b| {
        b.iter(|| black_box(s.contains("test")))
    });

    group.bench_function("CheetahString::find", |b| {
        b.iter(|| black_box(cs.find("test")))
    });

    group.bench_function("String::find", |b| b.iter(|| black_box(s.find("test"))));

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
            for part in cs.split(" ") {
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

    for size in [10, 23, 50, 100, 500, 1000].iter() {
        let text = "a".repeat(*size);

        group.bench_with_input(
            BenchmarkId::new("CheetahString::from", size),
            &text,
            |b, text| b.iter(|| black_box(CheetahString::from(text.as_str()))),
        );

        group.bench_with_input(BenchmarkId::new("String::from", size), &text, |b, text| {
            b.iter(|| black_box(String::from(text.as_str())))
        });

        let cs = CheetahString::from(text.as_str());
        let s = String::from(text.as_str());

        group.bench_with_input(
            BenchmarkId::new("CheetahString::clone", size),
            &cs,
            |b, cs| b.iter(|| black_box(cs.clone())),
        );

        group.bench_with_input(BenchmarkId::new("String::clone", size), &s, |b, s| {
            b.iter(|| black_box(s.clone()))
        });
    }

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
    bench_size_scaling
);
criterion_main!(benches);
