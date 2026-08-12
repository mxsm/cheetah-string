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

    pub trait SplitSealed {}
    impl SplitSealed for char {}
    impl SplitSealed for &str {}
}

/// A sealed pattern accepted by text query methods.
///
/// The supported pattern types are `char`, `&str`, and `&String`. External
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
