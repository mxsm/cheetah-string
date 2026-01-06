#![cfg_attr(not(feature = "std"), no_std)]

//! No more relying solely on the standard library's String! CheetahString is a versatile string type that can store static strings, dynamic strings, and byte arrays.
//! It is usable in both `std` and `no_std` environments. Additionally, CheetahString supports serde for serialization and deserialization.
//! CheetahString also supports the `bytes` feature, allowing conversion to the `bytes::Bytes` type.
//! This reduces memory allocations during cloning, enhancing performance.
//!
//! # SIMD Acceleration
//!
//! When compiled with the `simd` feature flag, CheetahString uses SIMD (Single Instruction, Multiple Data)
//! instructions to accelerate string matching operations on x86_64 platforms with SSE2 support.
//! SIMD acceleration is applied to:
//! - `starts_with()` - Pattern prefix matching
//! - `ends_with()` - Pattern suffix matching
//! - `contains()` / `find()` - Substring search
//! - Equality comparisons (`==`, `!=`)
//!
//! The implementation automatically uses SIMD for strings >= 16 bytes and falls back to scalar operations
//! for smaller inputs or when SIMD is not available.
//!
//! To enable SIMD acceleration:
//! ```toml
//! [dependencies]
//! cheetah-string = { version = "1.0.0", features = ["simd"] }  
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
//! Using SIMD-accelerated operations (when `simd` feature is enabled):
//! ```rust
//! use cheetah_string::CheetahString;
//!
//! let url = CheetahString::from("https://api.example.com/v1/users");
//!
//! // These operations use SIMD when the pattern is >= 16 bytes
//! if url.starts_with("https://") {
//!     println!("Secure connection");
//! }
//!
//! if url.contains("api") {
//!     println!("API endpoint");
//! }
//! ```
//!
mod cheetah_string;
mod error;

#[cfg(feature = "serde")]
mod serde;

#[cfg(all(feature = "simd", target_arch = "x86_64"))]
mod simd;

pub use cheetah_string::{CheetahString, SplitPattern, SplitStr, SplitWrapper, StrPattern};
pub use error::{Error, Result};
