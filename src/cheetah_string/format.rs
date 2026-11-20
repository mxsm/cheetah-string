//! Formatting straight into a `CheetahString`.

use alloc::string::String;
use core::fmt;

use super::repr::{InnerString, INLINE_CAPACITY};
use super::CheetahString;
use crate::inline::InlineStr;

/// `fmt::Write` target that stays on the stack until the output outgrows
/// inline storage.
enum Sink {
    Inline {
        buffer: [u8; INLINE_CAPACITY],
        len: usize,
    },
    Heap(String),
}

impl fmt::Write for Sink {
    #[inline]
    fn write_str(&mut self, piece: &str) -> fmt::Result {
        match self {
            Sink::Inline { buffer, len } => {
                let end = *len + piece.len();
                if end <= INLINE_CAPACITY {
                    buffer[*len..end].copy_from_slice(piece.as_bytes());
                    *len = end;
                } else {
                    // Spill once with room to keep growing: `format!` starts
                    // from a tiny estimate and reallocates several times on
                    // its way to a typical 50-byte key.
                    let mut heap = String::with_capacity(end.saturating_mul(2).max(64));
                    // SAFETY: the inline buffer only ever receives whole
                    // `&str` pieces, so its first `len` bytes are UTF-8.
                    heap.push_str(unsafe { core::str::from_utf8_unchecked(&buffer[..*len]) });
                    heap.push_str(piece);
                    *self = Sink::Heap(heap);
                }
            }
            Sink::Heap(heap) => heap.push_str(piece),
        }
        Ok(())
    }
}

impl CheetahString {
    /// Formats `args` into a new value.
    ///
    /// Output of up to 23 bytes is written to inline storage without
    /// allocating, which `CheetahString::from(format!(..))` cannot do. A
    /// constant format string with no arguments borrows it as a static value.
    /// Use the [`cheetah_format!`](crate::cheetah_format) macro for
    /// `format!`-style syntax.
    ///
    /// # Panics
    ///
    /// Panics if a `Display` or `Debug` implementation returns an error, as
    /// `format!` does.
    ///
    /// # Examples
    ///
    /// ```
    /// use cheetah_string::CheetahString;
    ///
    /// let queue_id = 3;
    /// let key = CheetahString::from_fmt(format_args!("orders@{queue_id}"));
    /// assert_eq!(key, "orders@3");
    /// ```
    #[must_use]
    pub fn from_fmt(args: fmt::Arguments<'_>) -> Self {
        if let Some(text) = args.as_str() {
            return CheetahString::from_static_str(text);
        }

        let mut sink = Sink::Inline {
            buffer: [0; INLINE_CAPACITY],
            len: 0,
        };
        fmt::write(&mut sink, args)
            .expect("a formatting trait implementation returned an error unexpectedly");
        match sink {
            Sink::Inline { buffer, len } => CheetahString {
                // SAFETY: only whole `&str` pieces were written, so the first
                // `len` bytes are UTF-8; the rest of the zeroed buffer was
                // never written; and `len <= INLINE_CAPACITY` by construction.
                inner: InnerString::Inline(unsafe { InlineStr::from_assembled(buffer, len) }),
            },
            Sink::Heap(heap) => CheetahString::from_string(heap),
        }
    }
}

/// Creates a [`CheetahString`] using `format!` syntax.
///
/// Output of up to 23 bytes is formatted into inline storage without
/// allocating; see [`CheetahString::from_fmt`].
///
/// ```
/// use cheetah_string::cheetah_format;
///
/// let (topic, queue_id) = ("orders", 4);
/// let key = cheetah_format!("{topic}@{queue_id}");
/// assert_eq!(key, "orders@4");
/// ```
#[macro_export]
macro_rules! cheetah_format {
    ($($arg:tt)*) => {
        $crate::CheetahString::from_fmt(::core::format_args!($($arg)*))
    };
}
