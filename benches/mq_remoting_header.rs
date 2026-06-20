use cheetah_string::CheetahString;
use compact_str::CompactString;
use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use smartstring::alias::String as SmartString;

fn header_fields() -> Vec<(&'static str, &'static str)> {
    vec![
        ("code", "310"),
        ("language", "RUST"),
        ("version", "455"),
        ("opaque", "10001"),
        ("flag", "0"),
        ("remark", ""),
        ("serializeTypeCurrentRPC", "JSON"),
        ("topic", "RMQ_SYS_TRACE_TOPIC_00001"),
        ("queueId", "4"),
        ("bornTimestamp", "1700000000000"),
        ("storeTimestamp", "1700000001000"),
        ("producerGroup", "order-producer"),
        ("consumerGroup", "order-consumer"),
    ]
}

fn encode_pairs<I, K, V>(fields: I) -> String
where
    I: IntoIterator<Item = (K, V)>,
    K: AsRef<str>,
    V: AsRef<str>,
{
    let mut encoded = String::with_capacity(256);
    for (key, value) in fields {
        encoded.push_str(key.as_ref());
        encoded.push('=');
        encoded.push_str(value.as_ref());
        encoded.push('\n');
    }
    encoded
}

fn bench_header_encode(c: &mut Criterion) {
    let fields = header_fields();
    let mut group = c.benchmark_group("mq_remoting_header_encode");
    group.throughput(Throughput::Elements(fields.len() as u64));

    group.bench_function("String", |b| {
        b.iter(|| {
            black_box(encode_pairs(
                fields
                    .iter()
                    .map(|(key, value)| ((*key).to_string(), (*value).to_string())),
            ))
        })
    });

    group.bench_function("CompactString", |b| {
        b.iter(|| {
            black_box(encode_pairs(fields.iter().map(|(key, value)| {
                (CompactString::from(*key), CompactString::from(*value))
            })))
        })
    });

    group.bench_function("SmartString", |b| {
        b.iter(|| {
            black_box(encode_pairs(fields.iter().map(|(key, value)| {
                (SmartString::from(*key), SmartString::from(*value))
            })))
        })
    });

    group.bench_function("CheetahString", |b| {
        b.iter(|| {
            black_box(encode_pairs(fields.iter().map(|(key, value)| {
                (CheetahString::from(*key), CheetahString::from(*value))
            })))
        })
    });

    group.finish();
}

fn bench_header_parse(c: &mut Criterion) {
    let encoded = encode_pairs(header_fields());
    let mut group = c.benchmark_group("mq_remoting_header_parse");
    group.throughput(Throughput::Bytes(encoded.len() as u64));

    group.bench_function("String", |b| {
        b.iter(|| {
            let mut fields = Vec::new();
            for line in encoded.lines() {
                if let Some((key, value)) = line.split_once('=') {
                    fields.push((black_box(key.to_string()), black_box(value.to_string())));
                }
            }
            black_box(fields)
        })
    });

    group.bench_function("CompactString", |b| {
        b.iter(|| {
            let mut fields = Vec::new();
            for line in encoded.lines() {
                if let Some((key, value)) = line.split_once('=') {
                    fields.push((
                        black_box(CompactString::from(key)),
                        black_box(CompactString::from(value)),
                    ));
                }
            }
            black_box(fields)
        })
    });

    group.bench_function("SmartString", |b| {
        b.iter(|| {
            let mut fields = Vec::new();
            for line in encoded.lines() {
                if let Some((key, value)) = line.split_once('=') {
                    fields.push((
                        black_box(SmartString::from(key)),
                        black_box(SmartString::from(value)),
                    ));
                }
            }
            black_box(fields)
        })
    });

    group.bench_function("CheetahString", |b| {
        b.iter(|| {
            let mut fields = Vec::new();
            for line in encoded.lines() {
                if let Some((key, value)) = line.split_once('=') {
                    fields.push((
                        black_box(CheetahString::from(key)),
                        black_box(CheetahString::from(value)),
                    ));
                }
            }
            black_box(fields)
        })
    });

    group.finish();
}

criterion_group!(benches, bench_header_encode, bench_header_parse);
criterion_main!(benches);
