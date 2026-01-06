use cheetah_string::CheetahString;

fn main() {
    println!("=== Testing starts_with ===");
    let s = CheetahString::from("+attribute");
    println!("s.starts_with('+'): {}", s.starts_with('+'));
    println!("s.starts_with(\"+attr\"): {}", s.starts_with("+attr"));

    println!("\n=== Testing ends_with ===");
    let s2 = CheetahString::from("hello-world");
    println!("s2.ends_with('d'): {}", s2.ends_with('d'));
    println!("s2.ends_with(\"-world\"): {}", s2.ends_with("-world"));

    println!("\n=== Testing contains ===");
    let path = CheetahString::from("C:\\Users\\test");
    println!("path.contains('\\\\'): {}", path.contains('\\'));
    println!("path.contains(\"Users\"): {}", path.contains("Users"));

    println!("\n=== Testing split with char ===");
    let csv = CheetahString::from("a_b_c");
    let parts: Vec<&str> = csv.split('_').collect();
    println!("csv.split('_'): {:?}", parts);

    println!("\n=== Testing split with str ===");
    let data = CheetahString::from("item1::item2::item3");
    let items: Vec<&str> = data.split("::").collect();
    println!("data.split(\"::\"): {:?}", items);

    println!("\n=== Testing chars with reverse ===");
    let crc = CheetahString::from("12345");
    let reversed: Vec<char> = crc.chars().rev().collect();
    println!("crc.chars().rev(): {:?}", reversed);

    println!("\n=== Combined example (similar to error case) ===");
    let content = CheetahString::from("file_name_123");
    let vec: Vec<&str> = content.split('_').collect();
    println!("content.split('_'): {:?}", vec);

    let key = CheetahString::from("+property");
    if key.starts_with('+') {
        println!("Key '{}' starts with '+'", key);
    }

    let key2 = CheetahString::from("-attribute");
    if key2.starts_with('-') {
        println!("Key '{}' starts with '-'", key2);
    }
}
