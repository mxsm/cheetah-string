#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_op_in_unsafe_fn)]

//! An immutable, clone-cheap UTF-8 value for latency-sensitive systems.
//!
//! [`CheetahString`] has one constructor-independent value contract:
//!
//! - values up to 23 bytes are stored inline;
//! - static values borrow their `&'static str`;
//! - other long values use a shared `Arc<str>` backing.
//!
//! Long clones are bounded O(1) and allocate zero times. Append-heavy
//! construction belongs to [`CheetahBuilder`]; call
//! [`CheetahBuilder::finish`] to freeze the value or
//! [`CheetahBuilder::into_string`] when mutation or spare capacity must
//! continue. `from_string` freezes its input and does not retain a mutable
//! `String` representation.
//!
//! The crate supports `no_std + alloc`. Optional `serde` integration preserves
//! the text contract, while the `bytes` feature exposes [`CheetahBytes`] for
//! byte-oriented data. Byte-to-text conversion validates and copies; only
//! `bytes::Bytes <-> CheetahBytes` is zero-copy.
//!
//! # Split capability
//!
//! [`CheetahString::split_char`] returns a double-ended standard iterator.
//! [`CheetahString::split_str`] is forward-only, so unsupported reverse
//! iteration fails at compile time rather than panicking at runtime.
//!
//! # Search and experimental SIMD
//!
//! Stable builds delegate equality, prefix, and suffix comparisons to the
//! standard slice/`str` implementations so the compiler and standard library
//! select the best portable strategy. The `experimental-simd` feature exposes
//! an x86_64 SSE2 experiment for controlled benchmarking only. The deprecated
//! `simd` feature remains an alpha compatibility alias.
//!
//! Substring search through `find()` and `contains()` continues to use
//! `memchr`/`memmem`, which is the stable default search backend.
//!
//! To opt into the isolated experiment:
//!
//! ```toml
//! [dependencies]
//! cheetah-string = { version = "=3.0.0-alpha.1", features = ["experimental-simd"] }
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
mod error;
mod inline;
mod search;

#[cfg(feature = "bytes")]
#[path = "bytes.rs"]
mod cheetah_bytes;

#[cfg(feature = "serde")]
mod serde;

#[cfg(all(feature = "experimental-simd", target_arch = "x86_64"))]
mod simd;

#[cfg(feature = "experimental-packed")]
pub mod packed;

#[cfg(feature = "bytes")]
pub use cheetah_bytes::{CheetahBytes, FromUtf8BytesError};

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
