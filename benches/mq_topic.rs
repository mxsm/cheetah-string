use cheetah_string::CheetahString;
use compact_str::CompactString;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use smartstring::alias::String as SmartString;
use std::collections::HashMap;
use std::sync::Arc;

const TOPIC_COUNT: usize = 10_000;

fn topics() -> Vec<String> {
    (0..TOPIC_COUNT)
        .map(|i| format!("RMQ_SYS_TRACE_TOPIC_{:05}", i))
        .collect()
}

fn bench_topic_insert(c: &mut Criterion) {
    let topics = topics();
    let mut group = c.benchmark_group("mq_topic_insert");
    group.throughput(Throughput::Elements(TOPIC_COUNT as u64));

    group.bench_function("String", |b| {
        b.iter(|| {
            let mut map = HashMap::with_capacity(TOPIC_COUNT);
            for (idx, topic) in topics.iter().enumerate() {
                map.insert(black_box(topic.clone()), idx);
            }
            black_box(map)
        })
    });

    group.bench_function("Arc<str>", |b| {
        b.iter(|| {
            let mut map = HashMap::with_capacity(TOPIC_COUNT);
            for (idx, topic) in topics.iter().enumerate() {
                map.insert(black_box(Arc::<str>::from(topic.as_str())), idx);
            }
            black_box(map)
        })
    });

    group.bench_function("CompactString", |b| {
        b.iter(|| {
            let mut map = HashMap::with_capacity(TOPIC_COUNT);
            for (idx, topic) in topics.iter().enumerate() {
                map.insert(black_box(CompactString::from(topic.as_str())), idx);
            }
            black_box(map)
        })
    });

    group.bench_function("SmartString", |b| {
        b.iter(|| {
            let mut map = HashMap::with_capacity(TOPIC_COUNT);
            for (idx, topic) in topics.iter().enumerate() {
                map.insert(black_box(SmartString::from(topic.as_str())), idx);
            }
            black_box(map)
        })
    });

    group.bench_function("CheetahString", |b| {
        b.iter(|| {
            let mut map = HashMap::with_capacity(TOPIC_COUNT);
            for (idx, topic) in topics.iter().enumerate() {
                map.insert(black_box(CheetahString::from(topic.as_str())), idx);
            }
            black_box(map)
        })
    });

    group.finish();
}

fn bench_topic_lookup(c: &mut Criterion) {
    let topics = topics();
    let string_map: HashMap<String, usize> = topics
        .iter()
        .enumerate()
        .map(|(idx, topic)| (topic.clone(), idx))
        .collect();
    let arc_map: HashMap<Arc<str>, usize> = topics
        .iter()
        .enumerate()
        .map(|(idx, topic)| (Arc::<str>::from(topic.as_str()), idx))
        .collect();
    let compact_map: HashMap<CompactString, usize> = topics
        .iter()
        .enumerate()
        .map(|(idx, topic)| (CompactString::from(topic.as_str()), idx))
        .collect();
    let smart_map: HashMap<SmartString, usize> = topics
        .iter()
        .enumerate()
        .map(|(idx, topic)| (SmartString::from(topic.as_str()), idx))
        .collect();
    let cheetah_map: HashMap<CheetahString, usize> = topics
        .iter()
        .enumerate()
        .map(|(idx, topic)| (CheetahString::from(topic.as_str()), idx))
        .collect();

    let needles = [
        "RMQ_SYS_TRACE_TOPIC_00000",
        "RMQ_SYS_TRACE_TOPIC_01024",
        "RMQ_SYS_TRACE_TOPIC_09999",
    ];

    let mut group = c.benchmark_group("mq_topic_lookup");
    group.throughput(Throughput::Elements(needles.len() as u64));

    for needle in needles {
        group.bench_with_input(BenchmarkId::new("String", needle), needle, |b, needle| {
            b.iter(|| black_box(string_map.get(black_box(needle))))
        });
        group.bench_with_input(BenchmarkId::new("Arc<str>", needle), needle, |b, needle| {
            b.iter(|| black_box(arc_map.get(black_box(needle))))
        });
        group.bench_with_input(
            BenchmarkId::new("CompactString", needle),
            needle,
            |b, needle| b.iter(|| black_box(compact_map.get(black_box(needle)))),
        );
        group.bench_with_input(
            BenchmarkId::new("SmartString", needle),
            needle,
            |b, needle| b.iter(|| black_box(smart_map.get(black_box(needle)))),
        );
        group.bench_with_input(
            BenchmarkId::new("CheetahString", needle),
            needle,
            |b, needle| b.iter(|| black_box(cheetah_map.get(black_box(needle)))),
        );
    }

    group.finish();
}

criterion_group!(benches, bench_topic_insert, bench_topic_lookup);
criterion_main!(benches);
