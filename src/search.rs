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
// `str::contains` has a SIMD fast path for needles of 2..=32 bytes once the
// haystack holds one 16-byte block plus the needle, and it beat a one-shot
// `memmem::find` at almost every such length. Below that size std falls back
// to a byte-window scan, and above 32-byte needles it builds a Two-Way
// searcher, so memmem stays the better choice there.
//
// std only has that fast path on these targets; elsewhere it always builds a
// Two-Way searcher, so memmem remains the better default.
const STD_HAS_SIMD_CONTAINS: bool = cfg!(any(
    all(target_arch = "x86_64", target_feature = "sse2"),
    all(target_arch = "loongarch64", target_feature = "lsx"),
    all(target_arch = "aarch64", target_feature = "neon")
));
const STD_CONTAINS_MAX_NEEDLE: usize = 32;
const STD_CONTAINS_BLOCK: usize = 16;

// One-shot `memmem::find` and `memmem::rfind` use Rabin-Karp below 64 bytes,
// whose cost grows with the number of candidate positions, while `str::find`
// and `str::rfind` build a Two-Way searcher whose setup grows with the needle.
// std therefore wins only when the haystack is long relative to the needle:
//
// - `find` uses std when `16 + 3 * needle <= haystack < 64`. From 64 bytes
//   memmem's SIMD prefilter won at every needle length.
// - `rfind` uses std when `haystack >= 24 + 2 * needle`. `memmem::rfind` has
//   no prefilter, so std stays ahead on long haystacks.
//
// On both targets these rules kept every measured case within 1.6x of the
// faster strategy and the geometric mean within 3% of it. The fixed cutoffs
// they replace, std for `find` from 24 to 127 bytes and for `rfind` from 24
// bytes, were up to 3.9x slower with 8 to 32-byte needles.
const STD_FIND_BASE: usize = 16;
const STD_FIND_PER_NEEDLE_BYTE: usize = 3;
const MEMMEM_FIND_MIN_HAYSTACK: usize = 64;
const STD_RFIND_BASE: usize = 24;
const STD_RFIND_PER_NEEDLE_BYTE: usize = 2;

#[inline]
fn std_find_is_faster(haystack: usize, needle: usize) -> bool {
    haystack < MEMMEM_FIND_MIN_HAYSTACK
        && haystack >= STD_FIND_BASE.saturating_add(needle.saturating_mul(STD_FIND_PER_NEEDLE_BYTE))
}

#[inline]
fn std_rfind_is_faster(haystack: usize, needle: usize) -> bool {
    haystack >= STD_RFIND_BASE.saturating_add(needle.saturating_mul(STD_RFIND_PER_NEEDLE_BYTE))
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
            && n <= STD_CONTAINS_MAX_NEEDLE
            && haystack.len() >= STD_CONTAINS_BLOCK + n - 1 =>
        {
            std_contains(haystack, needle)
        }
        _ => memchr::memmem::find(haystack.as_bytes(), needle.as_bytes()).is_some(),
    }
}

// The std paths stay out of line: inlined, their searcher state enlarged the
// stack frame of the short memchr/memmem paths by about 2 ns per call.
#[inline(never)]
fn std_contains(haystack: &str, needle: &str) -> bool {
    haystack.contains(needle)
}

#[inline(never)]
fn std_find(haystack: &str, needle: &str) -> Option<usize> {
    haystack.find(needle)
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
        n if std_find_is_faster(haystack.len(), n) => std_find(haystack, needle),
        _ => memchr::memmem::find(haystack.as_bytes(), needle.as_bytes()),
    }
}

#[inline]
pub(crate) fn rfind_str(haystack: &str, needle: &str) -> Option<usize> {
    match needle.len() {
        0 => Some(haystack.len()),
        1 => memchr::memrchr(needle.as_bytes()[0], haystack.as_bytes()),
        n if n >= haystack.len() => whole_match(haystack, needle).then_some(0),
        n if std_rfind_is_faster(haystack.len(), n) => std_rfind(haystack, needle),
        _ => memchr::memmem::rfind(haystack.as_bytes(), needle.as_bytes()),
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
