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
pub(super) enum InnerString {
    /// Inline storage for short strings (up to 23 bytes).
    /// Stores the length and data directly without heap allocation.
    Inline(InlineStr),
    /// Static string slice with 'static lifetime.
    Static(&'static str),
    /// Reference-counted string slice (single heap allocation).
    /// Preferred for long immutable strings created from owned or borrowed data.
    Shared(Arc<str>),
}

impl Clone for InnerString {
    /// Clones by bumping the shared count first and then copying all 24
    /// bytes at once.
    ///
    /// A derived `Clone` rebuilds each variant separately. Bytes 1..8 are
    /// inline data for `Inline` but padding for `Static` and `Shared`, so the
    /// merged result is assembled through overlapping stack moves that defeat
    /// store-to-load forwarding. A single bitwise copy avoids that stall.
    #[inline]
    fn clone(&self) -> Self {
        if let InnerString::Shared(shared) = self {
            // Take one extra strong reference; the bitwise copy below owns it.
            core::mem::forget(Arc::clone(shared));
        }
        // SAFETY: `Inline` and `Static` own no resources, and the strong count
        // of a `Shared` value was incremented above, so the copy is an
        // independently owned clone for every variant.
        unsafe { core::ptr::read(self) }
    }
}

#[cfg(test)]
mod tests {
    use super::InnerString;
    use crate::inline::InlineStr;
    use alloc::sync::Arc;

    #[test]
    fn clone_preserves_every_variant_and_shared_ownership() {
        let inline = InnerString::Inline(InlineStr::from_str("inline").expect("fits inline"));
        let static_value = InnerString::Static("a static value longer than twenty-three bytes");
        let shared: Arc<str> = Arc::from("a shared value longer than twenty-three bytes");
        let shared_value = InnerString::Shared(Arc::clone(&shared));

        for value in [&inline, &static_value, &shared_value] {
            let first = value.clone();
            let second = first.clone();
            match (value, &first, &second) {
                (InnerString::Inline(a), InnerString::Inline(b), InnerString::Inline(c)) => {
                    assert_eq!(a.as_str(), b.as_str());
                    assert_eq!(b.as_str(), c.as_str());
                }
                (InnerString::Static(a), InnerString::Static(b), InnerString::Static(c)) => {
                    assert_eq!((a.as_ptr(), b.as_ptr()), (c.as_ptr(), c.as_ptr()));
                }
                (InnerString::Shared(a), InnerString::Shared(b), InnerString::Shared(c)) => {
                    assert!(Arc::ptr_eq(a, b) && Arc::ptr_eq(b, c));
                }
                _ => panic!("clone changed the storage variant"),
            }
        }

        // The loop's clones are dropped; `shared`, `shared_value`, `extra` remain.
        let extra = shared_value.clone();
        assert_eq!(Arc::strong_count(&shared), 3);
        drop(extra);
        drop(shared_value);
        assert_eq!(Arc::strong_count(&shared), 1);
    }
}
