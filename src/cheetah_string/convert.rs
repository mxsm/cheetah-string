use alloc::borrow::Cow;
use alloc::string::{ParseError, String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::ops::Deref;
use core::str::{FromStr, Utf8Error};

use super::repr::InnerString;
use super::CheetahString;
use crate::inline::InlineStr;

impl Default for CheetahString {
    fn default() -> Self {
        CheetahString {
            inner: InnerString::Inline(InlineStr::empty()),
        }
    }
}

impl From<String> for CheetahString {
    #[inline]
    fn from(s: String) -> Self {
        CheetahString::from_string(s)
    }
}

impl From<Arc<String>> for CheetahString {
    #[inline]
    fn from(s: Arc<String>) -> Self {
        CheetahString::from_arc_string(s)
    }
}

impl<'a> From<&'a str> for CheetahString {
    #[inline]
    fn from(s: &'a str) -> Self {
        CheetahString::from_slice(s)
    }
}

impl<'a> TryFrom<&'a [u8]> for CheetahString {
    type Error = Utf8Error;

    #[inline]
    fn try_from(b: &'a [u8]) -> Result<Self, Self::Error> {
        CheetahString::try_from_bytes(b)
    }
}

impl FromStr for CheetahString {
    type Err = ParseError;
    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(CheetahString::from_slice(s))
    }
}

impl TryFrom<Vec<u8>> for CheetahString {
    type Error = Utf8Error;

    #[inline]
    fn try_from(v: Vec<u8>) -> Result<Self, Self::Error> {
        CheetahString::try_from_vec(v)
    }
}

impl From<Cow<'static, str>> for CheetahString {
    #[inline]
    fn from(cow: Cow<'static, str>) -> Self {
        match cow {
            Cow::Borrowed(s) => CheetahString::from_static_str(s),
            Cow::Owned(s) => CheetahString::from_string(s),
        }
    }
}

impl From<Cow<'_, String>> for CheetahString {
    #[inline]
    fn from(cow: Cow<'_, String>) -> Self {
        match cow {
            Cow::Borrowed(s) => CheetahString::from_slice(s),
            Cow::Owned(s) => CheetahString::from_string(s),
        }
    }
}

impl From<char> for CheetahString {
    /// Allocates an owned [`CheetahString`] from a single character.
    ///
    /// # Example
    /// ```rust
    /// use cheetah_string::CheetahString;
    /// let c: char = 'a';
    /// let s: CheetahString = CheetahString::from(c);
    /// assert_eq!("a", &s[..]);
    /// ```
    #[inline]
    fn from(c: char) -> Self {
        CheetahString::from_string(c.to_string())
    }
}

impl<'a> FromIterator<&'a char> for CheetahString {
    #[inline]
    fn from_iter<T: IntoIterator<Item = &'a char>>(iter: T) -> CheetahString {
        let mut buf = String::new();
        buf.extend(iter);
        CheetahString::from_string(buf)
    }
}

impl<'a> FromIterator<&'a str> for CheetahString {
    fn from_iter<I: IntoIterator<Item = &'a str>>(iter: I) -> CheetahString {
        let mut buf = String::new();
        buf.extend(iter);
        CheetahString::from_string(buf)
    }
}

impl FromIterator<String> for CheetahString {
    #[inline]
    fn from_iter<T: IntoIterator<Item = String>>(iter: T) -> Self {
        let mut buf = String::new();
        buf.extend(iter);
        CheetahString::from_string(buf)
    }
}

impl<'a> FromIterator<&'a String> for CheetahString {
    #[inline]
    fn from_iter<T: IntoIterator<Item = &'a String>>(iter: T) -> Self {
        let mut buf = String::new();
        buf.extend(iter.into_iter().map(|s| s.as_str()));
        CheetahString::from_string(buf)
    }
}

#[cfg(feature = "bytes")]
impl TryFrom<bytes::Bytes> for CheetahString {
    type Error = crate::FromUtf8BytesError;

    #[inline]
    fn try_from(b: bytes::Bytes) -> Result<Self, Self::Error> {
        CheetahString::try_copy_from_bytes(b)
    }
}

impl From<&CheetahString> for CheetahString {
    #[inline]
    fn from(s: &CheetahString) -> Self {
        s.clone()
    }
}

impl From<CheetahString> for String {
    #[inline]
    fn from(s: CheetahString) -> Self {
        match s {
            CheetahString {
                inner: InnerString::Inline(inline),
            } => inline.into_string(),
            CheetahString {
                inner: InnerString::Static(s),
            } => s.to_string(),
            CheetahString {
                inner: InnerString::Shared(s),
            } => s.to_string(),
        }
    }
}

impl Deref for CheetahString {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl AsRef<str> for CheetahString {
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl AsRef<[u8]> for CheetahString {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl AsRef<CheetahString> for CheetahString {
    #[inline]
    fn as_ref(&self) -> &CheetahString {
        self
    }
}

impl From<&String> for CheetahString {
    #[inline]
    fn from(s: &String) -> Self {
        CheetahString::from_slice(s)
    }
}
