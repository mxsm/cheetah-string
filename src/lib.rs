#![cfg_attr(not(feature = "std"), no_std)]

//! No more relying solely on the standard library's String! CheetahString is a versatile string type that can store static strings, dynamic strings, and byte arrays.
//! It is usable in both `std` and `no_std` environments. Additionally, CheetahString supports serde for serialization and deserialization.
//! CheetahString also supports the `bytes` feature, allowing conversion to the `bytes::Bytes` type.
//! This reduces memory allocations during cloning, enhancing performance.
//! example:
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
mod cheetah_string;

#[cfg(feature = "serde")]
mod serde;

pub use cheetah_string::CheetahString;
