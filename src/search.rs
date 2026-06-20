pub(crate) fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }

    if needle.len() == 1 {
        return memchr::memchr(needle[0], haystack);
    }

    memchr::memmem::find(haystack, needle)
}

pub(crate) fn rfind_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(haystack.len());
    }

    if needle.len() == 1 {
        return memchr::memrchr(needle[0], haystack);
    }

    memchr::memmem::rfind(haystack, needle)
}

/// Reusable substring finder for repeated searches with the same needle.
pub struct CheetahFinder<'a> {
    needle: &'a str,
    finder: Option<memchr::memmem::Finder<'a>>,
}

impl<'a> CheetahFinder<'a> {
    #[inline]
    pub fn new(needle: &'a str) -> Self {
        let finder = (needle.len() > 1).then(|| memchr::memmem::Finder::new(needle.as_bytes()));
        Self { needle, finder }
    }

    #[inline]
    pub fn needle(&self) -> &'a str {
        self.needle
    }

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

    #[inline]
    pub fn is_match<S>(&self, haystack: &S) -> bool
    where
        S: AsRef<str> + ?Sized,
    {
        self.find_in(haystack).is_some()
    }
}
