use cheetah_string::CheetahString;
use std::sync::Arc;

#[test]
fn test_empty() {
    let s = CheetahString::new();
    assert!(s.is_empty());
    assert_eq!(s.len(), 0);
    assert_eq!(s.as_str(), "");
}

#[test]
fn test_default() {
    let s = CheetahString::default();
    assert!(s.is_empty());
    assert_eq!(s, CheetahString::new());
}

#[test]
fn test_from_static() {
    let s = CheetahString::from_static_str("hello");
    assert_eq!(s, "hello");
    assert_eq!(s.len(), 5);
    assert!(!s.is_empty());
}

#[test]
fn test_from_string() {
    let owned = String::from("hello world");
    let s = CheetahString::from(owned);
    assert_eq!(s, "hello world");
    assert_eq!(s.len(), 11);
}

#[test]
fn test_from_str() {
    let s = CheetahString::from("hello");
    assert_eq!(s, "hello");
    assert_eq!(s.len(), 5);
}

#[test]
fn test_from_char() {
    let s = CheetahString::from('a');
    assert_eq!(s, "a");
    assert_eq!(s.len(), 1);

    let s = CheetahString::from('你');
    assert_eq!(s, "你");
    assert_eq!(s.len(), 3); // UTF-8 encoding is 3 bytes
}

#[test]
fn test_clone() {
    let s1 = CheetahString::from("hello");
    let s2 = s1.clone();
    assert_eq!(s1, s2);
    assert_eq!(s1.as_str(), s2.as_str());
}

#[test]
fn test_clone_arc_sharing() {
    let s1 = CheetahString::from_string("hello".to_string());
    let s2 = s1.clone();

    // Both should point to the same string
    assert_eq!(s1, s2);
}

#[test]
fn test_eq() {
    let s1 = CheetahString::from("hello");
    let s2 = CheetahString::from("hello");
    let s3 = CheetahString::from("world");

    assert_eq!(s1, s2);
    assert_ne!(s1, s3);

    // Test equality with str
    assert_eq!(s1, "hello");
    assert_eq!("hello", s1);

    // Test equality with String
    assert_eq!(s1, String::from("hello"));
    assert_eq!(String::from("hello"), s1);
}

#[test]
fn test_ord() {
    let s1 = CheetahString::from("apple");
    let s2 = CheetahString::from("banana");
    let s3 = CheetahString::from("apple");

    assert!(s1 < s2);
    assert!(s2 > s1);
    assert!(s1 <= s3);
    assert!(s1 >= s3);
}

#[test]
fn test_hash() {
    use std::collections::HashMap;

    let mut map = HashMap::new();
    let key = CheetahString::from("key");
    map.insert(key.clone(), 42);

    assert_eq!(map.get(&key), Some(&42));
    assert_eq!(map.get(&CheetahString::from("key")), Some(&42));
}

#[test]
fn test_display() {
    let s = CheetahString::from("hello");
    assert_eq!(format!("{}", s), "hello");
}

#[test]
fn test_debug() {
    let s = CheetahString::from("hello");
    assert_eq!(format!("{:?}", s), "\"hello\"");
}

#[test]
fn test_as_ref_str() {
    let s = CheetahString::from("hello");
    let r: &str = s.as_ref();
    assert_eq!(r, "hello");
}

#[test]
fn test_as_ref_bytes() {
    let s = CheetahString::from("hello");
    let b: &[u8] = s.as_ref();
    assert_eq!(b, b"hello");
}

#[test]
fn test_deref() {
    let s = CheetahString::from("hello");
    assert_eq!(s.len(), 5);
    assert!(s.starts_with("hel"));
}

#[test]
fn test_borrow() {
    use std::borrow::Borrow;

    let s = CheetahString::from("hello");
    let borrowed: &str = s.borrow();
    assert_eq!(borrowed, "hello");
}

#[test]
fn test_from_arc_string() {
    let arc = Arc::new(String::from("hello"));
    let s = CheetahString::from(arc);
    assert_eq!(s, "hello");
}

#[test]
fn test_from_iter_str() {
    let parts = vec!["hello", " ", "world"];
    let s: CheetahString = parts.into_iter().collect();
    assert_eq!(s, "hello world");
}

#[test]
fn test_from_iter_string() {
    let parts = vec![
        String::from("hello"),
        String::from(" "),
        String::from("world"),
    ];
    let s: CheetahString = parts.into_iter().collect();
    assert_eq!(s, "hello world");
}

#[test]
fn test_from_iter_chars() {
    let chars = vec!['h', 'e', 'l', 'l', 'o'];
    let s: CheetahString = chars.iter().collect();
    assert_eq!(s, "hello");
}

#[test]
fn test_try_from_valid_bytes() {
    let bytes = b"hello";
    let s = CheetahString::try_from_bytes(bytes).unwrap();
    assert_eq!(s, "hello");
}

#[test]
fn test_try_from_invalid_bytes() {
    let invalid = vec![0xFF, 0xFE];
    let result = CheetahString::try_from_bytes(&invalid);
    assert!(result.is_err());
}

#[test]
fn test_try_from_valid_vec() {
    let bytes = vec![104, 101, 108, 108, 111]; // "hello"
    let s = CheetahString::try_from_vec(bytes).unwrap();
    assert_eq!(s, "hello");
}

#[test]
fn test_try_from_invalid_vec() {
    let invalid = vec![0xFF, 0xFE];
    let result = CheetahString::try_from_vec(invalid);
    assert!(result.is_err());
}

#[test]
fn test_try_from_bytes_method() {
    let bytes = b"hello world";
    let s = CheetahString::try_from_bytes(bytes).unwrap();
    assert_eq!(s, "hello world");

    let invalid = &[0xFF, 0xFE];
    assert!(CheetahString::try_from_bytes(invalid).is_err());
}

#[test]
fn test_try_from_vec_method() {
    let bytes = vec![104, 101, 108, 108, 111];
    let s = CheetahString::try_from_vec(bytes).unwrap();
    assert_eq!(s, "hello");

    let invalid = vec![0xFF, 0xFE];
    assert!(CheetahString::try_from_vec(invalid).is_err());
}

#[test]
fn test_unicode() {
    let s = CheetahString::from("你好世界");
    assert_eq!(s, "你好世界");
    assert_eq!(s.len(), 12); // 4 chars * 3 bytes each
}

#[test]
fn test_empty_string() {
    let s = CheetahString::empty();
    assert!(s.is_empty());
    assert_eq!(s.len(), 0);
}

#[test]
fn test_from_string_ref() {
    let owned = String::from("hello");
    let s = CheetahString::from(&owned);
    assert_eq!(s, "hello");
}

#[test]
fn test_to_string() {
    let s = CheetahString::from("hello");
    let owned: String = s.into();
    assert_eq!(owned, "hello");
}

#[test]
fn test_cow_static() {
    use std::borrow::Cow;

    let cow: Cow<'static, str> = Cow::Borrowed("hello");
    let s = CheetahString::from(cow);
    assert_eq!(s, "hello");
}

#[test]
fn test_cow_owned() {
    use std::borrow::Cow;

    let cow: Cow<'static, str> = Cow::Owned(String::from("hello"));
    let s = CheetahString::from(cow);
    assert_eq!(s, "hello");
}

#[test]
fn test_parse() {
    use std::str::FromStr;

    let s = CheetahString::from_str("hello").unwrap();
    assert_eq!(s, "hello");
}

#[cfg(feature = "bytes")]
#[test]
fn test_from_bytes_feature() {
    use bytes::Bytes;

    let bytes = Bytes::from("hello");
    let s = CheetahString::from(bytes);
    assert_eq!(s, "hello");
}
