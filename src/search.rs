// Thresholds measured on x86_64 (i7-11700K, rustc 1.99) with 4-byte needles
// against haystacks of 20 B to 16 KiB, both hits and misses.
//
// `str::contains` has an SSE2 fast path for needles of 2..=32 bytes once the
// haystack holds one 16-byte block plus the needle; it beat a one-shot
// `memmem::find` at every such length. Below that size std falls back to a
// byte-window scan, and above 32-byte needles it builds a Two-Way searcher,
// so memmem stays the better choice there.
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

// A one-shot `memmem` search pays roughly 15-20 ns to build its searcher,
// while `str::find`/`str::rfind` build a Two-Way searcher of their own. memmem
// won on haystacks shorter than 24 bytes (every inline value), std won from
// 24 to about 128 bytes, and memmem won again for `find` beyond that.
const STD_SEARCH_MIN_HAYSTACK: usize = 24;
const MEMMEM_FIND_MIN_HAYSTACK: usize = 128;

#[inline]
pub(crate) fn contains_str(haystack: &str, needle: &str) -> bool {
    match needle.len() {
        0 => true,
        1 => memchr::memchr(needle.as_bytes()[0], haystack.as_bytes()).is_some(),
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
        _ if (STD_SEARCH_MIN_HAYSTACK..MEMMEM_FIND_MIN_HAYSTACK).contains(&haystack.len()) => {
            std_find(haystack, needle)
        }
        _ => memchr::memmem::find(haystack.as_bytes(), needle.as_bytes()),
    }
}

// `memmem::rfind` has no prefilter; from 24 bytes up `str::rfind` matched or
// beat it at every measured length.
#[inline]
pub(crate) fn rfind_str(haystack: &str, needle: &str) -> Option<usize> {
    match needle.len() {
        0 => Some(haystack.len()),
        1 => memchr::memrchr(needle.as_bytes()[0], haystack.as_bytes()),
        _ if haystack.len() < STD_SEARCH_MIN_HAYSTACK => {
            memchr::memmem::rfind(haystack.as_bytes(), needle.as_bytes())
        }
        _ => std_rfind(haystack, needle),
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
