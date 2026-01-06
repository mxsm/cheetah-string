# 🐆 CheetahString

[![Crates.io](https://img.shields.io/crates/v/cheetah-string.svg)](https://crates.io/crates/cheetah-string)
[![Documentation](https://docs.rs/cheetah-string/badge.svg)](https://docs.rs/cheetah-string)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](https://github.com/mxsm/cheetah-string)
[![Rust Version](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org)

**A lightweight, high-performance string type optimized for real-world use cases.**

CheetahString is a versatile string type that goes beyond the standard library's `String`, providing zero-allocation optimizations for common patterns and seamless interoperability with various string representations. It's designed for both `std` and `no_std` environments.

## ✨ Features

- **🚀 Zero-Allocation Optimization**
  - Small String Optimization (SSO): Strings ≤ 23 bytes stored inline (no heap allocation)
  - Static string support with `'static` lifetime
  - Efficient Arc-based sharing for larger strings

- **🔧 Rich API**
  - Query methods: `starts_with`, `ends_with`, `contains`, `find`, `rfind`
  - Transformation: `to_uppercase`, `to_lowercase`, `replace`, `trim`
  - Iteration: `split`, `lines`, `chars`
  - Builder pattern: `with_capacity`, `push_str`, `reserve`
  - String concatenation with `+` and `+=` operators

- **🌐 Flexible Integration**
  - Optional `bytes` support for zero-copy interop with the `bytes` crate
  - Optional `serde` support for serialization/deserialization
  - `no_std` compatible (requires `alloc`)

- **⚡ Performance Focused**
  - Optimized for common string operations
  - Reduced memory allocations via intelligent internal representation
  - Optional SIMD acceleration for string matching operations (x86_64 SSE2)
  - Benchmarked against standard library types

- **🛡️ Safe & Correct**
  - UTF-8 validation with safe constructors (`try_from_bytes`, `try_from_vec`)
  - Comprehensive test coverage
  - Well-documented API with examples

## 📦 Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
cheetah-string = "1.0.0"
```

### Optional Features

```toml
[dependencies]
cheetah-string = { version = "1.0.0", features = ["bytes", "serde", "simd"] }
```

Available features:
- `std` (default): Enable standard library support
- `bytes`: Integration with the `bytes` crate
- `serde`: Serialization support via serde
- `simd`: SIMD-accelerated string operations (x86_64 SSE2)

## 🚀 Quick Start

```rust
use cheetah_string::CheetahString;

// Create from various sources
let s1 = CheetahString::from("hello");           // From &str
let s2 = CheetahString::from(String::from("world")); // From String
let s3 = CheetahString::from_static_str("static");   // From 'static str (zero-cost)

// Small strings (≤ 23 bytes) use no heap allocation
let small = CheetahString::from("short");  // Stored inline!

// String operations
let s = CheetahString::from("Hello, World!");
assert!(s.starts_with("Hello"));  // Supports &str
assert!(s.starts_with('H'));      // Also supports char
assert!(s.contains("World"));
assert!(s.contains('W'));
assert_eq!(s.to_lowercase(), "hello, world!");

// Concatenation
let greeting = CheetahString::from("Hello");
let name = CheetahString::from(" Rust");
let message = greeting + name.as_str();  // "Hello Rust"

// Builder pattern for efficient construction
let mut builder = CheetahString::with_capacity(100);
builder.push_str("Hello");
builder.push_str(", ");
builder.push_str("World!");

// Safe UTF-8 validation
let bytes = b"hello";
let s = CheetahString::try_from_bytes(bytes).unwrap();
```

## 📊 Performance

CheetahString is designed with performance in mind:

- **Small String Optimization (SSO)**: Strings up to 23 bytes are stored inline without heap allocation
- **Efficient Sharing**: Large strings use `Arc<str>` for cheap cloning
- **Optimized Operations**: Common operations like concatenation have fast-path implementations
- **SIMD Acceleration** (with `simd` feature): String matching operations (`starts_with`, `ends_with`, `contains`, `find`, equality comparisons) are accelerated using SSE2 SIMD instructions on x86_64 platforms. The implementation automatically falls back to scalar code for small inputs or when SIMD is not available.

Run benchmarks:
```bash
cargo bench

# With SIMD feature
cargo bench --features simd
```

## 🔍 Internal Representation

CheetahString intelligently chooses the most efficient storage:

| String Type | Storage | Heap Allocations | Use Case |
|-------------|---------|------------------|----------|
| ≤ 23 bytes | Inline (SSO) | 0 | Short strings, identifiers |
| Static | `&'static str` | 0 | String literals |
| Dynamic | `Arc<str>` | 1 | Long strings, shared data |
| From Arc | `Arc<String>` | 1 | Interop with existing Arc |
| Bytes | `bytes::Bytes` | 1 | Network buffers (with feature) |

## 🔧 API Overview

### Construction
- `new()`, `empty()`, `default()` - Create empty strings
- `from(s)` - From `&str`, `String`, `&String`, `char`, etc.
- `from_static_str(s)` - Zero-cost wrapper for `'static str`
- `from_string(s)` - From owned `String`
- `try_from_bytes(b)` - Safe construction from bytes with UTF-8 validation
- `with_capacity(n)` - Pre-allocate capacity

### Query Methods
- `len()`, `is_empty()`, `as_str()`, `as_bytes()`
- `starts_with()`, `ends_with()`, `contains()` - Support both `&str` and `char` patterns
- `find()`, `rfind()`

### Transformation
- `to_uppercase()`, `to_lowercase()`
- `replace()`, `replacen()`
- `trim()`, `trim_start()`, `trim_end()`
- `substring()`, `repeat()`

### Iteration
- `chars()` - Iterate over characters (double-ended iterator)
- `split()` - Split by pattern (supports `&str` and `char`)
- `lines()` - Iterate over lines

### Mutation
- `push_str()` - Append string slice
- `reserve()` - Reserve additional capacity

### Operators
- `+` - Concatenation
- `+=` - Append in-place (optimized)
- `==`, `!=` - Equality comparison with `str`, `String`, etc.

## 🎯 Use Cases

CheetahString is ideal for:

- **High-performance servers**: Reduce allocations in hot paths
- **Memory-constrained environments**: Efficient memory usage with SSO
- **Network protocols**: Integration with `bytes` crate
- **Configuration systems**: Fast handling of static and dynamic strings
- **No-std applications**: Embedded systems and WASM

## 🏗️ Projects Using CheetahString

- [**RocketMQ Rust**](https://github.com/mxsm/rocketmq-rust) - Apache RocketMQ Rust implementation

## 🤝 Contributing

Contributions are welcome! Here's how you can help:

1. **Report Issues**: Found a bug? [Open an issue](https://github.com/mxsm/cheetah-string/issues)
2. **Submit PRs**: Improvements and bug fixes are appreciated
3. **Add Benchmarks**: Help us track performance across use cases
4. **Improve Documentation**: Better docs help everyone

### Development Setup

```bash
# Clone the repository
git clone https://github.com/mxsm/cheetah-string.git
cd cheetah-string

# Run tests
cargo test

# Run benchmarks
cargo bench

# Run with all features
cargo test --all-features
```

## 📝 License

This project is licensed under either of:

- Apache License, Version 2.0 ([LICENSE](LICENSE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## 🙏 Acknowledgments

CheetahString is inspired by the need for a flexible, high-performance string type in Rust that bridges the gap between `String`, `&str`, `Arc<str>`, and specialized types like `bytes::Bytes`.