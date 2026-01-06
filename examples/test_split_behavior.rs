use cheetah_string::CheetahString;

fn main() {
    println!("=== Comparing CheetahString and std::str split behavior ===\n");

    let test_cases = vec![
        ("a,b,c", ','),
        ("", ','),
        (",a,b", ','),
        ("a,b,", ','),
        ("a,,b", ','),
        (",", ','),
        (",,", ','),
        ("no_separator", ','),
    ];

    for (input, sep) in test_cases {
        let std_result: Vec<&str> = input.split(sep).collect();

        let cheetah = CheetahString::from(input);
        let cheetah_result: Vec<&str> = cheetah.split(sep).collect();

        let match_str = if std_result == cheetah_result {
            "[OK]"
        } else {
            "[FAIL]"
        };

        println!("{} Input: {:?}, Sep: {:?}", match_str, input, sep);
        println!("  std:     {:?}", std_result);
        println!("  cheetah: {:?}", cheetah_result);

        if std_result != cheetah_result {
            println!("  WARNING: MISMATCH!");
        }
        println!();
    }

    println!("\n=== Testing string patterns ===\n");

    let str_test_cases = vec![
        ("a::b::c", "::"),
        ("", "::"),
        ("::a::b", "::"),
        ("a::b::", "::"),
        ("::", "::"),
        ("a::::b", "::"),
        ("no separator", "::"),
    ];

    for (input, sep) in str_test_cases {
        let std_result: Vec<&str> = input.split(sep).collect();

        let cheetah = CheetahString::from(input);
        let cheetah_result: Vec<&str> = cheetah.split(sep).collect();

        let match_str = if std_result == cheetah_result {
            "[OK]"
        } else {
            "[FAIL]"
        };

        println!("{} Input: {:?}, Sep: {:?}", match_str, input, sep);
        println!("  std:     {:?}", std_result);
        println!("  cheetah: {:?}", cheetah_result);

        if std_result != cheetah_result {
            println!("  WARNING: MISMATCH!");
        }
        println!();
    }
}
