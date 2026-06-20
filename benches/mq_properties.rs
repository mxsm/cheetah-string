use cheetah_string::CheetahString;
use compact_str::CompactString;
use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use smartstring::alias::String as SmartString;
use std::collections::HashMap;

fn properties() -> Vec<(&'static str, &'static str)> {
    vec![
        ("KEYS", "order-10001"),
        ("TAGS", "paid"),
        ("WAIT", "false"),
        ("DELAY", "0"),
        ("RETRY_TOPIC", "order-service"),
        ("REAL_TOPIC", "order-created"),
        ("REAL_QID", "4"),
        ("TRAN_MSG", "false"),
        ("PGROUP", "order-consumer"),
        ("MIN_OFFSET", "1024"),
        ("MAX_OFFSET", "2048"),
        ("BUYER_ID", "u-10001"),
        ("TRACE_ON", "true"),
        ("INSTANCE_ID", "rmq-prod-a"),
        ("CORRELATION_ID", "corr-10001"),
        ("REPLY_TO_CLIENT", "client-7"),
        ("TTL", "30000"),
        ("UNIQ_KEY", "7F00000100002A9F000000000001"),
        ("BORN_HOST", "10.0.0.1"),
        ("STORE_HOST", "10.0.0.2"),
    ]
}

fn bench_property_build(c: &mut Criterion) {
    let props = properties();
    let mut group = c.benchmark_group("mq_property_build");
    group.throughput(Throughput::Elements(props.len() as u64));

    group.bench_function("String", |b| {
        b.iter(|| {
            let mut map = HashMap::with_capacity(props.len());
            for (key, value) in &props {
                map.insert(
                    black_box((*key).to_string()),
                    black_box((*value).to_string()),
                );
            }
            black_box(map)
        })
    });

    group.bench_function("CompactString", |b| {
        b.iter(|| {
            let mut map = HashMap::with_capacity(props.len());
            for (key, value) in &props {
                map.insert(
                    black_box(CompactString::from(*key)),
                    black_box(CompactString::from(*value)),
                );
            }
            black_box(map)
        })
    });

    group.bench_function("SmartString", |b| {
        b.iter(|| {
            let mut map = HashMap::with_capacity(props.len());
            for (key, value) in &props {
                map.insert(
                    black_box(SmartString::from(*key)),
                    black_box(SmartString::from(*value)),
                );
            }
            black_box(map)
        })
    });

    group.bench_function("CheetahString", |b| {
        b.iter(|| {
            let mut map = HashMap::with_capacity(props.len());
            for (key, value) in &props {
                map.insert(
                    black_box(CheetahString::from(*key)),
                    black_box(CheetahString::from(*value)),
                );
            }
            black_box(map)
        })
    });

    group.finish();
}

fn bench_property_lookup(c: &mut Criterion) {
    let props = properties();
    let string_map: HashMap<String, String> = props
        .iter()
        .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
        .collect();
    let compact_map: HashMap<CompactString, CompactString> = props
        .iter()
        .map(|(key, value)| (CompactString::from(*key), CompactString::from(*value)))
        .collect();
    let smart_map: HashMap<SmartString, SmartString> = props
        .iter()
        .map(|(key, value)| (SmartString::from(*key), SmartString::from(*value)))
        .collect();
    let cheetah_map: HashMap<CheetahString, CheetahString> = props
        .iter()
        .map(|(key, value)| (CheetahString::from(*key), CheetahString::from(*value)))
        .collect();

    let mut group = c.benchmark_group("mq_property_lookup");
    group.throughput(Throughput::Elements(4));

    group.bench_function("String", |b| {
        b.iter(|| {
            black_box(string_map.get("UNIQ_KEY"));
            black_box(string_map.get("TAGS"));
            black_box(string_map.get("PGROUP"));
            black_box(string_map.get("MISSING"))
        })
    });
    group.bench_function("CompactString", |b| {
        b.iter(|| {
            black_box(compact_map.get("UNIQ_KEY"));
            black_box(compact_map.get("TAGS"));
            black_box(compact_map.get("PGROUP"));
            black_box(compact_map.get("MISSING"))
        })
    });
    group.bench_function("SmartString", |b| {
        b.iter(|| {
            black_box(smart_map.get("UNIQ_KEY"));
            black_box(smart_map.get("TAGS"));
            black_box(smart_map.get("PGROUP"));
            black_box(smart_map.get("MISSING"))
        })
    });
    group.bench_function("CheetahString", |b| {
        b.iter(|| {
            black_box(cheetah_map.get("UNIQ_KEY"));
            black_box(cheetah_map.get("TAGS"));
            black_box(cheetah_map.get("PGROUP"));
            black_box(cheetah_map.get("MISSING"))
        })
    });

    group.finish();
}

criterion_group!(benches, bench_property_build, bench_property_lookup);
criterion_main!(benches);
