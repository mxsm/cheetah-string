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

/// A pattern that can be used with `split` method.
pub trait SplitPattern<'a>: private::SplitSealed {
    #[doc(hidden)]
    fn split_str(self, s: &'a str) -> SplitWrapper<'a>;
}

impl SplitPattern<'_> for char {
    fn split_str(self, s: &str) -> SplitWrapper<'_> {
        SplitWrapper::Char(s.split(self))
    }
}

impl<'a> SplitPattern<'a> for &'a str {
    fn split_str(self, s: &'a str) -> SplitWrapper<'a> {
        let inner = match single_char_pattern(self) {
            Some(ch) => SplitStrInner::Char(s.split(ch)),
            None => SplitStrInner::Str(s.split(self)),
        };

        SplitWrapper::Str(SplitStr(inner))
    }
}

/// Helper struct for splitting strings by a string pattern.
pub struct SplitStr<'a>(SplitStrInner<'a>);

enum SplitStrInner<'a> {
    Str(str::Split<'a, &'a str>),
    Char(str::Split<'a, char>),
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

impl<'a> Iterator for SplitStr<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.0 {
            SplitStrInner::Str(iter) => iter.next(),
            SplitStrInner::Char(iter) => iter.next(),
        }
    }
}

/// Wrapper for split iterator that supports both char and str patterns.
pub enum SplitWrapper<'a> {
    #[doc(hidden)]
    Char(str::Split<'a, char>),
    #[doc(hidden)]
    Str(SplitStr<'a>),
}

impl<'a> Iterator for SplitWrapper<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            SplitWrapper::Char(iter) => iter.next(),
            SplitWrapper::Str(iter) => iter.next(),
        }
    }
}

impl<'a> DoubleEndedIterator for SplitWrapper<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        match self {
            SplitWrapper::Char(iter) => iter.next_back(),
            SplitWrapper::Str(_) => {
                // String pattern split doesn't support reverse iteration.
                panic!("split with string pattern does not support reverse iteration")
            }
        }
    }
}
