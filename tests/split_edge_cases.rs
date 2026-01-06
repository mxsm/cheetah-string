use cheetah_string::CheetahString;

#[test]
fn test_split_edge_cases() {
    // Test empty string
    let s = CheetahString::from("");
    let parts: Vec<&str> = s.split(',').collect();
    println!("Empty split: {:?}", parts);
    assert_eq!(parts, vec![""]);

    // Test leading separator
    let s = CheetahString::from(",a,b");
    let parts: Vec<&str> = s.split(',').collect();
    println!("Leading separator: {:?}", parts);
    assert_eq!(parts, vec!["", "a", "b"]);

    // Test trailing separator
    let s = CheetahString::from("a,b,");
    let parts: Vec<&str> = s.split(',').collect();
    println!("Trailing separator: {:?}", parts);
    assert_eq!(parts, vec!["a", "b", ""]);

    // Test consecutive separators
    let s = CheetahString::from("a,,b");
    let parts: Vec<&str> = s.split(',').collect();
    println!("Consecutive separators: {:?}", parts);
    assert_eq!(parts, vec!["a", "", "b"]);

    // Test only separator
    let s = CheetahString::from(",");
    let parts: Vec<&str> = s.split(',').collect();
    println!("Only separator: {:?}", parts);
    assert_eq!(parts, vec!["", ""]);

    // Test string pattern
    let s = CheetahString::from("a::b::c");
    let parts: Vec<&str> = s.split("::").collect();
    println!("String pattern: {:?}", parts);
    assert_eq!(parts, vec!["a", "b", "c"]);

    // Test string pattern with leading separator
    let s = CheetahString::from("::a::b");
    let parts: Vec<&str> = s.split("::").collect();
    println!("String pattern leading: {:?}", parts);
    assert_eq!(parts, vec!["", "a", "b"]);

    // Test string pattern with trailing separator
    let s = CheetahString::from("a::b::");
    let parts: Vec<&str> = s.split("::").collect();
    println!("String pattern trailing: {:?}", parts);
    assert_eq!(parts, vec!["a", "b", ""]);
}

#[test]
fn test_empty_pattern() {
    // Empty pattern should split between each character
    let s = CheetahString::from("hello");
    let parts: Vec<&str> = s.split("").collect();
    println!("Empty pattern: {:?}", parts);
    // Standard library behavior: empty pattern splits between each character
}

fn main() {
    test_split_edge_cases();
    test_empty_pattern();
    println!("\nAll tests passed!");
}
