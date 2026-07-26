use cheetah_string::CheetahString;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

#[cfg(feature = "experimental-simd")]
const ACTIVE_BACKEND: &str = "experimental-simd";
#[cfg(not(feature = "experimental-simd"))]
const ACTIVE_BACKEND: &str = "portable-default";

const SHORT_NEEDLE_CASES: [(&str, &str, &str); 3] = [
    ("two_bytes", "xy", "zz"),
    ("four_bytes", "wxyz", "zzzz"),
    ("eight_bytes", "qrstuvwx", "zzzzzzzz"),
];
const SHORT_NEEDLE_SIZES: [usize; 4] = [16, 64, 256, 1024];
const SHORT_NEEDLE_COMPARE_SIZES: [usize; 2] = [64, 1024];
const FIXED_DATA_SEED: u64 = 0xC4EE_7A51_95AA_0001;
const FULL_SIZE_MATRIX: [usize; 12] = [16, 22, 23, 24, 25, 32, 64, 128, 256, 512, 1024, 4096];

fn deterministic_ascii(len: usize, seed: u64) -> String {
    let mut state = seed;
    let bytes = (0..len)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            b'a' + (state % 26) as u8
        })
        .collect::<Vec<_>>();
    String::from_utf8(bytes).expect("fixed generator emits ASCII")
}

fn bench_equality(c: &mut Criterion) {
    let mut group = c.benchmark_group(format!("{ACTIVE_BACKEND}/equality"));

    for size in FULL_SIZE_MATRIX {
        let source = deterministic_ascii(size, FIXED_DATA_SEED ^ size as u64);
        let s1 = CheetahString::from(source.as_str());
        let s2 = CheetahString::from(source.as_str());

        group.throughput(Throughput::Bytes(size as u64));

        group.bench_with_input(BenchmarkId::new("equal", size), &size, |b, _| {
            b.iter(|| black_box(black_box(&s1) == black_box(&s2)))
        });

        for (position, index) in [
            ("mismatch_start", 0),
            ("mismatch_middle", size / 2),
            ("mismatch_end", size - 1),
        ] {
            let mut mismatch = source.clone().into_bytes();
            mismatch[index] = if mismatch[index] == b'~' { b'!' } else { b'~' };
            let mismatch = CheetahString::from(
                String::from_utf8(mismatch).expect("replacement remains ASCII"),
            );
            group.bench_with_input(BenchmarkId::new(position, size), &size, |b, _| {
                b.iter(|| black_box(black_box(&s1) == black_box(&mismatch)))
            });
        }
    }

    group.finish();
}

fn bench_starts_with(c: &mut Criterion) {
    let mut group = c.benchmark_group(format!("{ACTIVE_BACKEND}/starts_with"));

    for size in FULL_SIZE_MATRIX {
        let source = deterministic_ascii(size, FIXED_DATA_SEED ^ size as u64);
        let haystack = CheetahString::from(source.as_str());
        let needle_match = source[..size / 2].to_owned();
        let needle_no_match = "~".repeat(size / 2);

        group.throughput(Throughput::Bytes(size as u64));

        group.bench_with_input(BenchmarkId::new("match", size), &size, |b, _| {
            b.iter(|| black_box(black_box(&haystack).starts_with(black_box(needle_match.as_str()))))
        });

        group.bench_with_input(BenchmarkId::new("no_match", size), &size, |b, _| {
            b.iter(|| {
                black_box(black_box(&haystack).starts_with(black_box(needle_no_match.as_str())))
            })
        });
    }

    group.finish();
}

fn bench_ends_with(c: &mut Criterion) {
    let mut group = c.benchmark_group(format!("{ACTIVE_BACKEND}/ends_with"));

    for size in FULL_SIZE_MATRIX {
        let source = deterministic_ascii(size, FIXED_DATA_SEED ^ size as u64);
        let haystack = CheetahString::from(source.as_str());
        let needle_match = source[size / 2..].to_owned();
        let needle_no_match = "~".repeat(size / 2);

        group.throughput(Throughput::Bytes(size as u64));

        group.bench_with_input(BenchmarkId::new("match", size), &size, |b, _| {
            b.iter(|| black_box(black_box(&haystack).ends_with(black_box(needle_match.as_str()))))
        });

        group.bench_with_input(BenchmarkId::new("no_match", size), &size, |b, _| {
            b.iter(|| {
                black_box(black_box(&haystack).ends_with(black_box(needle_no_match.as_str())))
            })
        });
    }

    group.finish();
}

fn bench_contains(c: &mut Criterion) {
    let mut group = c.benchmark_group(format!("{ACTIVE_BACKEND}/contains"));

    for size in FULL_SIZE_MATRIX {
        let base = deterministic_ascii(size, FIXED_DATA_SEED ^ size as u64);
        let needle_match = "01234567";
        let needle_no_match = "76543210";

        group.throughput(Throughput::Bytes(size as u64));

        for (position, index) in [
            ("match_start", 0),
            ("match_middle", (size - needle_match.len()) / 2),
            ("match_end", size - needle_match.len()),
        ] {
            let mut source = base.clone();
            source.replace_range(index..index + needle_match.len(), needle_match);
            let haystack = CheetahString::from(source);
            group.bench_with_input(BenchmarkId::new(position, size), &size, |b, _| {
                b.iter(|| black_box(black_box(&haystack).contains(black_box(needle_match))))
            });
        }

        let haystack = CheetahString::from(base);
        group.bench_with_input(BenchmarkId::new("no_match", size), &size, |b, _| {
            b.iter(|| black_box(black_box(&haystack).contains(black_box(needle_no_match))))
        });
    }

    group.finish();
}

fn bench_find(c: &mut Criterion) {
    let mut group = c.benchmark_group(format!("{ACTIVE_BACKEND}/find"));

    for size in FULL_SIZE_MATRIX {
        let base = deterministic_ascii(size, FIXED_DATA_SEED ^ size as u64);
        let needle_match = "01234567";
        let needle_no_match = "76543210";

        group.throughput(Throughput::Bytes(size as u64));

        for (position, index) in [
            ("match_start", 0),
            ("match_middle", (size - needle_match.len()) / 2),
            ("match_end", size - needle_match.len()),
        ] {
            let mut source = base.clone();
            source.replace_range(index..index + needle_match.len(), needle_match);
            let haystack = CheetahString::from(source);
            group.bench_with_input(BenchmarkId::new(position, size), &size, |b, _| {
                b.iter(|| black_box(black_box(&haystack).find(black_box(needle_match))))
            });
        }

        let haystack = CheetahString::from(base);
        group.bench_with_input(BenchmarkId::new("no_match", size), &size, |b, _| {
            b.iter(|| black_box(black_box(&haystack).find(black_box(needle_no_match))))
        });
    }

    group.finish();
}

fn bench_realistic_workload(c: &mut Criterion) {
    let mut group = c.benchmark_group(format!("{ACTIVE_BACKEND}/realistic"));

    // Simulate URL parsing
    let url = CheetahString::from("https://api.example.com/v1/users/12345?filter=active&sort=name");

    group.bench_function("url_parsing", |b| {
        b.iter(|| {
            black_box(
                black_box(&url).starts_with(black_box("https://"))
                    && black_box(&url).contains(black_box("api"))
                    && black_box(&url).contains(black_box("users")),
            )
        })
    });

    // Simulate log filtering
    let log =
        CheetahString::from("[2024-01-01 12:00:00] INFO: Processing request for user_id=12345");

    group.bench_function("log_filtering", |b| {
        b.iter(|| {
            black_box(
                black_box(&log).starts_with(black_box("[2024"))
                    && black_box(&log).contains(black_box("INFO"))
                    && black_box(&log).contains(black_box("user_id")),
            )
        })
    });

    // Simulate content type checking
    let content_type = CheetahString::from("application/json; charset=utf-8");

    group.bench_function("content_type_check", |b| {
        b.iter(|| {
            black_box(
                black_box(&content_type).starts_with(black_box("application/"))
                    && black_box(&content_type).contains(black_box("json")),
            )
        })
    });

    group.finish();
}

fn bench_contains_short_needles(c: &mut Criterion) {
    let mut group = c.benchmark_group(format!("{ACTIVE_BACKEND}/contains_short_needle"));

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
    let mut group = c.benchmark_group(format!("{ACTIVE_BACKEND}/find_short_needle"));

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
    let mut group = c.benchmark_group(format!("{ACTIVE_BACKEND}/compare_short_needle_contains"));

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
    let mut group = c.benchmark_group(format!("{ACTIVE_BACKEND}/compare_short_needle_find"));

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
