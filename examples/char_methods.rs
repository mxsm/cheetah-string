use cheetah_string::CheetahString;

fn main() {
    let s = CheetahString::from("hello world");

    // Use starts_with_char method to check characters
    println!("starts_with_char('h'): {}", s.starts_with_char('h'));
    println!("starts_with_char('+'): {}", s.starts_with_char('+'));

    // Use ends_with_char method to check characters
    println!("ends_with_char('d'): {}", s.ends_with_char('d'));
    println!("ends_with_char('+'): {}", s.ends_with_char('+'));

    // Use contains_char method to check characters
    println!("contains_char('o'): {}", s.contains_char('o'));
    println!("contains_char('+'): {}", s.contains_char('+'));

    // Original methods still work with strings
    println!("starts_with(\"hello\"): {}", s.starts_with("hello"));
    println!("ends_with(\"world\"): {}", s.ends_with("world"));
    println!("contains(\"llo\"): {}", s.contains("llo"));
}
