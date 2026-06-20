use alloc::string::String;
use core::fmt;

use crate::{CheetahStr, CheetahString};

/// Append-heavy builder for constructing Cheetah string values.
///
/// `CheetahBuilder` keeps mutable construction separate from immutable
/// clone-cheap `CheetahStr` values and stable string values.
#[derive(Clone, Default)]
pub struct CheetahBuilder {
    inner: String,
}

impl CheetahBuilder {
    /// Creates an empty builder.
    #[inline]
    pub fn new() -> Self {
        Self {
            inner: String::new(),
        }
    }

    /// Creates an empty builder with at least `capacity` bytes.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: String::with_capacity(capacity),
        }
    }

    /// Creates a builder from existing owned storage.
    #[inline]
    pub fn from_string(value: String) -> Self {
        Self { inner: value }
    }

    /// Appends a string slice.
    #[inline]
    pub fn push_str(&mut self, value: &str) {
        self.inner.push_str(value);
    }

    /// Appends a character.
    #[inline]
    pub fn push(&mut self, value: char) {
        self.inner.push(value);
    }

    /// Reserves capacity for at least `additional` more bytes.
    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        self.inner.reserve(additional);
    }

    /// Clears the current contents while preserving capacity.
    #[inline]
    pub fn clear(&mut self) {
        self.inner.clear();
    }

    /// Returns the current contents.
    #[inline]
    pub fn as_str(&self) -> &str {
        self.inner.as_str()
    }

    /// Returns the current length in bytes.
    #[inline]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns whether the builder is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns the allocated capacity in bytes.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    /// Finishes into a mutable string value, preserving spare capacity when it
    /// is useful for subsequent mutation.
    #[inline]
    pub fn finish_string(self) -> CheetahString {
        CheetahString::from_string_owned(self.inner)
    }

    /// Finishes into an immutable clone-cheap string value.
    #[inline]
    pub fn finish_str(self) -> CheetahStr {
        CheetahStr::from_string(self.inner)
    }

    /// Returns the owned `String` backing this builder.
    #[inline]
    pub fn into_string(self) -> String {
        self.inner
    }
}

impl From<String> for CheetahBuilder {
    #[inline]
    fn from(value: String) -> Self {
        Self::from_string(value)
    }
}

impl From<&str> for CheetahBuilder {
    #[inline]
    fn from(value: &str) -> Self {
        let mut builder = Self::with_capacity(value.len());
        builder.push_str(value);
        builder
    }
}

impl Extend<char> for CheetahBuilder {
    #[inline]
    fn extend<T: IntoIterator<Item = char>>(&mut self, iter: T) {
        self.inner.extend(iter);
    }
}

impl<'a> Extend<&'a str> for CheetahBuilder {
    #[inline]
    fn extend<T: IntoIterator<Item = &'a str>>(&mut self, iter: T) {
        for item in iter {
            self.push_str(item);
        }
    }
}

impl fmt::Debug for CheetahBuilder {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CheetahBuilder")
            .field("value", &self.inner)
            .field("capacity", &self.inner.capacity())
            .finish()
    }
}
