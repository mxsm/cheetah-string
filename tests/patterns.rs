#[cfg(test)]
mod pattern_tests {
    use cheetah_string::CheetahString;

    #[test]
    fn test_starts_with_char() {
        let s = CheetahString::from("+attribute");
        assert!(s.starts_with('+'));
        assert!(!s.starts_with('-'));
    }

    #[test]
    fn test_ends_with_char() {
        let s = CheetahString::from("hello-world");
        assert!(s.ends_with('d'));
        assert!(!s.ends_with('x'));
    }

    #[test]
    fn test_contains_char() {
        let s = CheetahString::from("C:\\Users\\test");
        assert!(s.contains('\\'));
        assert!(s.contains('U'));
        assert!(!s.contains('x'));
    }

    #[test]
    fn test_split_char() {
        let s = CheetahString::from("a_b_c");
        let parts: Vec<&str> = s.split('_').collect();
        assert_eq!(parts, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_split_str() {
        let s = CheetahString::from("a::b::c");
        let parts: Vec<&str> = s.split("::").collect();
        assert_eq!(parts, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_chars_reverse() {
        let s = CheetahString::from("12345");
        let reversed: Vec<char> = s.chars().rev().collect();
        assert_eq!(reversed, vec!['5', '4', '3', '2', '1']);
    }

    #[test]
    fn test_chars_double_ended() {
        let s = CheetahString::from("hello");
        let mut chars = s.chars();
        assert_eq!(chars.next(), Some('h'));
        assert_eq!(chars.next_back(), Some('o'));
        assert_eq!(chars.next(), Some('e'));
        assert_eq!(chars.next_back(), Some('l'));
        assert_eq!(chars.next(), Some('l'));
        assert_eq!(chars.next(), None);
        assert_eq!(chars.next_back(), None);
    }

    #[test]
    fn test_split_char_reverse() {
        let s = CheetahString::from("a_b_c");
        let parts: Vec<&str> = s.split('_').rev().collect();
        assert_eq!(parts, vec!["c", "b", "a"]);
    }

    #[test]
    fn test_combined_patterns() {
        // Test case similar to the original error
        let content = CheetahString::from("file_name_123");
        let vec: Vec<&str> = content.split('_').collect();
        assert_eq!(vec, vec!["file", "name", "123"]);

        let key = CheetahString::from("+attribute");
        assert!(key.starts_with('+'));

        let key2 = CheetahString::from("-property");
        assert!(key2.starts_with('-'));
    }
}
