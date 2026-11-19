use alloc::sync::Arc;
use core::str;

use super::pattern::{classify, SplitPattern, SplitStr, StrPattern, StrPatternKind};
use super::repr::{InnerString, INLINE_CAPACITY};
use super::CheetahString;
use crate::inline::InlineStr;

impl CheetahString {
    // Query methods - delegate to &str

    /// Returns `true` if the string starts with the given pattern.
    ///
    /// String patterns of up to 64 bytes are compared with inline loads
    /// instead of a `memcmp` call; longer ones use `memcmp`.
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
    #[must_use]
    #[inline]
    pub fn starts_with<P: StrPattern>(&self, pat: P) -> bool {
        match classify(&pat) {
            StrPatternKind::Char(c) => self.as_str().starts_with(c),
            StrPatternKind::Str(s) => {
                #[cfg(all(feature = "experimental-simd", target_arch = "x86_64"))]
                {
                    if s.len() >= crate::simd::SIMD_THRESHOLD {
                        return crate::simd::starts_with_bytes(self.as_bytes(), s.as_bytes());
                    }
                }

                crate::compare::starts_with(self.as_bytes(), s.as_bytes())
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
    #[must_use]
    #[inline]
    pub fn starts_with_char(&self, pat: char) -> bool {
        self.as_str().starts_with(pat)
    }

    /// Returns `true` if the string ends with the given pattern.
    ///
    /// String patterns of up to 64 bytes are compared with inline loads
    /// instead of a `memcmp` call; longer ones use `memcmp`.
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
    #[must_use]
    #[inline]
    pub fn ends_with<P: StrPattern>(&self, pat: P) -> bool {
        match classify(&pat) {
            StrPatternKind::Char(c) => self.as_str().ends_with(c),
            StrPatternKind::Str(s) => {
                #[cfg(all(feature = "experimental-simd", target_arch = "x86_64"))]
                {
                    if s.len() >= crate::simd::SIMD_THRESHOLD {
                        return crate::simd::ends_with_bytes(self.as_bytes(), s.as_bytes());
                    }
                }

                crate::compare::ends_with(self.as_bytes(), s.as_bytes())
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
    #[must_use]
    #[inline]
    pub fn ends_with_char(&self, pat: char) -> bool {
        self.as_str().ends_with(pat)
    }

    /// Returns `true` if the string contains the given pattern.
    ///
    /// Needles of 2 to 32 bytes on text long enough for a 16-byte block use the
    /// standard library's SIMD search; other cases use `memchr`/`memmem`.
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
    #[must_use]
    #[inline]
    pub fn contains<P: StrPattern>(&self, pat: P) -> bool {
        match classify(&pat) {
            StrPatternKind::Char(c) => self.as_str().contains(c),
            StrPatternKind::Str(s) => crate::search::contains_str(self.as_str(), s),
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
    #[must_use]
    #[inline]
    pub fn contains_char(&self, pat: char) -> bool {
        self.as_str().contains(pat)
    }

    /// Returns the byte index of the first occurrence of the pattern, or `None` if not found.
    ///
    /// Text of 24 to 127 bytes uses `str::find`; shorter and longer text uses
    /// `memchr::memmem`. Use
    /// [`CheetahString::find_char`] for a `char` pattern and
    /// [`crate::CheetahFinder`] to search for the same needle repeatedly.
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
    #[must_use]
    #[inline]
    pub fn find<P: AsRef<str>>(&self, pat: P) -> Option<usize> {
        crate::search::find_str(self.as_str(), pat.as_ref())
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
    #[must_use]
    #[inline]
    pub fn rfind<P: AsRef<str>>(&self, pat: P) -> Option<usize> {
        crate::search::rfind_str(self.as_str(), pat.as_ref())
    }

    /// Returns the byte index of the first occurrence of `pat`, or `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// use cheetah_string::CheetahString;
    ///
    /// let s = CheetahString::from("orders@group-a");
    /// assert_eq!(s.find_char('@'), Some(6));
    /// assert_eq!(s.find_char('#'), None);
    /// ```
    #[must_use]
    #[inline]
    pub fn find_char(&self, pat: char) -> Option<usize> {
        self.as_str().find(pat)
    }

    /// Returns the byte index of the last occurrence of `pat`, or `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// use cheetah_string::CheetahString;
    ///
    /// let s = CheetahString::from("a.b.c");
    /// assert_eq!(s.rfind_char('.'), Some(3));
    /// ```
    #[must_use]
    #[inline]
    pub fn rfind_char(&self, pat: char) -> Option<usize> {
        self.as_str().rfind(pat)
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
    #[must_use]
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
    #[must_use]
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
    #[must_use]
    #[inline]
    pub fn trim_end(&self) -> &str {
        self.as_str().trim_end()
    }

    /// Splits the string by a character pattern.
    ///
    /// The returned iterator supports reverse iteration.
    ///
    /// # Examples
    ///
    /// ```
    /// use cheetah_string::CheetahString;
    ///
    /// let s = CheetahString::from("a,b,c");
    /// let parts: Vec<&str> = s.split_char(',').rev().collect();
    /// assert_eq!(parts, vec!["c", "b", "a"]);
    /// ```
    #[must_use]
    #[inline]
    pub fn split_char(&self, pat: char) -> str::Split<'_, char> {
        self.as_str().split(pat)
    }

    /// Splits the string by a string pattern.
    ///
    /// The returned iterator is forward-only. This makes unsupported reverse
    /// iteration a compile-time error instead of a runtime panic.
    ///
    /// # Examples
    ///
    /// ```
    /// use cheetah_string::CheetahString;
    ///
    /// let s = CheetahString::from("a::b::c");
    /// let parts: Vec<&str> = s.split_str("::").collect();
    /// assert_eq!(parts, vec!["a", "b", "c"]);
    /// ```
    ///
    /// ```compile_fail
    /// use cheetah_string::CheetahString;
    ///
    /// let s = CheetahString::from("a::b::c");
    /// let _ = s.split_str("::").rev();
    /// ```
    #[must_use]
    #[inline]
    pub fn split_str<'a, 'p>(&'a self, pat: &'p str) -> SplitStr<'a, 'p> {
        SplitStr::new(self.as_str(), pat)
    }

    /// Splits with a v2-compatible pattern while retaining its concrete
    /// iterator capability in the return type.
    ///
    /// New code should use [`CheetahString::split_char`] or
    /// [`CheetahString::split_str`] for a self-documenting capability.
    #[deprecated(
        since = "3.0.0",
        note = "use split_char() for char patterns or split_str() for string patterns"
    )]
    #[inline]
    pub fn split<'a, P>(&'a self, pat: P) -> P::Iter
    where
        P: SplitPattern<'a>,
    {
        pat.split_pattern(self.as_str())
    }

    /// Returns an iterator over the lines of the string.
    ///
    /// The opaque iterator exposes reverse iteration, cloning, and fused
    /// iteration because those capabilities are guaranteed by `str::Lines`.
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
    pub fn lines(
        &self,
    ) -> impl DoubleEndedIterator<Item = &str> + Clone + core::iter::FusedIterator {
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
    #[must_use]
    #[inline]
    pub fn to_uppercase(&self) -> CheetahString {
        if self.as_str().is_ascii() {
            return self.map_ascii_case(true);
        }
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
    #[must_use]
    #[inline]
    pub fn to_lowercase(&self) -> CheetahString {
        if self.as_str().is_ascii() {
            return self.map_ascii_case(false);
        }
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
    #[must_use]
    #[inline]
    pub fn replace<P: AsRef<str>>(&self, from: P, to: &str) -> CheetahString {
        self.replace_matches(from.as_ref(), to, usize::MAX)
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
    #[must_use]
    #[inline]
    pub fn replacen<P: AsRef<str>>(&self, from: P, to: &str, count: usize) -> CheetahString {
        self.replace_matches(from.as_ref(), to, count)
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
    #[must_use]
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
    #[must_use]
    #[inline]
    pub fn repeat(&self, n: usize) -> CheetahString {
        let value = self.as_str();
        if n == 1 {
            return self.clone();
        }
        if n == 0 || value.is_empty() {
            return CheetahString::empty();
        }
        let len = value.len().checked_mul(n).expect("capacity overflow");
        if len <= INLINE_CAPACITY {
            return super::assemble::assemble(len, |writer| {
                for _ in 0..n {
                    writer.push_str(value);
                }
            });
        }
        // `str::repeat` doubles its buffer, which beats one copy per
        // repetition when a short value is repeated many times.
        CheetahString::from_string(value.repeat(n))
    }

    /// Maps the ASCII letters of an all-ASCII value, reusing `self` when
    /// nothing changes.
    #[inline]
    fn map_ascii_case(&self, upper: bool) -> CheetahString {
        let value = self.as_str();
        debug_assert!(value.is_ascii());
        let changes = if upper {
            value.bytes().any(|byte| byte.is_ascii_lowercase())
        } else {
            value.bytes().any(|byte| byte.is_ascii_uppercase())
        };
        if !changes {
            return self.clone();
        }

        if let Some(mut inline) = InlineStr::from_str(value) {
            if upper {
                inline.make_ascii_uppercase();
            } else {
                inline.make_ascii_lowercase();
            }
            return CheetahString {
                inner: InnerString::Inline(inline),
            };
        }

        let mut shared: Arc<str> = Arc::from(value);
        let target = Arc::get_mut(&mut shared).expect("a new Arc is uniquely owned");
        if upper {
            target.make_ascii_uppercase();
        } else {
            target.make_ascii_lowercase();
        }
        CheetahString {
            inner: InnerString::Shared(shared),
        }
    }

    /// Replaces up to `limit` non-overlapping matches. The text is scanned
    /// twice so the result can be written once, at its exact length.
    #[inline]
    fn replace_matches(&self, from: &str, to: &str, limit: usize) -> CheetahString {
        let value = self.as_str();
        if from.is_empty() {
            // An empty pattern matches at every character boundary, including
            // the end of the text.
            return self.replace_starts(0, to, limit, || {
                value
                    .char_indices()
                    .map(|(index, _)| index)
                    .chain(core::iter::once(value.len()))
            });
        }

        if let [byte] = from.as_bytes() {
            // A one-byte pattern is ASCII, so every match is a char boundary.
            return self.replace_starts(from.len(), to, limit, || {
                memchr::memchr_iter(*byte, value.as_bytes())
            });
        }
        let finder = memchr::memmem::Finder::new(from.as_bytes());
        self.replace_starts(from.len(), to, limit, || finder.find_iter(value.as_bytes()))
    }

    /// Writes the replacement once at its exact length, given a way to list
    /// the non-overlapping match starts twice: once to size, once to copy.
    #[inline]
    fn replace_starts<I>(
        &self,
        from_len: usize,
        to: &str,
        limit: usize,
        starts: impl Fn() -> I,
    ) -> CheetahString
    where
        I: Iterator<Item = usize>,
    {
        let value = self.as_str();
        let matches = starts().take(limit).count();
        if matches == 0 {
            return self.clone();
        }

        let len = (value.len() - matches * from_len)
            .checked_add(matches.checked_mul(to.len()).expect("capacity overflow"))
            .expect("capacity overflow");
        super::assemble::assemble(len, |writer| {
            let mut last = 0;
            for start in starts().take(limit) {
                writer.push_str(&value[last..start]);
                writer.push_str(to);
                last = start + from_len;
            }
            writer.push_str(&value[last..]);
        })
    }
}
