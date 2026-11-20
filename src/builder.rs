use alloc::string::String;
use core::fmt;

use crate::inline::{InlineStr, INLINE_CAPACITY};
use crate::CheetahString;

/// Append-heavy builder for constructing Cheetah string values.
///
/// `CheetahBuilder` keeps mutable construction separate from immutable,
/// clone-cheap [`CheetahString`] values.
///
/// A builder created with [`new`](Self::new) or with a capacity of at most 23
/// bytes keeps its contents on the stack until they outgrow inline storage,
/// so building a short key and calling [`finish`](Self::finish) allocates
/// nothing. A larger [`with_capacity`](Self::with_capacity) allocates that
/// capacity up front, as `String::with_capacity` does.
#[derive(Clone)]
pub struct CheetahBuilder {
    inner: Buffer,
}

#[derive(Clone)]
enum Buffer {
    Inline(InlineStr),
    Heap(String),
}

impl Default for CheetahBuilder {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl CheetahBuilder {
    /// Creates an empty builder that starts in inline storage.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Buffer::Inline(InlineStr::empty()),
        }
    }

    /// Creates an empty builder with room for at least `capacity` bytes.
    ///
    /// Up to 23 bytes fit inline without allocating; a larger capacity is
    /// allocated immediately.
    #[inline]
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        if capacity <= INLINE_CAPACITY {
            Self::new()
        } else {
            Self {
                inner: Buffer::Heap(String::with_capacity(capacity)),
            }
        }
    }

    /// Creates a builder from existing owned storage.
    #[inline]
    #[must_use]
    pub fn from_string(value: String) -> Self {
        Self {
            inner: Buffer::Heap(value),
        }
    }

    /// Moves inline contents to a heap buffer with room for `additional`
    /// more bytes.
    #[cold]
    fn spill(&mut self, additional: usize) -> &mut String {
        if let Buffer::Inline(inline) = &self.inner {
            let needed = inline.len().saturating_add(additional);
            let mut heap = String::with_capacity(needed.max(2 * INLINE_CAPACITY));
            heap.push_str(inline.as_str());
            self.inner = Buffer::Heap(heap);
        }
        match &mut self.inner {
            Buffer::Heap(heap) => heap,
            Buffer::Inline(_) => unreachable!("inline contents were just moved to the heap"),
        }
    }

    /// Appends a string slice.
    #[inline]
    pub fn push_str(&mut self, value: &str) {
        match &mut self.inner {
            Buffer::Inline(inline) => {
                if !inline.push_str(value) {
                    self.spill(value.len()).push_str(value);
                }
            }
            Buffer::Heap(heap) => heap.push_str(value),
        }
    }

    /// Appends a character.
    #[inline]
    pub fn push(&mut self, value: char) {
        match &mut self.inner {
            Buffer::Inline(_) => {
                let mut encoded = [0; 4];
                self.push_str(value.encode_utf8(&mut encoded));
            }
            // `String::push` has a one-byte fast path for ASCII.
            Buffer::Heap(heap) => heap.push(value),
        }
    }

    /// Reserves capacity for at least `additional` more bytes.
    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        match &mut self.inner {
            Buffer::Inline(inline) => {
                if inline.len().saturating_add(additional) > INLINE_CAPACITY {
                    self.spill(additional);
                }
            }
            Buffer::Heap(heap) => heap.reserve(additional),
        }
    }

    /// Clears the current contents while preserving capacity.
    #[inline]
    pub fn clear(&mut self) {
        match &mut self.inner {
            Buffer::Inline(inline) => *inline = InlineStr::empty(),
            Buffer::Heap(heap) => heap.clear(),
        }
    }

    /// Returns the current contents.
    #[inline]
    #[must_use]
    pub fn as_str(&self) -> &str {
        match &self.inner {
            Buffer::Inline(inline) => inline.as_str(),
            Buffer::Heap(heap) => heap.as_str(),
        }
    }

    /// Returns the current length in bytes.
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        match &self.inner {
            Buffer::Inline(inline) => inline.len(),
            Buffer::Heap(heap) => heap.len(),
        }
    }

    /// Returns whether the builder is empty.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the capacity in bytes: 23 while the contents are inline,
    /// otherwise the allocated capacity.
    #[inline]
    #[must_use]
    pub fn capacity(&self) -> usize {
        match &self.inner {
            Buffer::Inline(_) => INLINE_CAPACITY,
            Buffer::Heap(heap) => heap.capacity(),
        }
    }

    /// Freezes this builder into the canonical clone-cheap string value.
    ///
    /// Inline contents become an inline value without allocating. Use
    /// [`CheetahBuilder::into_string`] when construction is followed by more
    /// mutation or when spare capacity must be retained.
    #[inline]
    #[must_use]
    pub fn finish(self) -> CheetahString {
        match self.inner {
            Buffer::Inline(inline) => CheetahString::from_inline(inline),
            Buffer::Heap(heap) => CheetahString::from_string(heap),
        }
    }

    /// Freezes the builder into the canonical clone-cheap string value.
    ///
    /// This compatibility name is retained so downstream v2 consumers can
    /// adopt the immutable v3 core without a source migration.
    #[deprecated(since = "3.0.0", note = "use finish()")]
    #[inline]
    #[must_use]
    pub fn finish_string(self) -> CheetahString {
        self.finish()
    }

    /// Returns the contents as an owned `String`.
    ///
    /// Heap contents are returned without copying, keeping their spare
    /// capacity; inline contents are copied into a new `String`.
    #[inline]
    #[must_use]
    pub fn into_string(self) -> String {
        match self.inner {
            Buffer::Inline(inline) => String::from(inline.as_str()),
            Buffer::Heap(heap) => heap,
        }
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
        for character in iter {
            self.push(character);
        }
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

impl fmt::Write for CheetahBuilder {
    #[inline]
    fn write_str(&mut self, value: &str) -> fmt::Result {
        self.push_str(value);
        Ok(())
    }

    #[inline]
    fn write_char(&mut self, value: char) -> fmt::Result {
        self.push(value);
        Ok(())
    }
}

impl fmt::Debug for CheetahBuilder {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CheetahBuilder")
            .field("value", &self.as_str())
            .field("capacity", &self.capacity())
            .finish()
    }
}
