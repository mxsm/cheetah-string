use alloc::borrow::Cow;
use alloc::string::{ParseError, String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::borrow::Borrow;
use core::cmp::Ordering;
use core::fmt::{self, Display};
use core::hash::{Hash, Hasher};
use core::ops::{Add, AddAssign, Deref};
use core::str::{self, FromStr, Utf8Error};

mod pattern;
mod repr;

use crate::inline::InlineStr;
use pattern::StrPatternImpl;
pub use pattern::{SplitPattern, SplitStr, SplitWrapper, StrPattern};
use repr::{InnerString, INLINE_CAPACITY};

#[derive(Clone)]
#[repr(transparent)]
pub struct CheetahString {
    inner: InnerString,
}

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
    type Error = Utf8Error;

    #[inline]
    fn try_from(b: bytes::Bytes) -> Result<Self, Self::Error> {
        CheetahString::try_from_bytes_buf(b)
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
            CheetahString {
                inner: InnerString::Owned(s),
            } => s,
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
        CheetahString::from_builder_string(unsafe { String::from_utf8_unchecked(s) })
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
        CheetahString::from_string_owned(s)
    }

    /// Creates a `CheetahString` from an owned `String` while preserving
    /// ownership and spare capacity for later mutation.
    ///
    /// This constructor is intended for builder-style paths that will continue
    /// appending to the string. It keeps long strings in owned storage instead
    /// of converting them to shared storage.
    #[inline]
    pub fn from_string_owned(s: String) -> Self {
        CheetahString::from_builder_string(s)
    }

    /// Creates a `CheetahString` from an owned `String` using shared storage
    /// for long immutable strings.
    ///
    /// New code that needs clone-cheap immutable strings should prefer
    /// `CheetahStr`.
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
    fn from_builder_string(s: String) -> Self {
        if s.len() <= INLINE_CAPACITY && s.capacity() <= INLINE_CAPACITY {
            let inline = InlineStr::from_str(&s).expect("short String must fit inline storage");
            CheetahString {
                inner: InnerString::Inline(inline),
            }
        } else {
            CheetahString {
                inner: InnerString::Owned(s),
            }
        }
    }

    #[inline]
    pub fn from_arc_string(s: Arc<String>) -> Self {
        match Arc::try_unwrap(s) {
            Ok(s) => CheetahString::from_builder_string(s),
            Err(s) => CheetahString::from_slice(s.as_str()),
        }
    }

    #[inline]
    #[cfg(feature = "bytes")]
    pub fn try_from_bytes_buf(b: bytes::Bytes) -> Result<Self, Utf8Error> {
        str::from_utf8(b.as_ref())?;
        Ok(CheetahString::from_validated_bytes_unchecked(b))
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
            InnerString::Owned(s) => s.as_str(),
        }
    }

    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        match &self.inner {
            InnerString::Inline(inline) => inline.as_bytes(),
            InnerString::Static(s) => s.as_bytes(),
            InnerString::Shared(s) => s.as_bytes(),
            InnerString::Owned(s) => s.as_bytes(),
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        match &self.inner {
            InnerString::Inline(inline) => inline.len(),
            InnerString::Static(s) => s.len(),
            InnerString::Shared(s) => s.len(),
            InnerString::Owned(s) => s.len(),
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        match &self.inner {
            InnerString::Inline(inline) => inline.is_empty(),
            InnerString::Static(s) => s.is_empty(),
            InnerString::Shared(s) => s.is_empty(),
            InnerString::Owned(s) => s.is_empty(),
        }
    }

    // Query methods - delegate to &str

    /// Returns `true` if the string starts with the given pattern.
    ///
    /// When the `simd` feature is enabled, this method uses SIMD instructions
    /// for improved performance on longer patterns.
    ///
    /// # Examples
    ///
    /// ```
    /// use cheetah_string::CheetahString;
    ///
    /// let s = CheetahString::from("hello world");
    /// assert!(s.starts_with("hello"));
    /// assert!(!s.starts_with("world"));
    /// assert!(s.starts_with('h'));
    /// ```
    #[inline]
    pub fn starts_with<P: StrPattern>(&self, pat: P) -> bool {
        match pat.as_str_pattern() {
            StrPatternImpl::Char(c) => self.as_str().starts_with(c),
            StrPatternImpl::Str(s) => {
                #[cfg(all(feature = "simd", target_arch = "x86_64"))]
                {
                    if s.len() >= crate::simd::SIMD_THRESHOLD {
                        return crate::simd::starts_with_bytes(self.as_bytes(), s.as_bytes());
                    }
                }

                self.as_str().starts_with(s)
            }
        }
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
    /// When the `simd` feature is enabled, this method uses SIMD instructions
    /// for improved performance on longer patterns.
    ///
    /// # Examples
    ///
    /// ```
    /// use cheetah_string::CheetahString;
    ///
    /// let s = CheetahString::from("hello world");
    /// assert!(s.ends_with("world"));
    /// assert!(!s.ends_with("hello"));
    /// assert!(s.ends_with('d'));
    /// ```
    #[inline]
    pub fn ends_with<P: StrPattern>(&self, pat: P) -> bool {
        match pat.as_str_pattern() {
            StrPatternImpl::Char(c) => self.as_str().ends_with(c),
            StrPatternImpl::Str(s) => {
                #[cfg(all(feature = "simd", target_arch = "x86_64"))]
                {
                    if s.len() >= crate::simd::SIMD_THRESHOLD {
                        return crate::simd::ends_with_bytes(self.as_bytes(), s.as_bytes());
                    }
                }

                self.as_str().ends_with(s)
            }
        }
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
    /// This method uses the `memchr`/`memmem` search backend. The `simd`
    /// feature currently accelerates equality, prefix, and suffix checks, not
    /// substring search.
    ///
    /// # Examples
    ///
    /// ```
    /// use cheetah_string::CheetahString;
    ///
    /// let s = CheetahString::from("hello world");
    /// assert!(s.contains("llo"));
    /// assert!(!s.contains("xyz"));
    /// assert!(s.contains('o'));
    /// ```
    #[inline]
    pub fn contains<P: StrPattern>(&self, pat: P) -> bool {
        match pat.as_str_pattern() {
            StrPatternImpl::Char(c) => self.as_str().contains(c),
            StrPatternImpl::Str(s) => {
                crate::search::find_bytes(self.as_bytes(), s.as_bytes()).is_some()
            }
        }
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
    /// This method uses the `memchr`/`memmem` search backend. The `simd`
    /// feature currently accelerates equality, prefix, and suffix checks, not
    /// substring search.
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
        let pat = pat.as_ref();
        crate::search::find_bytes(self.as_bytes(), pat.as_bytes())
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
        crate::search::rfind_bytes(self.as_bytes(), pat.as_ref().as_bytes())
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
    /// let parts2: Vec<&str> = s.split(',').collect();
    /// assert_eq!(parts2, vec!["a", "b", "c"]);
    /// ```
    #[inline]
    pub fn split<'a, P>(&'a self, pat: P) -> SplitWrapper<'a>
    where
        P: SplitPattern<'a>,
    {
        pat.split_str(self.as_str())
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
    /// let reversed: Vec<char> = s.chars().rev().collect();
    /// assert_eq!(reversed, vec!['o', 'l', 'l', 'e', 'h']);
    /// ```
    #[inline]
    pub fn chars(&self) -> str::Chars<'_> {
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
    /// Panics if the range is out of bounds, inverted, or not on valid UTF-8
    /// character boundaries. Use [`CheetahString::try_substring`] for a
    /// recoverable error.
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
        self.try_substring(start, end)
            .expect("substring range must be in bounds and on UTF-8 character boundaries")
    }

    /// Returns a substring as a new `CheetahString`, or a public error when
    /// the requested range is invalid.
    ///
    /// # Examples
    ///
    /// ```
    /// use cheetah_string::CheetahString;
    ///
    /// let s = CheetahString::from("hello world");
    /// assert_eq!(s.try_substring(0, 5).unwrap(), "hello");
    /// assert!(s.try_substring(0, 20).is_err());
    /// ```
    #[inline]
    pub fn try_substring(&self, start: usize, end: usize) -> crate::Result<CheetahString> {
        let value = self.as_str();
        let len = value.len();

        if start > end {
            return Err(crate::Error::InvalidRange { start, end });
        }

        if start > len {
            return Err(crate::Error::IndexOutOfBounds { index: start, len });
        }

        if end > len {
            return Err(crate::Error::IndexOutOfBounds { index: end, len });
        }

        if !value.is_char_boundary(start) {
            return Err(crate::Error::InvalidCharBoundary { index: start });
        }

        if !value.is_char_boundary(end) {
            return Err(crate::Error::InvalidCharBoundary { index: end });
        }

        Ok(CheetahString::from_slice(&value[start..end]))
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
            CheetahString::from_builder_string(String::with_capacity(capacity))
        }
    }

    #[inline]
    fn push_str_internal(&mut self, string: &str) {
        if string.is_empty() {
            return;
        }

        match &mut self.inner {
            InnerString::Inline(inline) => {
                if inline.push_str(string) {
                    return;
                }
            }
            InnerString::Owned(s) => {
                s.push_str(string);
                return;
            }
            _ => {}
        }

        let total_len = self.len() + string.len();
        let mut result = String::with_capacity(total_len);
        result.push_str(self.as_str());
        result.push_str(string);
        *self = CheetahString::from_builder_string(result);
    }

    /// Appends a string slice to the end of this `CheetahString`.
    ///
    /// This method is optimized for incremental building and will:
    /// - Mutate inline storage when possible
    /// - Mutate owned heap storage in-place when capacity allows
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
        self.push_str_internal(string);
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
        if additional == 0 {
            return;
        }

        match &mut self.inner {
            InnerString::Inline(inline) if inline.len() + additional <= INLINE_CAPACITY => {
                return;
            }
            InnerString::Inline(_) => {}
            InnerString::Owned(s) => {
                s.reserve(additional);
                return;
            }
            _ => {}
        }

        let new_len = self.len() + additional;
        let mut s = String::with_capacity(new_len);
        s.push_str(self.as_str());
        *self = CheetahString::from_builder_string(s);
    }
}

impl PartialEq for CheetahString {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        #[cfg(all(feature = "simd", target_arch = "x86_64"))]
        {
            crate::simd::eq_bytes(self.as_bytes(), other.as_bytes())
        }
        #[cfg(not(all(feature = "simd", target_arch = "x86_64")))]
        {
            self.as_str() == other.as_str()
        }
    }
}

impl PartialEq<str> for CheetahString {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        #[cfg(all(feature = "simd", target_arch = "x86_64"))]
        {
            crate::simd::eq_bytes(self.as_bytes(), other.as_bytes())
        }
        #[cfg(not(all(feature = "simd", target_arch = "x86_64")))]
        {
            self.as_str() == other
        }
    }
}

impl PartialEq<String> for CheetahString {
    #[inline]
    fn eq(&self, other: &String) -> bool {
        #[cfg(all(feature = "simd", target_arch = "x86_64"))]
        {
            crate::simd::eq_bytes(self.as_bytes(), other.as_bytes())
        }
        #[cfg(not(all(feature = "simd", target_arch = "x86_64")))]
        {
            self.as_str() == other.as_str()
        }
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
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}

impl Display for CheetahString {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_str().fmt(f)
    }
}

impl fmt::Debug for CheetahString {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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

impl Add<&str> for CheetahString {
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
    fn add(mut self, rhs: &str) -> Self::Output {
        self.push_str_internal(rhs);
        self
    }
}

impl Add<&CheetahString> for CheetahString {
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
    fn add(mut self, rhs: &CheetahString) -> Self::Output {
        self.push_str_internal(rhs.as_str());
        self
    }
}

impl Add<String> for CheetahString {
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
    fn add(mut self, rhs: String) -> Self::Output {
        if self.is_empty() {
            return CheetahString::from_string_owned(rhs);
        }

        self.push_str_internal(&rhs);
        self
    }
}

impl AddAssign<&str> for CheetahString {
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
        self.push_str_internal(rhs);
    }
}

impl AddAssign<&CheetahString> for CheetahString {
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
        self.push_str_internal(rhs.as_str());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::{format, vec};

    #[test]
    fn with_capacity_above_inline_uses_heap_storage() {
        let s = CheetahString::with_capacity(INLINE_CAPACITY + 8);

        match &s.inner {
            InnerString::Owned(inner) => {
                assert!(inner.capacity() >= INLINE_CAPACITY + 8);
            }
            other => panic!(
                "expected heap-backed storage from with_capacity, got {:?}",
                core::mem::discriminant(other)
            ),
        }
    }

    #[test]
    fn push_str_promotes_builder_growth_to_owned_storage() {
        let suffix = "a".repeat(INLINE_CAPACITY);
        let expected = format!("hello{suffix}");
        let mut s = CheetahString::from("hello");

        s.push_str(&suffix);

        match &s.inner {
            InnerString::Owned(inner) => {
                assert_eq!(inner.as_str(), expected.as_str());
                assert!(inner.capacity() >= expected.len());
            }
            other => panic!(
                "expected owned heap storage after builder growth, got {:?}",
                core::mem::discriminant(other)
            ),
        }
    }

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
    fn long_vec_conversion_uses_owned_storage() {
        let value = "a".repeat(INLINE_CAPACITY + 1).into_bytes();
        let s = CheetahString::try_from_vec(value).expect("valid utf-8");

        match &s.inner {
            InnerString::Owned(inner) => {
                assert_eq!(inner.len(), INLINE_CAPACITY + 1);
                assert_eq!(inner.as_bytes(), vec![b'a'; INLINE_CAPACITY + 1].as_slice());
            }
            other => panic!(
                "expected Owned for long Vec<u8> conversion, got {:?}",
                core::mem::discriminant(other)
            ),
        }
    }
}
