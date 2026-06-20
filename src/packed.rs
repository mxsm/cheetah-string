//! Experimental 24-byte packed string prototype.
//!
//! This module is available only with `feature = "experimental-packed"` and is
//! not used by the stable `CheetahString` representation.

#![cfg(all(target_pointer_width = "64", target_endian = "little"))]

use alloc::string::{String, ToString};
use core::fmt;
use core::hash::{Hash, Hasher};
use core::mem::ManuallyDrop;
use core::ops::Deref;
use core::slice;
use core::str;

pub const INLINE_CAPACITY: usize = 23;

const HEAP_TAG: usize = 1usize << 63;
const CAP_MASK: usize = !HEAP_TAG;

/// A 24-byte experimental packed string.
///
/// The layout is intentionally private. Inline strings store up to 23 bytes and
/// one length byte. Heap strings store `String` raw parts with the high capacity
/// bit used as a heap tag.
#[repr(C)]
pub struct PackedCheetahString {
    raw: [usize; 3],
}

impl PackedCheetahString {
    #[inline]
    pub const fn new() -> Self {
        Self { raw: [0; 3] }
    }

    #[inline]
    pub fn from_string(s: String) -> Self {
        if s.len() <= INLINE_CAPACITY {
            return Self::from_inline(s.as_str());
        }

        let mut s = ManuallyDrop::new(s);
        let ptr = s.as_mut_ptr() as usize;
        let len = s.len();
        let cap = s.capacity();
        assert_eq!(cap & HEAP_TAG, 0, "capacity exceeds packed representation");

        Self {
            raw: [ptr, len, cap | HEAP_TAG],
        }
    }

    #[inline]
    fn from_inline(s: &str) -> Self {
        debug_assert!(s.len() <= INLINE_CAPACITY);

        let mut packed = Self::new();
        let bytes = packed.raw_as_mut_bytes();
        bytes[..s.len()].copy_from_slice(s.as_bytes());
        bytes[INLINE_CAPACITY] = s.len() as u8;
        packed
    }

    #[inline]
    pub fn as_str(&self) -> &str {
        if self.is_inline() {
            let bytes = self.raw_as_bytes();
            let len = bytes[INLINE_CAPACITY] as usize;
            // SAFETY: constructors only copy valid UTF-8 from &str/String.
            unsafe { str::from_utf8_unchecked(&bytes[..len]) }
        } else {
            let (ptr, len, _) = self.heap_parts();
            // SAFETY: heap values are created from String raw parts and remain
            // owned by self until Drop.
            let bytes = unsafe { slice::from_raw_parts(ptr as *const u8, len) };
            // SAFETY: the original String guaranteed UTF-8.
            unsafe { str::from_utf8_unchecked(bytes) }
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.as_str().len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    pub fn is_inline(&self) -> bool {
        self.raw[2] & HEAP_TAG == 0
    }

    #[inline]
    pub fn push_str(&mut self, rhs: &str) {
        if rhs.is_empty() {
            return;
        }

        if self.is_inline() {
            let current_len = self.raw_as_bytes()[INLINE_CAPACITY] as usize;
            let total_len = current_len + rhs.len();
            if total_len <= INLINE_CAPACITY {
                let bytes = self.raw_as_mut_bytes();
                bytes[current_len..total_len].copy_from_slice(rhs.as_bytes());
                bytes[INLINE_CAPACITY] = total_len as u8;
                return;
            }
        }

        let mut owned = self.as_str().to_string();
        owned.push_str(rhs);
        *self = Self::from_string(owned);
    }

    #[inline]
    fn heap_parts(&self) -> (usize, usize, usize) {
        debug_assert!(!self.is_inline());
        (self.raw[0], self.raw[1], self.raw[2] & CAP_MASK)
    }

    #[inline]
    fn raw_as_bytes(&self) -> &[u8] {
        // SAFETY: raw is an in-place byte buffer for the packed representation.
        unsafe { slice::from_raw_parts(self.raw.as_ptr() as *const u8, INLINE_CAPACITY + 1) }
    }

    #[inline]
    fn raw_as_mut_bytes(&mut self) -> &mut [u8] {
        // SAFETY: raw is an in-place byte buffer for the packed representation.
        unsafe { slice::from_raw_parts_mut(self.raw.as_mut_ptr() as *mut u8, INLINE_CAPACITY + 1) }
    }
}

impl Default for PackedCheetahString {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for PackedCheetahString {
    #[inline]
    fn clone(&self) -> Self {
        if self.is_inline() {
            Self { raw: self.raw }
        } else {
            Self::from_string(self.as_str().to_string())
        }
    }
}

impl Drop for PackedCheetahString {
    fn drop(&mut self) {
        if !self.is_inline() {
            let (ptr, len, cap) = self.heap_parts();
            // SAFETY: heap values are created from String raw parts exactly once
            // and Drop reconstructs the same allocation exactly once.
            unsafe {
                drop(String::from_raw_parts(ptr as *mut u8, len, cap));
            }
        }
    }
}

impl Deref for PackedCheetahString {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl AsRef<str> for PackedCheetahString {
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl From<&str> for PackedCheetahString {
    #[inline]
    fn from(s: &str) -> Self {
        Self::from_string(s.to_string())
    }
}

impl From<String> for PackedCheetahString {
    #[inline]
    fn from(s: String) -> Self {
        Self::from_string(s)
    }
}

impl fmt::Debug for PackedCheetahString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), f)
    }
}

impl fmt::Display for PackedCheetahString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl PartialEq for PackedCheetahString {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl Eq for PackedCheetahString {}

impl Hash for PackedCheetahString {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}

#[cfg(test)]
mod tests {
    use super::PackedCheetahString;
    use core::mem::{align_of, size_of};

    #[test]
    fn packed_size_is_three_words() {
        assert_eq!(size_of::<PackedCheetahString>(), 24);
        assert_eq!(align_of::<PackedCheetahString>(), 8);
    }
}
