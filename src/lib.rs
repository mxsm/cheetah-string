#![cfg_attr(not(feature = "std"), no_std)]

mod cheetah_string;

#[cfg(feature = "serde")]
mod serde;

pub use cheetah_string::CheetahString;
