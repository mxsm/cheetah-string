use crate::CheetahString;
use alloc::vec::Vec;
use core::fmt;
use core::ops::Deref;
use core::str::Utf8Error;

/// Error returned when a `bytes::Bytes` buffer is not valid UTF-8.
///
/// Unlike `Utf8Error` alone, this error owns the original buffer so callers
/// can recover, log, or route the exact bytes without copying them again.
#[derive(Clone, Debug)]
pub struct FromUtf8BytesError {
    bytes: ::bytes::Bytes,
    error: Utf8Error,
}

impl FromUtf8BytesError {
    pub(crate) fn new(bytes: ::bytes::Bytes, error: Utf8Error) -> Self {
        Self { bytes, error }
    }

    /// Returns the UTF-8 validation error.
    #[inline]
    pub fn utf8_error(&self) -> Utf8Error {
        self.error
    }

    /// Borrows the original byte buffer.
    #[inline]
    pub fn bytes(&self) -> &::bytes::Bytes {
        &self.bytes
    }

    /// Recovers the original byte buffer without copying.
    #[inline]
    pub fn into_bytes(self) -> ::bytes::Bytes {
        self.bytes
    }

    /// Recovers both the original buffer and its validation error.
    #[inline]
    pub fn into_parts(self) -> (::bytes::Bytes, Utf8Error) {
        (self.bytes, self.error)
    }
}

impl fmt::Display for FromUtf8BytesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid UTF-8 byte buffer: {}", self.error)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for FromUtf8BytesError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}

/// Byte-oriented companion type for `CheetahString`.
///
/// `CheetahBytes` does not promise UTF-8 and never dereferences to `str`.
/// Convert it to `CheetahString` with `TryFrom` or an explicit unsafe method.
#[derive(Clone, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct CheetahBytes {
    inner: ::bytes::Bytes,
}

impl CheetahBytes {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn from_static(bytes: &'static [u8]) -> Self {
        Self {
            inner: ::bytes::Bytes::from_static(bytes),
        }
    }

    #[inline]
    pub fn from_vec(bytes: Vec<u8>) -> Self {
        Self {
            inner: ::bytes::Bytes::from(bytes),
        }
    }

    #[inline]
    pub fn from_bytes(bytes: ::bytes::Bytes) -> Self {
        Self { inner: bytes }
    }

    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        self.inner.as_ref()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    #[inline]
    pub fn into_bytes(self) -> ::bytes::Bytes {
        self.inner
    }

    #[deprecated(
        since = "2.2.0",
        note = "use try_copy_to_cheetah_string for a borrowed copy, or CheetahString::try_copy_from_bytes(self.into_bytes()) to recover invalid input"
    )]
    #[inline]
    pub fn try_into_string(self) -> Result<CheetahString, Utf8Error> {
        CheetahString::try_copy_from_bytes(self.inner).map_err(|error| error.into_parts().1)
    }

    /// Validates UTF-8 and copies this buffer into a `CheetahString`.
    ///
    /// The `CheetahBytes` value remains available regardless of success.
    #[inline]
    pub fn try_copy_to_cheetah_string(&self) -> Result<CheetahString, Utf8Error> {
        CheetahString::try_from_bytes(self.as_bytes())
    }

    /// Converts bytes into `CheetahString` without validating UTF-8.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that the bytes contain valid UTF-8.
    #[inline]
    pub unsafe fn into_string_unchecked(self) -> CheetahString {
        // SAFETY: The caller guarantees valid UTF-8.
        unsafe { CheetahString::from_utf8_unchecked_bytes_buf(self.inner) }
    }
}

impl AsRef<[u8]> for CheetahBytes {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl Deref for CheetahBytes {
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_bytes()
    }
}

impl fmt::Debug for CheetahBytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("CheetahBytes")
            .field(&self.as_bytes())
            .finish()
    }
}

impl From<Vec<u8>> for CheetahBytes {
    #[inline]
    fn from(bytes: Vec<u8>) -> Self {
        Self::from_vec(bytes)
    }
}

impl From<&'static [u8]> for CheetahBytes {
    #[inline]
    fn from(bytes: &'static [u8]) -> Self {
        Self::from_static(bytes)
    }
}

impl From<::bytes::Bytes> for CheetahBytes {
    #[inline]
    fn from(bytes: ::bytes::Bytes) -> Self {
        Self::from_bytes(bytes)
    }
}

impl From<CheetahBytes> for ::bytes::Bytes {
    #[inline]
    fn from(bytes: CheetahBytes) -> Self {
        bytes.into_bytes()
    }
}

impl TryFrom<CheetahBytes> for CheetahString {
    type Error = FromUtf8BytesError;

    #[inline]
    fn try_from(bytes: CheetahBytes) -> Result<Self, Self::Error> {
        CheetahString::try_copy_from_bytes(bytes.into_bytes())
    }
}

impl TryFrom<&CheetahBytes> for CheetahString {
    type Error = Utf8Error;

    #[inline]
    fn try_from(bytes: &CheetahBytes) -> Result<Self, Self::Error> {
        bytes.try_copy_to_cheetah_string()
    }
}

#[cfg(feature = "serde")]
mod serde_impl {
    use super::CheetahBytes;
    use alloc::vec::Vec;
    use core::fmt;
    use serde::de::{Error, SeqAccess, Visitor};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    impl Serialize for CheetahBytes {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            serializer.serialize_bytes(self.as_bytes())
        }
    }

    impl<'de> Deserialize<'de> for CheetahBytes {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            struct CheetahBytesVisitor;

            impl<'de> Visitor<'de> for CheetahBytesVisitor {
                type Value = CheetahBytes;

                fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                    formatter.write_str("a byte buffer")
                }

                fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
                where
                    E: Error,
                {
                    Ok(CheetahBytes::from_vec(v.to_vec()))
                }

                fn visit_borrowed_bytes<E>(self, v: &'de [u8]) -> Result<Self::Value, E>
                where
                    E: Error,
                {
                    Ok(CheetahBytes::from_vec(v.to_vec()))
                }

                fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
                where
                    E: Error,
                {
                    Ok(CheetahBytes::from_vec(v))
                }

                fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
                where
                    E: Error,
                {
                    Ok(CheetahBytes::from_vec(v.as_bytes().to_vec()))
                }

                fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
                where
                    A: SeqAccess<'de>,
                {
                    let mut bytes = Vec::<u8>::with_capacity(seq.size_hint().unwrap_or(0));
                    while let Some(byte) = seq.next_element()? {
                        bytes.push(byte);
                    }
                    Ok(CheetahBytes::from_vec(bytes))
                }
            }

            deserializer.deserialize_byte_buf(CheetahBytesVisitor)
        }
    }
}
