use alloc::string::String;
use core::str;

pub enum StrPatternKind<'a> {
    Char(char),
    Str(&'a str),
}

// Sealed trait pattern to support both &str and char in starts_with/ends_with/contains.
mod private {
    use alloc::string::String;

    pub trait Sealed {
        fn classify(&self) -> super::StrPatternKind<'_>;
    }

    impl Sealed for char {
        #[inline]
        fn classify(&self) -> super::StrPatternKind<'_> {
            super::StrPatternKind::Char(*self)
        }
    }

    impl Sealed for &str {
        #[inline]
        fn classify(&self) -> super::StrPatternKind<'_> {
            super::StrPatternKind::Str(self)
        }
    }

    impl Sealed for &String {
        #[inline]
        fn classify(&self) -> super::StrPatternKind<'_> {
            super::StrPatternKind::Str(self.as_str())
        }
    }

    impl Sealed for &super::super::CheetahString {
        #[inline]
        fn classify(&self) -> super::StrPatternKind<'_> {
            super::StrPatternKind::Str(self.as_str())
        }
    }

    pub trait SplitSealed {}
    impl SplitSealed for char {}
    impl SplitSealed for &str {}
}

/// A sealed pattern accepted by text query methods.
///
/// The supported pattern types are `char`, `&str`, `&String`, and
/// `&CheetahString`. External
/// implementations are intentionally rejected so the crate can preserve query
/// semantics across compatible releases.
///
/// ```compile_fail
/// use cheetah_string::StrPattern;
///
/// struct ExternalPattern;
/// impl StrPattern for ExternalPattern {}
/// ```
pub trait StrPattern: private::Sealed {
    #[doc(hidden)]
    fn as_str_pattern(&self) -> StrPatternImpl<'_> {
        match private::Sealed::classify(self) {
            StrPatternKind::Char(value) => StrPatternImpl::Char(value),
            StrPatternKind::Str(value) => StrPatternImpl::Str(value),
        }
    }
}

/// Compatibility dispatch value returned by [`StrPattern::as_str_pattern`].
///
/// Query implementation uses a private classifier; this type remains unchanged
/// so existing 3.1 source continues to compile.
#[doc(hidden)]
pub enum StrPatternImpl<'a> {
    Char(char),
    Str(&'a str),
}

impl StrPattern for char {}
impl StrPattern for &str {}
impl StrPattern for &String {}
impl StrPattern for &super::CheetahString {}

#[inline]
pub(super) fn classify<P: StrPattern>(pattern: &P) -> StrPatternKind<'_> {
    private::Sealed::classify(pattern)
}

/// A compatibility pattern whose iterator type exposes its capabilities.
///
/// Unlike the legacy v2 erased iterator, this associated type cannot promise
/// reverse iteration for string patterns and therefore cannot panic from an
/// unsupported `next_back()` call.
pub trait SplitPattern<'a>: private::SplitSealed {
    /// Iterator produced for this pattern.
    type Iter: Iterator<Item = &'a str>;

    #[doc(hidden)]
    fn split_pattern(self, value: &'a str) -> Self::Iter;
}

impl<'a> SplitPattern<'a> for char {
    type Iter = str::Split<'a, char>;

    #[inline]
    fn split_pattern(self, value: &'a str) -> Self::Iter {
        value.split(self)
    }
}

impl<'a, 'p> SplitPattern<'a> for &'p str {
    type Iter = SplitStr<'a, 'p>;

    #[inline]
    fn split_pattern(self, value: &'a str) -> Self::Iter {
        SplitStr::new(value, self)
    }
}

/// Helper struct for splitting strings by a string pattern.
///
/// This iterator is intentionally forward-only because Rust's standard string
/// pattern splitter does not support reverse iteration for `&str` patterns.
#[derive(Clone, Debug)]
pub struct SplitStr<'a, 'p>(SplitStrInner<'a, 'p>);

#[derive(Clone, Debug)]
enum SplitStrInner<'a, 'p> {
    Str(str::Split<'a, &'p str>),
    Char(str::Split<'a, char>),
    Short(ShortSplit<'a, 'p>),
}

/// Separators of 2 up to this many bytes are found with `memchr` on their
/// first byte and a check of the rest, which skips building the Two-Way
/// searcher that `str::split` sets up on every call.
const SHORT_SEPARATOR_MAX: usize = 32;

/// Candidates whose first byte matches but whose rest does not, tolerated in
/// one search before it hands the remaining text to `str::find`. Each costs a
/// `memchr` call, so text where the separator's first byte is common but the
/// separator is not would otherwise cost a call every few bytes.
const SHORT_SEPARATOR_FALSE_CANDIDATES: usize = 2;

impl<'a, 'p> SplitStr<'a, 'p> {
    #[inline]
    pub(super) fn new(value: &'a str, pattern: &'p str) -> Self {
        let inner = match single_char_pattern(pattern) {
            Some(ch) => SplitStrInner::Char(value.split(ch)),
            None if (2..=SHORT_SEPARATOR_MAX).contains(&pattern.len()) => {
                SplitStrInner::Short(ShortSplit {
                    rest: Some(value),
                    separator: pattern,
                })
            }
            None => SplitStrInner::Str(value.split(pattern)),
        };

        Self(inner)
    }
}

/// Forward split by a separator of 2 to [`SHORT_SEPARATOR_MAX`] bytes that
/// yields the same pieces as `str::split`: matches are leftmost and do not
/// overlap. A match of valid UTF-8 inside valid UTF-8 starts and ends on
/// character boundaries, so the slices never split a character.
#[derive(Clone, Debug)]
struct ShortSplit<'a, 'p> {
    /// The text after the last separator, or `None` once it was yielded.
    rest: Option<&'a str>,
    separator: &'p str,
}

impl<'a> ShortSplit<'a, '_> {
    #[inline]
    fn next(&mut self) -> Option<&'a str> {
        let rest = self.rest?;
        match find_short(rest, self.separator) {
            Some(at) => {
                self.rest = Some(&rest[at + self.separator.len()..]);
                Some(&rest[..at])
            }
            None => {
                self.rest = None;
                Some(rest)
            }
        }
    }
}

/// Returns the first position of `needle`, of at least two bytes, in
/// `haystack`.
#[inline]
fn find_short(haystack: &str, needle: &str) -> Option<usize> {
    let bytes = haystack.as_bytes();
    let (&first, tail) = needle.as_bytes().split_first()?;
    // The last position where a whole separator still fits.
    let last = bytes.len().checked_sub(needle.len())?;
    let mut from = 0;
    for _ in 0..SHORT_SEPARATOR_FALSE_CANDIDATES {
        let at = from + memchr::memchr(first, &bytes[from..=last])?;
        if crate::compare::bytes_eq(&bytes[at + 1..at + needle.len()], tail) {
            return Some(at);
        }
        from = at + 1;
        if from > last {
            return None;
        }
    }
    // `first` begins a character, so the last candidate, `from - 1`, is a
    // character boundary. Searching from it again costs one comparison.
    let start = from - 1;
    haystack[start..].find(needle).map(|offset| start + offset)
}

#[inline]
fn single_char_pattern(pattern: &str) -> Option<char> {
    let mut chars = pattern.chars();
    let ch = chars.next()?;

    if chars.next().is_none() {
        Some(ch)
    } else {
        None
    }
}

impl<'a, 'p> Iterator for SplitStr<'a, 'p> {
    type Item = &'a str;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.0 {
            SplitStrInner::Str(iter) => iter.next(),
            SplitStrInner::Char(iter) => iter.next(),
            SplitStrInner::Short(iter) => iter.next(),
        }
    }
}

// The inner `str::Split` iterators are fused, and `ShortSplit` keeps
// returning `None` once its remainder was yielded.
impl core::iter::FusedIterator for SplitStr<'_, '_> {}
