use core::str;

use super::pattern::{SplitPattern, SplitStr, StrPattern, StrPatternImpl};
use super::CheetahString;

impl CheetahString {
    // Query methods - delegate to &str

    /// Returns `true` if the string starts with the given pattern.
    ///
    /// The stable path delegates to `str::starts_with`. The optional
    /// `experimental-simd` feature enables a benchmark-gated x86_64 experiment.
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
                #[cfg(all(feature = "experimental-simd", target_arch = "x86_64"))]
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
    /// The stable path delegates to `str::ends_with`. The optional
    /// `experimental-simd` feature enables a benchmark-gated x86_64 experiment.
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
                #[cfg(all(feature = "experimental-simd", target_arch = "x86_64"))]
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
    /// This method uses the `memchr`/`memmem` search backend.
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
    /// This method uses the `memchr`/`memmem` search backend.
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
}
