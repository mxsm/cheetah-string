use alloc::string::String;
use core::str;

// Sealed trait pattern to support both &str and char in starts_with/ends_with/contains.
mod private {
    use alloc::string::String;

    pub trait Sealed {}
    impl Sealed for char {}
    impl Sealed for &str {}
    impl Sealed for &String {}

    pub trait SplitSealed {}
    impl SplitSealed for char {}
    impl SplitSealed for &str {}
}

/// A pattern that can be used with `starts_with` and `ends_with` methods.
pub trait StrPattern: private::Sealed {
    #[doc(hidden)]
    fn as_str_pattern(&self) -> StrPatternImpl<'_>;
}

#[doc(hidden)]
pub enum StrPatternImpl<'a> {
    Char(char),
    Str(&'a str),
}

impl StrPattern for char {
    #[inline]
    fn as_str_pattern(&self) -> StrPatternImpl<'_> {
        StrPatternImpl::Char(*self)
    }
}

impl StrPattern for &str {
    #[inline]
    fn as_str_pattern(&self) -> StrPatternImpl<'_> {
        StrPatternImpl::Str(self)
    }
}

impl StrPattern for &String {
    #[inline]
    fn as_str_pattern(&self) -> StrPatternImpl<'_> {
        StrPatternImpl::Str(self.as_str())
    }
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
pub struct SplitStr<'a, 'p>(SplitStrInner<'a, 'p>);

enum SplitStrInner<'a, 'p> {
    Str(str::Split<'a, &'p str>),
    Char(str::Split<'a, char>),
}

impl<'a, 'p> SplitStr<'a, 'p> {
    #[inline]
    pub(super) fn new(value: &'a str, pattern: &'p str) -> Self {
        let inner = match single_char_pattern(pattern) {
            Some(ch) => SplitStrInner::Char(value.split(ch)),
            None => SplitStrInner::Str(value.split(pattern)),
        };

        Self(inner)
    }
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

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.0 {
            SplitStrInner::Str(iter) => iter.next(),
            SplitStrInner::Char(iter) => iter.next(),
        }
    }
}
