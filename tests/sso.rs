use cheetah_string::CheetahString;

#[test]
fn test_sso_empty_string() {
    let s = CheetahString::new();
    assert!(s.is_empty());
    assert_eq!(s.len(), 0);
    assert_eq!(s.as_str(), "");
}

#[test]
fn test_sso_short_string() {
    // Test strings at various lengths up to 23 bytes
    let test_cases = vec![
        ("a", 1),
        ("hello", 5),
        ("hello world", 11),
        ("12345678901234567890123", 23), // Exactly 23 bytes
    ];
    
    for (text, expected_len) in test_cases {
        let s = CheetahString::from(text);
        assert_eq!(s.len(), expected_len);
        assert_eq!(s.as_str(), text);
        assert!(!s.is_empty());
    }
}

#[test]
fn test_sso_boundary_23_bytes() {
    // Test the exact boundary case: 23 bytes (should use inline)
    let s23 = "a".repeat(23);
    let cs = CheetahString::from(s23.as_str());
    assert_eq!(cs.len(), 23);
    assert_eq!(cs.as_str(), s23);
}

#[test]
fn test_sso_boundary_24_bytes() {
    // Test 24 bytes (should use Arc)
    let s24 = "a".repeat(24);
    let cs = CheetahString::from(s24.as_str());
    assert_eq!(cs.len(), 24);
    assert_eq!(cs.as_str(), s24);
}

#[test]
fn test_sso_long_string() {
    // Test long strings that should use Arc storage
    let long = "a".repeat(100);
    let s = CheetahString::from(long.as_str());
    assert_eq!(s.len(), 100);
    assert_eq!(s.as_str(), long);
}

#[test]
fn test_sso_clone_short_string() {
    let s1 = CheetahString::from("hello");
    let s2 = s1.clone();
    assert_eq!(s1, s2);
    assert_eq!(s1.as_str(), "hello");
    assert_eq!(s2.as_str(), "hello");
}

#[test]
fn test_sso_unicode_short() {
    // Test short unicode strings
    let s = CheetahString::from("你好");
    assert_eq!(s.len(), 6); // 2 chars * 3 bytes each
    assert_eq!(s.as_str(), "你好");
}

#[test]
fn test_sso_unicode_boundary() {
    // Test unicode at the boundary
    // "你好世界" = 12 bytes (4 chars * 3 bytes)
    let s = CheetahString::from("你好世界啊啊啊"); // 21 bytes
    assert_eq!(s.len(), 21);
    assert_eq!(s.as_str(), "你好世界啊啊啊");
}

#[test]
fn test_sso_from_string() {
    let owned = String::from("short");
    let cs = CheetahString::from(owned);
    assert_eq!(cs.as_str(), "short");
    assert_eq!(cs.len(), 5);
}

#[test]
fn test_sso_to_string() {
    let cs = CheetahString::from("hello");
    let s: String = cs.into();
    assert_eq!(s, "hello");
}

#[test]
fn test_sso_equality() {
    let s1 = CheetahString::from("test");
    let s2 = CheetahString::from("test");
    let s3 = CheetahString::from("different");
    
    assert_eq!(s1, s2);
    assert_ne!(s1, s3);
    assert_eq!(s1, "test");
    assert_eq!("test", s1);
}

#[test]
fn test_sso_hash() {
    use std::collections::HashMap;
    
    let mut map = HashMap::new();
    let key1 = CheetahString::from("key");
    let key2 = CheetahString::from("key");
    
    map.insert(key1, 42);
    assert_eq!(map.get(&key2), Some(&42));
}

#[test]
fn test_sso_ordering() {
    let s1 = CheetahString::from("apple");
    let s2 = CheetahString::from("banana");
    let s3 = CheetahString::from("apple");
    
    assert!(s1 < s2);
    assert!(s2 > s1);
    assert!(s1 <= s3);
    assert!(s1 >= s3);
}

#[test]
fn test_sso_as_bytes() {
    let s = CheetahString::from("hello");
    assert_eq!(s.as_bytes(), b"hello");
}

#[test]
fn test_sso_deref() {
    let s = CheetahString::from("hello");
    assert!(s.starts_with("hel"));
    assert!(s.ends_with("llo"));
}

#[test]
fn test_sso_display_debug() {
    let s = CheetahString::from("test");
    assert_eq!(format!("{}", s), "test");
    assert_eq!(format!("{:?}", s), "\"test\"");
}

#[test]
fn test_sso_mixed_lengths() {
    // Test that we can handle mixed inline and arc strings properly
    let short = CheetahString::from("short");
    let long = CheetahString::from("a".repeat(100));
    
    assert_eq!(short.len(), 5);
    assert_eq!(long.len(), 100);
    
    assert_eq!(short.as_str(), "short");
    assert_eq!(long.as_str(), &"a".repeat(100));
}

#[test]
fn test_sso_empty() {
    let s = CheetahString::empty();
    assert!(s.is_empty());
    assert_eq!(s.len(), 0);
    assert_eq!(s, "");
}

#[test]
fn test_sso_from_char() {
    let s = CheetahString::from('x');
    assert_eq!(s.len(), 1);
    assert_eq!(s.as_str(), "x");
}

#[test]
fn test_sso_special_chars() {
    // Test special characters
    let test_cases = vec![
        "\n",
        "\t",
        "\\",
        "\"",
        "hello\nworld",
        "tab\there",
    ];
    
    for text in test_cases {
        let s = CheetahString::from(text);
        assert_eq!(s.as_str(), text);
    }
}

#[test]
fn test_sso_try_from_bytes() {
    // Test that try_from_bytes works with SSO
    let bytes = b"hello";
    let s = CheetahString::try_from_bytes(bytes).unwrap();
    assert_eq!(s.as_str(), "hello");
    assert_eq!(s.len(), 5);
}

#[test]
fn test_sso_try_from_vec() {
    // Test that try_from_vec works with SSO
    let bytes = vec![104, 101, 108, 108, 111]; // "hello"
    let s = CheetahString::try_from_vec(bytes).unwrap();
    assert_eq!(s.as_str(), "hello");
    assert_eq!(s.len(), 5);
}
