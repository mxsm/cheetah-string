use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::borrow::Borrow;
use core::cmp::Ordering;
use core::fmt::{self, Display};
use core::hash::{Hash, Hasher};
use core::ops::Add;
use core::str;

use super::repr::InnerString;
use super::CheetahString;

#[inline]
fn concatenate(left: &str, right: &str) -> CheetahString {
    CheetahString::concat(&[left, right])
}

impl PartialEq for CheetahString {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        match (&self.inner, &other.inner) {
            // Inline padding is always zero, so whole-value equality is exact
            // and avoids a variable-length `memcmp` call.
            (InnerString::Inline(left), InnerString::Inline(right)) => left == right,
            // Clones of one shared allocation are equal without reading it.
            (InnerString::Shared(left), InnerString::Shared(right)) if Arc::ptr_eq(left, right) => {
                true
            }
            _ => {
                #[cfg(all(feature = "experimental-simd", target_arch = "x86_64"))]
                {
                    crate::simd::eq_bytes(self.as_bytes(), other.as_bytes())
                }
                #[cfg(not(all(feature = "experimental-simd", target_arch = "x86_64")))]
                {
                    crate::compare::bytes_eq(self.as_bytes(), other.as_bytes())
                }
            }
        }
    }
}

impl PartialEq<str> for CheetahString {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        #[cfg(all(feature = "experimental-simd", target_arch = "x86_64"))]
        {
            crate::simd::eq_bytes(self.as_bytes(), other.as_bytes())
        }
        #[cfg(not(all(feature = "experimental-simd", target_arch = "x86_64")))]
        {
            crate::compare::bytes_eq(self.as_bytes(), other.as_bytes())
        }
    }
}

impl PartialEq<String> for CheetahString {
    #[inline]
    fn eq(&self, other: &String) -> bool {
        #[cfg(all(feature = "experimental-simd", target_arch = "x86_64"))]
        {
            crate::simd::eq_bytes(self.as_bytes(), other.as_bytes())
        }
        #[cfg(not(all(feature = "experimental-simd", target_arch = "x86_64")))]
        {
            crate::compare::bytes_eq(self.as_bytes(), other.as_bytes())
        }
    }
}

impl PartialEq<Vec<u8>> for CheetahString {
    #[inline]
    fn eq(&self, other: &Vec<u8>) -> bool {
        crate::compare::bytes_eq(self.as_bytes(), other)
    }
}

impl<'a> PartialEq<&'a str> for CheetahString {
    #[inline]
    fn eq(&self, other: &&'a str) -> bool {
        crate::compare::bytes_eq(self.as_bytes(), other.as_bytes())
    }
}

impl PartialEq<CheetahString> for str {
    #[inline]
    fn eq(&self, other: &CheetahString) -> bool {
        crate::compare::bytes_eq(self.as_bytes(), other.as_bytes())
    }
}

impl PartialEq<CheetahString> for String {
    #[inline]
    fn eq(&self, other: &CheetahString) -> bool {
        crate::compare::bytes_eq(self.as_bytes(), other.as_bytes())
    }
}

impl PartialEq<CheetahString> for &str {
    #[inline]
    fn eq(&self, other: &CheetahString) -> bool {
        crate::compare::bytes_eq(self.as_bytes(), other.as_bytes())
    }
}

impl Eq for CheetahString {}

impl PartialOrd for CheetahString {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CheetahString {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        // Two inline values compare as whole buffers, without a `memcmp` call;
        // the order is the same as `str`'s. Matching on `self` first, as the
        // `str` path does anyway, keeps the other combinations as cheap as
        // before.
        match &self.inner {
            InnerString::Inline(left) => match &other.inner {
                InnerString::Inline(right) => left.cmp_bytes(right),
                _ => left.as_str().cmp(other.as_str()),
            },
            _ => self.as_str().cmp(other.as_str()),
        }
    }
}

impl Hash for CheetahString {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}

impl Display for CheetahString {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_str().fmt(f)
    }
}

impl fmt::Debug for CheetahString {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), f)
    }
}

impl Borrow<str> for CheetahString {
    #[inline]
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

// Add trait implementations for string concatenation

impl Add<&str> for CheetahString {
    type Output = CheetahString;

    /// Concatenates a `CheetahString` with a string slice.
    ///
    /// # Examples
    ///
    /// ```
    /// use cheetah_string::CheetahString;
    ///
    /// let s = CheetahString::from("Hello");
    /// let result = s + " World";
    /// assert_eq!(result, "Hello World");
    /// ```
    #[inline]
    fn add(self, rhs: &str) -> Self::Output {
        if rhs.is_empty() {
            return self;
        }

        concatenate(self.as_str(), rhs)
    }
}

impl Add<&CheetahString> for CheetahString {
    type Output = CheetahString;

    /// Concatenates two `CheetahString` values.
    ///
    /// # Examples
    ///
    /// ```
    /// use cheetah_string::CheetahString;
    ///
    /// let s1 = CheetahString::from("Hello");
    /// let s2 = CheetahString::from(" World");
    /// let result = s1 + &s2;
    /// assert_eq!(result, "Hello World");
    /// ```
    #[inline]
    fn add(self, rhs: &CheetahString) -> Self::Output {
        self + rhs.as_str()
    }
}

impl Add<String> for CheetahString {
    type Output = CheetahString;

    /// Concatenates a `CheetahString` with a `String`.
    ///
    /// # Examples
    ///
    /// ```
    /// use cheetah_string::CheetahString;
    ///
    /// let s = CheetahString::from("Hello");
    /// let result = s + String::from(" World");
    /// assert_eq!(result, "Hello World");
    /// ```
    #[inline]
    fn add(self, rhs: String) -> Self::Output {
        if rhs.is_empty() {
            return self;
        }

        if self.is_empty() {
            return CheetahString::from_string(rhs);
        }

        concatenate(self.as_str(), &rhs)
    }
}
