# CheetahString

[![Crates.io](https://img.shields.io/crates/v/cheetah-string.svg)](https://crates.io/crates/cheetah-string)
[![Documentation](https://docs.rs/cheetah-string/badge.svg)](https://docs.rs/cheetah-string)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](https://github.com/mxsm/cheetah-string)
[![Rust Version](https://img.shields.io/badge/rust-1.95%2B-orange.svg)](https://www.rust-lang.org)

`CheetahString` is an immutable, clone-cheap UTF-8 value for latency-sensitive
systems. It stores short text inline, keeps static text allocation-free, and
shares long dynamic text through `Arc<str>`. The same value contract works with
`std` and `no_std + alloc`.

Version `3.1.0` is the supported 3.x release line for the immutable
architecture.

## Design contract

| Input path | Storage | Allocation events during conversion | Clone allocation |
|---|---|---:|---:|
| Explicit `from_static_str` | Static | 0 | 0 |
| Other UTF-8 input ≤ 23 bytes | Inline | 0 | 0 |
| Long `Arc<str>` | Shared | 0; payload pointer is retained | 0 |
| Long borrowed text or exact-capacity `String` | Shared | 1 | 0 |
| Long spare-capacity `String` / builder | Shared | 2: shrink/reallocate, then Arc backing | 0 |

On supported 32-bit and 64-bit targets, both `CheetahString` and
`Option<CheetahString>` occupy 24 bytes. A 10,000-element vector therefore uses
240,000 bytes of element slots instead of the previous 320,000-byte contract.
This is achieved with safe Rust enum niches; string pointers are never converted
to integers. `tests/layout_snapshot.rs` and `tests/allocation_contract.rs`
enforce the representation, container footprint, and portability contract.

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

The minimum supported Rust version is 1.95.

The packaged consumer matrix can be reproduced with
`bash scripts/check-msrv-package.sh 1.95` on Unix or
`pwsh -File scripts/check-msrv-package.ps1 -Msrv 1.95` on Windows.

## Quick start

```rust
use cheetah_string::{CheetahBuilder, CheetahString};

let inline = CheetahString::from("orders");
let static_value = CheetahString::from_static_str("system-topic");
let shared = CheetahString::from_string("long-dynamic-value-".repeat(8));
let adopted = CheetahString::from(std::sync::Arc::<str>::from(
    "ownership-preserving-shared-value",
));
let cloned = shared.clone();

assert_eq!(inline, "orders");
assert_eq!(static_value, "system-topic");
assert_eq!(shared, cloned);
assert_eq!(shared.as_bytes().as_ptr(), cloned.as_bytes().as_ptr());
assert_eq!(adopted, "ownership-preserving-shared-value");

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

let reverse_lines: Vec<_> = CheetahString::from("a\nb\nc").lines().rev().collect();
assert_eq!(reverse_lines, ["c", "b", "a"]);
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
Miri rejected. The stable immutable `CheetahString` now reaches 24 bytes through
safe Rust enum niches, but it is not a mutable `PackedCheetahString` drop-in.
Use `CheetahBuilder` or `String` while mutation continues.

## Performance evidence

The repository includes RocketMQ-shaped Criterion workloads for property
building, remoting-header parsing, topic insertion and lookup, plus explicit
layout and allocation contracts. Blocking timing decisions run only on a
dedicated fixed CPU with two reversed base/head rounds.

```bash
cargo test --test layout_snapshot --all-features
cargo test --test allocation_contract --all-features -- --test-threads=1
cargo bench --bench shared_backing -- __allocation_evidence_only__ --noplot \
  2>&1 | tee target/allocation-evidence.log
python scripts/verify-allocation-evidence.py target/allocation-evidence.log
cargo bench --bench comprehensive
cargo bench --bench mq_properties
cargo bench --bench mq_remoting_header
cargo bench --bench mq_topic
```

Hosted-runner and local benchmark results are diagnostic; they do not
independently establish a release-grade performance pass. The versioned
allocation and layout tests are the deterministic performance contracts.
`scripts/verify-allocation-evidence.py` independently validates the schema-v3
allocation record emitted by the shared-backing benchmark.

## Safety and portability

The repository's workflows are the authoritative record of automated checks.
Release validation is fail-closed: formatting, linting, tests, dependency audit,
and package construction must complete before any tag or publication step.

The unsafe constructors are explicitly named and require the caller to prove
UTF-8 validity. Safe byte constructors validate before creating text.

CI enforces the Rust 1.95 packaged-consumer matrix, warning-free rustdoc,
locked dependency auditing, and repository workflow contracts. The Safety
workflow runs Miri over the stable text/byte invariants and compiles every
libFuzzer target with AddressSanitizer on pull requests and on a weekly
schedule. The maintained commands and toolchain setup are encoded directly in
`.github/workflows/safety.yml`; unsafe constructor obligations remain next to
their public APIs in `src/cheetah_string/construct.rs` and `src/bytes.rs`.

Pattern and error signatures are covered by `tests/api_contract.rs`. The
dedicated `.github/workflows/api-compatibility.yml` workflow compares every pull
request with `origin/main` under minor-release semver rules.

## Projects using CheetahString

- [RocketMQ Rust](https://github.com/mxsm/rocketmq-rust)

## License

Licensed under either of Apache License 2.0 or MIT, at your option.
