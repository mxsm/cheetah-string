use alloc::string::String;
use core::str;

/// Maximum capacity for inline string storage (23 bytes + 1 byte for length = 24 bytes total).
pub(crate) const INLINE_CAPACITY: usize = 23;

// The unused discriminants are layout niches for `InnerString`. Supported
// toolchains use them to keep the stable Rust enum at 24 bytes without
// integerizing or reconstructing pointers. Layout tests gate that optimization.
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
enum InlineLength {
    L0 = 0,
    L1,
    L2,
    L3,
    L4,
    L5,
    L6,
    L7,
    L8,
    L9,
    L10,
    L11,
    L12,
    L13,
    L14,
    L15,
    L16,
    L17,
    L18,
    L19,
    L20,
    L21,
    L22,
    L23,
}

impl InlineLength {
    #[inline]
    const fn new(len: usize) -> Option<Self> {
        match len {
            0 => Some(Self::L0),
            1 => Some(Self::L1),
            2 => Some(Self::L2),
            3 => Some(Self::L3),
            4 => Some(Self::L4),
            5 => Some(Self::L5),
            6 => Some(Self::L6),
            7 => Some(Self::L7),
            8 => Some(Self::L8),
            9 => Some(Self::L9),
            10 => Some(Self::L10),
            11 => Some(Self::L11),
            12 => Some(Self::L12),
            13 => Some(Self::L13),
            14 => Some(Self::L14),
            15 => Some(Self::L15),
            16 => Some(Self::L16),
            17 => Some(Self::L17),
            18 => Some(Self::L18),
            19 => Some(Self::L19),
            20 => Some(Self::L20),
            21 => Some(Self::L21),
            22 => Some(Self::L22),
            23 => Some(Self::L23),
            _ => None,
        }
    }

    #[inline]
    const fn get(self) -> usize {
        self as usize
    }
}

/// Shared inline storage for short UTF-8 strings.
#[derive(Clone, Copy)]
pub(crate) struct InlineStr {
    len: InlineLength,
    data: [u8; INLINE_CAPACITY],
}

impl InlineStr {
    #[inline]
    pub(crate) const fn empty() -> Self {
        Self {
            len: InlineLength::L0,
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
        inline.len = InlineLength::new(value.len())?;
        Some(inline)
    }

    #[inline]
    pub(crate) fn as_str(&self) -> &str {
        // SAFETY: InlineStr is only constructed from valid UTF-8 strings.
        unsafe { str::from_utf8_unchecked(self.as_bytes()) }
    }

    #[inline]
    pub(crate) fn as_bytes(&self) -> &[u8] {
        &self.data[..self.len.get()]
    }

    #[inline]
    pub(crate) fn len(&self) -> usize {
        self.len.get()
    }

    #[inline]
    pub(crate) fn is_empty(&self) -> bool {
        self.len == InlineLength::L0
    }

    #[inline]
    pub(crate) fn into_string(self) -> String {
        // SAFETY: InlineStr is only constructed from valid UTF-8 strings.
        unsafe { String::from_utf8_unchecked(self.as_bytes().to_vec()) }
    }
}

#[cfg(test)]
mod tests {
    use super::{InlineLength, InlineStr, INLINE_CAPACITY};
    use core::mem::size_of;

    #[test]
    fn constrained_length_preserves_all_inline_lengths_and_layout_niches() {
        assert_eq!(size_of::<InlineLength>(), 1);
        assert_eq!(size_of::<InlineStr>(), 24);

        for len in 0..=INLINE_CAPACITY {
            let length = InlineLength::new(len).expect("inline length must be represented");
            assert_eq!(length.get(), len);
        }
        assert!(InlineLength::new(INLINE_CAPACITY + 1).is_none());
        assert!(InlineLength::new(usize::MAX).is_none());
    }
}
