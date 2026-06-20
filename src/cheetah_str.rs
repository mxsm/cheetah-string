use alloc::borrow::Cow;
use alloc::string::{ParseError, String, ToString};
use alloc::sync::Arc;
use core::borrow::Borrow;
use core::cmp::Ordering;
use core::fmt;
use core::hash::{Hash, Hasher};
use core::ops::Deref;
use core::str::{self, FromStr};

use crate::CheetahString;

const INLINE_CAPACITY: usize = 23;

/// Immutable, clone-cheap string value for key/name/topic style workloads.
///
/// `CheetahStr` intentionally has no mutation API. Use [`CheetahBuilder`] or
/// [`CheetahString`] when the value is still being constructed.
///
/// [`CheetahBuilder`]: crate::CheetahBuilder
#[derive(Clone)]
pub struct CheetahStr {
    inner: Repr,
}

#[derive(Clone)]
enum Repr {
    Inline {
        len: u8,
        data: [u8; INLINE_CAPACITY],
    },
    Static(&'static str),
    Shared(Arc<str>),
}

impl CheetahStr {
    /// Creates an empty immutable string.
    #[inline]
    pub const fn empty() -> Self {
        Self {
            inner: Repr::Inline {
                len: 0,
                data: [0; INLINE_CAPACITY],
            },
        }
    }

    /// Creates an empty immutable string.
    #[inline]
    pub fn new() -> Self {
        Self::empty()
    }

    /// Creates a zero-copy immutable string from a static string slice.
    #[inline]
    pub const fn from_static_str(s: &'static str) -> Self {
        Self {
            inner: Repr::Static(s),
        }
    }

    /// Creates an immutable string from a borrowed string slice.
    #[inline]
    pub fn from_slice(s: &str) -> Self {
        if s.len() <= INLINE_CAPACITY {
            let mut data = [0u8; INLINE_CAPACITY];
            data[..s.len()].copy_from_slice(s.as_bytes());
            Self {
                inner: Repr::Inline {
                    len: s.len() as u8,
                    data,
                },
            }
        } else {
            Self {
                inner: Repr::Shared(Arc::from(s)),
            }
        }
    }

    /// Creates an immutable string from owned storage.
    #[inline]
    pub fn from_string(s: String) -> Self {
        if s.len() <= INLINE_CAPACITY {
            Self::from_slice(&s)
        } else {
            Self {
                inner: Repr::Shared(s.into_boxed_str().into()),
            }
        }
    }

    /// Returns the string slice.
    #[inline]
    pub fn as_str(&self) -> &str {
        match &self.inner {
            Repr::Inline { len, data } => {
                // SAFETY: Inline data is copied only from valid UTF-8 strings.
                unsafe { str::from_utf8_unchecked(&data[..*len as usize]) }
            }
            Repr::Static(s) => s,
            Repr::Shared(s) => s.as_ref(),
        }
    }

    /// Returns the UTF-8 bytes.
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        self.as_str().as_bytes()
    }

    /// Returns the byte length.
    #[inline]
    pub fn len(&self) -> usize {
        self.as_str().len()
    }

    /// Returns whether this string is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.as_str().is_empty()
    }
}

impl Default for CheetahStr {
    #[inline]
    fn default() -> Self {
        Self::empty()
    }
}

impl From<&str> for CheetahStr {
    #[inline]
    fn from(value: &str) -> Self {
        Self::from_slice(value)
    }
}

impl From<String> for CheetahStr {
    #[inline]
    fn from(value: String) -> Self {
        Self::from_string(value)
    }
}

impl From<Cow<'static, str>> for CheetahStr {
    #[inline]
    fn from(value: Cow<'static, str>) -> Self {
        match value {
            Cow::Borrowed(s) => Self::from_static_str(s),
            Cow::Owned(s) => Self::from_string(s),
        }
    }
}

impl From<&CheetahString> for CheetahStr {
    #[inline]
    fn from(value: &CheetahString) -> Self {
        Self::from_slice(value.as_str())
    }
}

impl From<CheetahString> for CheetahStr {
    #[inline]
    fn from(value: CheetahString) -> Self {
        Self::from_string(String::from(value))
    }
}

impl From<CheetahStr> for String {
    #[inline]
    fn from(value: CheetahStr) -> Self {
        value.as_str().to_string()
    }
}

impl FromStr for CheetahStr {
    type Err = ParseError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::from_slice(s))
    }
}

impl Deref for CheetahStr {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl AsRef<str> for CheetahStr {
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl AsRef<[u8]> for CheetahStr {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl Borrow<str> for CheetahStr {
    #[inline]
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for CheetahStr {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_str().fmt(f)
    }
}

impl fmt::Debug for CheetahStr {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), f)
    }
}

impl Hash for CheetahStr {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}

impl PartialEq for CheetahStr {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl PartialEq<str> for CheetahStr {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<&str> for CheetahStr {
    #[inline]
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<CheetahStr> for str {
    #[inline]
    fn eq(&self, other: &CheetahStr) -> bool {
        self == other.as_str()
    }
}

impl PartialEq<CheetahStr> for &str {
    #[inline]
    fn eq(&self, other: &CheetahStr) -> bool {
        *self == other.as_str()
    }
}

impl Eq for CheetahStr {}

impl PartialOrd for CheetahStr {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CheetahStr {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_str().cmp(other.as_str())
    }
}
