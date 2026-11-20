use crate::search::{find_short, ShortSearch, SHORT_NEEDLE_MAX};
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

impl<'a, 'p> SplitStr<'a, 'p> {
    #[inline]
    pub(super) fn new(value: &'a str, pattern: &'p str) -> Self {
        let inner = match single_char_pattern(pattern) {
            Some(ch) => SplitStrInner::Char(value.split(ch)),
            None if (2..=SHORT_NEEDLE_MAX).contains(&pattern.len()) => {
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

/// Forward split by a separator of 2 to [`SHORT_NEEDLE_MAX`] bytes that
/// yields the same pieces as `str::split`: matches are leftmost and do not
/// overlap. A match of valid UTF-8 inside valid UTF-8 starts and ends on
/// character boundaries, so the slices never split a character.
#[derive(Clone, Debug)]
struct ShortSplit<'a, 'p> {
    /// The text after the last separator, or `None` once it was yielded.
    rest: Option<&'a str>,
    separator: &'p str,
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
            SplitStrInner::Short(iter) => {
                let rest = iter.rest?;
                let separator = iter.separator;
                match find_short(rest, separator) {
                    ShortSearch::Found(at) => {
                        iter.rest = Some(&rest[at + separator.len()..]);
                        Some(&rest[..at])
                    }
                    ShortSearch::Missing => {
                        iter.rest = None;
                        Some(rest)
                    }
                    ShortSearch::TooManyFalseCandidates(from) => {
                        self.hand_off(rest, separator, from)
                    }
                }
            }
        }
    }
}

impl<'a, 'p> SplitStr<'a, 'p> {
    /// Finishes the current piece with `str::find` from `from`, the last
    /// false candidate, and splits the rest with one `str::split`, after the
    /// screen let through too many false candidates. The Two-Way searchers are
    /// then built at most twice per split, not once per piece. Kept out of
    /// line so the common path of every split carries neither the searcher
    /// setup nor the write of the larger iterator state.
    #[cold]
    #[inline(never)]
    fn hand_off(&mut self, rest: &'a str, separator: &'p str, from: usize) -> Option<&'a str> {
        match rest[from..].find(separator) {
            Some(offset) => {
                let at = from + offset;
                self.0 = SplitStrInner::Str(rest[at + separator.len()..].split(separator));
                Some(&rest[..at])
            }
            None => {
                self.0 = SplitStrInner::Short(ShortSplit {
                    rest: None,
                    separator,
                });
                Some(rest)
            }
        }
    }
}

// The inner `str::Split` iterators are fused, `ShortSplit` keeps returning
// `None` once its remainder was yielded, and a hand-off replaces it with a
// fused `str::Split` over the text not yet yielded.
impl core::iter::FusedIterator for SplitStr<'_, '_> {}
