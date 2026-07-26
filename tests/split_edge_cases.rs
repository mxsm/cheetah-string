use cheetah_string::CheetahString;

#[test]
fn typed_split_iterators_match_standard_library() {
    for input in ["", ",", ",a,b,", "火,水,風", "a,,b"] {
        let value = CheetahString::from(input);
        assert_eq!(
            value.split_char(',').collect::<Vec<_>>(),
            input.split(',').collect::<Vec<_>>()
        );
        assert_eq!(
            value.split_char(',').rev().collect::<Vec<_>>(),
            input.split(',').rev().collect::<Vec<_>>()
        );
    }

    for (input, pattern) in [
        ("", ""),
        ("火::水::風", "::"),
        ("a::::b::", "::"),
        ("aaaa", "aa"),
        ("no-match", "界"),
    ] {
        let value = CheetahString::from(input);
        assert_eq!(
            value.split_str(pattern).collect::<Vec<_>>(),
            input.split(pattern).collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_split_edge_cases() {
    // Test empty string
    let s = CheetahString::from("");
    let parts: Vec<&str> = s.split_char(',').collect();
    assert_eq!(parts, vec![""]);

    // Test leading separator
    let s = CheetahString::from(",a,b");
    let parts: Vec<&str> = s.split_char(',').collect();
    assert_eq!(parts, vec!["", "a", "b"]);

    // Test trailing separator
    let s = CheetahString::from("a,b,");
    let parts: Vec<&str> = s.split_char(',').collect();
    assert_eq!(parts, vec!["a", "b", ""]);

    // Test consecutive separators
    let s = CheetahString::from("a,,b");
    let parts: Vec<&str> = s.split_char(',').collect();
    assert_eq!(parts, vec!["a", "", "b"]);

    // Test only separator
    let s = CheetahString::from(",");
    let parts: Vec<&str> = s.split_char(',').collect();
    assert_eq!(parts, vec!["", ""]);

    // Test string pattern
    let s = CheetahString::from("a::b::c");
    let parts: Vec<&str> = s.split_str("::").collect();
    assert_eq!(parts, vec!["a", "b", "c"]);

    // Test string pattern with leading separator
    let s = CheetahString::from("::a::b");
    let parts: Vec<&str> = s.split_str("::").collect();
    assert_eq!(parts, vec!["", "a", "b"]);

    // Test string pattern with trailing separator
    let s = CheetahString::from("a::b::");
    let parts: Vec<&str> = s.split_str("::").collect();
    assert_eq!(parts, vec!["a", "b", ""]);
}

#[test]
fn test_empty_pattern() {
    // Empty pattern should split between each character
    let s = CheetahString::from("hello");
    let parts: Vec<&str> = s.split_str("").collect();
    assert_eq!(parts, vec!["", "h", "e", "l", "l", "o", ""]);

    let s = CheetahString::from("");
    let parts: Vec<&str> = s.split_str("").collect();
    assert_eq!(parts, vec!["", ""]);
}

#[test]
fn test_string_pattern_consecutive_separators() {
    let s = CheetahString::from("a::b::::c::");
    let parts: Vec<&str> = s.split_str("::").collect();
    assert_eq!(parts, vec!["a", "b", "", "c", ""]);
}
