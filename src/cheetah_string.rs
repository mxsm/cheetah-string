use core::fmt;
use core::str::Utf8Error;
use std::borrow::{Borrow, Cow};
use std::cmp::Ordering;
use std::fmt::Display;
use std::hash::Hash;
use std::ops::Deref;
use std::str::FromStr;
use std::sync::Arc;

pub const EMPTY_STRING: &str = "";

#[derive(Clone)]
#[repr(transparent)]
pub struct CheetahString {
    pub(super) inner: InnerString,
}

impl Default for CheetahString {
    fn default() -> Self {
        CheetahString {
            inner: InnerString::Empty,
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

/// # Safety Warning
///
/// This implementation uses `unsafe` code and may cause undefined behavior
/// if the bytes are not valid UTF-8. Consider using `CheetahString::try_from_bytes()`
/// for safe UTF-8 validation.
///
/// This implementation will be deprecated in a future version.
impl From<&[u8]> for CheetahString {
    #[inline]
    fn from(b: &[u8]) -> Self {
        // SAFETY: This is unsafe and may cause UB if bytes are not valid UTF-8.
        // This will be deprecated in favor of try_from_bytes in the next version.
        CheetahString::from_slice(unsafe { std::str::from_utf8_unchecked(b) })
    }
}

impl FromStr for CheetahString {
    type Err = std::string::ParseError;
    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(CheetahString::from_slice(s))
    }
}

/// # Safety Warning
///
/// This implementation uses `unsafe` code and may cause undefined behavior
/// if the bytes are not valid UTF-8. Consider using `CheetahString::try_from_vec()`
/// for safe UTF-8 validation.
///
/// This implementation will be deprecated in a future version.
impl From<Vec<u8>> for CheetahString {
    #[inline]
    fn from(v: Vec<u8>) -> Self {
        // SAFETY: This is unsafe and may cause UB if bytes are not valid UTF-8.
        // This will be deprecated in favor of try_from_vec in the next version.
        CheetahString::from_slice(unsafe { std::str::from_utf8_unchecked(&v) })
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
impl From<bytes::Bytes> for CheetahString {
    #[inline]
    fn from(b: bytes::Bytes) -> Self {
        CheetahString::from_bytes(b)
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
                inner: InnerString::ArcString(s),
            } => s.as_ref().clone(),
            CheetahString {
                inner: InnerString::StaticStr(s),
            } => s.to_string(),
            CheetahString {
                inner: InnerString::ArcVecString(s),
            } => {
                // SAFETY: ArcVecString should only be created from valid UTF-8 sources
                unsafe { String::from_utf8_unchecked(s.to_vec()) }
            }
            #[cfg(feature = "bytes")]
            CheetahString {
                inner: InnerString::Bytes(b),
            } => {
                // SAFETY: Bytes variant should only be created from valid UTF-8 sources
                unsafe { String::from_utf8_unchecked(b.to_vec()) }
            }
            CheetahString {
                inner: InnerString::Empty,
            } => String::new(),
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

impl CheetahString {
    #[inline]
    pub const fn empty() -> Self {
        CheetahString {
            inner: InnerString::Empty,
        }
    }

    #[inline]
    pub fn new() -> Self {
        CheetahString::default()
    }

    #[inline]
    pub const fn from_static_str(s: &'static str) -> Self {
        CheetahString {
            inner: InnerString::StaticStr(s),
        }
    }

    #[inline]
    pub fn from_vec(s: Vec<u8>) -> Self {
        CheetahString {
            inner: InnerString::ArcVecString(Arc::new(s)),
        }
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
        // Validate UTF-8
        std::str::from_utf8(&v)?;
        Ok(CheetahString {
            inner: InnerString::ArcVecString(Arc::new(v)),
        })
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
        let s = std::str::from_utf8(b)?;
        Ok(CheetahString::from_slice(s))
    }

    #[inline]
    pub fn from_arc_vec(s: Arc<Vec<u8>>) -> Self {
        CheetahString {
            inner: InnerString::ArcVecString(s),
        }
    }

    #[inline]
    pub fn from_slice(s: &str) -> Self {
        CheetahString {
            inner: InnerString::ArcString(Arc::new(s.to_owned())),
        }
    }

    #[inline]
    pub fn from_string(s: String) -> Self {
        CheetahString {
            inner: InnerString::ArcString(Arc::new(s)),
        }
    }
    #[inline]
    pub fn from_arc_string(s: Arc<String>) -> Self {
        CheetahString {
            inner: InnerString::ArcString(s),
        }
    }

    #[inline]
    #[cfg(feature = "bytes")]
    pub fn from_bytes(b: bytes::Bytes) -> Self {
        CheetahString {
            inner: InnerString::Bytes(b),
        }
    }

    #[inline]
    pub fn as_str(&self) -> &str {
        match &self.inner {
            InnerString::ArcString(s) => s.as_str(),
            InnerString::StaticStr(s) => s,
            InnerString::ArcVecString(s) => {
                // SAFETY: ArcVecString is only created from validated UTF-8 sources.
                // All constructors ensure this invariant is maintained.
                unsafe { std::str::from_utf8_unchecked(s.as_ref()) }
            }
            #[cfg(feature = "bytes")]
            InnerString::Bytes(b) => {
                // SAFETY: Bytes variant is only created from validated UTF-8 sources.
                // The from_bytes constructor ensures this invariant.
                unsafe { std::str::from_utf8_unchecked(b.as_ref()) }
            }
            InnerString::Empty => EMPTY_STRING,
        }
    }

    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        match &self.inner {
            InnerString::ArcString(s) => s.as_bytes(),
            InnerString::StaticStr(s) => s.as_bytes(),
            InnerString::ArcVecString(s) => s.as_ref(),
            #[cfg(feature = "bytes")]
            InnerString::Bytes(b) => b.as_ref(),
            InnerString::Empty => &[],
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        match &self.inner {
            InnerString::ArcString(s) => s.len(),
            InnerString::StaticStr(s) => s.len(),
            InnerString::ArcVecString(s) => s.len(),
            #[cfg(feature = "bytes")]
            InnerString::Bytes(b) => b.len(),
            InnerString::Empty => 0,
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        match &self.inner {
            InnerString::ArcString(s) => s.is_empty(),
            InnerString::StaticStr(s) => s.is_empty(),
            InnerString::ArcVecString(s) => s.is_empty(),
            #[cfg(feature = "bytes")]
            InnerString::Bytes(b) => b.is_empty(),
            InnerString::Empty => true,
        }
    }
}

impl PartialEq for CheetahString {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl PartialEq<str> for CheetahString {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<String> for CheetahString {
    #[inline]
    fn eq(&self, other: &String) -> bool {
        self.as_str() == other.as_str()
    }
}

impl PartialEq<Vec<u8>> for CheetahString {
    #[inline]
    fn eq(&self, other: &Vec<u8>) -> bool {
        self.as_bytes() == other.as_slice()
    }
}

impl<'a> PartialEq<&'a str> for CheetahString {
    #[inline]
    fn eq(&self, other: &&'a str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<CheetahString> for str {
    #[inline]
    fn eq(&self, other: &CheetahString) -> bool {
        self == other.as_str()
    }
}

impl PartialEq<CheetahString> for String {
    #[inline]
    fn eq(&self, other: &CheetahString) -> bool {
        self.as_str() == other.as_str()
    }
}

impl PartialEq<CheetahString> for &str {
    #[inline]
    fn eq(&self, other: &CheetahString) -> bool {
        *self == other.as_str()
    }
}

impl Eq for CheetahString {}

impl PartialOrd for CheetahString {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CheetahString {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl Hash for CheetahString {
    #[inline]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}

impl Display for CheetahString {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}

impl std::fmt::Debug for CheetahString {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt::Debug::fmt(self.as_str(), f)
    }
}

impl Borrow<str> for CheetahString {
    #[inline]
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

/// The `InnerString` enum represents different types of string storage.
///
/// Variants:
///
/// * `ArcString(Arc<String>)` - A reference-counted string.
/// * `StaticStr(&'static str)` - A static string slice.
/// * `Bytes(bytes::Bytes)` - A byte buffer (available when the "bytes" feature is enabled).
/// * `Empty` - An empty string.
#[derive(Clone)]
pub(super) enum InnerString {
    ArcString(Arc<String>),
    StaticStr(&'static str),
    ArcVecString(Arc<Vec<u8>>),
    #[cfg(feature = "bytes")]
    Bytes(bytes::Bytes),
    Empty,
}
