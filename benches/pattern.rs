use cheetah_string::{CheetahFinder, CheetahString};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

fn bench_pathological_find(c: &mut Criterion) {
    let mut group = c.benchmark_group("pathological_find");

    for size in [128, 1024, 16 * 1024, 64 * 1024] {
        let haystack = format!("{}b", "a".repeat(size));
        let cheetah = CheetahString::from(haystack.as_str());
        let needle = "aaaab";
        let finder = CheetahFinder::new(needle);

        group.throughput(Throughput::Bytes(haystack.len() as u64));

        group.bench_with_input(BenchmarkId::new("cheetah_find", size), &size, |b, _| {
            b.iter(|| black_box(&cheetah).find(black_box(needle)))
        });

        group.bench_with_input(BenchmarkId::new("finder", size), &size, |b, _| {
            b.iter(|| finder.find_in(black_box(&cheetah)))
        });

        group.bench_with_input(BenchmarkId::new("std_find", size), &size, |b, _| {
            b.iter(|| black_box(haystack.as_str()).find(black_box(needle)))
        });
    }

    group.finish();
}

fn bench_single_byte_find(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_byte_find");

    for size in [128, 1024, 16 * 1024, 64 * 1024] {
        let haystack = format!("{}z", "a".repeat(size));
        let cheetah = CheetahString::from(haystack.as_str());

        group.throughput(Throughput::Bytes(haystack.len() as u64));

        group.bench_with_input(BenchmarkId::new("cheetah_find", size), &size, |b, _| {
            b.iter(|| black_box(&cheetah).find(black_box("z")))
        });

        group.bench_with_input(BenchmarkId::new("std_find", size), &size, |b, _| {
            b.iter(|| black_box(haystack.as_str()).find(black_box("z")))
        });
    }

    group.finish();
}

criterion_group!(benches, bench_pathological_find, bench_single_byte_find);
criterion_main!(benches);
