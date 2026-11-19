#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]

//! An immutable, clone-cheap UTF-8 value for latency-sensitive systems.
//!
//! [`CheetahString`] has one constructor-independent value contract:
//!
//! - explicit static values borrow their `&'static str`;
//! - other values up to 23 bytes are stored inline;
//! - other long values use a shared `Arc<str>` backing.
//!
//! Long clones are bounded O(1) and allocate zero times. Append-heavy
//! construction belongs to [`CheetahBuilder`]; call
//! [`CheetahBuilder::finish`] to freeze the value or
//! [`CheetahBuilder::into_string`] when mutation or spare capacity must
//! continue. `from_string` freezes its input and does not retain a mutable
//! `String` representation. Long `Arc<str>` inputs can be adopted without
//! allocation through [`CheetahString::from_arc_str`].
//!
//! The crate supports `no_std + alloc`. Optional `serde` integration preserves
//! the text contract, while the `bytes` feature exposes `CheetahBytes` for
//! byte-oriented data and `CheetahText`, UTF-8 text backed by `bytes::Bytes`.
//! Validating a `Bytes` buffer into `CheetahText`, and slicing it, never copy
//! the text; conversion into `CheetahString` copies.
//!
//! # Split capability
//!
//! [`CheetahString::split_char`] returns a double-ended standard iterator.
//! [`CheetahString::split_str`] is forward-only, so unsupported reverse
//! iteration fails at compile time rather than panicking at runtime.
//!
//! # Search and experimental SIMD
//!
//! Equality, prefix, and suffix comparisons of up to 64 bytes use
//! overlapping fixed-size loads instead of a `memcmp` call; longer inputs use
//! `memcmp`. The `experimental-simd` feature exposes an x86_64 SSE2
//! experiment for controlled benchmarking only. The deprecated `simd` feature
//! remains a 3.x compatibility alias.
//!
//! Substring search picks its backend by length: short needles on text that
//! spans a 16-byte block use the standard library's SIMD `contains`;
//! `find()` uses `memchr::memmem` below 24 and from 128 bytes. Use
//! [`CheetahFinder`] to search for one needle repeatedly.
//!
//! # Retired experimental features
//!
//! The former `experimental-packed` flag is a compatibility no-op. Its v1
//! public type was removed because integer-to-pointer reconstruction did not
//! satisfy Rust's strict-provenance model.
//!
//! ```compile_fail
//! use cheetah_string::packed::PackedCheetahString;
//! ```
//!
//! To opt into the isolated experiment:
//!
//! ```toml
//! [dependencies]
//! cheetah-string = { version = "3.1.0", features = ["experimental-simd"] }
//! ```
//!
//! # Example
//!
//! ```rust
//! use cheetah_string::{CheetahBuilder, CheetahString};
//!
//! let topic = CheetahString::from_static_str("orders");
//! assert!(topic.starts_with("ord"));
//!
//! let mut builder = CheetahBuilder::with_capacity(32);
//! builder.push_str(topic.as_str());
//! builder.push('@');
//! builder.push_str("group-a");
//!
//! let route = builder.finish();
//! assert_eq!(route, "orders@group-a");
//! ```
extern crate alloc;

mod builder;
mod cheetah_string;
mod compare;
mod error;
mod inline;
mod search;

#[cfg(feature = "bytes")]
#[path = "bytes.rs"]
mod cheetah_bytes;

#[cfg(feature = "serde")]
mod serde;

#[cfg(feature = "bytes")]
mod text;

#[cfg(all(feature = "experimental-simd", target_arch = "x86_64"))]
mod simd;

#[cfg(feature = "bytes")]
pub use cheetah_bytes::{CheetahBytes, FromUtf8BytesError};
#[cfg(feature = "bytes")]
pub use text::CheetahText;

pub use builder::CheetahBuilder;
pub use cheetah_string::{CheetahString, SplitPattern, SplitStr, StrPattern};
pub use error::{Error, Result};
pub use search::CheetahFinder;

/// Deprecated v3 compatibility name for [`CheetahString`].
///
/// `CheetahString` is now itself immutable and clone-cheap, so a second value
/// type no longer carries a distinct contract.
#[deprecated(since = "3.0.0", note = "use CheetahString")]
pub type CheetahStr = CheetahString;
