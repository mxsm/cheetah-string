#![cfg(feature = "experimental-packed")]

use cheetah_string::packed::PackedCheetahString;
use cheetah_string::CheetahString;
use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use std::collections::HashMap;

const TOPIC_COUNT: usize = 1_000;

fn short_topics() -> Vec<String> {
    (0..TOPIC_COUNT).map(|i| format!("T{:022}", i)).collect()
}

fn long_topics() -> Vec<String> {
    (0..TOPIC_COUNT)
        .map(|i| format!("RMQ_SYS_TRACE_TOPIC_{:05}_LONG_ROUTE_SEGMENT", i))
        .collect()
}

fn bench_packed_construction(c: &mut Criterion) {
    let short = "topic-a";
    let long = "RMQ_SYS_TRACE_TOPIC_00001_LONG_ROUTE_SEGMENT";

    let mut group = c.benchmark_group("packed_construction");

    group.bench_function("CheetahString short", |b| {
        b.iter(|| CheetahString::from(black_box(short)))
    });
    group.bench_function("PackedCheetahString short", |b| {
        b.iter(|| PackedCheetahString::from(black_box(short)))
    });
    group.bench_function("CheetahString long", |b| {
        b.iter(|| CheetahString::from(black_box(long)))
    });
    group.bench_function("PackedCheetahString long", |b| {
        b.iter(|| PackedCheetahString::from(black_box(long)))
    });

    group.finish();
}

fn bench_packed_push_str(c: &mut Criterion) {
    let mut group = c.benchmark_group("packed_push_str");

    group.bench_function("CheetahString inline append", |b| {
        b.iter(|| {
            let mut value = CheetahString::from("topic");
            value.push_str(black_box("-a"));
            black_box(value)
        })
    });
    group.bench_function("PackedCheetahString inline append", |b| {
        b.iter(|| {
            let mut value = PackedCheetahString::from("topic");
            value.push_str(black_box("-a"));
            black_box(value)
        })
    });
    group.bench_function("CheetahString promote to heap", |b| {
        b.iter(|| {
            let mut value = CheetahString::from("12345678901234567890123");
            value.push_str(black_box("-overflow"));
            black_box(value)
        })
    });
    group.bench_function("PackedCheetahString promote to heap", |b| {
        b.iter(|| {
            let mut value = PackedCheetahString::from("12345678901234567890123");
            value.push_str(black_box("-overflow"));
            black_box(value)
        })
    });

    group.finish();
}

fn bench_packed_topic_insert(c: &mut Criterion) {
    let short_topics = short_topics();
    let long_topics = long_topics();
    let mut group = c.benchmark_group("packed_mq_topic_insert");
    group.throughput(Throughput::Elements(TOPIC_COUNT as u64));

    group.bench_function("CheetahString short topics", |b| {
        b.iter(|| {
            let mut map = HashMap::with_capacity(TOPIC_COUNT);
            for (idx, topic) in short_topics.iter().enumerate() {
                map.insert(black_box(CheetahString::from(topic.as_str())), idx);
            }
            black_box(map)
        })
    });
    group.bench_function("PackedCheetahString short topics", |b| {
        b.iter(|| {
            let mut map = HashMap::with_capacity(TOPIC_COUNT);
            for (idx, topic) in short_topics.iter().enumerate() {
                map.insert(black_box(PackedCheetahString::from(topic.as_str())), idx);
            }
            black_box(map)
        })
    });
    group.bench_function("CheetahString long topics", |b| {
        b.iter(|| {
            let mut map = HashMap::with_capacity(TOPIC_COUNT);
            for (idx, topic) in long_topics.iter().enumerate() {
                map.insert(black_box(CheetahString::from(topic.as_str())), idx);
            }
            black_box(map)
        })
    });
    group.bench_function("PackedCheetahString long topics", |b| {
        b.iter(|| {
            let mut map = HashMap::with_capacity(TOPIC_COUNT);
            for (idx, topic) in long_topics.iter().enumerate() {
                map.insert(black_box(PackedCheetahString::from(topic.as_str())), idx);
            }
            black_box(map)
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_packed_construction,
    bench_packed_push_str,
    bench_packed_topic_insert
);
criterion_main!(benches);
