use cheetah_string::CheetahString;

fn main() {
    let s = CheetahString::from("+hello-world");

    // Test starts_with with char
    println!("starts_with('+'): {}", s.starts_with('+'));
    println!("starts_with('-'): {}", s.starts_with('-'));

    // Test starts_with with &str
    println!("starts_with(\"+hello\"): {}", s.starts_with("+hello"));
    println!("starts_with(\"hello\"): {}", s.starts_with("hello"));

    // Test ends_with with char
    println!("ends_with('d'): {}", s.ends_with('d'));
    println!("ends_with('+'): {}", s.ends_with('+'));

    // Test ends_with with &str
    println!("ends_with(\"-world\"): {}", s.ends_with("-world"));
    println!("ends_with(\"world\"): {}", s.ends_with("world"));

    // Example similar to the error case
    let key = CheetahString::from("+attribute");
    if key.starts_with('+') {
        println!("\nKey '{}' starts with '+'", key);
    }
}
