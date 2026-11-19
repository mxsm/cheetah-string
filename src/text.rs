//! UTF-8 text backed by `bytes::Bytes`.

use alloc::boxed::Box;
use alloc::string::String;
use core::borrow::Borrow;
use core::cmp::Ordering;
use core::fmt;
use core::hash::{Hash, Hasher};
use core::ops::{Bound, Deref, RangeBounds};
use core::str;

use bytes::Bytes;

use crate::{CheetahBytes, CheetahString, FromUtf8BytesError};

/// Immutable UTF-8 text backed by [`bytes::Bytes`].
///
/// `CheetahText` is the zero-copy companion of [`CheetahString`] for text that
/// arrives in network buffers. Validating a `Bytes` buffer keeps its storage,
/// and [`slice`](Self::slice) and [`slice_ref`](Self::slice_ref) return owned
/// sub-strings that share it, so a decoder can hand out keys and values
/// without copying them. Cloning only adjusts the buffer's reference count.
///
/// A `CheetahText` is 32 bytes, the size of `Bytes`, and never stores text
/// inline. A slice keeps its whole underlying buffer alive, so convert values
/// that outlive the buffer with [`to_cheetah_string`](Self::to_cheetah_string).
///
/// Every slice and every drop updates the buffer's atomic reference count.
/// Values of up to 23 bytes are therefore usually cheaper to copy into an
/// inline [`CheetahString`]; share the longer ones. Decoding message
/// properties into `CheetahString` keys and `CheetahText` values follows that
/// rule.
///
/// Equality, ordering, and hashing match `str`, so a
/// `HashMap<CheetahText, _>` can be queried with `&str`.
///
/// ```
/// use bytes::Bytes;
/// use cheetah_string::CheetahText;
///
/// let frame = Bytes::from_static(b"topic=orders;group=billing");
/// let text = CheetahText::try_from_bytes(frame).unwrap();
///
/// let (key, value) = text.split_once('=').unwrap();
/// assert_eq!(key, "topic");
/// let topic = text.slice_ref(&value[..6]);
/// assert_eq!(topic, "orders");
/// assert_eq!(topic.as_bytes().as_ptr(), text[6..].as_ptr());
/// ```
#[derive(Clone, Default)]
pub struct CheetahText {
    bytes: Bytes,
}

impl CheetahText {
    /// Creates empty text.
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self {
            bytes: Bytes::new(),
        }
    }

    /// Wraps a static string without allocating or copying.
    #[inline]
    #[must_use]
    pub const fn from_static(text: &'static str) -> Self {
        Self {
            bytes: Bytes::from_static(text.as_bytes()),
        }
    }

    /// Validates `bytes` as UTF-8 and keeps its storage without copying.
    ///
    /// # Errors
    ///
    /// Returns the original buffer together with the validation error when
    /// `bytes` is not valid UTF-8.
    #[inline]
    pub fn try_from_bytes(bytes: Bytes) -> Result<Self, FromUtf8BytesError> {
        match str::from_utf8(&bytes) {
            Ok(_) => Ok(Self { bytes }),
            Err(error) => Err(FromUtf8BytesError::new(bytes, error)),
        }
    }

    /// Wraps `bytes` without validating UTF-8.
    ///
    /// # Safety
    ///
    /// `bytes` must contain valid UTF-8.
    #[inline]
    #[must_use]
    pub unsafe fn from_bytes_unchecked(bytes: Bytes) -> Self {
        Self { bytes }
    }

    /// Copies `text` into a new buffer.
    #[inline]
    #[must_use]
    pub fn copy_from_str(text: &str) -> Self {
        Self {
            bytes: Bytes::copy_from_slice(text.as_bytes()),
        }
    }

    /// Borrows the text.
    #[inline]
    #[must_use]
    pub fn as_str(&self) -> &str {
        // SAFETY: every constructor validates UTF-8 or requires the caller to
        // guarantee it, and slicing only happens on character boundaries or
        // at the bounds of a `&str` that already lies inside the buffer.
        unsafe { str::from_utf8_unchecked(&self.bytes) }
    }

    /// Borrows the UTF-8 encoding.
    #[inline]
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Borrows the backing buffer.
    #[inline]
    #[must_use]
    pub fn bytes(&self) -> &Bytes {
        &self.bytes
    }

    /// Returns the length in bytes.
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    /// Returns `true` when the text is empty.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// Returns the backing buffer without copying.
    #[inline]
    #[must_use]
    pub fn into_bytes(self) -> Bytes {
        self.bytes
    }

    /// Returns the text in `range` as a new value that shares this buffer.
    ///
    /// # Panics
    ///
    /// Panics if the range is out of bounds, inverted, or not on character
    /// boundaries. Use [`CheetahText::try_slice`] for a recoverable error.
    #[inline]
    #[must_use]
    pub fn slice(&self, range: impl RangeBounds<usize>) -> Self {
        self.try_slice(range)
            .expect("slice range must be in bounds and on UTF-8 character boundaries")
    }

    /// Returns the text in `range` as a new value that shares this buffer, or
    /// an error when the range is invalid.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::InvalidRange`] for an inverted range,
    /// [`crate::Error::IndexOutOfBounds`] past the end, and
    /// [`crate::Error::InvalidCharBoundary`] inside a character.
    pub fn try_slice(&self, range: impl RangeBounds<usize>) -> crate::Result<Self> {
        let len = self.len();
        let start = match range.start_bound() {
            Bound::Included(&start) => start,
            Bound::Excluded(&start) => start
                .checked_add(1)
                .ok_or(crate::Error::IndexOutOfBounds { index: start, len })?,
            Bound::Unbounded => 0,
        };
        let end = match range.end_bound() {
            Bound::Included(&end) => end
                .checked_add(1)
                .ok_or(crate::Error::IndexOutOfBounds { index: end, len })?,
            Bound::Excluded(&end) => end,
            Bound::Unbounded => len,
        };

        if start > end {
            return Err(crate::Error::InvalidRange { start, end });
        }
        if end > len {
            return Err(crate::Error::IndexOutOfBounds { index: end, len });
        }
        let text = self.as_str();
        if !text.is_char_boundary(start) {
            return Err(crate::Error::InvalidCharBoundary { index: start });
        }
        if !text.is_char_boundary(end) {
            return Err(crate::Error::InvalidCharBoundary { index: end });
        }

        Ok(Self {
            bytes: self.bytes.slice(start..end),
        })
    }

    /// Returns `subset`, a `&str` borrowed from this text, as a value that
    /// shares this buffer.
    ///
    /// This turns the results of `str` methods such as `split`, `trim`, or
    /// `split_once` into owned values without copying.
    ///
    /// # Panics
    ///
    /// Panics if a non-empty `subset` does not lie within this text.
    #[inline]
    #[must_use]
    pub fn slice_ref(&self, subset: &str) -> Self {
        // A `&str` is valid UTF-8 on its own, so any in-bounds `&str` names a
        // valid UTF-8 range of the buffer.
        Self {
            bytes: self.bytes.slice_ref(subset.as_bytes()),
        }
    }

    /// Copies the text into a [`CheetahString`].
    ///
    /// Text of up to 23 bytes is stored inline without allocating, which
    /// also releases the dependency on this buffer.
    #[inline]
    #[must_use]
    pub fn to_cheetah_string(&self) -> CheetahString {
        CheetahString::from_slice(self.as_str())
    }
}

impl Deref for CheetahText {
    type Target = str;

    #[inline]
    fn deref(&self) -> &str {
        self.as_str()
    }
}

impl AsRef<str> for CheetahText {
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl AsRef<[u8]> for CheetahText {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl Borrow<str> for CheetahText {
    #[inline]
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl PartialEq for CheetahText {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        crate::compare::bytes_eq(self.as_bytes(), other.as_bytes())
    }
}

impl Eq for CheetahText {}

impl PartialOrd for CheetahText {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CheetahText {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl Hash for CheetahText {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Hash as `str`, not as `Bytes`, to stay consistent with `Borrow<str>`.
        self.as_str().hash(state);
    }
}

macro_rules! impl_text_eq {
    ($($other:ty),*) => {$(
        impl PartialEq<$other> for CheetahText {
            #[inline]
            fn eq(&self, other: &$other) -> bool {
                crate::compare::bytes_eq(self.as_bytes(), AsRef::<str>::as_ref(other).as_bytes())
            }
        }

        impl PartialEq<CheetahText> for $other {
            #[inline]
            fn eq(&self, other: &CheetahText) -> bool {
                crate::compare::bytes_eq(AsRef::<str>::as_ref(self).as_bytes(), other.as_bytes())
            }
        }
    )*};
}

impl_text_eq!(str, &str, String, CheetahString);

impl fmt::Display for CheetahText {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.as_str(), f)
    }
}

impl fmt::Debug for CheetahText {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), f)
    }
}

impl From<&'static str> for CheetahText {
    #[inline]
    fn from(text: &'static str) -> Self {
        Self::from_static(text)
    }
}

impl From<String> for CheetahText {
    /// Adopts the `String` allocation without copying.
    #[inline]
    fn from(text: String) -> Self {
        Self {
            bytes: Bytes::from(text),
        }
    }
}

impl From<Box<str>> for CheetahText {
    /// Adopts the boxed allocation without copying.
    #[inline]
    fn from(text: Box<str>) -> Self {
        Self {
            bytes: Bytes::from(text.into_boxed_bytes()),
        }
    }
}

impl TryFrom<Bytes> for CheetahText {
    type Error = FromUtf8BytesError;

    #[inline]
    fn try_from(bytes: Bytes) -> Result<Self, Self::Error> {
        Self::try_from_bytes(bytes)
    }
}

impl TryFrom<CheetahBytes> for CheetahText {
    type Error = FromUtf8BytesError;

    #[inline]
    fn try_from(bytes: CheetahBytes) -> Result<Self, Self::Error> {
        Self::try_from_bytes(bytes.into_bytes())
    }
}

impl From<CheetahText> for Bytes {
    #[inline]
    fn from(text: CheetahText) -> Self {
        text.bytes
    }
}

impl From<CheetahText> for CheetahBytes {
    #[inline]
    fn from(text: CheetahText) -> Self {
        CheetahBytes::from_bytes(text.bytes)
    }
}

impl From<&CheetahText> for CheetahString {
    /// Copies the text; see [`CheetahText::to_cheetah_string`].
    #[inline]
    fn from(text: &CheetahText) -> Self {
        text.to_cheetah_string()
    }
}

impl From<CheetahText> for CheetahString {
    /// Copies the text; see [`CheetahText::to_cheetah_string`].
    #[inline]
    fn from(text: CheetahText) -> Self {
        text.to_cheetah_string()
    }
}

#[cfg(feature = "serde")]
mod serde_impl {
    use super::CheetahText;
    use alloc::string::String;
    use alloc::vec::Vec;
    use core::fmt;
    use serde::de::{Error, Visitor};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    impl Serialize for CheetahText {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            serializer.serialize_str(self.as_str())
        }
    }

    impl<'de> Deserialize<'de> for CheetahText {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            struct TextVisitor;

            impl Visitor<'_> for TextVisitor {
                type Value = CheetahText;

                fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                    formatter.write_str("a string")
                }

                fn visit_str<E: Error>(self, value: &str) -> Result<Self::Value, E> {
                    Ok(CheetahText::copy_from_str(value))
                }

                fn visit_string<E: Error>(self, value: String) -> Result<Self::Value, E> {
                    Ok(CheetahText::from(value))
                }

                fn visit_bytes<E: Error>(self, value: &[u8]) -> Result<Self::Value, E> {
                    core::str::from_utf8(value)
                        .map(CheetahText::copy_from_str)
                        .map_err(E::custom)
                }

                fn visit_byte_buf<E: Error>(self, value: Vec<u8>) -> Result<Self::Value, E> {
                    String::from_utf8(value)
                        .map(CheetahText::from)
                        .map_err(E::custom)
                }
            }

            deserializer.deserialize_string(TextVisitor)
        }
    }
}
