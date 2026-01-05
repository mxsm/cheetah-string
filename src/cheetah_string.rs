use core::fmt;
use core::str::Utf8Error;
use std::borrow::{Borrow, Cow};
use std::cmp::Ordering;
use std::fmt::Display;
use std::hash::Hash;
use std::ops::Deref;
use std::str::FromStr;
use std::sync::Arc;

#[derive(Clone)]
#[repr(transparent)]
pub struct CheetahString {
    pub(super) inner: InnerString,
}

impl Default for CheetahString {
    fn default() -> Self {
        CheetahString {
            inner: InnerString::Inline {
                len: 0,
                data: [0; INLINE_CAPACITY],
            },
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
                inner: InnerString::Inline { len, data },
            } => {
                // SAFETY: Inline strings are always valid UTF-8
                unsafe { String::from_utf8_unchecked(data[..len as usize].to_vec()) }
            }
            CheetahString {
                inner: InnerString::StaticStr(s),
            } => s.to_string(),
            CheetahString {
                inner: InnerString::ArcStr(s),
            } => s.to_string(),
            CheetahString {
                inner: InnerString::ArcString(s),
            } => s.as_ref().clone(),
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
            inner: InnerString::Inline {
                len: 0,
                data: [0; INLINE_CAPACITY],
            },
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
        if s.len() <= INLINE_CAPACITY {
            // Use inline storage for short strings
            let mut data = [0u8; INLINE_CAPACITY];
            data[..s.len()].copy_from_slice(s.as_bytes());
            CheetahString {
                inner: InnerString::Inline {
                    len: s.len() as u8,
                    data,
                },
            }
        } else {
            // Use Arc for long strings
            CheetahString {
                inner: InnerString::ArcString(Arc::new(s.to_owned())),
            }
        }
    }

    #[inline]
    pub fn from_string(s: String) -> Self {
        if s.len() <= INLINE_CAPACITY {
            // Use inline storage for short strings
            let mut data = [0u8; INLINE_CAPACITY];
            data[..s.len()].copy_from_slice(s.as_bytes());
            CheetahString {
                inner: InnerString::Inline {
                    len: s.len() as u8,
                    data,
                },
            }
        } else {
            // Use Arc<str> for long strings to avoid double allocation
            let arc_str: Arc<str> = s.into_boxed_str().into();
            CheetahString {
                inner: InnerString::ArcStr(arc_str),
            }
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
            InnerString::Inline { len, data } => {
                // SAFETY: Inline strings are only created from valid UTF-8 sources.
                // The data is always valid UTF-8 up to len bytes.
                unsafe { std::str::from_utf8_unchecked(&data[..*len as usize]) }
            }
            InnerString::StaticStr(s) => s,
            InnerString::ArcStr(s) => s.as_ref(),
            InnerString::ArcString(s) => s.as_str(),
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
        }
    }

    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        match &self.inner {
            InnerString::Inline { len, data } => &data[..*len as usize],
            InnerString::StaticStr(s) => s.as_bytes(),
            InnerString::ArcStr(s) => s.as_bytes(),
            InnerString::ArcString(s) => s.as_bytes(),
            InnerString::ArcVecString(s) => s.as_ref(),
            #[cfg(feature = "bytes")]
            InnerString::Bytes(b) => b.as_ref(),
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        match &self.inner {
            InnerString::Inline { len, .. } => *len as usize,
            InnerString::StaticStr(s) => s.len(),
            InnerString::ArcStr(s) => s.len(),
            InnerString::ArcString(s) => s.len(),
            InnerString::ArcVecString(s) => s.len(),
            #[cfg(feature = "bytes")]
            InnerString::Bytes(b) => b.len(),
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        match &self.inner {
            InnerString::Inline { len, .. } => *len == 0,
            InnerString::StaticStr(s) => s.is_empty(),
            InnerString::ArcStr(s) => s.is_empty(),
            InnerString::ArcString(s) => s.is_empty(),
            InnerString::ArcVecString(s) => s.is_empty(),
            #[cfg(feature = "bytes")]
            InnerString::Bytes(b) => b.is_empty(),
        }
    }

    // Query methods - delegate to &str

    /// Returns `true` if the string starts with the given pattern.
    ///
    /// # Examples
    ///
    /// ```
    /// use cheetah_string::CheetahString;
    ///
    /// let s = CheetahString::from("hello world");
    /// assert!(s.starts_with("hello"));
    /// assert!(!s.starts_with("world"));
    /// ```
    #[inline]
    pub fn starts_with<P: AsRef<str>>(&self, pat: P) -> bool {
        self.as_str().starts_with(pat.as_ref())
    }

    /// Returns `true` if the string starts with the given character.
    ///
    /// # Examples
    ///
    /// ```
    /// use cheetah_string::CheetahString;
    ///
    /// let s = CheetahString::from("hello world");
    /// assert!(s.starts_with_char('h'));
    /// assert!(!s.starts_with_char('w'));
    /// ```
    #[inline]
    pub fn starts_with_char(&self, pat: char) -> bool {
        self.as_str().starts_with(pat)
    }

    /// Returns `true` if the string ends with the given pattern.
    ///
    /// # Examples
    ///
    /// ```
    /// use cheetah_string::CheetahString;
    ///
    /// let s = CheetahString::from("hello world");
    /// assert!(s.ends_with("world"));
    /// assert!(!s.ends_with("hello"));
    /// ```
    #[inline]
    pub fn ends_with<P: AsRef<str>>(&self, pat: P) -> bool {
        self.as_str().ends_with(pat.as_ref())
    }

    /// Returns `true` if the string ends with the given character.
    ///
    /// # Examples
    ///
    /// ```
    /// use cheetah_string::CheetahString;
    ///
    /// let s = CheetahString::from("hello world");
    /// assert!(s.ends_with_char('d'));
    /// assert!(!s.ends_with_char('h'));
    /// ```
    #[inline]
    pub fn ends_with_char(&self, pat: char) -> bool {
        self.as_str().ends_with(pat)
    }

    /// Returns `true` if the string contains the given pattern.
    ///
    /// # Examples
    ///
    /// ```
    /// use cheetah_string::CheetahString;
    ///
    /// let s = CheetahString::from("hello world");
    /// assert!(s.contains("llo"));
    /// assert!(!s.contains("xyz"));
    /// ```
    #[inline]
    pub fn contains<P: AsRef<str>>(&self, pat: P) -> bool {
        self.as_str().contains(pat.as_ref())
    }

    /// Returns `true` if the string contains the given character.
    ///
    /// # Examples
    ///
    /// ```
    /// use cheetah_string::CheetahString;
    ///
    /// let s = CheetahString::from("hello world");
    /// assert!(s.contains_char('o'));
    /// assert!(!s.contains_char('x'));
    /// ```
    #[inline]
    pub fn contains_char(&self, pat: char) -> bool {
        self.as_str().contains(pat)
    }

    /// Returns the byte index of the first occurrence of the pattern, or `None` if not found.
    ///
    /// # Examples
    ///
    /// ```
    /// use cheetah_string::CheetahString;
    ///
    /// let s = CheetahString::from("hello world");
    /// assert_eq!(s.find("world"), Some(6));
    /// assert_eq!(s.find("xyz"), None);
    /// ```
    #[inline]
    pub fn find<P: AsRef<str>>(&self, pat: P) -> Option<usize> {
        self.as_str().find(pat.as_ref())
    }

    /// Returns the byte index of the last occurrence of the pattern, or `None` if not found.
    ///
    /// # Examples
    ///
    /// ```
    /// use cheetah_string::CheetahString;
    ///
    /// let s = CheetahString::from("hello hello");
    /// assert_eq!(s.rfind("hello"), Some(6));
    /// ```
    #[inline]
    pub fn rfind<P: AsRef<str>>(&self, pat: P) -> Option<usize> {
        self.as_str().rfind(pat.as_ref())
    }

    /// Returns a string slice with leading and trailing whitespace removed.
    ///
    /// # Examples
    ///
    /// ```
    /// use cheetah_string::CheetahString;
    ///
    /// let s = CheetahString::from("  hello  ");
    /// assert_eq!(s.trim(), "hello");
    /// ```
    #[inline]
    pub fn trim(&self) -> &str {
        self.as_str().trim()
    }

    /// Returns a string slice with leading whitespace removed.
    ///
    /// # Examples
    ///
    /// ```
    /// use cheetah_string::CheetahString;
    ///
    /// let s = CheetahString::from("  hello");
    /// assert_eq!(s.trim_start(), "hello");
    /// ```
    #[inline]
    pub fn trim_start(&self) -> &str {
        self.as_str().trim_start()
    }

    /// Returns a string slice with trailing whitespace removed.
    ///
    /// # Examples
    ///
    /// ```
    /// use cheetah_string::CheetahString;
    ///
    /// let s = CheetahString::from("hello  ");
    /// assert_eq!(s.trim_end(), "hello");
    /// ```
    #[inline]
    pub fn trim_end(&self) -> &str {
        self.as_str().trim_end()
    }

    /// Splits the string by the given pattern.
    ///
    /// # Examples
    ///
    /// ```
    /// use cheetah_string::CheetahString;
    ///
    /// let s = CheetahString::from("a,b,c");
    /// let parts: Vec<&str> = s.split(",").collect();
    /// assert_eq!(parts, vec!["a", "b", "c"]);
    /// ```
    #[inline]
    pub fn split<'a>(&'a self, pat: &'a str) -> impl Iterator<Item = &'a str> {
        self.as_str().split(pat)
    }

    /// Returns an iterator over the lines of the string.
    ///
    /// # Examples
    ///
    /// ```
    /// use cheetah_string::CheetahString;
    ///
    /// let s = CheetahString::from("line1\nline2\nline3");
    /// let lines: Vec<&str> = s.lines().collect();
    /// assert_eq!(lines, vec!["line1", "line2", "line3"]);
    /// ```
    #[inline]
    pub fn lines(&self) -> impl Iterator<Item = &str> {
        self.as_str().lines()
    }

    /// Returns an iterator over the characters of the string.
    ///
    /// # Examples
    ///
    /// ```
    /// use cheetah_string::CheetahString;
    ///
    /// let s = CheetahString::from("hello");
    /// let chars: Vec<char> = s.chars().collect();
    /// assert_eq!(chars, vec!['h', 'e', 'l', 'l', 'o']);
    /// ```
    #[inline]
    pub fn chars(&self) -> impl Iterator<Item = char> + '_ {
        self.as_str().chars()
    }

    // Transformation methods - create new CheetahString

    /// Returns a new `CheetahString` with all characters converted to uppercase.
    ///
    /// # Examples
    ///
    /// ```
    /// use cheetah_string::CheetahString;
    ///
    /// let s = CheetahString::from("hello");
    /// assert_eq!(s.to_uppercase(), "HELLO");
    /// ```
    #[inline]
    pub fn to_uppercase(&self) -> CheetahString {
        CheetahString::from_string(self.as_str().to_uppercase())
    }

    /// Returns a new `CheetahString` with all characters converted to lowercase.
    ///
    /// # Examples
    ///
    /// ```
    /// use cheetah_string::CheetahString;
    ///
    /// let s = CheetahString::from("HELLO");
    /// assert_eq!(s.to_lowercase(), "hello");
    /// ```
    #[inline]
    pub fn to_lowercase(&self) -> CheetahString {
        CheetahString::from_string(self.as_str().to_lowercase())
    }

    /// Replaces all occurrences of a pattern with another string.
    ///
    /// # Examples
    ///
    /// ```
    /// use cheetah_string::CheetahString;
    ///
    /// let s = CheetahString::from("hello world");
    /// assert_eq!(s.replace("world", "rust"), "hello rust");
    /// ```
    #[inline]
    pub fn replace<P: AsRef<str>>(&self, from: P, to: &str) -> CheetahString {
        CheetahString::from_string(self.as_str().replace(from.as_ref(), to))
    }

    /// Returns a new `CheetahString` with the specified range replaced.
    ///
    /// # Examples
    ///
    /// ```
    /// use cheetah_string::CheetahString;
    ///
    /// let s = CheetahString::from("hello world");
    /// assert_eq!(s.replacen("l", "L", 1), "heLlo world");
    /// ```
    #[inline]
    pub fn replacen<P: AsRef<str>>(&self, from: P, to: &str, count: usize) -> CheetahString {
        CheetahString::from_string(self.as_str().replacen(from.as_ref(), to, count))
    }

    /// Returns a substring as a new `CheetahString`.
    ///
    /// # Panics
    ///
    /// Panics if the indices are not on valid UTF-8 character boundaries.
    ///
    /// # Examples
    ///
    /// ```
    /// use cheetah_string::CheetahString;
    ///
    /// let s = CheetahString::from("hello world");
    /// assert_eq!(s.substring(0, 5), "hello");
    /// assert_eq!(s.substring(6, 11), "world");
    /// ```
    #[inline]
    pub fn substring(&self, start: usize, end: usize) -> CheetahString {
        CheetahString::from_slice(&self.as_str()[start..end])
    }

    /// Repeats the string `n` times.
    ///
    /// # Examples
    ///
    /// ```
    /// use cheetah_string::CheetahString;
    ///
    /// let s = CheetahString::from("abc");
    /// assert_eq!(s.repeat(3), "abcabcabc");
    /// ```
    #[inline]
    pub fn repeat(&self, n: usize) -> CheetahString {
        CheetahString::from_string(self.as_str().repeat(n))
    }

    // Incremental building methods

    /// Creates a new `CheetahString` with the specified capacity.
    ///
    /// The string will be able to hold at least `capacity` bytes without reallocating.
    /// If `capacity` is less than or equal to the inline capacity (23 bytes),
    /// an empty inline string is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// use cheetah_string::CheetahString;
    ///
    /// let mut s = CheetahString::with_capacity(100);
    /// s.push_str("hello");
    /// assert_eq!(s, "hello");
    /// ```
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        if capacity <= INLINE_CAPACITY {
            CheetahString::empty()
        } else {
            CheetahString::from_string(String::with_capacity(capacity))
        }
    }

    /// Appends a string slice to the end of this `CheetahString`.
    ///
    /// This method is optimized for incremental building and will:
    /// - Mutate inline storage when possible
    /// - Mutate unique Arc<String> in-place when available
    /// - Only allocate when necessary
    ///
    /// # Examples
    ///
    /// ```
    /// use cheetah_string::CheetahString;
    ///
    /// let mut s = CheetahString::from("Hello");
    /// s.push_str(" ");
    /// s.push_str("World");
    /// assert_eq!(s, "Hello World");
    /// ```
    #[inline]
    pub fn push_str(&mut self, string: &str) {
        *self += string;
    }

    /// Reserves capacity for at least `additional` more bytes.
    ///
    /// This method will modify the internal representation if needed to ensure
    /// that the string can hold at least `additional` more bytes without reallocating.
    ///
    /// # Examples
    ///
    /// ```
    /// use cheetah_string::CheetahString;
    ///
    /// let mut s = CheetahString::from("hello");
    /// s.reserve(100);
    /// s.push_str(" world");
    /// ```
    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        let new_len = self.len() + additional;

        // If it still fits inline, nothing to do
        if new_len <= INLINE_CAPACITY {
            return;
        }

        match &mut self.inner {
            InnerString::Inline { .. } => {
                // Convert inline to Arc<String> with capacity
                let mut s = String::with_capacity(new_len);
                s.push_str(self.as_str());
                *self = CheetahString {
                    inner: InnerString::ArcString(Arc::new(s)),
                };
            }
            InnerString::ArcString(arc) if Arc::strong_count(arc) == 1 => {
                // Reserve in the unique Arc<String>
                if let Some(s) = Arc::get_mut(arc) {
                    s.reserve(additional);
                }
            }
            InnerString::StaticStr(_) | InnerString::ArcStr(_) => {
                // Convert to Arc<String> with capacity
                let mut s = String::with_capacity(new_len);
                s.push_str(self.as_str());
                *self = CheetahString {
                    inner: InnerString::ArcString(Arc::new(s)),
                };
            }
            _ => {
                // For shared Arc or other types, convert if needed
                if Arc::strong_count(match &self.inner {
                    InnerString::ArcString(arc) => arc,
                    _ => return,
                }) > 1
                {
                    let mut s = String::with_capacity(new_len);
                    s.push_str(self.as_str());
                    *self = CheetahString {
                        inner: InnerString::ArcString(Arc::new(s)),
                    };
                }
            }
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

// Add trait implementations for string concatenation

impl std::ops::Add<&str> for CheetahString {
    type Output = CheetahString;

    /// Concatenates a `CheetahString` with a string slice.
    ///
    /// # Examples
    ///
    /// ```
    /// use cheetah_string::CheetahString;
    ///
    /// let s = CheetahString::from("Hello");
    /// let result = s + " World";
    /// assert_eq!(result, "Hello World");
    /// ```
    #[inline]
    fn add(self, rhs: &str) -> Self::Output {
        let total_len = self.len() + rhs.len();

        // Fast path: result fits in inline storage
        if total_len <= INLINE_CAPACITY {
            let mut data = [0u8; INLINE_CAPACITY];
            let self_bytes = self.as_bytes();
            data[..self_bytes.len()].copy_from_slice(self_bytes);
            data[self_bytes.len()..total_len].copy_from_slice(rhs.as_bytes());
            return CheetahString {
                inner: InnerString::Inline {
                    len: total_len as u8,
                    data,
                },
            };
        }

        // Slow path: allocate for long result
        let mut result = String::with_capacity(total_len);
        result.push_str(self.as_str());
        result.push_str(rhs);
        CheetahString::from_string(result)
    }
}

impl std::ops::Add<&CheetahString> for CheetahString {
    type Output = CheetahString;

    /// Concatenates two `CheetahString` values.
    ///
    /// # Examples
    ///
    /// ```
    /// use cheetah_string::CheetahString;
    ///
    /// let s1 = CheetahString::from("Hello");
    /// let s2 = CheetahString::from(" World");
    /// let result = s1 + &s2;
    /// assert_eq!(result, "Hello World");
    /// ```
    #[inline]
    fn add(self, rhs: &CheetahString) -> Self::Output {
        let total_len = self.len() + rhs.len();

        // Fast path: result fits in inline storage
        if total_len <= INLINE_CAPACITY {
            let mut data = [0u8; INLINE_CAPACITY];
            let self_bytes = self.as_bytes();
            data[..self_bytes.len()].copy_from_slice(self_bytes);
            data[self_bytes.len()..total_len].copy_from_slice(rhs.as_bytes());
            return CheetahString {
                inner: InnerString::Inline {
                    len: total_len as u8,
                    data,
                },
            };
        }

        // Slow path: allocate for long result
        let mut result = String::with_capacity(total_len);
        result.push_str(self.as_str());
        result.push_str(rhs.as_str());
        CheetahString::from_string(result)
    }
}

impl std::ops::Add<String> for CheetahString {
    type Output = CheetahString;

    /// Concatenates a `CheetahString` with a `String`.
    ///
    /// # Examples
    ///
    /// ```
    /// use cheetah_string::CheetahString;
    ///
    /// let s = CheetahString::from("Hello");
    /// let result = s + String::from(" World");
    /// assert_eq!(result, "Hello World");
    /// ```
    #[inline]
    fn add(self, rhs: String) -> Self::Output {
        let total_len = self.len() + rhs.len();

        // Fast path: result fits in inline storage
        if total_len <= INLINE_CAPACITY {
            let mut data = [0u8; INLINE_CAPACITY];
            let self_bytes = self.as_bytes();
            data[..self_bytes.len()].copy_from_slice(self_bytes);
            data[self_bytes.len()..total_len].copy_from_slice(rhs.as_bytes());
            return CheetahString {
                inner: InnerString::Inline {
                    len: total_len as u8,
                    data,
                },
            };
        }

        // Slow path: allocate for long result
        let mut result = String::with_capacity(total_len);
        result.push_str(self.as_str());
        result.push_str(&rhs);
        CheetahString::from_string(result)
    }
}

impl std::ops::AddAssign<&str> for CheetahString {
    /// Appends a string slice to a `CheetahString`.
    ///
    /// # Examples
    ///
    /// ```
    /// use cheetah_string::CheetahString;
    ///
    /// let mut s = CheetahString::from("Hello");
    /// s += " World";
    /// assert_eq!(s, "Hello World");
    /// ```
    #[inline]
    fn add_assign(&mut self, rhs: &str) {
        let total_len = self.len() + rhs.len();

        match &mut self.inner {
            // Fast path 1: Both self and result fit in inline storage
            InnerString::Inline { len, data } if total_len <= INLINE_CAPACITY => {
                // Mutate inline buffer directly
                data[*len as usize..total_len].copy_from_slice(rhs.as_bytes());
                *len = total_len as u8;
                return;
            }
            // Fast path 2: Self is unique Arc<String>, mutate in-place
            InnerString::ArcString(arc) if Arc::strong_count(arc) == 1 => {
                // SAFETY: strong_count == 1 guarantees exclusive access
                if let Some(s) = Arc::get_mut(arc) {
                    s.push_str(rhs);
                    return;
                }
            }
            _ => {}
        }

        // Slow path: allocate new string
        let mut result = String::with_capacity(total_len);
        result.push_str(self.as_str());
        result.push_str(rhs);
        *self = CheetahString::from_string(result);
    }
}

impl std::ops::AddAssign<&CheetahString> for CheetahString {
    /// Appends a `CheetahString` to another `CheetahString`.
    ///
    /// # Examples
    ///
    /// ```
    /// use cheetah_string::CheetahString;
    ///
    /// let mut s1 = CheetahString::from("Hello");
    /// let s2 = CheetahString::from(" World");
    /// s1 += &s2;
    /// assert_eq!(s1, "Hello World");
    /// ```
    #[inline]
    fn add_assign(&mut self, rhs: &CheetahString) {
        let total_len = self.len() + rhs.len();

        match &mut self.inner {
            // Fast path 1: Both self and result fit in inline storage
            InnerString::Inline { len, data } if total_len <= INLINE_CAPACITY => {
                // Mutate inline buffer directly
                data[*len as usize..total_len].copy_from_slice(rhs.as_bytes());
                *len = total_len as u8;
                return;
            }
            // Fast path 2: Self is unique Arc<String>, mutate in-place
            InnerString::ArcString(arc) if Arc::strong_count(arc) == 1 => {
                // SAFETY: strong_count == 1 guarantees exclusive access
                if let Some(s) = Arc::get_mut(arc) {
                    s.push_str(rhs.as_str());
                    return;
                }
            }
            _ => {}
        }

        // Slow path: allocate new string
        let mut result = String::with_capacity(total_len);
        result.push_str(self.as_str());
        result.push_str(rhs.as_str());
        *self = CheetahString::from_string(result);
    }
}

/// Maximum capacity for inline string storage (23 bytes + 1 byte for length = 24 bytes total)
const INLINE_CAPACITY: usize = 23;

/// The `InnerString` enum represents different types of string storage.
///
/// This enum uses Small String Optimization (SSO) to avoid heap allocations for short strings.
///
/// Variants:
///
/// * `Inline` - Inline storage for strings <= 23 bytes (zero heap allocations).
/// * `StaticStr(&'static str)` - A static string slice (zero heap allocations).
/// * `ArcStr(Arc<str>)` - A reference-counted string slice (single heap allocation, optimized).
/// * `ArcString(Arc<String>)` - A reference-counted string (for backwards compatibility).
/// * `ArcVecString(Arc<Vec<u8>>)` - A reference-counted byte vector.
/// * `Bytes(bytes::Bytes)` - A byte buffer (available when the "bytes" feature is enabled).
#[derive(Clone)]
pub(super) enum InnerString {
    /// Inline storage for short strings (up to 23 bytes).
    /// Stores the length and data directly without heap allocation.
    Inline {
        len: u8,
        data: [u8; INLINE_CAPACITY],
    },
    /// Static string slice with 'static lifetime.
    StaticStr(&'static str),
    /// Reference-counted string slice (single heap allocation).
    /// Preferred over ArcString for long strings created from owned data.
    ArcStr(Arc<str>),
    /// Reference-counted heap-allocated string.
    /// Kept for backwards compatibility and when Arc<String> is explicitly provided.
    ArcString(Arc<String>),
    /// Reference-counted heap-allocated byte vector.
    ArcVecString(Arc<Vec<u8>>),
    /// Bytes type integration (requires "bytes" feature).
    #[cfg(feature = "bytes")]
    Bytes(bytes::Bytes),
}
