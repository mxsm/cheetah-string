use cheetah_string::CheetahString;

#[test]
fn test_unicode_split() {
    // Test Unicode characters
    let s = CheetahString::from("hello,world,Rust");
    let parts: Vec<&str> = s.split(',').collect();
    assert_eq!(parts, vec!["hello", "world", "Rust"]);

    let s = CheetahString::from("Crab::Rust::Rocket");
    let parts: Vec<&str> = s.split("::").collect();
    assert_eq!(parts, vec!["Crab", "Rust", "Rocket"]);
}

#[test]
fn test_split_iterator_behavior() {
    let s = CheetahString::from("a,b,c,d");
    let mut iter = s.split(',');

    assert_eq!(iter.next(), Some("a"));
    assert_eq!(iter.next(), Some("b"));
    assert_eq!(iter.next(), Some("c"));
    assert_eq!(iter.next(), Some("d"));
    assert_eq!(iter.next(), None);
    assert_eq!(iter.next(), None); // Multiple calls to next should continue returning None
}

#[test]
fn test_split_char_reverse() {
    let s = CheetahString::from("a,b,c,d");
    let parts: Vec<&str> = s.split(',').rev().collect();
    assert_eq!(parts, vec!["d", "c", "b", "a"]);

    // Test DoubleEndedIterator
    let mut iter = s.split(',');
    assert_eq!(iter.next(), Some("a"));
    assert_eq!(iter.next_back(), Some("d"));
    assert_eq!(iter.next(), Some("b"));
    assert_eq!(iter.next_back(), Some("c"));
    assert_eq!(iter.next(), None);
}

#[test]
#[should_panic(expected = "split with string pattern does not support reverse iteration")]
fn test_split_str_reverse_panics() {
    let s = CheetahString::from("a::b::c");
    let _: Vec<&str> = s.split("::").rev().collect();
}

#[test]
fn test_pattern_traits() {
    let s = CheetahString::from("+attribute-test");

    // StrPattern trait
    assert!(s.starts_with('+'));
    assert!(s.starts_with("+attr"));
    assert!(s.ends_with('t'));
    assert!(s.ends_with("-test"));
    assert!(s.contains('-'));
    assert!(s.contains("attr"));

    // SplitPattern trait
    let parts: Vec<&str> = s.split('-').collect();
    assert_eq!(parts, vec!["+attribute", "test"]);
}

#[test]
fn test_chars_double_ended() {
    let s = CheetahString::from("abcde");
    let mut chars = s.chars();

    assert_eq!(chars.next(), Some('a'));
    assert_eq!(chars.next_back(), Some('e'));
    assert_eq!(chars.next(), Some('b'));
    assert_eq!(chars.next_back(), Some('d'));
    assert_eq!(chars.next(), Some('c'));
    assert_eq!(chars.next(), None);
    assert_eq!(chars.next_back(), None);
}

#[test]
fn test_long_strings() {
    // Test strings exceeding inline capacity
    let long_str = "a".repeat(100) + "," + &"b".repeat(100);
    let s = CheetahString::from(long_str.as_str());
    let parts: Vec<&str> = s.split(',').collect();
    assert_eq!(parts.len(), 2);
    assert_eq!(parts[0].len(), 100);
    assert_eq!(parts[1].len(), 100);
}

#[test]
fn test_special_patterns() {
    // Test special characters
    let s = CheetahString::from("a\tb\tc");
    let parts: Vec<&str> = s.split('\t').collect();
    assert_eq!(parts, vec!["a", "b", "c"]);

    let s = CheetahString::from("a\nb\nc");
    let parts: Vec<&str> = s.split('\n').collect();
    assert_eq!(parts, vec!["a", "b", "c"]);

    let s = CheetahString::from("a\\b\\c");
    let parts: Vec<&str> = s.split('\\').collect();
    assert_eq!(parts, vec!["a", "b", "c"]);
}
