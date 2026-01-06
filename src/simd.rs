//! SIMD-accelerated string operations
//!
//! This module provides SIMD implementations for common string operations
//! when the `simd` feature is enabled. It automatically falls back to
//! scalar implementations when SIMD is not available or for small inputs.

#[cfg(feature = "simd")]
use core::arch::x86_64::*;

/// Minimum length threshold for using SIMD operations
const SIMD_THRESHOLD: usize = 16;

/// Compare two byte slices for equality using SIMD when available
#[inline]
pub(crate) fn eq_bytes(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    #[cfg(feature = "simd")]
    {
        if is_x86_feature_detected!("sse2") && a.len() >= SIMD_THRESHOLD {
            return unsafe { eq_bytes_sse2(a, b) };
        }
    }

    // Fallback to standard comparison
    a == b
}

/// Check if haystack starts with needle using SIMD when available
#[inline]
pub(crate) fn starts_with_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.len() > haystack.len() {
        return false;
    }

    if needle.is_empty() {
        return true;
    }

    #[cfg(feature = "simd")]
    {
        if is_x86_feature_detected!("sse2") && needle.len() >= SIMD_THRESHOLD {
            return unsafe { eq_bytes_sse2(&haystack[..needle.len()], needle) };
        }
    }

    // Fallback to standard comparison
    haystack.starts_with(needle)
}

/// Check if haystack ends with needle using SIMD when available
#[inline]
pub(crate) fn ends_with_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.len() > haystack.len() {
        return false;
    }

    if needle.is_empty() {
        return true;
    }

    #[cfg(feature = "simd")]
    {
        if is_x86_feature_detected!("sse2") && needle.len() >= SIMD_THRESHOLD {
            let start = haystack.len() - needle.len();
            return unsafe { eq_bytes_sse2(&haystack[start..], needle) };
        }
    }

    // Fallback to standard comparison
    haystack.ends_with(needle)
}

/// Find the first occurrence of needle in haystack using SIMD when available
#[inline]
pub(crate) fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }

    if needle.len() > haystack.len() {
        return None;
    }

    #[cfg(feature = "simd")]
    {
        if is_x86_feature_detected!("sse2")
            && needle.len() >= SIMD_THRESHOLD
            && haystack.len() >= SIMD_THRESHOLD
        {
            return unsafe { find_bytes_sse2(haystack, needle) };
        }
    }

    // Fallback to standard search
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

// SIMD implementations for x86_64 with SSE2

#[cfg(feature = "simd")]
#[target_feature(enable = "sse2")]
#[inline]
unsafe fn eq_bytes_sse2(a: &[u8], b: &[u8]) -> bool {
    debug_assert_eq!(a.len(), b.len());

    let len = a.len();
    let mut offset = 0;

    // Process 16 bytes at a time
    while offset + 16 <= len {
        let a_vec = _mm_loadu_si128(a.as_ptr().add(offset) as *const __m128i);
        let b_vec = _mm_loadu_si128(b.as_ptr().add(offset) as *const __m128i);
        let cmp = _mm_cmpeq_epi8(a_vec, b_vec);
        let mask = _mm_movemask_epi8(cmp);

        if mask != 0xFFFF {
            return false;
        }

        offset += 16;
    }

    // Handle remaining bytes
    for i in offset..len {
        if a[i] != b[i] {
            return false;
        }
    }

    true
}

#[cfg(feature = "simd")]
#[target_feature(enable = "sse2")]
#[inline]
unsafe fn find_bytes_sse2(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    let haystack_len = haystack.len();
    let needle_len = needle.len();

    if needle_len > haystack_len {
        return None;
    }

    // For small needles, use a simple SIMD approach
    if needle_len == 1 {
        return find_byte_sse2(haystack, needle[0]);
    }

    // For larger needles, use a hybrid approach
    // First, search for the first byte of the needle
    let first_byte = needle[0];
    let mut pos = 0;

    while pos + needle_len <= haystack_len {
        // Find the next occurrence of the first byte
        if let Some(offset) = find_byte_sse2(&haystack[pos..], first_byte) {
            let candidate_pos = pos + offset;

            // Check if the rest matches
            if candidate_pos + needle_len <= haystack_len {
                if eq_bytes_sse2(&haystack[candidate_pos..candidate_pos + needle_len], needle) {
                    return Some(candidate_pos);
                }
                pos = candidate_pos + 1;
            } else {
                return None;
            }
        } else {
            return None;
        }
    }

    None
}

#[cfg(feature = "simd")]
#[target_feature(enable = "sse2")]
#[inline]
unsafe fn find_byte_sse2(haystack: &[u8], needle: u8) -> Option<usize> {
    let len = haystack.len();
    let mut offset = 0;

    // Broadcast the needle byte to all positions in the vector
    let needle_vec = _mm_set1_epi8(needle as i8);

    // Process 16 bytes at a time
    while offset + 16 <= len {
        let haystack_vec = _mm_loadu_si128(haystack.as_ptr().add(offset) as *const __m128i);
        let cmp = _mm_cmpeq_epi8(haystack_vec, needle_vec);
        let mask = _mm_movemask_epi8(cmp);

        if mask != 0 {
            // Found at least one match
            let bit_pos = mask.trailing_zeros() as usize;
            return Some(offset + bit_pos);
        }

        offset += 16;
    }

    // Handle remaining bytes
    haystack[offset..len]
        .iter()
        .position(|&b| b == needle)
        .map(|pos| offset + pos)
}

#[cfg(all(test, feature = "simd"))]
mod tests {
    use super::*;

    #[test]
    fn test_eq_bytes() {
        let a = b"hello world, this is a test";
        let b = b"hello world, this is a test";
        let c = b"hello world, this is b test";

        assert!(eq_bytes(a, b));
        assert!(!eq_bytes(a, c));
        assert!(!eq_bytes(&a[..10], a));
    }

    #[test]
    fn test_starts_with_bytes() {
        let haystack = b"hello world, this is a test";
        assert!(starts_with_bytes(haystack, b"hello"));
        assert!(starts_with_bytes(haystack, b"hello world"));
        assert!(!starts_with_bytes(haystack, b"world"));
        assert!(starts_with_bytes(haystack, b""));
    }

    #[test]
    fn test_ends_with_bytes() {
        let haystack = b"hello world, this is a test";
        assert!(ends_with_bytes(haystack, b"test"));
        assert!(ends_with_bytes(haystack, b"a test"));
        assert!(!ends_with_bytes(haystack, b"hello"));
        assert!(ends_with_bytes(haystack, b""));
    }

    #[test]
    fn test_find_bytes() {
        let haystack = b"hello world, this is a test";
        assert_eq!(find_bytes(haystack, b"world"), Some(6));
        assert_eq!(find_bytes(haystack, b"test"), Some(23));
        assert_eq!(find_bytes(haystack, b"xyz"), None);
        assert_eq!(find_bytes(haystack, b""), Some(0));
    }

    #[test]
    fn test_find_byte() {
        let haystack = b"hello world";
        unsafe {
            assert_eq!(find_byte_sse2(haystack, b'w'), Some(6));
            assert_eq!(find_byte_sse2(haystack, b'h'), Some(0));
            assert_eq!(find_byte_sse2(haystack, b'd'), Some(10));
            assert_eq!(find_byte_sse2(haystack, b'x'), None);
        }
    }

    #[test]
    fn test_simd_threshold() {
        // Test with strings below SIMD threshold
        let small_a = b"hello";
        let small_b = b"hello";
        assert!(eq_bytes(small_a, small_b));

        // Test with strings above SIMD threshold
        let large_a = b"this is a longer string that exceeds the SIMD threshold";
        let large_b = b"this is a longer string that exceeds the SIMD threshold";
        assert!(eq_bytes(large_a, large_b));
    }
}
