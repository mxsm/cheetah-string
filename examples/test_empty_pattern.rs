use cheetah_string::CheetahString;

fn main() {
    println!("=== Testing empty pattern behavior ===\n");

    // Standard library empty pattern behavior
    let std_result: Vec<&str> = "hello".split("").collect();
    println!("std \"hello\".split(\"\"):");
    println!("  Result: {:?}", std_result);
    println!("  Length: {}", std_result.len());

    // CheetahString empty pattern behavior
    let cheetah = CheetahString::from("hello");
    let cheetah_result: Vec<&str> = cheetah.split("").collect();
    println!("\nCheetahString \"hello\".split(\"\"):");
    println!("  Result: {:?}", cheetah_result);
    println!("  Length: {}", cheetah_result.len());

    if std_result == cheetah_result {
        println!("\n[OK] Behavior matches");
    } else {
        println!("\n[FAIL] Behavior mismatch!");
        println!("Note: Standard library splits empty pattern between each character");
        println!("Current implementation returns the whole string for simplicity");
    }

    // Test empty string with empty pattern
    println!("\n=== Empty string + empty pattern ===");
    let std_empty: Vec<&str> = "".split("").collect();
    println!("std \"\".split(\"\"):");
    println!("  Result: {:?}", std_empty);

    let cheetah_empty = CheetahString::from("");
    let cheetah_empty_result: Vec<&str> = cheetah_empty.split("").collect();
    println!("\nCheetahString \"\".split(\"\"):");
    println!("  Result: {:?}", cheetah_empty_result);
}
