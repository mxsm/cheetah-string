//! Exact-length assembly of new values from UTF-8 pieces.
//!
//! Concatenation, `replace`, and `repeat` know their final length before they
//! copy anything. Writing straight into inline storage or into the final
//! `Arc<str>` allocation avoids the intermediate `String` and the second copy
//! that freezing it would require.

use alloc::sync::Arc;
use core::mem::MaybeUninit;

use super::repr::{InnerString, INLINE_CAPACITY};
use super::CheetahString;
use crate::inline::InlineStr;

/// Sequential writer over a buffer of exactly the requested length.
///
/// Only whole `&str` pieces can be written, optionally with one ASCII byte
/// swapped for another, and [`assemble`] checks that the pieces filled the
/// buffer exactly, so the finished bytes are always the concatenation of
/// valid UTF-8 strings.
pub(super) struct Writer<'a> {
    target: Target<'a>,
    filled: usize,
}

enum Target<'a> {
    Inline(&'a mut [u8]),
    Shared(&'a mut [MaybeUninit<u8>]),
}

impl Writer<'_> {
    /// Appends `piece`.
    ///
    /// # Panics
    ///
    /// Panics if the pieces exceed the length passed to [`assemble`].
    #[inline]
    pub(super) fn push_str(&mut self, piece: &str) {
        let end = self.filled + piece.len();
        match &mut self.target {
            Target::Inline(bytes) => bytes[self.filled..end].copy_from_slice(piece.as_bytes()),
            Target::Shared(slots) => {
                slots[self.filled..end].write_copy_of_slice(piece.as_bytes());
            }
        }
        self.filled = end;
    }

    /// Appends `piece` with every `from` byte replaced by `to`.
    ///
    /// ASCII bytes never occur inside a multi-byte UTF-8 sequence, so swapping
    /// one ASCII byte for another leaves the piece valid UTF-8. The loop has
    /// no data-dependent branch and compiles to vector selects.
    ///
    /// # Panics
    ///
    /// Panics if `from` or `to` is not ASCII, or if the pieces exceed the
    /// length passed to [`assemble`].
    #[inline]
    pub(super) fn push_str_swapping_ascii(&mut self, piece: &str, from: u8, to: u8) {
        assert!(
            from.is_ascii() && to.is_ascii(),
            "only ASCII bytes can be swapped"
        );
        let end = self.filled + piece.len();
        let swap = |byte: u8| if byte == from { to } else { byte };
        match &mut self.target {
            Target::Inline(bytes) => {
                for (slot, &byte) in bytes[self.filled..end].iter_mut().zip(piece.as_bytes()) {
                    *slot = swap(byte);
                }
            }
            Target::Shared(slots) => {
                for (slot, &byte) in slots[self.filled..end].iter_mut().zip(piece.as_bytes()) {
                    slot.write(swap(byte));
                }
            }
        }
        self.filled = end;
    }

    #[inline]
    fn filled(&self) -> usize {
        self.filled
    }
}

/// Builds a value of exactly `len` bytes from the pieces written by `fill`.
///
/// Results of up to 23 bytes are assembled on the stack and stored inline.
/// Longer results are written directly into one `Arc<str>` allocation.
///
/// # Panics
///
/// Panics if `fill` writes more or fewer than `len` bytes.
#[inline]
pub(super) fn assemble(len: usize, fill: impl FnOnce(&mut Writer<'_>)) -> CheetahString {
    if len <= INLINE_CAPACITY {
        assemble_inline(len, fill)
    } else {
        assemble_shared(len, fill)
    }
}

#[inline]
fn assemble_inline(len: usize, fill: impl FnOnce(&mut Writer<'_>)) -> CheetahString {
    let mut buffer = [0u8; INLINE_CAPACITY];
    let mut writer = Writer {
        target: Target::Inline(&mut buffer[..len]),
        filled: 0,
    };
    fill(&mut writer);
    assert_eq!(writer.filled(), len, "assembled pieces must fill the value");
    // SAFETY: the writer only accepts whole `&str` pieces, possibly with one
    // ASCII byte swapped for another, and they filled the first `len` bytes
    // exactly, so those bytes are a concatenation of valid UTF-8 strings. The rest of the zero-initialised buffer was never
    // written, and `len <= INLINE_CAPACITY` was checked by `assemble`.
    let inline = unsafe { InlineStr::from_assembled(buffer, len) };
    CheetahString {
        inner: InnerString::Inline(inline),
    }
}

// Kept out of line so callers whose results are usually short do not carry
// the allocation path in their inlined body.
#[inline(never)]
fn assemble_shared(len: usize, fill: impl FnOnce(&mut Writer<'_>)) -> CheetahString {
    let mut slots = Arc::<[u8]>::new_uninit_slice(len);
    let target = Arc::get_mut(&mut slots).expect("a new Arc is uniquely owned");
    let mut writer = Writer {
        target: Target::Shared(target),
        filled: 0,
    };
    fill(&mut writer);
    assert_eq!(writer.filled(), len, "assembled pieces must fill the value");
    // SAFETY: the writer initialised all `len` slots above.
    let bytes: Arc<[u8]> = unsafe { slots.assume_init() };
    // SAFETY: the bytes are a concatenation of whole `&str` pieces, possibly
    // with one ASCII byte swapped for another, so they are valid UTF-8, and
    // `str` has the same layout as `[u8]`. This is the
    // conversion the standard library uses for `From<&str> for Arc<str>`.
    let shared: Arc<str> = unsafe { Arc::from_raw(Arc::into_raw(bytes) as *const str) };
    CheetahString {
        inner: InnerString::Shared(shared),
    }
}

#[cfg(test)]
mod tests {
    use super::assemble;
    use crate::cheetah_string::repr::InnerString;
    use alloc::string::String;

    fn assembled(parts: &[&str]) -> crate::CheetahString {
        let len = parts.iter().map(|part| part.len()).sum();
        assemble(len, |writer| {
            for part in parts {
                writer.push_str(part);
            }
        })
    }

    #[test]
    fn short_results_are_inline_and_long_results_are_shared() {
        for parts in [
            &[][..],
            &["", ""][..],
            &["inline", "-", "value"][..],
            &["exactly-twenty-three-b", "!"][..],
            &["twenty-four-bytes-value", "!"][..],
            &[
                "\u{1f980}",
                "\u{e9}t\u{e9}",
                " shared value well past the inline limit",
            ][..],
        ] {
            let value = assembled(parts);
            let expected: String = parts.concat();
            assert_eq!(value.as_str(), expected);
            match &value.inner {
                InnerString::Inline(_) => assert!(expected.len() <= 23),
                InnerString::Shared(_) => assert!(expected.len() > 23),
                InnerString::Static(_) => panic!("assembled values are never static"),
            }
            let cloned = value.clone();
            assert_eq!(cloned, value);
        }
    }

    #[test]
    #[should_panic(expected = "assembled pieces must fill the value")]
    fn short_fill_is_rejected_before_the_value_is_read() {
        let _ = assemble(40, |writer| writer.push_str("too short"));
    }

    #[test]
    #[should_panic]
    fn overlong_fill_is_rejected() {
        let _ = assemble(3, |writer| writer.push_str("too long"));
    }
}
