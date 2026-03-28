use cheetah_string::CheetahString;

#[test]
fn test_split_edge_cases() {
    // Test empty string
    let s = CheetahString::from("");
    let parts: Vec<&str> = s.split(',').collect();
    assert_eq!(parts, vec![""]);

    // Test leading separator
    let s = CheetahString::from(",a,b");
    let parts: Vec<&str> = s.split(',').collect();
    assert_eq!(parts, vec!["", "a", "b"]);

    // Test trailing separator
    let s = CheetahString::from("a,b,");
    let parts: Vec<&str> = s.split(',').collect();
    assert_eq!(parts, vec!["a", "b", ""]);

    // Test consecutive separators
    let s = CheetahString::from("a,,b");
    let parts: Vec<&str> = s.split(',').collect();
    assert_eq!(parts, vec!["a", "", "b"]);

    // Test only separator
    let s = CheetahString::from(",");
    let parts: Vec<&str> = s.split(',').collect();
    assert_eq!(parts, vec!["", ""]);

    // Test string pattern
    let s = CheetahString::from("a::b::c");
    let parts: Vec<&str> = s.split("::").collect();
    assert_eq!(parts, vec!["a", "b", "c"]);

    // Test string pattern with leading separator
    let s = CheetahString::from("::a::b");
    let parts: Vec<&str> = s.split("::").collect();
    assert_eq!(parts, vec!["", "a", "b"]);

    // Test string pattern with trailing separator
    let s = CheetahString::from("a::b::");
    let parts: Vec<&str> = s.split("::").collect();
    assert_eq!(parts, vec!["a", "b", ""]);
}

#[test]
fn test_empty_pattern() {
    // Empty pattern should split between each character
    let s = CheetahString::from("hello");
    let parts: Vec<&str> = s.split("").collect();
    assert_eq!(parts, vec!["", "h", "e", "l", "l", "o", ""]);

    let s = CheetahString::from("");
    let parts: Vec<&str> = s.split("").collect();
    assert_eq!(parts, vec!["", ""]);
}

#[test]
fn test_string_pattern_consecutive_separators() {
    let s = CheetahString::from("a::b::::c::");
    let parts: Vec<&str> = s.split("::").collect();
    assert_eq!(parts, vec!["a", "b", "", "c", ""]);
}

#[test]
#[should_panic(expected = "split with string pattern does not support reverse iteration")]
fn test_single_char_string_pattern_reverse_panics() {
    let s = CheetahString::from("a b c");
    let _: Vec<&str> = s.split(" ").rev().collect();
}
