use alloc::string::String;
use core::str;

/// Maximum capacity for inline string storage (23 bytes + 1 byte for length = 24 bytes total).
pub(crate) const INLINE_CAPACITY: usize = 23;

/// Shared inline storage for short UTF-8 strings.
#[derive(Clone, Copy)]
pub(crate) struct InlineStr {
    len: u8,
    data: [u8; INLINE_CAPACITY],
}

impl InlineStr {
    #[inline]
    pub(crate) const fn empty() -> Self {
        Self {
            len: 0,
            data: [0; INLINE_CAPACITY],
        }
    }

    #[inline]
    pub(crate) fn from_str(value: &str) -> Option<Self> {
        if value.len() > INLINE_CAPACITY {
            return None;
        }

        let mut inline = Self::empty();
        inline.data[..value.len()].copy_from_slice(value.as_bytes());
        inline.len = value.len() as u8;
        Some(inline)
    }

    #[inline]
    pub(crate) fn as_str(&self) -> &str {
        // SAFETY: InlineStr is only constructed from valid UTF-8 strings.
        unsafe { str::from_utf8_unchecked(self.as_bytes()) }
    }

    #[inline]
    pub(crate) fn as_bytes(&self) -> &[u8] {
        &self.data[..self.len as usize]
    }

    #[inline]
    pub(crate) fn len(&self) -> usize {
        self.len as usize
    }

    #[inline]
    pub(crate) fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline]
    pub(crate) fn push_str(&mut self, value: &str) -> bool {
        let current_len = self.len();
        let total_len = current_len + value.len();
        if total_len > INLINE_CAPACITY {
            return false;
        }

        self.data[current_len..total_len].copy_from_slice(value.as_bytes());
        self.len = total_len as u8;
        true
    }

    #[inline]
    pub(crate) fn into_string(self) -> String {
        // SAFETY: InlineStr is only constructed from valid UTF-8 strings.
        unsafe { String::from_utf8_unchecked(self.as_bytes().to_vec()) }
    }
}
