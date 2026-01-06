use cheetah_string::CheetahString;

// Query methods tests

#[test]
fn test_starts_with() {
    let s = CheetahString::from("hello world");
    assert!(s.starts_with("hello"));
    assert!(s.starts_with(""));
    assert!(!s.starts_with("world"));
    assert!(!s.starts_with("Hello"));
}

#[test]
fn test_ends_with() {
    let s = CheetahString::from("hello world");
    assert!(s.ends_with("world"));
    assert!(s.ends_with(""));
    assert!(!s.ends_with("hello"));
    assert!(!s.ends_with("World"));
}

#[test]
fn test_contains() {
    let s = CheetahString::from("hello world");
    assert!(s.contains("llo"));
    assert!(s.contains("o w"));
    assert!(s.contains(""));
    assert!(!s.contains("xyz"));
    assert!(!s.contains("Hello"));
}

#[test]
fn test_find() {
    let s = CheetahString::from("hello world");
    assert_eq!(s.find("world"), Some(6));
    assert_eq!(s.find("hello"), Some(0));
    assert_eq!(s.find("o"), Some(4));
    assert_eq!(s.find("xyz"), None);
    assert_eq!(s.find(""), Some(0));
}

#[test]
fn test_rfind() {
    let s = CheetahString::from("hello hello");
    assert_eq!(s.rfind("hello"), Some(6));
    assert_eq!(s.rfind("l"), Some(9));
    assert_eq!(s.rfind("xyz"), None);
}

#[test]
fn test_trim() {
    let s = CheetahString::from("  hello world  ");
    assert_eq!(s.trim(), "hello world");

    let s2 = CheetahString::from("hello");
    assert_eq!(s2.trim(), "hello");

    let s3 = CheetahString::from("   ");
    assert_eq!(s3.trim(), "");
}

#[test]
fn test_trim_start() {
    let s = CheetahString::from("  hello  ");
    assert_eq!(s.trim_start(), "hello  ");

    let s2 = CheetahString::from("hello");
    assert_eq!(s2.trim_start(), "hello");
}

#[test]
fn test_trim_end() {
    let s = CheetahString::from("  hello  ");
    assert_eq!(s.trim_end(), "  hello");

    let s2 = CheetahString::from("hello");
    assert_eq!(s2.trim_end(), "hello");
}

#[test]
fn test_split() {
    let s = CheetahString::from("a,b,c");
    let parts: Vec<&str> = s.split(",").collect();
    assert_eq!(parts, vec!["a", "b", "c"]);

    let s2 = CheetahString::from("one");
    let parts2: Vec<&str> = s2.split(",").collect();
    assert_eq!(parts2, vec!["one"]);

    let s3 = CheetahString::from("");
    let parts3: Vec<&str> = s3.split(",").collect();
    assert_eq!(parts3, vec![""]);
}

#[test]
fn test_lines() {
    let s = CheetahString::from("line1\nline2\nline3");
    let lines: Vec<&str> = s.lines().collect();
    assert_eq!(lines, vec!["line1", "line2", "line3"]);

    let s2 = CheetahString::from("single line");
    let lines2: Vec<&str> = s2.lines().collect();
    assert_eq!(lines2, vec!["single line"]);

    let s3 = CheetahString::from("");
    let lines3: Vec<&str> = s3.lines().collect();
    assert_eq!(lines3, Vec::<&str>::new());
}

#[test]
fn test_chars() {
    let s = CheetahString::from("hello");
    let chars: Vec<char> = s.chars().collect();
    assert_eq!(chars, vec!['h', 'e', 'l', 'l', 'o']);

    let s2 = CheetahString::from("\u{00E9}\u{00E7}"); // accented chars
    let chars2: Vec<char> = s2.chars().collect();
    assert_eq!(chars2, vec!['\u{00E9}', '\u{00E7}']);
}

// Transformation methods tests

#[test]
fn test_to_uppercase() {
    let s = CheetahString::from("hello");
    assert_eq!(s.to_uppercase(), "HELLO");

    let s2 = CheetahString::from("Hello World");
    assert_eq!(s2.to_uppercase(), "HELLO WORLD");

    let s3 = CheetahString::from("");
    assert_eq!(s3.to_uppercase(), "");
}

#[test]
fn test_to_lowercase() {
    let s = CheetahString::from("HELLO");
    assert_eq!(s.to_lowercase(), "hello");

    let s2 = CheetahString::from("Hello World");
    assert_eq!(s2.to_lowercase(), "hello world");

    let s3 = CheetahString::from("");
    assert_eq!(s3.to_lowercase(), "");
}

#[test]
fn test_replace() {
    let s = CheetahString::from("hello world");
    assert_eq!(s.replace("world", "rust"), "hello rust");
    assert_eq!(s.replace("l", "L"), "heLLo worLd");
    assert_eq!(s.replace("xyz", "abc"), "hello world");
    assert_eq!(s.replace("", "x"), "xhxexlxlxox xwxoxrxlxdx");
}

#[test]
fn test_replacen() {
    let s = CheetahString::from("hello world");
    assert_eq!(s.replacen("l", "L", 1), "heLlo world");
    assert_eq!(s.replacen("l", "L", 2), "heLLo world");
    assert_eq!(s.replacen("o", "O", 1), "hellO world");
}

#[test]
fn test_substring() {
    let s = CheetahString::from("hello world");
    assert_eq!(s.substring(0, 5), "hello");
    assert_eq!(s.substring(6, 11), "world");
    assert_eq!(s.substring(0, 0), "");
    assert_eq!(s.substring(5, 5), "");
}

#[test]
#[should_panic]
fn test_substring_invalid_range() {
    let s = CheetahString::from("hello");
    let _ = s.substring(0, 10); // Out of bounds
}

#[test]
fn test_repeat() {
    let s = CheetahString::from("abc");
    assert_eq!(s.repeat(0), "");
    assert_eq!(s.repeat(1), "abc");
    assert_eq!(s.repeat(3), "abcabcabc");

    let s2 = CheetahString::from("");
    assert_eq!(s2.repeat(5), "");
}

// Add trait tests

#[test]
fn test_add_str() {
    let s = CheetahString::from("Hello");
    let result = s + " World";
    assert_eq!(result, "Hello World");
}

#[test]
fn test_add_cheetah_string() {
    let s1 = CheetahString::from("Hello");
    let s2 = CheetahString::from(" World");
    let result = s1 + &s2;
    assert_eq!(result, "Hello World");
}

#[test]
fn test_add_string() {
    let s = CheetahString::from("Hello");
    let result = s + String::from(" World");
    assert_eq!(result, "Hello World");
}

#[test]
fn test_add_assign_str() {
    let mut s = CheetahString::from("Hello");
    s += " World";
    assert_eq!(s, "Hello World");
}

#[test]
fn test_add_assign_cheetah_string() {
    let mut s1 = CheetahString::from("Hello");
    let s2 = CheetahString::from(" World");
    s1 += &s2;
    assert_eq!(s1, "Hello World");
}

#[test]
fn test_add_chain() {
    let s = CheetahString::from("a");
    let result = s + "b" + "c";
    assert_eq!(result, "abc");
}

#[test]
fn test_add_assign_chain() {
    let mut s = CheetahString::from("a");
    s += "b";
    s += "c";
    assert_eq!(s, "abc");
}

#[test]
fn test_add_empty_strings() {
    let s1 = CheetahString::from("");
    let s2 = CheetahString::from("hello");
    let result = s1 + "hello";
    assert_eq!(result, s2);

    let s3 = CheetahString::from("hello");
    let result2 = s3 + "";
    assert_eq!(result2, "hello");
}

#[test]
fn test_add_sso_strings() {
    // Test SSO strings (<=23 bytes)
    let s1 = CheetahString::from("short");
    let result = s1 + " str";
    assert_eq!(result, "short str");
    assert_eq!(result.len(), 9);
}

#[test]
fn test_add_long_strings() {
    // Test long strings (>23 bytes)
    let s1 = CheetahString::from("This is a very long string that exceeds SSO capacity");
    let result = s1 + " and more";
    assert_eq!(
        result,
        "This is a very long string that exceeds SSO capacity and more"
    );
}

// Edge cases and unicode tests

#[test]
fn test_unicode_queries() {
    let s = CheetahString::from("caf\u{00E9}"); // cafe with e-acute
    assert!(s.contains("\u{00E9}"));
    assert!(s.starts_with("caf"));
    assert!(s.ends_with("\u{00E9}"));
    assert_eq!(s.find("f"), Some(2));
}

#[test]
fn test_unicode_transform() {
    let s = CheetahString::from("caf\u{00E9}");
    let upper = s.to_uppercase();
    let lower = s.to_lowercase();
    // e-acute uppercases to E-acute
    assert_eq!(upper, "CAF\u{00C9}");
    assert_eq!(lower, "caf\u{00E9}");
}

#[test]
fn test_unicode_split() {
    let s = CheetahString::from("\u{00E9},\u{00E7},\u{00F1},\u{00FC}");
    let parts: Vec<&str> = s.split(",").collect();
    assert_eq!(parts, vec!["\u{00E9}", "\u{00E7}", "\u{00F1}", "\u{00FC}"]);
}

#[test]
fn test_empty_string_operations() {
    let s = CheetahString::from("");
    assert_eq!(s.trim(), "");
    assert_eq!(s.to_uppercase(), "");
    assert_eq!(s.to_lowercase(), "");
    assert!(s.starts_with(""));
    assert!(s.ends_with(""));
    assert_eq!(s.find(""), Some(0));

    let result = s.clone() + "hello";
    assert_eq!(result, "hello");
}

#[test]
fn test_whitespace_operations() {
    let s = CheetahString::from("  \t\n  ");
    assert_eq!(s.trim(), "");
    assert_eq!(s.len(), 6);
    assert_eq!(s.trim().len(), 0);
}
