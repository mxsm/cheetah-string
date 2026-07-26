use cheetah_string::{CheetahBuilder, CheetahString};
use criterion::{black_box, criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};

fn bench_builder_push_str(c: &mut Criterion) {
    let mut group = c.benchmark_group("builder_push_str");

    for capacity in [0, 16, 128, 1024] {
        group.bench_with_input(
            BenchmarkId::new("append", capacity),
            &capacity,
            |b, capacity| {
                b.iter(|| {
                    let mut builder = CheetahBuilder::with_capacity(*capacity);
                    builder.push_str("hello");
                    builder.push_str(black_box(" world"));
                    black_box(builder)
                })
            },
        );
    }

    group.finish();
}

fn bench_immutable_add(c: &mut Criterion) {
    let mut group = c.benchmark_group("immutable_add");

    for rhs_len in [1, 8, 32, 128] {
        let rhs = "x".repeat(rhs_len);
        group.bench_with_input(BenchmarkId::from_parameter(rhs_len), &rhs, |b, rhs| {
            b.iter(|| black_box(CheetahString::from("hello") + black_box(rhs.as_str())))
        });
    }

    group.finish();
}

fn bench_builder_reserve(c: &mut Criterion) {
    let mut group = c.benchmark_group("builder_reserve");

    for additional in [0, 8, 128] {
        group.bench_with_input(
            BenchmarkId::from_parameter(additional),
            &additional,
            |b, extra| {
                b.iter(|| {
                    let mut builder = CheetahBuilder::with_capacity(64);
                    builder.push_str("hello");
                    builder.reserve(black_box(*extra));
                    black_box(builder)
                })
            },
        );
    }

    group.finish();
}

fn bench_builder_freeze(c: &mut Criterion) {
    let input = "builder-value-".repeat(64);
    let mut group = c.benchmark_group("builder_freeze");

    group.bench_function("finish_canonical", |b| {
        b.iter_batched(
            || {
                let mut builder = CheetahBuilder::with_capacity(input.len());
                builder.push_str(black_box(&input));
                builder
            },
            |builder| black_box(builder.finish()),
            BatchSize::SmallInput,
        )
    });

    group.bench_function("into_string_mutable", |b| {
        b.iter_batched(
            || {
                let mut builder = CheetahBuilder::with_capacity(input.len() * 2);
                builder.push_str(black_box(&input));
                builder
            },
            |builder| black_box(builder.into_string()),
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_builder_push_str,
    bench_immutable_add,
    bench_builder_reserve,
    bench_builder_freeze
);
criterion_main!(benches);
