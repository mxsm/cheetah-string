use std::cmp::Ordering;
use std::fmt::Display;
use std::hash::Hash;
use std::ops::Deref;
use std::sync::Arc;

const EMPTY_STRING: &str = "";

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
    fn from(s: String) -> Self {
        CheetahString::from_string(s)
    }
}

impl From<Arc<String>> for CheetahString {
    fn from(s: Arc<String>) -> Self {
        CheetahString::from_arc_string(s)
    }
}

impl<'a> From<&'a str> for CheetahString {
    fn from(s: &'a str) -> Self {
        CheetahString::from_slice(s)
    }
}

impl From<&[u8]> for CheetahString {
    fn from(b: &[u8]) -> Self {
        CheetahString::from_slice(unsafe { std::str::from_utf8_unchecked(b) })
    }
}

#[cfg(feature = "bytes")]
impl From<bytes::Bytes> for CheetahString {
    fn from(b: bytes::Bytes) -> Self {
        CheetahString::from_bytes(b)
    }
}

impl From<CheetahString> for String {
    fn from(s: CheetahString) -> Self {
        match s {
            CheetahString {
                inner: InnerString::ArcString(s),
            } => s.as_ref().clone(),
            CheetahString {
                inner: InnerString::StaticStr(s),
            } => s.to_string(),
            #[cfg(feature = "bytes")]
            CheetahString {
                inner: InnerString::Bytes(b),
            } => unsafe { String::from_utf8_unchecked(b.to_vec()) },
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
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl AsRef<[u8]> for CheetahString {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl CheetahString {
    #[inline]
    pub fn empty() -> Self {
        CheetahString {
            inner: InnerString::Empty,
        }
    }

    #[inline]
    pub fn new() -> Self {
        CheetahString::default()
    }

    #[inline]
    pub fn from_static_str(s: &'static str) -> Self {
        CheetahString {
            inner: InnerString::StaticStr(s),
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
            #[cfg(feature = "bytes")]
            InnerString::Bytes(b) => std::str::from_utf8(b.as_ref()).unwrap(),
            InnerString::Empty => EMPTY_STRING,
        }
    }

    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        match &self.inner {
            InnerString::ArcString(s) => s.as_bytes(),
            InnerString::StaticStr(s) => s.as_bytes(),
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
            #[cfg(feature = "bytes")]
            InnerString::Bytes(b) => b.is_empty(),
            InnerString::Empty => true,
        }
    }
}

impl PartialEq for CheetahString {
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl PartialEq<str> for CheetahString {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<String> for CheetahString {
    fn eq(&self, other: &String) -> bool {
        self.as_str() == other.as_str()
    }
}

impl<'a> PartialEq<&'a str> for CheetahString {
    fn eq(&self, other: &&'a str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<CheetahString> for str {
    fn eq(&self, other: &CheetahString) -> bool {
        self == other.as_str()
    }
}

impl PartialEq<CheetahString> for String {
    fn eq(&self, other: &CheetahString) -> bool {
        self.as_str() == other.as_str()
    }
}

impl PartialEq<CheetahString> for &str {
    fn eq(&self, other: &CheetahString) -> bool {
        *self == other.as_str()
    }
}

impl Eq for CheetahString {}

impl PartialOrd for CheetahString {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CheetahString {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl Hash for CheetahString {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}

impl Display for CheetahString {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}

impl std::fmt::Debug for CheetahString {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.as_str().fmt(f)
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
    #[cfg(feature = "bytes")]
    Bytes(bytes::Bytes),
    Empty,
}
