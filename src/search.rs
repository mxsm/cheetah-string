// Thresholds measured with `benches/search_thresholds.rs` on x86_64 (an
// i7-11700K and the `ubuntu-latest` runner) and aarch64 (the
// `ubuntu-24.04-arm` runner): needles of 2 to 32 bytes made of common or rare
// haystack bytes, haystacks of 8 B to 4 KiB, hits and misses. The
// `Search thresholds` workflow reruns it on both targets when this file
// changes.
//
// A needle at least as long as the haystack can only match the whole
// haystack, so one comparison answers it. std does the same for `contains`,
// while `memmem` would still hash the single candidate position.
//
// Needles of 2 to 32 bytes on haystacks shorter than 256 bytes use the
// screened search below, which needs no setup: it beat one-shot `memmem`
// (Rabin-Karp below 64 bytes) and std's Two-Way searchers there. From 256
// bytes the SIMD scans of `memmem` and std win.
//
// `str::contains` has a SIMD fast path for needles of 2..=32 bytes once the
// haystack holds one 16-byte block plus the needle, and it beat the screen
// there, so `contains` keeps it and screens only shorter haystacks. Above
// 32-byte needles std builds a Two-Way searcher, so memmem stays the better
// choice there.
//
// std only has that fast path on these targets; elsewhere it always builds a
// Two-Way searcher, so `contains` screens up to 256 bytes like `find`.
const STD_HAS_SIMD_CONTAINS: bool = cfg!(any(
    all(target_arch = "x86_64", target_feature = "sse2"),
    all(target_arch = "loongarch64", target_feature = "lsx"),
    all(target_arch = "aarch64", target_feature = "neon")
));
const STD_CONTAINS_BLOCK: usize = 16;

// `memmem::rfind` has no prefilter, so outside the screen `rfind` uses
// `str::rfind` once `haystack >= 24 + 2 * needle`: a one-shot `memmem::rfind`
// uses Rabin-Karp below 64 bytes, whose cost grows with the candidate
// positions, while std's setup grows with the needle.
const STD_RFIND_BASE: usize = 24;
const STD_RFIND_PER_NEEDLE_BYTE: usize = 2;

#[inline]
fn std_rfind_is_faster(haystack: usize, needle: usize) -> bool {
    haystack >= STD_RFIND_BASE.saturating_add(needle.saturating_mul(STD_RFIND_PER_NEEDLE_BYTE))
}

/// Needles of 2 up to this many bytes are found by [`find_short`] and
/// [`rfind_short`] in short haystacks, and by `split_str` at any length. The
/// middle of a candidate is checked with `compare::bytes_eq`, which handles up
/// to 64 bytes without a `memcmp` call.
pub(crate) const SHORT_NEEDLE_MAX: usize = 32;

/// Candidate positions screened together by [`find_short`].
const SCREEN_BLOCK: usize = 16;

/// `find`, `rfind`, and `contains` without std's SIMD fast path screen only
/// haystacks shorter than this.
const SCREENED_MAX_HAYSTACK: usize = 256;

/// `find` screens haystacks shorter than this for a needle of `needle` bytes.
///
/// On aarch64 a one-shot `memmem::find` uses NEON from 64 bytes and overtook
/// the screen sooner for short needles on the `ubuntu-24.04-arm` runner: from
/// 64 bytes for 2- and 3-byte needles and from 160 bytes for 4- to 15-byte
/// needles. Longer needles, and every needle on x86_64, gained up to the
/// 256-byte limit.
#[inline]
const fn find_screen_limit(needle: usize) -> usize {
    if cfg!(target_arch = "aarch64") {
        match needle {
            0..=3 => 64,
            4..=15 => 160,
            _ => SCREENED_MAX_HAYSTACK,
        }
    } else {
        SCREENED_MAX_HAYSTACK
    }
}

/// Positions that pass the screen but not the full comparison, tolerated in
/// one search before the caller hands the rest of the text to a Two-Way
/// searcher. Ordinary text rarely has any; text built to defeat the screen pays
/// for at most this many.
const SHORT_NEEDLE_FALSE_CANDIDATES: usize = 4;

/// The outcome of [`find_short`] and [`rfind_short`].
pub(crate) enum ShortSearch {
    /// The match nearest the scan's start begins here: the first match for
    /// [`find_short`], the last for [`rfind_short`].
    Found(usize),
    /// The text holds no match.
    Missing,
    /// More than [`SHORT_NEEDLE_FALSE_CANDIDATES`] positions passed the
    /// screen without matching, the last at this position; a Two-Way searcher
    /// suits this text better. Every position the scan has passed is ruled
    /// out, including this one: those before it for [`find_short`], those
    /// after it for [`rfind_short`]. The position holds the needle's first
    /// byte, so it is a character boundary.
    TooManyFalseCandidates(usize),
}

/// Returns a bit for each of the [`SCREEN_BLOCK`] positions from `start`
/// whose byte is `first` and whose byte `span` later is `last`. The fixed-size
/// loop has no branch on the data, so the compiler turns it into vector
/// compares. The caller keeps `start + span + SCREEN_BLOCK` within `bytes`.
#[inline(always)]
fn screen_block(bytes: &[u8], start: usize, span: usize, first: u8, last: u8) -> u32 {
    let heads: &[u8; SCREEN_BLOCK] = bytes[start..start + SCREEN_BLOCK]
        .try_into()
        .expect("a block of heads");
    let tails: &[u8; SCREEN_BLOCK] = bytes[start + span..start + span + SCREEN_BLOCK]
        .try_into()
        .expect("a block of tails");
    let mut hits = 0u32;
    for i in 0..SCREEN_BLOCK {
        hits |= u32::from((heads[i] == first) & (tails[i] == last)) << i;
    }
    hits
}

/// Returns the first position of `needle`, of 2 to [`SHORT_NEEDLE_MAX`]
/// bytes, in `haystack`.
///
/// Positions are screened [`SCREEN_BLOCK`] at a time on the needle's first and
/// last bytes, in a fixed-size loop the compiler turns into vector compares,
/// and only positions that pass compare the middle. Text that merely repeats
/// the separator's first byte, or its leading bytes, produces no candidates.
#[inline]
pub(crate) fn find_short(haystack: &str, needle: &str) -> ShortSearch {
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
        let mut hits = screen_block(bytes, at, span, first, last_byte);
        while hits != 0 {
            let candidate = at + hits.trailing_zeros() as usize;
            if middle_matches(candidate) {
                return ShortSearch::Found(candidate);
            }
            false_candidates += 1;
            if false_candidates > SHORT_NEEDLE_FALSE_CANDIDATES {
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
/// Returns the last position of `needle`, of 2 to [`SHORT_NEEDLE_MAX`] bytes,
/// in `haystack`: [`find_short`] scanning from the end.
///
/// `TooManyFalseCandidates(position)` means no match starts at or after the
/// position, which holds the needle's first byte.
#[inline]
fn rfind_short(haystack: &str, needle: &str) -> ShortSearch {
    let bytes = haystack.as_bytes();
    let pattern = needle.as_bytes();
    let span = pattern.len() - 1;
    let (first, last_byte, middle) = (pattern[0], pattern[span], &pattern[1..span]);
    let Some(last) = bytes.len().checked_sub(pattern.len()) else {
        return ShortSearch::Missing;
    };
    let middle_matches = |at: usize| crate::compare::bytes_eq(&bytes[at + 1..at + span], middle);
    let mut false_candidates = 0;

    // Positions below `end` are still unscanned.
    let mut end = last + 1;
    while end >= SCREEN_BLOCK {
        let start = end - SCREEN_BLOCK;
        let mut hits = screen_block(bytes, start, span, first, last_byte);
        while hits != 0 {
            let offset = 31 - hits.leading_zeros() as usize;
            let candidate = start + offset;
            if middle_matches(candidate) {
                return ShortSearch::Found(candidate);
            }
            false_candidates += 1;
            if false_candidates > SHORT_NEEDLE_FALSE_CANDIDATES {
                return ShortSearch::TooManyFalseCandidates(candidate);
            }
            hits &= !(1 << offset);
        }
        end = start;
    }
    match (0..end)
        .rev()
        .find(|&i| bytes[i] == first && bytes[i + span] == last_byte && middle_matches(i))
    {
        Some(found) => ShortSearch::Found(found),
        None => ShortSearch::Missing,
    }
}

/// Answers a search whose needle is at least as long as the haystack: only
/// an identical haystack contains it, at offset 0.
#[inline]
fn whole_match(haystack: &str, needle: &str) -> bool {
    crate::compare::bytes_eq(haystack.as_bytes(), needle.as_bytes())
}

#[inline]
pub(crate) fn contains_str(haystack: &str, needle: &str) -> bool {
    match needle.len() {
        0 => true,
        1 => memchr::memchr(needle.as_bytes()[0], haystack.as_bytes()).is_some(),
        n if n >= haystack.len() => whole_match(haystack, needle),
        n if STD_HAS_SIMD_CONTAINS
            && n <= SHORT_NEEDLE_MAX
            && haystack.len() >= STD_CONTAINS_BLOCK + n - 1 =>
        {
            std_contains(haystack, needle)
        }
        // Everything else, the screen or `memmem`, is one out-of-line call,
        // which keeps this inlined body about as small as before the screen.
        _ => contains_without_std_simd(haystack, needle),
    }
}

// `str::contains` and `str::rfind` with a `&str` pattern are called only
// through these wrappers, and each wrapper is the only call site of its search
// in the crate. Keep it that way. With one call site, LLVM inlines the whole
// std search into the wrapper. A second call site anywhere in the crate makes
// it keep the search as a shared function instead, and the wrapper becomes a
// jump to it: the instruction-count gate measured 7 more instructions on every
// short `contains` when the screened hand-off called `str::contains` directly.
//
// `str::find` is called directly, only from the cold hand-offs of
// `find_screened` and `split_str`. A third wrapper for it moved core's shared
// Two-Way code to another codegen unit in default builds, which made
// `std_rfind` on long haystacks 9 to 21% slower.
//
// The wrappers and the screened searches also stay out of line, each as one
// call that returns the final answer: inlined into the dispatch, or followed
// by more work in it, their state enlarged the stack frame of every other arm
// by 2 to 6 ns per call.
#[inline(never)]
fn std_contains(haystack: &str, needle: &str) -> bool {
    haystack.contains(needle)
}

#[inline(never)]
fn std_rfind(haystack: &str, needle: &str) -> Option<usize> {
    haystack.rfind(needle)
}

#[inline]
pub(crate) fn find_str(haystack: &str, needle: &str) -> Option<usize> {
    match needle.len() {
        0 => Some(0),
        1 => memchr::memchr(needle.as_bytes()[0], haystack.as_bytes()),
        n if n >= haystack.len() => whole_match(haystack, needle).then_some(0),
        n if n <= SHORT_NEEDLE_MAX && haystack.len() < find_screen_limit(n) => {
            find_screened(haystack, needle)
        }
        _ => memchr::memmem::find(haystack.as_bytes(), needle.as_bytes()),
    }
}

#[inline]
pub(crate) fn rfind_str(haystack: &str, needle: &str) -> Option<usize> {
    match needle.len() {
        0 => Some(haystack.len()),
        1 => memchr::memrchr(needle.as_bytes()[0], haystack.as_bytes()),
        n if n >= haystack.len() => whole_match(haystack, needle).then_some(0),
        n if n <= SHORT_NEEDLE_MAX && haystack.len() < SCREENED_MAX_HAYSTACK => {
            rfind_screened(haystack, needle)
        }
        n if std_rfind_is_faster(haystack.len(), n) => std_rfind(haystack, needle),
        _ => memchr::memmem::rfind(haystack.as_bytes(), needle.as_bytes()),
    }
}

// After too many false candidates the screened searches hand the text they
// have not ruled out to a Two-Way searcher. A false candidate holds the
// needle's first byte, which begins a character, so it is a character
// boundary.

/// `contains` for the inputs std's SIMD fast path does not take: the screen
/// for short needles in short haystacks, `memmem` for the rest.
#[inline(never)]
fn contains_without_std_simd(haystack: &str, needle: &str) -> bool {
    if needle.len() <= SHORT_NEEDLE_MAX
        && (STD_HAS_SIMD_CONTAINS || haystack.len() < SCREENED_MAX_HAYSTACK)
    {
        return match find_short(haystack, needle) {
            ShortSearch::Found(_) => true,
            ShortSearch::Missing => false,
            ShortSearch::TooManyFalseCandidates(from) => std_contains(&haystack[from..], needle),
        };
    }
    memchr::memmem::find(haystack.as_bytes(), needle.as_bytes()).is_some()
}

#[inline(never)]
fn find_screened(haystack: &str, needle: &str) -> Option<usize> {
    match find_short(haystack, needle) {
        ShortSearch::Found(at) => Some(at),
        ShortSearch::Missing => None,
        ShortSearch::TooManyFalseCandidates(from) => {
            haystack[from..].find(needle).map(|offset| from + offset)
        }
    }
}

#[inline(never)]
fn rfind_screened(haystack: &str, needle: &str) -> Option<usize> {
    match rfind_short(haystack, needle) {
        ShortSearch::Found(at) => Some(at),
        ShortSearch::Missing => None,
        ShortSearch::TooManyFalseCandidates(before) => {
            // No match starts at or after `before`, so any match ends by
            // `before + needle.len() - 1`. The text is cut at the first
            // character boundary from there; the up to three extra bytes
            // cannot hold a match start, as those positions were ruled out.
            let mut end = before + needle.len() - 1;
            while !haystack.is_char_boundary(end) {
                end += 1;
            }
            std_rfind(&haystack[..end], needle)
        }
    }
}

/// Reusable substring finder for repeated searches with the same needle.
pub struct CheetahFinder<'a> {
    needle: &'a str,
    finder: Option<memchr::memmem::Finder<'a>>,
}

impl<'a> CheetahFinder<'a> {
    /// Creates a reusable finder for `needle`.
    #[inline]
    pub fn new(needle: &'a str) -> Self {
        let finder = (needle.len() > 1).then(|| memchr::memmem::Finder::new(needle.as_bytes()));
        Self { needle, finder }
    }

    /// Returns the needle used by this finder.
    #[inline]
    pub fn needle(&self) -> &'a str {
        self.needle
    }

    /// Finds the first occurrence of the needle in `haystack`.
    #[inline]
    pub fn find_in<S>(&self, haystack: &S) -> Option<usize>
    where
        S: AsRef<str> + ?Sized,
    {
        let haystack = haystack.as_ref().as_bytes();

        if self.needle.is_empty() {
            return Some(0);
        }

        if self.needle.len() == 1 {
            return memchr::memchr(self.needle.as_bytes()[0], haystack);
        }

        self.finder
            .as_ref()
            .and_then(|finder| finder.find(haystack))
    }

    /// Returns whether the needle occurs in `haystack`.
    #[inline]
    pub fn is_match<S>(&self, haystack: &S) -> bool
    where
        S: AsRef<str> + ?Sized,
    {
        self.find_in(haystack).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::{contains_without_std_simd, find_screen_limit, SCREENED_MAX_HAYSTACK};
    use alloc::format;
    use alloc::string::String;

    /// On targets with std's SIMD `contains`, `contains_str` only screens
    /// haystacks too short to fill a screening block. Calling the screened
    /// path directly runs the blocks and the hand-off on every target, as
    /// targets without that search do through `contains_str`.
    #[test]
    fn screened_contains_matches_str_with_full_blocks() {
        let mut texts: alloc::vec::Vec<String> = alloc::vec::Vec::new();
        for len in [16usize, 17, 31, 32, 33, 47, 48, 64, 100, 128, 200, 255] {
            texts.push("xy".chars().cycle().take(len).collect());
        }
        texts.push(format!("{}a_b", "aXb".repeat(30)));
        texts.push(format!("a_b{}", "aXb".repeat(30)));
        texts.push("\u{e9}X\u{e9}".repeat(20) + "\u{e9}_\u{e9}");
        for text in &texts {
            for needle in [
                "xy",
                "yx",
                "yy",
                "a_b",
                "<-->",
                "\u{e9}_\u{e9}",
                "exactly-thirty-two-bytes-long!!!",
            ] {
                if needle.len() >= text.len() || text.len() >= SCREENED_MAX_HAYSTACK {
                    continue;
                }
                assert_eq!(
                    contains_without_std_simd(text, needle),
                    text.contains(needle),
                    "{text:?} {needle:?}"
                );
                // A match placed at every block boundary and in the tail.
                for at in [0, 15, 16, 17, 31, 32, text.len() - needle.len()] {
                    if at + needle.len() > text.len() || !text.is_char_boundary(at) {
                        continue;
                    }
                    let end = at + needle.len();
                    if !text.is_char_boundary(end) {
                        continue;
                    }
                    let mut placed = text.clone();
                    placed.replace_range(at..end, needle);
                    assert_eq!(
                        contains_without_std_simd(&placed, needle),
                        placed.contains(needle),
                        "{placed:?} {needle:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn find_screen_limit_stays_within_the_screened_range() {
        for needle in 2..=super::SHORT_NEEDLE_MAX {
            let limit = find_screen_limit(needle);
            assert!(limit <= SCREENED_MAX_HAYSTACK);
            assert!(
                limit > needle,
                "a needle of {needle} bytes must be screenable"
            );
        }
        if cfg!(target_arch = "aarch64") {
            assert_eq!(find_screen_limit(2), 64);
            assert_eq!(find_screen_limit(8), 160);
            assert_eq!(find_screen_limit(16), SCREENED_MAX_HAYSTACK);
        } else {
            assert_eq!(find_screen_limit(2), SCREENED_MAX_HAYSTACK);
        }
    }
}
