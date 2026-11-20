//! Differential tests: every transform and search must match `str` exactly,
//! across the inline/shared boundary and the search dispatch thresholds.

use cheetah_string::CheetahString;

const TEXTS: &[&str] = &[
    "",
    "a",
    "orders",
    "orders@group-a",
    "exactly-twenty-three-b!",
    "twenty-four-bytes-value!",
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "MiXeD-Case-ASCII-value-0123456789",
    "TopicTest@consumer-group;tag=pay|TopicTest@consumer-group;tag=pay|",
    "\u{e9}t\u{e9} \u{1f980} stra\u{df}e \u{130}stanbul",
    "\u{1f980}\u{1f980}\u{1f980}",
];

const NEEDLES: &[&str] = &[
    "",
    "a",
    "aa",
    "aaa",
    "@",
    "group",
    "pay|",
    "q@z!",
    "consumer",
    "consumer-group;t",
    "-",
    "\u{1f980}",
    "\u{df}",
    "TopicTest@consumer-group;tag=pay|",
    "a needle that is longer than thirty-two bytes overall",
];

fn haystacks() -> Vec<String> {
    let mut out: Vec<String> = TEXTS.iter().map(|text| text.to_string()).collect();
    // Lengths around the std SIMD block threshold, the needle-dependent `find`
    // and `rfind` cutoffs, and memmem's 64-byte Rabin-Karp limit.
    let unit = "TopicTest@consumer-group;tag=pay|";
    for len in [
        15usize, 16, 18, 19, 20, 21, 22, 27, 28, 29, 31, 32, 33, 39, 40, 41, 47, 48, 55, 56, 57,
        63, 64, 65, 127, 128, 129, 300, 1024,
    ] {
        let base: String = unit.chars().cycle().take(len).collect();
        out.push(base.clone());
        let mut with_tail = base.clone();
        with_tail.push_str("q@z!");
        out.push(with_tail);
        let mut with_head = String::from("q@z!");
        with_head.push_str(&base);
        out.push(with_head);
    }
    out
}

#[test]
fn search_matches_str_on_every_dispatch_path() {
    for hay in haystacks() {
        let value = CheetahString::from(hay.as_str());
        for needle in NEEDLES {
            assert_eq!(
                value.contains(*needle),
                hay.contains(needle),
                "contains {hay:?} {needle:?}"
            );
            assert_eq!(
                value.find(needle),
                hay.find(needle),
                "find {hay:?} {needle:?}"
            );
            assert_eq!(
                value.rfind(needle),
                hay.rfind(needle),
                "rfind {hay:?} {needle:?}"
            );
        }
        for character in ['a', '@', '|', 'q', '\u{1f980}', '\u{df}', 'z'] {
            assert_eq!(value.find_char(character), hay.find(character));
            assert_eq!(value.rfind_char(character), hay.rfind(character));
            assert_eq!(value.contains(character), hay.contains(character));
        }
    }
}

#[test]
fn search_matches_str_when_the_needle_is_not_shorter_than_the_haystack() {
    for hay in haystacks() {
        let value = CheetahString::from(hay.as_str());
        let mut longer = hay.clone();
        longer.push('x');
        let mut changed = hay.clone();
        if let Some(last) = changed.pop() {
            changed.push(if last == 'x' { 'y' } else { 'x' });
        }
        for needle in [hay.as_str(), longer.as_str(), changed.as_str()] {
            assert_eq!(
                value.contains(needle),
                hay.contains(needle),
                "contains {hay:?} {needle:?}"
            );
            assert_eq!(
                value.find(needle),
                hay.find(needle),
                "find {hay:?} {needle:?}"
            );
            assert_eq!(
                value.rfind(needle),
                hay.rfind(needle),
                "rfind {hay:?} {needle:?}"
            );
        }
    }
}

#[test]
fn transforms_match_str() {
    for hay in haystacks() {
        let value = CheetahString::from(hay.as_str());
        assert_eq!(
            value.to_uppercase(),
            hay.to_uppercase(),
            "to_uppercase {hay:?}"
        );
        assert_eq!(
            value.to_lowercase(),
            hay.to_lowercase(),
            "to_lowercase {hay:?}"
        );
        for needle in NEEDLES {
            for replacement in [
                "",
                "X",
                "@@",
                "\u{1f980}",
                "a replacement longer than the inline limit",
            ] {
                assert_eq!(
                    value.replace(*needle, replacement),
                    hay.replace(needle, replacement),
                    "replace {hay:?} {needle:?} {replacement:?}"
                );
                for count in [0usize, 1, 2, 5] {
                    assert_eq!(
                        value.replacen(*needle, replacement, count),
                        hay.replacen(needle, replacement, count),
                        "replacen {hay:?} {needle:?} {replacement:?} {count}"
                    );
                }
            }
        }
        for times in [0usize, 1, 2, 3, 7, 24] {
            assert_eq!(
                value.repeat(times),
                hay.repeat(times),
                "repeat {hay:?} {times}"
            );
        }
    }
}

#[test]
fn concat_matches_joined_parts() {
    let cases: &[&[&str]] = &[
        &[],
        &[""],
        &["", "", ""],
        &["orders", "@", "group-a"],
        &["exactly-twenty-three", "-b!"],
        &["twenty-four-bytes-value", "!"],
        &["%RETRY%order-consumer-group", "@", "broker-a-instance-0001"],
        &["\u{1f980}", "\u{e9}", "t\u{e9}"],
    ];
    for parts in cases {
        let joined = parts.concat();
        assert_eq!(CheetahString::concat(parts), joined.as_str());
        if let [left, rest @ ..] = parts {
            let mut value = CheetahString::from(*left);
            for part in rest {
                value = value + *part;
            }
            assert_eq!(value, joined.as_str());
        }
    }
}

#[test]
fn unchanged_results_reuse_the_original_value() {
    let long = CheetahString::from("ALREADY-UPPER-CASE-VALUE-WITH-NO-LOWER-CASE");
    let same = long.to_uppercase();
    assert_eq!(same.as_bytes().as_ptr(), long.as_bytes().as_ptr());

    let no_match = long.replace("missing", "x");
    assert_eq!(no_match.as_bytes().as_ptr(), long.as_bytes().as_ptr());

    let once = long.repeat(1);
    assert_eq!(once.as_bytes().as_ptr(), long.as_bytes().as_ptr());
}

#[test]
fn cheetah_string_values_are_accepted_as_patterns() {
    let value = CheetahString::from("orders@group-a");
    let prefix = CheetahString::from("orders");
    let suffix = CheetahString::from("group-a");
    let middle = CheetahString::from("@group");
    assert!(value.starts_with(&prefix));
    assert!(value.ends_with(&suffix));
    assert!(value.contains(&middle));
    assert!(!value.contains(&CheetahString::from("missing")));

    let owned: String = String::from(&value);
    assert_eq!(owned, "orders@group-a");
    assert_eq!(owned.capacity(), owned.len());
}
