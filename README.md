# CheetahString

[![Crates.io](https://img.shields.io/crates/v/cheetah-string.svg)](https://crates.io/crates/cheetah-string)
[![Documentation](https://docs.rs/cheetah-string/badge.svg)](https://docs.rs/cheetah-string)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](https://github.com/mxsm/cheetah-string)
[![Rust Version](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org)

`CheetahString` is an immutable, clone-cheap UTF-8 value for latency-sensitive
systems. It stores short text inline, keeps static text allocation-free, and
shares long dynamic text through `Arc<str>`. The same value contract works with
`std` and `no_std + alloc`.

Version `3.0.0-alpha.1` is the opt-in preview of the immutable architecture.

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

Opt into the alpha explicitly:

```toml
[dependencies]
cheetah-string = "=3.0.0-alpha.1"
```

With optional integrations:

```toml
[dependencies]
cheetah-string = {
  version = "=3.0.0-alpha.1",
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

The full executable contract is in
[`docs/bytes-interop.md`](docs/bytes-interop.md).

## Features

| Feature | Default | Contract |
|---|---:|---|
| `std` | Yes | Standard-library integration |
| `serde` | No | Serialization and deserialization |
| `bytes` | No | `CheetahBytes` and explicit byte/text conversion |
| `experimental-simd` | No | Isolated x86_64 SSE2 benchmark path; not recommended for production |
| `simd` | No | Deprecated alpha compatibility alias for `experimental-simd` |
| `experimental-packed` | No | Unstable packed-representation prototype |

Optional features do not change the stable `CheetahString` layout.

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

Thresholds, metadata requirements, and reproduction commands are documented in
[`docs/performance-gates.md`](docs/performance-gates.md). Hosted-runner and local
benchmark results are diagnostic; they do not independently establish a
release-grade performance pass.

The architecture/optimization design scores 96/100 only when all 14 versioned
conditions are evidenced. PR performance may pass while that aggregate remains
incomplete; final comparison only certifies its performance scope, while
release verification fails closed on every missing live attestation. See the
[release evidence discovery contract](bench-results/release/README.md).

## Safety and portability

Every deterministic CI run covers formatting, Clippy, Rust 1.75, all features,
`no_std`, exact layout, and allocation contracts. Nightly validation adds Miri,
Linux AddressSanitizer, transition fuzzing, and split differential fuzzing.

The unsafe constructors are explicitly named and require the caller to prove
UTF-8 validity. Safe byte constructors validate before creating text.

The historical local diagnostic record, with its own candidate identity,
execution counts, exclusions, and SHA-256 log digests, is in
[`bench-results/safety/2026-07-26-local/summary.md`](bench-results/safety/2026-07-26-local/summary.md).
Historical downstream compile and representative-test diagnostics are in the
[archived crater summary](bench-results/crater/rocketmq-6d286fadd/summary.md).
Neither record substitutes for an exact-candidate release attestation. The
stable unsafe-site inventory is in
[`docs/stable-unsafe-audit.md`](docs/stable-unsafe-audit.md).

## Migration and architecture

- [v2 to v3 migration](docs/migration-v2-to-v3.md)
- [ADR 001: immutable canonical value](docs/adr/001-immutable-cheetah-string.md)
- [ADR 002: bytes copy boundary](docs/adr/002-bytes-copy-boundary.md)
- [ADR 003: SIMD policy](docs/adr/003-simd-policy.md)
- [ADR 004: split capability](docs/adr/004-split-iterator-capability.md)
- [ADR 005: performance gates](docs/adr/005-performance-gates.md)
- [ADR 006: rejected packed boundary](docs/adr/006-experimental-packed-boundary.md)

The v3 alpha temporarily retains several deprecated v2 spellings so large
read-only consumers can validate the new representation incrementally.
Deprecated names do not retain mutable v2 semantics.

## Projects using CheetahString

- [RocketMQ Rust](https://github.com/mxsm/rocketmq-rust)

## License

Licensed under either of Apache License 2.0 or MIT, at your option.
