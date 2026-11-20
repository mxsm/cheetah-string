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

/// Separators of 2 up to this many bytes are found by [`find_short`], which
/// needs no setup, instead of the Two-Way searcher that `str::split` builds on
/// every call. The middle of a candidate is checked with `compare::bytes_eq`,
/// which handles up to 64 bytes without a `memcmp` call.
const SHORT_SEPARATOR_MAX: usize = 32;

/// Candidate positions screened together by [`find_short`].
const SCREEN_BLOCK: usize = 16;

/// Positions that pass the screen but not the full comparison, tolerated in
/// one search before the split hands the rest of the text to `str::split`.
/// Ordinary text rarely has any; text built to defeat the screen pays for at
/// most this many, once per split.
const SHORT_SEPARATOR_FALSE_CANDIDATES: usize = 4;

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

/// The outcome of [`find_short`].
enum ShortSearch {
    /// The first match starts here.
    Found(usize),
    /// The text holds no match.
    Missing,
    /// More than [`SHORT_SEPARATOR_FALSE_CANDIDATES`] positions passed the
    /// screen without matching, the last at this position; a Two-Way searcher
    /// suits this text better. Nothing before the position matches, and the
    /// position holds the needle's first byte, so it is a character boundary.
    TooManyFalseCandidates(usize),
}

/// Returns the first position of `needle`, of 2 to [`SHORT_SEPARATOR_MAX`]
/// bytes, in `haystack`.
///
/// Positions are screened [`SCREEN_BLOCK`] at a time on the needle's first and
/// last bytes, in a fixed-size loop the compiler turns into vector compares,
/// and only positions that pass compare the middle. Text that merely repeats
/// the separator's first byte, or its leading bytes, produces no candidates.
#[inline]
fn find_short(haystack: &str, needle: &str) -> ShortSearch {
    let bytes = haystack.as_bytes();
    let pattern = needle.as_bytes();
    let span = pattern.len() - 1;
    let (first, last_byte, middle) = (pattern[0], pattern[span], &pattern[1..span]);
    // The last position where a whole separator still fits.
    let Some(last) = bytes.len().checked_sub(pattern.len()) else {
        return ShortSearch::Missing;
    };
    let middle_matches = |at: usize| crate::compare::bytes_eq(&bytes[at + 1..at + span], middle);
    let mut false_candidates = 0;

    let mut at = 0;
    // Every position of a block, plus `span`, stays inside the text while
    // `at + SCREEN_BLOCK <= last + 1`.
    while at + SCREEN_BLOCK <= last + 1 {
        let heads: &[u8; SCREEN_BLOCK] = bytes[at..at + SCREEN_BLOCK]
            .try_into()
            .expect("a block of heads");
        let tails: &[u8; SCREEN_BLOCK] = bytes[at + span..at + span + SCREEN_BLOCK]
            .try_into()
            .expect("a block of tails");
        let mut hits = 0u32;
        for i in 0..SCREEN_BLOCK {
            hits |= u32::from((heads[i] == first) & (tails[i] == last_byte)) << i;
        }
        while hits != 0 {
            let candidate = at + hits.trailing_zeros() as usize;
            if middle_matches(candidate) {
                return ShortSearch::Found(candidate);
            }
            false_candidates += 1;
            if false_candidates > SHORT_SEPARATOR_FALSE_CANDIDATES {
                return ShortSearch::TooManyFalseCandidates(candidate);
            }
            hits &= hits - 1;
        }
        at += SCREEN_BLOCK;
    }
    match (at..=last)
        .find(|&i| bytes[i] == first && bytes[i + span] == last_byte && middle_matches(i))
    {
        Some(found) => ShortSearch::Found(found),
        None => ShortSearch::Missing,
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
