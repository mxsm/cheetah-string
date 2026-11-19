//! Equality of short byte strings without a `memcmp` call.
//!
//! For short inputs the call into `memcmp` costs more than the comparison
//! itself. Up to 64 bytes, two or four overlapping fixed-size loads per side
//! cover every byte, so equality becomes a few XORs. The `experimental-simd`
//! SSE2 loop was faster than `memcmp` only for this reason; this path gets the
//! same effect in portable safe code. Longer inputs still use `memcmp`.

/// Longest input compared with inline loads.
const INLINE_COMPARE_MAX: usize = 64;

#[inline]
fn word4(bytes: &[u8], at: usize) -> u32 {
    u32::from_ne_bytes(bytes[at..at + 4].try_into().expect("4 bytes"))
}

#[inline]
fn word8(bytes: &[u8], at: usize) -> u64 {
    u64::from_ne_bytes(bytes[at..at + 8].try_into().expect("8 bytes"))
}

#[inline]
fn word16(bytes: &[u8], at: usize) -> u128 {
    u128::from_ne_bytes(bytes[at..at + 16].try_into().expect("16 bytes"))
}

/// Returns whether `a` and `b` hold the same bytes.
#[inline]
pub(crate) fn bytes_eq(a: &[u8], b: &[u8]) -> bool {
    let n = a.len();
    if n != b.len() {
        return false;
    }
    if n > INLINE_COMPARE_MAX {
        // Checked first so long inputs reach `memcmp` without passing the
        // short-length dispatch.
        return a == b;
    }
    match n {
        0 => true,
        // Bytes 0, n/2, and n-1 cover every position of a 1- to 3-byte input.
        1..=3 => (a[0] == b[0]) & (a[n / 2] == b[n / 2]) & (a[n - 1] == b[n - 1]),
        4..=7 => ((word4(a, 0) ^ word4(b, 0)) | (word4(a, n - 4) ^ word4(b, n - 4))) == 0,
        8..=16 => ((word8(a, 0) ^ word8(b, 0)) | (word8(a, n - 8) ^ word8(b, n - 8))) == 0,
        17..=32 => ((word16(a, 0) ^ word16(b, 0)) | (word16(a, n - 16) ^ word16(b, n - 16))) == 0,
        33..=INLINE_COMPARE_MAX => {
            ((word16(a, 0) ^ word16(b, 0))
                | (word16(a, 16) ^ word16(b, 16))
                | (word16(a, n - 32) ^ word16(b, n - 32))
                | (word16(a, n - 16) ^ word16(b, n - 16)))
                == 0
        }
        _ => unreachable_long(),
    }
}

#[cold]
fn unreachable_long() -> bool {
    unreachable!("inputs longer than INLINE_COMPARE_MAX return earlier")
}

/// Returns whether `haystack` begins with `prefix`.
#[inline]
pub(crate) fn starts_with(haystack: &[u8], prefix: &[u8]) -> bool {
    prefix.len() <= haystack.len() && bytes_eq(&haystack[..prefix.len()], prefix)
}

/// Returns whether `haystack` ends with `suffix`.
#[inline]
pub(crate) fn ends_with(haystack: &[u8], suffix: &[u8]) -> bool {
    suffix.len() <= haystack.len() && bytes_eq(&haystack[haystack.len() - suffix.len()..], suffix)
}

#[cfg(test)]
mod tests {
    use super::{bytes_eq, ends_with, starts_with};

    #[test]
    fn matches_slice_equality_at_every_length_and_position() {
        let base: [u8; 80] = core::array::from_fn(|index| (index * 7 + 3) as u8);
        for len in 0..=base.len() {
            let a = &base[..len];
            assert!(bytes_eq(a, a));
            assert!(!bytes_eq(a, &base[..len.saturating_sub(1)]) || len == 0);
            for position in 0..len {
                let mut changed = base;
                changed[position] ^= 0x80;
                assert!(
                    !bytes_eq(a, &changed[..len]),
                    "len {len}, position {position}"
                );
            }
        }
    }

    #[test]
    fn prefix_and_suffix_checks_match_slices() {
        let text = b"orders@group-a;tag=pay;tag=refund;tag=pay;region=eu-west-1;zone=b";
        for len in 0..=text.len() {
            assert!(starts_with(text, &text[..len]));
            assert!(ends_with(text, &text[text.len() - len..]));
            if len > 0 {
                let mut prefix = text[..len].to_vec();
                prefix[len - 1] ^= 1;
                assert!(!starts_with(text, &prefix));
                let mut suffix = text[text.len() - len..].to_vec();
                suffix[0] ^= 1;
                assert!(!ends_with(text, &suffix));
            }
        }
        assert!(!starts_with(b"ab", b"abc"));
        assert!(!ends_with(b"ab", b"abc"));
    }
}
