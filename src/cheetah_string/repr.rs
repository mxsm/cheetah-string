use alloc::string::String;
use alloc::sync::Arc;

use crate::inline::InlineStr;
pub(super) use crate::inline::INLINE_CAPACITY;

/// The `InnerString` enum represents different types of string storage.
///
/// This enum uses Small String Optimization (SSO) to avoid heap allocations for short strings.
///
/// Variants:
///
/// * `Inline` - Inline storage for strings <= 23 bytes (zero heap allocations).
/// * `Static(&'static str)` - A static string slice (zero heap allocations).
/// * `Shared(Arc<str>)` - A reference-counted string slice (single heap allocation, optimized).
/// * `Owned(String)` - An owned heap string used for builder-style mutation.
#[derive(Clone)]
pub(super) enum InnerString {
    /// Inline storage for short strings (up to 23 bytes).
    /// Stores the length and data directly without heap allocation.
    Inline(InlineStr),
    /// Static string slice with 'static lifetime.
    Static(&'static str),
    /// Reference-counted string slice (single heap allocation).
    /// Preferred for long immutable strings created from owned or borrowed data.
    Shared(Arc<str>),
    /// Owned heap-allocated string used when exclusive mutability matters.
    Owned(String),
}
