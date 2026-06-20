use cheetah_string::CheetahString;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

fn bench_push_str(c: &mut Criterion) {
    let mut group = c.benchmark_group("push_str");

    group.bench_function("inline_in_place", |b| {
        b.iter(|| {
            let mut s = CheetahString::from("hello");
            s.push_str(black_box(" world"));
            black_box(s)
        })
    });

    group.bench_function("owned_spare_capacity", |b| {
        b.iter(|| {
            let mut s = CheetahString::with_capacity(128);
            s.push_str("hello");
            s.push_str(black_box(" world"));
            black_box(s)
        })
    });

    group.bench_function("static_fallback", |b| {
        b.iter(|| {
            let mut s = CheetahString::from_static_str("hello");
            s.push_str(black_box(" world"));
            black_box(s)
        })
    });

    group.finish();
}

fn bench_add(c: &mut Criterion) {
    let mut group = c.benchmark_group("add");

    for rhs_len in [1, 8, 32, 128] {
        let rhs = "x".repeat(rhs_len);

        group.bench_with_input(
            BenchmarkId::new("owned_capacity_str", rhs_len),
            &rhs,
            |b, rhs| {
                b.iter(|| {
                    let mut s = CheetahString::with_capacity(256);
                    s.push_str("hello");
                    black_box(s + black_box(rhs.as_str()))
                })
            },
        );

        group.bench_with_input(BenchmarkId::new("inline_str", rhs_len), &rhs, |b, rhs| {
            b.iter(|| black_box(CheetahString::from("h") + black_box(rhs.as_str())))
        });
    }

    group.finish();
}

fn bench_reserve(c: &mut Criterion) {
    let mut group = c.benchmark_group("reserve");

    for additional in [0, 8, 128] {
        group.bench_with_input(
            BenchmarkId::from_parameter(additional),
            &additional,
            |b, extra| {
                b.iter(|| {
                    let mut s = CheetahString::with_capacity(64);
                    s.push_str("hello");
                    s.reserve(black_box(*extra));
                    black_box(s)
                })
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_push_str, bench_add, bench_reserve);
criterion_main!(benches);
