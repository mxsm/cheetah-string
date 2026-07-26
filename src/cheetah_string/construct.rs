use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::str::{self, Utf8Error};

use super::repr::{InnerString, INLINE_CAPACITY};
use super::CheetahString;
use crate::inline::InlineStr;

impl CheetahString {
    #[inline]
    pub const fn empty() -> Self {
        CheetahString {
            inner: InnerString::Inline(InlineStr::empty()),
        }
    }

    #[inline]
    pub fn new() -> Self {
        CheetahString::default()
    }

    #[inline]
    pub const fn from_static_str(s: &'static str) -> Self {
        CheetahString {
            inner: InnerString::Static(s),
        }
    }

    /// Creates a `CheetahString` from a byte vector without validating UTF-8.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `s` contains valid UTF-8 for the entire
    /// lifetime of the returned `CheetahString`.
    #[inline]
    pub unsafe fn from_utf8_unchecked_vec(s: Vec<u8>) -> Self {
        CheetahString::from_validated_vec_unchecked(s)
    }

    #[inline]
    fn from_validated_vec_unchecked(s: Vec<u8>) -> Self {
        if s.len() <= INLINE_CAPACITY {
            // SAFETY: Callers validate UTF-8 before reaching this helper.
            let value = unsafe { str::from_utf8_unchecked(&s) };
            let inline = InlineStr::from_str(value).expect("short str must fit inline storage");
            return CheetahString {
                inner: InnerString::Inline(inline),
            };
        }

        // SAFETY: Callers validate UTF-8 before reaching this helper.
        CheetahString::from_string(unsafe { String::from_utf8_unchecked(s) })
    }

    /// Creates a `CheetahString` from a byte vector with UTF-8 validation.
    ///
    /// # Errors
    ///
    /// Returns an error if the bytes are not valid UTF-8.
    ///
    /// # Examples
    ///
    /// ```
    /// use cheetah_string::CheetahString;
    ///
    /// let bytes = vec![104, 101, 108, 108, 111]; // "hello"
    /// let s = CheetahString::try_from_vec(bytes).unwrap();
    /// assert_eq!(s, "hello");
    ///
    /// let invalid = vec![0xFF, 0xFE];
    /// assert!(CheetahString::try_from_vec(invalid).is_err());
    /// ```
    pub fn try_from_vec(v: Vec<u8>) -> Result<Self, Utf8Error> {
        str::from_utf8(&v)?;
        Ok(CheetahString::from_validated_vec_unchecked(v))
    }

    /// Creates a `CheetahString` from a byte slice with UTF-8 validation.
    ///
    /// # Errors
    ///
    /// Returns an error if the bytes are not valid UTF-8.
    ///
    /// # Examples
    ///
    /// ```
    /// use cheetah_string::CheetahString;
    ///
    /// let bytes = b"hello";
    /// let s = CheetahString::try_from_bytes(bytes).unwrap();
    /// assert_eq!(s, "hello");
    ///
    /// let invalid = &[0xFF, 0xFE];
    /// assert!(CheetahString::try_from_bytes(invalid).is_err());
    /// ```
    pub fn try_from_bytes(b: &[u8]) -> Result<Self, Utf8Error> {
        let s = str::from_utf8(b)?;
        Ok(CheetahString::from_slice(s))
    }

    /// Creates a `CheetahString` from a byte slice without validating UTF-8.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `b` contains valid UTF-8.
    #[inline]
    pub unsafe fn from_utf8_unchecked_bytes(b: &[u8]) -> Self {
        // SAFETY: The caller guarantees that `b` contains valid UTF-8.
        CheetahString::from_slice(unsafe { str::from_utf8_unchecked(b) })
    }

    /// Creates a `CheetahString` from a shared byte vector with UTF-8 validation.
    ///
    /// # Errors
    ///
    /// Returns an error if the bytes are not valid UTF-8.
    #[inline]
    pub fn try_from_arc_vec(s: Arc<Vec<u8>>) -> Result<Self, Utf8Error> {
        match Arc::try_unwrap(s) {
            Ok(v) => CheetahString::try_from_vec(v),
            Err(s) => {
                let s = str::from_utf8(s.as_slice())?;
                Ok(CheetahString::from_slice(s))
            }
        }
    }

    /// Creates a `CheetahString` from a shared byte vector without validating UTF-8.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `s` contains valid UTF-8.
    #[inline]
    pub unsafe fn from_utf8_unchecked_arc_vec(s: Arc<Vec<u8>>) -> Self {
        CheetahString::from_validated_arc_vec_unchecked(s)
    }

    #[inline]
    fn from_validated_arc_vec_unchecked(s: Arc<Vec<u8>>) -> Self {
        match Arc::try_unwrap(s) {
            Ok(v) => CheetahString::from_validated_vec_unchecked(v),
            Err(s) => {
                // SAFETY: Callers validate UTF-8 before reaching this helper.
                unsafe { CheetahString::from_utf8_unchecked_bytes(s.as_slice()) }
            }
        }
    }

    #[inline]
    pub fn from_slice(s: &str) -> Self {
        if let Some(inline) = InlineStr::from_str(s) {
            CheetahString {
                inner: InnerString::Inline(inline),
            }
        } else {
            // Use Arc<str> for long borrowed strings to avoid the extra String header.
            let arc_str: Arc<str> = Arc::from(s);
            CheetahString {
                inner: InnerString::Shared(arc_str),
            }
        }
    }

    #[inline]
    pub fn from_string(s: String) -> Self {
        CheetahString::from_string_shared(s)
    }

    /// Freezes an owned string into the canonical immutable representation.
    ///
    /// This compatibility constructor no longer promises to retain spare
    /// capacity. Use [`CheetahString::from_string`] for immutable values or
    /// [`crate::CheetahBuilder`] when mutation must continue.
    #[deprecated(since = "3.0.0", note = "use from_string(), CheetahBuilder, or String")]
    #[inline]
    pub fn from_string_owned(s: String) -> Self {
        Self::from_string(s)
    }

    /// Creates a `CheetahString` from an owned `String` using shared storage
    /// for long immutable strings.
    ///
    /// This is the same storage policy used by
    /// [`CheetahString::from_string`]. The explicit name is retained for
    /// source compatibility.
    #[inline]
    pub fn from_string_shared(s: String) -> Self {
        if let Some(inline) = InlineStr::from_str(&s) {
            CheetahString {
                inner: InnerString::Inline(inline),
            }
        } else {
            // Use Arc<str> for long strings to avoid double allocation
            let arc_str: Arc<str> = s.into_boxed_str().into();
            CheetahString {
                inner: InnerString::Shared(arc_str),
            }
        }
    }

    #[inline]
    pub fn from_arc_string(s: Arc<String>) -> Self {
        match Arc::try_unwrap(s) {
            Ok(s) => CheetahString::from_string(s),
            Err(s) => CheetahString::from_slice(s.as_str()),
        }
    }

    #[inline]
    #[cfg(feature = "bytes")]
    pub fn try_copy_from_bytes(b: bytes::Bytes) -> Result<Self, crate::FromUtf8BytesError> {
        match str::from_utf8(b.as_ref()) {
            Ok(value) => Ok(CheetahString::from_slice(value)),
            Err(error) => Err(crate::FromUtf8BytesError::new(b, error)),
        }
    }

    #[deprecated(
        since = "2.2.0",
        note = "use try_copy_from_bytes; its error preserves the original buffer"
    )]
    #[inline]
    #[cfg(feature = "bytes")]
    pub fn try_from_bytes_buf(b: bytes::Bytes) -> Result<Self, Utf8Error> {
        CheetahString::try_copy_from_bytes(b).map_err(|error| error.into_parts().1)
    }

    /// Creates a `CheetahString` from `bytes::Bytes` without validating UTF-8.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `b` contains valid UTF-8.
    #[inline]
    #[cfg(feature = "bytes")]
    pub unsafe fn from_utf8_unchecked_bytes_buf(b: bytes::Bytes) -> Self {
        CheetahString::from_validated_bytes_unchecked(b)
    }

    #[inline]
    #[cfg(feature = "bytes")]
    fn from_validated_bytes_unchecked(b: bytes::Bytes) -> Self {
        // SAFETY: Callers validate UTF-8 before reaching this helper.
        unsafe { CheetahString::from_utf8_unchecked_bytes(b.as_ref()) }
    }

    #[inline]
    pub fn as_str(&self) -> &str {
        match &self.inner {
            InnerString::Inline(inline) => inline.as_str(),
            InnerString::Static(s) => s,
            InnerString::Shared(s) => s.as_ref(),
        }
    }

    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        match &self.inner {
            InnerString::Inline(inline) => inline.as_bytes(),
            InnerString::Static(s) => s.as_bytes(),
            InnerString::Shared(s) => s.as_bytes(),
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        match &self.inner {
            InnerString::Inline(inline) => inline.len(),
            InnerString::Static(s) => s.len(),
            InnerString::Shared(s) => s.len(),
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        match &self.inner {
            InnerString::Inline(inline) => inline.is_empty(),
            InnerString::Static(s) => s.is_empty(),
            InnerString::Shared(s) => s.is_empty(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_borrowed_str_uses_shared_storage() {
        let value = "a".repeat(INLINE_CAPACITY + 1);
        let s = CheetahString::from_slice(&value);

        match &s.inner {
            InnerString::Shared(inner) => assert_eq!(inner.as_ref(), value.as_str()),
            other => panic!(
                "expected Shared for long borrowed input, got {:?}",
                core::mem::discriminant(other)
            ),
        }
    }

    #[test]
    fn try_from_vec_short_input_uses_inline_storage() {
        let s = CheetahString::try_from_vec(b"hello".to_vec()).expect("valid utf-8");

        match &s.inner {
            InnerString::Inline(inline) => {
                assert_eq!(inline.len(), 5);
                assert_eq!(inline.as_bytes(), b"hello");
            }
            other => panic!(
                "expected inline storage for short validated Vec<u8>, got {:?}",
                core::mem::discriminant(other)
            ),
        }
    }

    #[test]
    fn long_vec_conversion_uses_shared_storage() {
        let value = "a".repeat(INLINE_CAPACITY + 1).into_bytes();
        let s = CheetahString::try_from_vec(value).expect("valid utf-8");

        match &s.inner {
            InnerString::Shared(inner) => {
                assert_eq!(inner.len(), INLINE_CAPACITY + 1);
                assert!(inner.bytes().all(|byte| byte == b'a'));
            }
            other => panic!(
                "expected Shared for long Vec<u8> conversion, got {:?}",
                core::mem::discriminant(other)
            ),
        }
    }
}
