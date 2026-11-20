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
///
/// Bytes after `len` are always zero, so two values hold the same text exactly
/// when the whole structure is equal. The derived `PartialEq` relies on that.
#[derive(Clone, Copy, PartialEq, Eq)]
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

        let b = value.as_bytes();
        let n = b.len();
        let len = InlineLength::new(n)?;
        // Gather the bytes with overlapping fixed-size loads and write the
        // whole value once; a variable-length memcpy call costs about 5 ns.
        let (w0, w1, w2) = if n >= 16 {
            let w0 = u64::from_le_bytes(b[..8].try_into().expect("8 bytes"));
            let w1 = u64::from_le_bytes(b[8..16].try_into().expect("8 bytes"));
            let w2 = if n == 16 {
                0
            } else {
                u64::from_le_bytes(b[n - 8..].try_into().expect("8 bytes")) >> ((24 - n) * 8)
            };
            (w0, w1, w2)
        } else if n >= 8 {
            let w0 = u64::from_le_bytes(b[..8].try_into().expect("8 bytes"));
            let w1 = if n == 8 {
                0
            } else {
                u64::from_le_bytes(b[n - 8..].try_into().expect("8 bytes")) >> ((16 - n) * 8)
            };
            (w0, w1, 0)
        } else if n >= 4 {
            let first = u64::from(u32::from_le_bytes(b[..4].try_into().expect("4 bytes")));
            let last = u64::from(u32::from_le_bytes(b[n - 4..].try_into().expect("4 bytes")));
            (first | (last << ((n - 4) * 8)), 0, 0)
        } else if n > 0 {
            let w0 = u64::from(b[0])
                | (u64::from(b[n / 2]) << ((n / 2) * 8))
                | (u64::from(b[n - 1]) << ((n - 1) * 8));
            (w0, 0, 0)
        } else {
            (0, 0, 0)
        };
        let mut data = [0u8; INLINE_CAPACITY];
        data[..8].copy_from_slice(&w0.to_le_bytes());
        data[8..16].copy_from_slice(&w1.to_le_bytes());
        data[16..].copy_from_slice(&w2.to_le_bytes()[..7]);
        Some(Self { len, data })
    }

    /// Wraps an assembly buffer whose first `len` bytes are already written.
    ///
    /// # Safety
    ///
    /// `data[..len]` must be valid UTF-8, every byte of `data[len..]` must be
    /// zero, and `len` must not exceed [`INLINE_CAPACITY`].
    #[inline]
    pub(crate) unsafe fn from_assembled(data: [u8; INLINE_CAPACITY], len: usize) -> Self {
        Self {
            len: InlineLength::new(len).expect("assembled length fits inline"),
            data,
        }
    }

    #[inline]
    pub(crate) fn as_str(&self) -> &str {
        // SAFETY: InlineStr is only constructed from valid UTF-8 strings.
        unsafe { str::from_utf8_unchecked(self.as_bytes()) }
    }

    #[inline]
    pub(crate) fn as_bytes(&self) -> &[u8] {
        // `InlineLength` never exceeds the capacity. The clamp lets the
        // compiler prove the range, so the slice bounds check and its panic
        // path disappear from every inlined call site.
        &self.data[..self.len.get().min(INLINE_CAPACITY)]
    }

    /// Appends `value` when the result still fits inline.
    ///
    /// Returns `false` and leaves `self` unchanged when it does not. Only
    /// whole `&str` pieces are appended and bytes past the new length stay
    /// zero, so the UTF-8 and padding invariants hold.
    #[inline]
    pub(crate) fn push_str(&mut self, value: &str) -> bool {
        let len = self.len();
        let end = len.saturating_add(value.len());
        match InlineLength::new(end) {
            Some(new_len) => {
                self.data[len..end].copy_from_slice(value.as_bytes());
                self.len = new_len;
                true
            }
            None => false,
        }
    }

    /// Converts ASCII letters to upper case in place.
    ///
    /// ASCII case mapping never touches non-ASCII bytes or the zero padding,
    /// so the UTF-8 and padding invariants are preserved.
    #[inline]
    pub(crate) fn make_ascii_uppercase(&mut self) {
        let len = self.len();
        self.data[..len].make_ascii_uppercase();
    }

    /// Converts ASCII letters to lower case in place.
    #[inline]
    pub(crate) fn make_ascii_lowercase(&mut self) {
        let len = self.len();
        self.data[..len].make_ascii_lowercase();
    }

    #[inline]
    pub(crate) fn len(&self) -> usize {
        self.len.get()
    }

    /// Orders two values as `str` does: byte by byte, then shorter first.
    ///
    /// Bytes past `len` are zero, so comparing the whole buffers as
    /// big-endian words finds the first differing byte. When the buffers are
    /// equal, one text is the other followed only by zero bytes, and the
    /// shorter one sorts first. No `memcmp` call is needed.
    #[inline]
    pub(crate) fn cmp_bytes(&self, other: &Self) -> core::cmp::Ordering {
        self.ordered_words()
            .cmp(&other.ordered_words())
            .then(self.len.get().cmp(&other.len.get()))
    }

    /// The buffer as three big-endian words whose order is the byte order.
    ///
    /// The last word overlaps the second by one byte. Tuples compare the
    /// last word only when the second words are equal, and then the shared
    /// byte is equal too, so the overlap does not change the order and saves
    /// assembling a 7-byte word.
    #[inline]
    fn ordered_words(&self) -> (u64, u64, u64) {
        let data = &self.data;
        let word = |at: usize| u64::from_be_bytes(data[at..at + 8].try_into().expect("8 bytes"));
        (word(0), word(8), word(INLINE_CAPACITY - 8))
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

    #[test]
    fn register_copy_matches_a_byte_copy_for_every_inline_length() {
        // ASCII covers every length; the multi-byte alphabets put the
        // overlapping loads across character boundaries.
        for alphabet in [
            "abcdefghijklmnopqrstuvwxyz",
            "\u{e9}a\u{1f980}b\u{4e2d}",
            "\u{0}\u{7f}\u{80}",
        ] {
            let mut text = alloc::string::String::new();
            for character in alphabet.chars().cycle() {
                if text.len() + character.len_utf8() > INLINE_CAPACITY {
                    break;
                }
                text.push(character);
            }
            for end in (0..=text.len()).filter(|&end| text.is_char_boundary(end)) {
                let value = &text[..end];
                let inline = InlineStr::from_str(value).expect("fits inline");
                assert_eq!(inline.as_str(), value);
                assert_eq!(inline.len(), value.len());

                let mut expected = [0u8; INLINE_CAPACITY];
                expected[..value.len()].copy_from_slice(value.as_bytes());
                assert_eq!(
                    inline.data, expected,
                    "padding must stay zero for {value:?}"
                );
            }
        }
        assert!(InlineStr::from_str("twenty-four-bytes-value!").is_none());
    }

    #[test]
    fn whole_buffer_order_matches_str_order() {
        use alloc::string::String;
        use alloc::vec::Vec;

        let mut texts: Vec<String> = [
            "",
            "\u{0}",
            "\u{0}\u{0}",
            "a",
            "a\u{0}",
            "a\u{0}\u{0}",
            "a\u{0}b",
            "a\u{1}",
            "ab",
            "b",
            "\u{7f}",
            "\u{e9}",
            "\u{e9}a",
            "\u{1f980}",
            "orders@group-a",
            "orders@group-b",
            "orders",
        ]
        .iter()
        .map(|text| String::from(*text))
        .collect();
        // Differences in each word and at the bytes where words meet or
        // overlap: 7/8, 15/16, and the last byte, 22.
        for at in [0usize, 6, 7, 8, 9, 14, 15, 16, 17, 21, 22] {
            for len in [at + 1, INLINE_CAPACITY] {
                if len > INLINE_CAPACITY {
                    continue;
                }
                let base: String = "m".repeat(len);
                for replacement in ["a", "z", "\u{0}"] {
                    let mut text = base.clone();
                    text.replace_range(at..at + 1, replacement);
                    texts.push(text);
                }
                texts.push(base);
            }
        }
        for left in &texts {
            for right in &texts {
                let (a, b) = (
                    InlineStr::from_str(left).expect("fits inline"),
                    InlineStr::from_str(right).expect("fits inline"),
                );
                assert_eq!(
                    a.cmp_bytes(&b),
                    left.as_str().cmp(right.as_str()),
                    "{left:?} vs {right:?}"
                );
            }
        }
    }
}
