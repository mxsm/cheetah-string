//! Fixed workloads for `scripts/instruction-counts.py`.
//!
//! The script builds this file against two checkouts of the crate and runs
//! each scenario under Valgrind's cachegrind with two iteration counts. The
//! difference between the two runs, divided by the difference in iterations,
//! is the instruction count of one iteration, free of process start-up and
//! setup costs.
//!
//! Usage: `cheetah-icount --list` or `cheetah-icount <scenario> <iterations>`.
//!
//! Scenarios use only API that exists on the base branch, so both builds
//! succeed. Hashing uses fixed keys so every run executes the same
//! instructions.

use std::collections::HashMap;
use std::hash::{BuildHasher, BuildHasherDefault, DefaultHasher};
use std::hint::black_box;

use cheetah_string::{cheetah_format, CheetahBuilder, CheetahString};

/// A 14-byte key, stored inline.
const SHORT: &str = "orders@group-a";
/// A 50-byte key, stored in a shared allocation.
const LONG: &str = "%RETRY%order-consumer-group@broker-a-instance-0001";

type FixedState = BuildHasherDefault<DefaultHasher>;

fn repeat<T>(iterations: u64, mut operation: impl FnMut() -> T) {
    for _ in 0..iterations {
        black_box(operation());
    }
}

fn text(len: usize) -> String {
    "topic-orders-group-consumer-broker-queue-"
        .chars()
        .cycle()
        .take(len)
        .collect()
}

fn from_str_inline(iterations: u64) {
    repeat(iterations, || CheetahString::from(black_box(SHORT)));
}

fn from_str_shared(iterations: u64) {
    repeat(iterations, || CheetahString::from(black_box(LONG)));
}

fn from_string_shared(iterations: u64) {
    repeat(iterations, || {
        CheetahString::from_string(String::from(black_box(LONG)))
    });
}

fn clone_drop_inline(iterations: u64) {
    let value = CheetahString::from(SHORT);
    repeat(iterations, || black_box(&value).clone());
}

fn clone_drop_shared(iterations: u64) {
    let value = CheetahString::from(LONG);
    repeat(iterations, || black_box(&value).clone());
}

fn eq_inline(iterations: u64) {
    let (a, b) = (CheetahString::from(SHORT), CheetahString::from(SHORT));
    repeat(iterations, || black_box(&a) == black_box(&b));
}

fn eq_shared_distinct(iterations: u64) {
    let (a, b) = (CheetahString::from(LONG), CheetahString::from(LONG));
    repeat(iterations, || black_box(&a) == black_box(&b));
}

fn eq_str(iterations: u64) {
    let a = CheetahString::from(LONG);
    repeat(iterations, || black_box(&a) == black_box(LONG));
}

fn cmp_inline(iterations: u64) {
    let (a, b) = (CheetahString::from(SHORT), CheetahString::from("orders@group-b"));
    repeat(iterations, || black_box(&a).cmp(black_box(&b)));
}

fn cmp_shared(iterations: u64) {
    let a = CheetahString::from(LONG);
    let b = CheetahString::from("%RETRY%order-consumer-group@broker-a-instance-0002");
    repeat(iterations, || black_box(&a).cmp(black_box(&b)));
}

fn hash_inline(iterations: u64) {
    let value = CheetahString::from(SHORT);
    let state = FixedState::default();
    repeat(iterations, || state.hash_one(black_box(&value)));
}

fn map_get_64(iterations: u64) {
    let keys: Vec<CheetahString> = (0..64)
        .map(|i| CheetahString::from(format!("topic-{i:03}@group").as_str()))
        .collect();
    let map: HashMap<CheetahString, usize, FixedState> =
        keys.iter().cloned().zip(0..).collect();
    let mut index = 0;
    repeat(iterations, || {
        index = (index + 1) & 63;
        map.get(black_box(&keys[index])).copied()
    });
}

fn as_bytes_scan_64(iterations: u64) {
    let values: Vec<CheetahString> = (0..64)
        .map(|i| CheetahString::from(text(4 + (i * 7) % 60).as_str()))
        .collect();
    // Read the first and last byte of each slice, so the pointer and length
    // decoding both count and the byte loads cannot be optimised away.
    repeat(iterations, || {
        black_box(&values)
            .iter()
            .map(|value| {
                let bytes = black_box(value.as_bytes());
                bytes.len() + usize::from(bytes[0]) + usize::from(bytes[bytes.len() - 1])
            })
            .sum::<usize>()
    });
}

fn starts_ends_with(iterations: u64) {
    let value = CheetahString::from(LONG);
    repeat(iterations, || {
        black_box(&value).starts_with(black_box("%RETRY%order"))
            && black_box(&value).ends_with(black_box("instance-0001"))
    });
}

fn contains_32b(iterations: u64) {
    let value = CheetahString::from(text(32).as_str());
    repeat(iterations, || black_box(&value).contains(black_box("roup-cz")));
}

fn find_48b(iterations: u64) {
    let value = CheetahString::from(text(48).as_str());
    repeat(iterations, || black_box(&value).find(black_box("consumez")));
}

fn rfind_48b(iterations: u64) {
    let value = CheetahString::from(text(48).as_str());
    repeat(iterations, || black_box(&value).rfind(black_box("consumez")));
}

fn find_1k(iterations: u64) {
    let value = CheetahString::from(text(1024).as_str());
    repeat(iterations, || black_box(&value).find(black_box("consumez")));
}

fn split_count(iterations: u64) {
    let value = CheetahString::from("a::b::c::d::e");
    repeat(iterations, || black_box(&value).split_str("::").count());
}

fn split_dense_no_match(iterations: u64) {
    // The separator's first byte is everywhere but the separator never occurs.
    let value = CheetahString::from("a:".repeat(30).as_str());
    repeat(iterations, || black_box(&value).split_str("::").count());
}

fn concat_inline(iterations: u64) {
    repeat(iterations, || {
        CheetahString::concat(&[black_box("orders"), "@", black_box("group-a")])
    });
}

fn concat_shared(iterations: u64) {
    repeat(iterations, || {
        CheetahString::concat(&[
            black_box("%RETRY%order-consumer-group"),
            "@",
            black_box("broker-a-instance-0001"),
        ])
    });
}

fn builder_inline(iterations: u64) {
    repeat(iterations, || {
        let mut builder = CheetahBuilder::new();
        builder.push_str(black_box("orders"));
        builder.push('@');
        builder.push_str(black_box("group-a"));
        builder.finish()
    });
}

fn builder_shared(iterations: u64) {
    repeat(iterations, || {
        let mut builder = CheetahBuilder::with_capacity(50);
        builder.push_str(black_box("%RETRY%order-consumer-group"));
        builder.push('@');
        builder.push_str(black_box("broker-a-instance-0001"));
        builder.finish()
    });
}

fn format_inline(iterations: u64) {
    repeat(iterations, || {
        cheetah_format!("{}@{}", black_box("orders"), black_box("group-a"))
    });
}

fn to_uppercase_inline(iterations: u64) {
    let value = CheetahString::from(SHORT);
    repeat(iterations, || black_box(&value).to_uppercase());
}

fn to_uppercase_shared(iterations: u64) {
    let value = CheetahString::from(LONG);
    repeat(iterations, || black_box(&value).to_uppercase());
}

fn replace_shared(iterations: u64) {
    let value = CheetahString::from(LONG);
    repeat(iterations, || black_box(&value).replace(black_box("-"), "_"));
}

fn replace_two_byte(iterations: u64) {
    let value = CheetahString::from(LONG);
    repeat(iterations, || black_box(&value).replace(black_box("er"), "ER"));
}

fn replacen_shared(iterations: u64) {
    let value = CheetahString::from(LONG);
    repeat(iterations, || {
        black_box(&value).replacen(black_box("-"), "_", 2)
    });
}

const SCENARIOS: &[(&str, fn(u64))] = &[
    ("from_str/inline_14B", from_str_inline),
    ("from_str/shared_50B", from_str_shared),
    ("from_string/shared_50B", from_string_shared),
    ("clone_drop/inline", clone_drop_inline),
    ("clone_drop/shared", clone_drop_shared),
    ("eq/inline_14B", eq_inline),
    ("eq/shared_50B_distinct", eq_shared_distinct),
    ("eq/str_50B", eq_str),
    ("cmp/inline_14B", cmp_inline),
    ("cmp/shared_50B", cmp_shared),
    ("hash/inline_14B", hash_inline),
    ("hash_map/get_64_keys", map_get_64),
    ("as_bytes/scan_64_values", as_bytes_scan_64),
    ("starts_ends_with/50B", starts_ends_with),
    ("contains/32B_miss", contains_32b),
    ("find/48B_miss", find_48b),
    ("rfind/48B_miss", rfind_48b),
    ("find/1KiB_miss", find_1k),
    ("split_str/count_5_parts", split_count),
    ("split_str/2B_dense_no_match", split_dense_no_match),
    ("concat/inline_14B", concat_inline),
    ("concat/shared_50B", concat_shared),
    ("builder/inline_14B", builder_inline),
    ("builder/shared_50B", builder_shared),
    ("cheetah_format/inline_14B", format_inline),
    ("to_uppercase/inline_14B", to_uppercase_inline),
    ("to_uppercase/shared_50B", to_uppercase_shared),
    ("replace/shared_50B", replace_shared),
    ("replace/two_byte_50B", replace_two_byte),
    ("replacen/first_2_of_50B", replacen_shared),
];

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(name) = args.next() else {
        eprintln!("usage: cheetah-icount --list | <scenario> <iterations>");
        std::process::exit(2);
    };
    if name == "--list" {
        for (name, _) in SCENARIOS {
            println!("{name}");
        }
        return;
    }
    let iterations: u64 = args
        .next()
        .and_then(|value| value.parse().ok())
        .expect("iteration count");
    let (_, run) = SCENARIOS
        .iter()
        .find(|(candidate, _)| *candidate == name)
        .unwrap_or_else(|| panic!("unknown scenario {name}"));
    run(iterations);
}
