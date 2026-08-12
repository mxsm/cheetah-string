# CheetahString

[![Crates.io](https://img.shields.io/crates/v/cheetah-string.svg)](https://crates.io/crates/cheetah-string)
[![Documentation](https://docs.rs/cheetah-string/badge.svg)](https://docs.rs/cheetah-string)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](https://github.com/mxsm/cheetah-string)
[![Rust Version](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org)

`CheetahString` is an immutable, clone-cheap UTF-8 value for latency-sensitive
systems. It stores short text inline, keeps static text allocation-free, and
shares long dynamic text through `Arc<str>`. The same value contract works with
`std` and `no_std + alloc`.

Version `3.1.0` is the supported 3.x release line for the immutable
architecture.

## Design contract

| Storage | Condition | Construction allocation | Clone allocation |
|---|---|---:|---:|
| Inline | UTF-8 length ≤ 23 bytes | 0 | 0 |
| Static | `&'static str` | 0 | 0 |
| Shared | Other long text | 1 live backing allocation | 0 |

The representation has no mutable `Owned(String)` state. Construction history
therefore cannot change clone complexity. Use:

- `CheetahString` for protocol text, immutable fields, and collection keys;
- `CheetahBuilder` for append-heavy construction followed by `finish()`;
- standard `String` when mutation or spare capacity must continue;
- `CheetahBytes` for byte semantics when the optional `bytes` feature is active.

## Installation

Add the crate to your project:

```toml
[dependencies]
cheetah-string = "3.1.0"
```

With optional integrations:

```toml
[dependencies]
cheetah-string = {
  version = "3.1.0",
  features = ["serde", "bytes"]
}
```

The minimum supported Rust version is 1.75.

## Quick start

```rust
use cheetah_string::{CheetahBuilder, CheetahString};

let inline = CheetahString::from("orders");
let static_value = CheetahString::from_static_str("system-topic");
let shared = CheetahString::from_string("long-dynamic-value-".repeat(8));
let cloned = shared.clone();

assert_eq!(inline, "orders");
assert_eq!(static_value, "system-topic");
assert_eq!(shared, cloned);
assert_eq!(shared.as_bytes().as_ptr(), cloned.as_bytes().as_ptr());

let mut builder = CheetahBuilder::with_capacity(64);
builder.push_str("orders");
builder.push('@');
builder.push_str("group-a");
let route_key = builder.finish();

assert_eq!(route_key, "orders@group-a");
```

When mutation continues, keep the builder's `String`:

```rust
use cheetah_string::CheetahBuilder;

let mut builder = CheetahBuilder::with_capacity(128);
builder.push_str("orders");
let mut value = builder.into_string();
value.push_str("@group-a");
```

## Search and split

Equality, prefix, and suffix checks use Rust's portable slice/`str` paths.
Substring search uses `memchr`/`memmem`.

Iterator capabilities are explicit:

```rust
use cheetah_string::CheetahString;

let value = CheetahString::from("a::b::c");
let forward: Vec<_> = value.split_str("::").collect();
assert_eq!(forward, ["a", "b", "c"]);

let csv = CheetahString::from("a,b,c");
let reverse: Vec<_> = csv.split_char(',').rev().collect();
assert_eq!(reverse, ["c", "b", "a"]);
```

`split_str` is intentionally forward-only. Unsupported reverse iteration fails
at compile time instead of panicking at runtime.

## Bytes interoperability

The ownership boundary is explicit:

| Conversion | UTF-8 validation | Payload copy |
|---|---:|---:|
| `bytes::Bytes -> CheetahBytes` | No | No |
| `CheetahBytes -> bytes::Bytes` | No | No |
| `Bytes -> CheetahString::try_from` | Yes | Yes |
| `CheetahBytes -> CheetahString::try_from` | Yes | Yes |
| `Bytes -> CheetahString::try_copy_from_bytes` | Yes | Yes |
| `&CheetahBytes -> try_copy_to_cheetah_string` | Yes | Yes |

```rust
use bytes::Bytes;
use cheetah_string::{CheetahBytes, CheetahString};

let raw = Bytes::from_static(b"orders");
let bytes = CheetahBytes::from(raw);
let text = bytes.try_copy_to_cheetah_string().unwrap();
assert_eq!(text, "orders");

let invalid = Bytes::from_static(&[0xff]);
let error = CheetahString::try_copy_from_bytes(invalid.clone()).unwrap_err();
assert_eq!(error.into_bytes(), invalid);
```

The conversion matrix above is covered by `tests/bytes.rs` and
`tests/allocation_contract.rs`.

## Features

| Feature | Default | Contract |
|---|---:|---|
| `std` | Yes | Standard-library integration |
| `serde` | No | Serialization and deserialization |
| `bytes` | No | `CheetahBytes` and explicit byte/text conversion |
| `experimental-simd` | No | Isolated x86_64 SSE2 benchmark path; not recommended for production |
| `simd` | No | Deprecated alpha compatibility alias for `experimental-simd` |
| `experimental-packed` | No | Deprecated no-op retained for 3.x dependency compatibility |

Optional features do not change the stable `CheetahString` layout.

The former packed v1 type was removed in 3.1 because its heap representation
round-tripped an allocation pointer through `usize`, which strict-provenance
Miri rejected. There is no safe 24-byte drop-in replacement. Use
`CheetahString` for immutable text or `CheetahBuilder`/`String` while mutation
continues.

## Performance evidence

The repository includes RocketMQ-shaped Criterion workloads for property
building, remoting-header parsing, topic insertion and lookup, plus explicit
layout and allocation contracts. Blocking timing decisions run only on a
dedicated fixed CPU with two reversed base/head rounds.

```bash
cargo test --test layout_snapshot --all-features
cargo test --test allocation_contract --all-features -- --test-threads=1
cargo bench --bench comprehensive
cargo bench --bench mq_properties
cargo bench --bench mq_remoting_header
cargo bench --bench mq_topic
```

Hosted-runner and local benchmark results are diagnostic; they do not
independently establish a release-grade performance pass. The versioned
allocation and layout tests are the deterministic performance contracts.

## Safety and portability

The repository's workflows are the authoritative record of automated checks.
Release validation is fail-closed: formatting, linting, tests, dependency audit,
and package construction must complete before any tag or publication step.

The unsafe constructors are explicitly named and require the caller to prove
UTF-8 validity. Safe byte constructors validate before creating text.

## Projects using CheetahString

- [RocketMQ Rust](https://github.com/mxsm/rocketmq-rust)

## License

Licensed under either of Apache License 2.0 or MIT, at your option.
