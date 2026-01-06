use cheetah_string::CheetahString;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

fn bench_equality(c: &mut Criterion) {
    let mut group = c.benchmark_group("equality");

    for size in [16, 32, 64, 128, 256, 512, 1024, 4096] {
        let s1 = CheetahString::from("a".repeat(size));
        let s2 = CheetahString::from("a".repeat(size));
        let s3 = CheetahString::from(format!("{}b", "a".repeat(size - 1)));

        group.throughput(Throughput::Bytes(size as u64));

        group.bench_with_input(BenchmarkId::new("equal", size), &size, |b, _| {
            b.iter(|| black_box(&s1) == black_box(&s2))
        });

        group.bench_with_input(BenchmarkId::new("not_equal", size), &size, |b, _| {
            b.iter(|| black_box(&s1) == black_box(&s3))
        });
    }

    group.finish();
}

fn bench_starts_with(c: &mut Criterion) {
    let mut group = c.benchmark_group("starts_with");

    for size in [16, 32, 64, 128, 256, 512, 1024, 4096] {
        let haystack = CheetahString::from("a".repeat(size));
        let needle_match = "a".repeat(size / 2);
        let needle_no_match = "b".repeat(size / 2);

        group.throughput(Throughput::Bytes(size as u64));

        group.bench_with_input(BenchmarkId::new("match", size), &size, |b, _| {
            b.iter(|| black_box(&haystack).starts_with(black_box(&needle_match)))
        });

        group.bench_with_input(BenchmarkId::new("no_match", size), &size, |b, _| {
            b.iter(|| black_box(&haystack).starts_with(black_box(&needle_no_match)))
        });
    }

    group.finish();
}

fn bench_ends_with(c: &mut Criterion) {
    let mut group = c.benchmark_group("ends_with");

    for size in [16, 32, 64, 128, 256, 512, 1024, 4096] {
        let haystack = CheetahString::from("a".repeat(size));
        let needle_match = "a".repeat(size / 2);
        let needle_no_match = "b".repeat(size / 2);

        group.throughput(Throughput::Bytes(size as u64));

        group.bench_with_input(BenchmarkId::new("match", size), &size, |b, _| {
            b.iter(|| black_box(&haystack).ends_with(black_box(&needle_match)))
        });

        group.bench_with_input(BenchmarkId::new("no_match", size), &size, |b, _| {
            b.iter(|| black_box(&haystack).ends_with(black_box(&needle_no_match)))
        });
    }

    group.finish();
}

fn bench_contains(c: &mut Criterion) {
    let mut group = c.benchmark_group("contains");

    for size in [16, 32, 64, 128, 256, 512, 1024, 4096] {
        let haystack =
            CheetahString::from(format!("{}x{}", "a".repeat(size / 2), "a".repeat(size / 2)));
        let needle_match = "x";
        let needle_no_match = "z";

        group.throughput(Throughput::Bytes(size as u64));

        group.bench_with_input(BenchmarkId::new("match", size), &size, |b, _| {
            b.iter(|| black_box(&haystack).contains(black_box(needle_match)))
        });

        group.bench_with_input(BenchmarkId::new("no_match", size), &size, |b, _| {
            b.iter(|| black_box(&haystack).contains(black_box(needle_no_match)))
        });
    }

    group.finish();
}

fn bench_find(c: &mut Criterion) {
    let mut group = c.benchmark_group("find");

    for size in [16, 32, 64, 128, 256, 512, 1024, 4096] {
        let haystack =
            CheetahString::from(format!("{}x{}", "a".repeat(size / 2), "a".repeat(size / 2)));
        let needle_match = "x";
        let needle_no_match = "z";

        group.throughput(Throughput::Bytes(size as u64));

        group.bench_with_input(BenchmarkId::new("match", size), &size, |b, _| {
            b.iter(|| black_box(&haystack).find(black_box(needle_match)))
        });

        group.bench_with_input(BenchmarkId::new("no_match", size), &size, |b, _| {
            b.iter(|| black_box(&haystack).find(black_box(needle_no_match)))
        });
    }

    group.finish();
}

fn bench_realistic_workload(c: &mut Criterion) {
    let mut group = c.benchmark_group("realistic");

    // Simulate URL parsing
    let url = CheetahString::from("https://api.example.com/v1/users/12345?filter=active&sort=name");

    group.bench_function("url_parsing", |b| {
        b.iter(|| {
            black_box(&url).starts_with("https://")
                && black_box(&url).contains("api")
                && black_box(&url).contains("users")
        })
    });

    // Simulate log filtering
    let log =
        CheetahString::from("[2024-01-01 12:00:00] INFO: Processing request for user_id=12345");

    group.bench_function("log_filtering", |b| {
        b.iter(|| {
            black_box(&log).starts_with("[2024")
                && black_box(&log).contains("INFO")
                && black_box(&log).contains("user_id")
        })
    });

    // Simulate content type checking
    let content_type = CheetahString::from("application/json; charset=utf-8");

    group.bench_function("content_type_check", |b| {
        b.iter(|| {
            black_box(&content_type).starts_with("application/")
                && black_box(&content_type).contains("json")
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_equality,
    bench_starts_with,
    bench_ends_with,
    bench_contains,
    bench_find,
    bench_realistic_workload
);
criterion_main!(benches);
