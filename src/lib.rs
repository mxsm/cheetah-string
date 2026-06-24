#![cfg_attr(not(feature = "std"), no_std)]

//! No more relying solely on the standard library's String! CheetahString is a versatile string type that can store static and dynamic strings.
//! It is usable in both `std` and `no_std` environments. Additionally, CheetahString supports serde for serialization and deserialization.
//! `CheetahStr` is available for immutable clone-cheap string values, and
//! `CheetahBuilder` is available for append-heavy construction.
//! The `bytes` feature exposes `CheetahBytes` for byte-oriented data.
//! It minimizes allocations across small, shared, and builder-oriented string workloads.
//! `from_string` preserves owned storage for mutable string workflows.
//! Use `CheetahStr` for clone-cheap immutable values.
//! Substring search uses `memchr`/`memmem` by default.
//!
//! # SIMD Acceleration
//!
//! When compiled with the `simd` feature flag, CheetahString uses SIMD (Single Instruction, Multiple Data)
//! instructions to accelerate selected byte comparisons on x86_64 platforms with SSE2 support.
//! SIMD acceleration is applied to:
//! - `starts_with()` - Pattern prefix matching
//! - `ends_with()` - Pattern suffix matching
//! - Equality comparisons (`==`, `!=`)
//!
//! Substring search through `find()` and `contains()` continues to use
//! `memchr`/`memmem`, which is the stable default search backend.
//!
//! The implementation automatically uses SIMD for strings >= 16 bytes and falls back to scalar operations
//! for smaller inputs or when SIMD is not available.
//!
//! To enable SIMD acceleration:
//! ```toml
//! [dependencies]
//! cheetah-string = { version = "2.1.0", features = ["simd"] }
//! ```
//!
//! # Examples
//!
//! Basic usage:
//! ```rust
//! use cheetah_string::CheetahString;
//!
//!
//!  let s = CheetahString::from("Hello, world!");
//!
//!  let s2:&'static str = "Hello, world!";
//!  let s3 = CheetahString::from_static_str(s2);
//!
//!  let s4 = CheetahString::new();
//!
//! ```
//!
//! Using search operations:
//! ```rust
//! use cheetah_string::CheetahString;
//!
//! let url = CheetahString::from("https://api.example.com/v1/users");
//!
//! // Substring search uses memchr/memmem by default.
//! if url.starts_with("https://") {
//!     println!("Secure connection");
//! }
//!
//! if url.contains("api") {
//!     println!("API endpoint");
//! }
//! ```
//!
extern crate alloc;

mod builder;
mod cheetah_str;
mod cheetah_string;
mod error;
mod inline;
mod search;

#[cfg(feature = "bytes")]
#[path = "bytes.rs"]
mod cheetah_bytes;

#[cfg(feature = "serde")]
mod serde;

#[cfg(all(feature = "simd", target_arch = "x86_64"))]
mod simd;

#[cfg(feature = "experimental-packed")]
pub mod packed;

#[cfg(feature = "bytes")]
pub use cheetah_bytes::CheetahBytes;

pub use builder::CheetahBuilder;
pub use cheetah_str::CheetahStr;
pub use cheetah_string::{CheetahString, SplitPattern, SplitStr, SplitWrapper, StrPattern};
pub use error::{Error, Result};
pub use search::CheetahFinder;
