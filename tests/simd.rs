#![cfg(feature = "simd")]

use cheetah_string::CheetahString;

#[test]
fn test_simd_equality() {
    // Short strings (below SIMD threshold)
    let s1 = CheetahString::from("hello");
    let s2 = CheetahString::from("hello");
    let s3 = CheetahString::from("world");
    assert_eq!(s1, s2);
    assert_ne!(s1, s3);

    // Long strings (above SIMD threshold)
    let long1 = CheetahString::from("a".repeat(1024));
    let long2 = CheetahString::from("a".repeat(1024));
    let long3 = CheetahString::from(format!("{}b", "a".repeat(1023)));
    assert_eq!(long1, long2);
    assert_ne!(long1, long3);

    // Equality with str
    assert_eq!(s1, "hello");
    assert_ne!(s1, "world");

    // Equality with String
    assert_eq!(s1, String::from("hello"));
    assert_ne!(s1, String::from("world"));
}

#[test]
fn test_simd_starts_with() {
    let s = CheetahString::from("hello world, this is a test");

    // Short patterns
    assert!(s.starts_with("hello"));
    assert!(s.starts_with("hello world"));
    assert!(!s.starts_with("world"));

    // Long patterns (above SIMD threshold)
    let long = CheetahString::from("a".repeat(1024));
    assert!(long.starts_with(&"a".repeat(100)));
    assert!(long.starts_with(&"a".repeat(500)));
    assert!(!long.starts_with(&"b".repeat(100)));

    // Edge cases
    assert!(s.starts_with(""));
    let empty = CheetahString::from("");
    assert!(empty.starts_with(""));
    assert!(!empty.starts_with("a"));
}

#[test]
fn test_simd_ends_with() {
    let s = CheetahString::from("hello world, this is a test");

    // Short patterns
    assert!(s.ends_with("test"));
    assert!(s.ends_with("a test"));
    assert!(!s.ends_with("hello"));

    // Long patterns (above SIMD threshold)
    let long = CheetahString::from("a".repeat(1024));
    assert!(long.ends_with(&"a".repeat(100)));
    assert!(long.ends_with(&"a".repeat(500)));
    assert!(!long.ends_with(&"b".repeat(100)));

    // Edge cases
    assert!(s.ends_with(""));
    let empty = CheetahString::from("");
    assert!(empty.ends_with(""));
    assert!(!empty.ends_with("a"));
}

#[test]
fn test_simd_contains() {
    let s = CheetahString::from("hello world, this is a test");

    // Short patterns
    assert!(s.contains("world"));
    assert!(s.contains("this"));
    assert!(!s.contains("xyz"));

    // Long strings
    let long = CheetahString::from(format!("{}needle{}", "a".repeat(500), "a".repeat(500)));
    assert!(long.contains("needle"));
    assert!(!long.contains("haystack"));

    // Edge cases
    assert!(s.contains(""));
    let empty = CheetahString::from("");
    assert!(empty.contains(""));
    assert!(!empty.contains("a"));
}

#[test]
fn test_simd_find() {
    let s = CheetahString::from("hello world, this is a test");

    // Short patterns
    assert_eq!(s.find("world"), Some(6));
    assert_eq!(s.find("this"), Some(13));
    assert_eq!(s.find("test"), Some(23));
    assert_eq!(s.find("xyz"), None);

    // Long strings
    let long = CheetahString::from(format!("{}needle{}", "a".repeat(500), "a".repeat(500)));
    assert_eq!(long.find("needle"), Some(500));
    assert_eq!(long.find("haystack"), None);

    // Edge cases
    assert_eq!(s.find(""), Some(0));
    let empty = CheetahString::from("");
    assert_eq!(empty.find(""), Some(0));
    assert_eq!(empty.find("a"), None);

    // First character
    assert_eq!(s.find("h"), Some(0));
    // First 't' character appears in "this"
    assert_eq!(s.find("t"), Some(13));
}

#[test]
fn test_simd_unicode() {
    // Test with unicode strings
    let s = CheetahString::from("Hello 世界! 🌍");

    assert!(s.starts_with("Hello"));
    assert!(s.contains("世界"));
    assert!(s.ends_with("🌍"));
    assert_eq!(s.find("世界"), Some(6));

    // Test equality with unicode
    let s1 = CheetahString::from("世界");
    let s2 = CheetahString::from("世界");
    assert_eq!(s1, s2);
}

#[test]
fn test_simd_aligned_and_unaligned() {
    // Test with different alignment scenarios
    for offset in 0..16 {
        let prefix = "x".repeat(offset);
        let content = "a".repeat(100);
        let s = CheetahString::from(format!("{}{}", prefix, content));

        assert!(s.starts_with(&prefix));
        assert!(s.contains(&content));
        assert!(s.ends_with(&content));
    }
}

#[test]
fn test_simd_boundary_conditions() {
    // Test strings of exactly SIMD_THRESHOLD length (16 bytes)
    let s16 = CheetahString::from("0123456789abcdef"); // exactly 16 bytes
    assert!(s16.starts_with("0123456789abcdef"));
    assert!(s16.ends_with("0123456789abcdef"));
    assert!(s16.contains("0123456789abcdef"));
    assert_eq!(s16, "0123456789abcdef");

    // Test strings just below and above threshold
    let s15 = CheetahString::from("0123456789abcde"); // 15 bytes
    let s17 = CheetahString::from("0123456789abcdefg"); // 17 bytes

    assert!(s15.starts_with("0123456789abcde"));
    assert!(s17.starts_with("0123456789abcdefg"));
}

#[test]
fn test_simd_pattern_at_end() {
    // Test finding pattern at the very end
    let s = CheetahString::from("aaaaaaaaaaaaaaab");
    assert_eq!(s.find("b"), Some(15));
    assert!(s.ends_with("b"));

    // Test with longer strings
    let long = CheetahString::from(format!("{}end", "a".repeat(1000)));
    assert_eq!(long.find("end"), Some(1000));
    assert!(long.ends_with("end"));
}

#[test]
fn test_simd_multiple_occurrences() {
    // Test that find returns the first occurrence
    let s = CheetahString::from("abcabcabc");
    assert_eq!(s.find("abc"), Some(0));
    assert_eq!(s.find("bc"), Some(1));
    assert_eq!(s.find("ca"), Some(2));
}

#[test]
fn test_simd_inline_storage() {
    // Test with inline-stored strings (≤ 23 bytes)
    let inline = CheetahString::from("short string");
    assert!(inline.starts_with("short"));
    assert!(inline.contains("string"));
    assert!(inline.ends_with("string"));
    assert_eq!(inline.find("string"), Some(6));

    let inline2 = CheetahString::from("short string");
    assert_eq!(inline, inline2);
}
