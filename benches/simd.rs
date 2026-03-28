use cheetah_string::CheetahString;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::time::Duration;

const SHORT_NEEDLE_CASES: [(&str, &str, &str); 3] = [
    ("two_bytes", "xy", "zz"),
    ("four_bytes", "wxyz", "zzzz"),
    ("eight_bytes", "qrstuvwx", "zzzzzzzz"),
];
const SHORT_NEEDLE_SIZES: [usize; 4] = [16, 64, 256, 1024];
const SHORT_NEEDLE_COMPARE_SIZES: [usize; 2] = [64, 1024];

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

fn bench_contains_short_needles(c: &mut Criterion) {
    let mut group = c.benchmark_group("contains_short_needle");
    group.sample_size(20);
    group.warm_up_time(Duration::from_millis(500));
    group.measurement_time(Duration::from_secs(2));

    for (label, needle_match, needle_no_match) in SHORT_NEEDLE_CASES {
        for size in SHORT_NEEDLE_SIZES {
            let prefix_len = size / 2;
            let suffix_len = size / 2;
            let haystack = CheetahString::from(format!(
                "{}{}{}",
                "a".repeat(prefix_len),
                needle_match,
                "a".repeat(suffix_len)
            ));

            group.throughput(Throughput::Bytes(
                (prefix_len + suffix_len + needle_match.len()) as u64,
            ));

            group.bench_with_input(
                BenchmarkId::new(format!("{label}_match"), size),
                &size,
                |b, _| b.iter(|| black_box(&haystack).contains(black_box(needle_match))),
            );

            group.bench_with_input(
                BenchmarkId::new(format!("{label}_no_match"), size),
                &size,
                |b, _| b.iter(|| black_box(&haystack).contains(black_box(needle_no_match))),
            );
        }
    }

    group.finish();
}

fn bench_find_short_needles(c: &mut Criterion) {
    let mut group = c.benchmark_group("find_short_needle");
    group.sample_size(20);
    group.warm_up_time(Duration::from_millis(500));
    group.measurement_time(Duration::from_secs(2));

    for (label, needle_match, needle_no_match) in SHORT_NEEDLE_CASES {
        for size in SHORT_NEEDLE_SIZES {
            let prefix_len = size / 2;
            let suffix_len = size / 2;
            let haystack = CheetahString::from(format!(
                "{}{}{}",
                "a".repeat(prefix_len),
                needle_match,
                "a".repeat(suffix_len)
            ));

            group.throughput(Throughput::Bytes(
                (prefix_len + suffix_len + needle_match.len()) as u64,
            ));

            group.bench_with_input(
                BenchmarkId::new(format!("{label}_match"), size),
                &size,
                |b, _| b.iter(|| black_box(&haystack).find(black_box(needle_match))),
            );

            group.bench_with_input(
                BenchmarkId::new(format!("{label}_no_match"), size),
                &size,
                |b, _| b.iter(|| black_box(&haystack).find(black_box(needle_no_match))),
            );
        }
    }

    group.finish();
}

fn bench_compare_short_needle_contains(c: &mut Criterion) {
    let mut group = c.benchmark_group("compare_short_needle_contains");
    group.sample_size(20);
    group.warm_up_time(Duration::from_millis(500));
    group.measurement_time(Duration::from_secs(2));

    for (label, needle_match, needle_no_match) in SHORT_NEEDLE_CASES {
        for size in SHORT_NEEDLE_COMPARE_SIZES {
            let prefix_len = size / 2;
            let suffix_len = size / 2;
            let haystack = format!(
                "{}{}{}",
                "a".repeat(prefix_len),
                needle_match,
                "a".repeat(suffix_len)
            );
            let cheetah_haystack = CheetahString::from(haystack.as_str());
            let string_haystack = haystack;

            group.throughput(Throughput::Bytes(string_haystack.len() as u64));

            group.bench_with_input(
                BenchmarkId::new(format!("cheetah_{label}_match"), size),
                &size,
                |b, _| b.iter(|| black_box(&cheetah_haystack).contains(black_box(needle_match))),
            );

            group.bench_with_input(
                BenchmarkId::new(format!("string_{label}_match"), size),
                &size,
                |b, _| b.iter(|| black_box(&string_haystack).contains(black_box(needle_match))),
            );

            group.bench_with_input(
                BenchmarkId::new(format!("cheetah_{label}_no_match"), size),
                &size,
                |b, _| b.iter(|| black_box(&cheetah_haystack).contains(black_box(needle_no_match))),
            );

            group.bench_with_input(
                BenchmarkId::new(format!("string_{label}_no_match"), size),
                &size,
                |b, _| b.iter(|| black_box(&string_haystack).contains(black_box(needle_no_match))),
            );
        }
    }

    group.finish();
}

fn bench_compare_short_needle_find(c: &mut Criterion) {
    let mut group = c.benchmark_group("compare_short_needle_find");
    group.sample_size(20);
    group.warm_up_time(Duration::from_millis(500));
    group.measurement_time(Duration::from_secs(2));

    for (label, needle_match, needle_no_match) in SHORT_NEEDLE_CASES {
        for size in SHORT_NEEDLE_COMPARE_SIZES {
            let prefix_len = size / 2;
            let suffix_len = size / 2;
            let haystack = format!(
                "{}{}{}",
                "a".repeat(prefix_len),
                needle_match,
                "a".repeat(suffix_len)
            );
            let cheetah_haystack = CheetahString::from(haystack.as_str());
            let string_haystack = haystack;

            group.throughput(Throughput::Bytes(string_haystack.len() as u64));

            group.bench_with_input(
                BenchmarkId::new(format!("cheetah_{label}_match"), size),
                &size,
                |b, _| b.iter(|| black_box(&cheetah_haystack).find(black_box(needle_match))),
            );

            group.bench_with_input(
                BenchmarkId::new(format!("string_{label}_match"), size),
                &size,
                |b, _| b.iter(|| black_box(&string_haystack).find(black_box(needle_match))),
            );

            group.bench_with_input(
                BenchmarkId::new(format!("cheetah_{label}_no_match"), size),
                &size,
                |b, _| b.iter(|| black_box(&cheetah_haystack).find(black_box(needle_no_match))),
            );

            group.bench_with_input(
                BenchmarkId::new(format!("string_{label}_no_match"), size),
                &size,
                |b, _| b.iter(|| black_box(&string_haystack).find(black_box(needle_no_match))),
            );
        }
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_equality,
    bench_starts_with,
    bench_ends_with,
    bench_contains,
    bench_find,
    bench_contains_short_needles,
    bench_find_short_needles,
    bench_compare_short_needle_contains,
    bench_compare_short_needle_find,
    bench_realistic_workload
);
criterion_main!(benches);
