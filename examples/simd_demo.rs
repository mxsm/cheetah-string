// Example demonstrating SIMD-accelerated string operations in CheetahString
// Run with: cargo run --example simd_demo --features simd

use cheetah_string::CheetahString;

fn main() {
    println!("CheetahString SIMD Demo");
    println!("=======================\n");

    // Example 1: Equality comparison
    println!("1. Equality Comparison:");
    let s1 = CheetahString::from("Hello, World! This is a SIMD-accelerated string comparison.");
    let s2 = CheetahString::from("Hello, World! This is a SIMD-accelerated string comparison.");
    let s3 = CheetahString::from("Hello, World! This is a different string.");

    println!("  s1 == s2: {}", s1 == s2); // true (uses SIMD for long strings)
    println!("  s1 == s3: {}\n", s1 == s3); // false

    // Example 2: starts_with
    println!("2. String Prefix Matching:");
    let url = CheetahString::from("https://api.example.com/v1/users/12345?filter=active&sort=name");
    println!("  URL: {}", url);
    println!("  Starts with 'https://': {}", url.starts_with("https://"));
    println!("  Starts with 'http://': {}\n", url.starts_with("http://"));

    // Example 3: ends_with
    println!("3. String Suffix Matching:");
    let filename = CheetahString::from("document.pdf");
    println!("  Filename: {}", filename);
    println!("  Ends with '.pdf': {}", filename.ends_with(".pdf"));
    println!("  Ends with '.txt': {}\n", filename.ends_with(".txt"));

    // Example 4: contains
    println!("4. Substring Search:");
    let log = CheetahString::from(
        "[2024-01-01 12:00:00] INFO: Processing request for user_id=12345 from ip=192.168.1.100",
    );
    println!("  Log entry: {}", log);
    println!("  Contains 'INFO': {}", log.contains("INFO"));
    println!("  Contains 'ERROR': {}", log.contains("ERROR"));
    println!("  Contains 'user_id': {}\n", log.contains("user_id"));

    // Example 5: find
    println!("5. Pattern Finding:");
    let text = CheetahString::from("The quick brown fox jumps over the lazy dog");
    println!("  Text: {}", text);
    if let Some(pos) = text.find("fox") {
        println!("  Found 'fox' at position: {}", pos);
    }
    if let Some(pos) = text.find("lazy") {
        println!("  Found 'lazy' at position: {}", pos);
    }
    if text.find("cat").is_none() {
        println!("  'cat' not found\n");
    }

    // Example 6: Real-world use case - URL validation
    println!("6. Real-world Use Case - URL Validation:");
    let urls = vec![
        "https://secure.example.com/api/v1/data",
        "http://insecure.example.com/page",
        "ftp://files.example.com/download",
    ];

    for url in urls {
        let url_str = CheetahString::from(url);
        let is_secure = url_str.starts_with("https://");
        let is_api = url_str.contains("/api/");
        println!(
            "  URL: {} - Secure: {}, API endpoint: {}",
            url, is_secure, is_api
        );
    }
    println!();

    // Example 7: Performance-sensitive pattern matching
    println!("7. Log Processing Example:");
    let logs = vec![
        "[2024-01-01 10:00:00] ERROR: Database connection failed",
        "[2024-01-01 10:01:00] INFO: Retrying connection...",
        "[2024-01-01 10:02:00] INFO: Connection established",
        "[2024-01-01 10:03:00] WARN: High memory usage detected",
    ];

    let mut errors = 0;
    let mut warnings = 0;

    for log in logs {
        let log_str = CheetahString::from(log);
        if log_str.contains("ERROR") {
            errors += 1;
            println!("  Error found: {}", log);
        } else if log_str.contains("WARN") {
            warnings += 1;
            println!("  Warning found: {}", log);
        }
    }

    println!("\n  Summary: {} errors, {} warnings", errors, warnings);
    println!("\nNote: When compiled with --features simd, these operations use SSE2 SIMD");
    println!("      instructions for improved performance on longer strings (>= 16 bytes).");
}
