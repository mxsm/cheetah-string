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
fn split_str_matches_str_split() {
    let separators = [
        "::",
        ":::",
        "aa",
        "aaa",
        "--",
        "pay|",
        "q@z!",
        "@consumer",
        "\u{df}\u{df}",
        "\u{1f980}x",
        "x\u{1f980}",
        "TopicTest@consumer-group;tag=pay|",
        "exactly-thirty-two-bytes-long!!!",
        "a separator that is longer than thirty-two bytes",
    ];
    let mut texts = haystacks();
    texts.extend(
        [
            "",
            "::",
            ":::",
            "::::",
            ":::::",
            "a:b:c:d:e:f:g:h:i:j::k",
            "a::b::c::d::e",
            "::leading and trailing::",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "x\u{1f980}\u{1f980}x\u{1f980}xx\u{1f980}",
            "\u{df}\u{df}\u{df}-\u{df}\u{df}",
        ]
        .map(String::from),
    );
    texts.push("a:".repeat(100));
    texts.push(format!("{}::tail", "a:".repeat(30)));
    // Texts that pass the first/last-byte screen often but rarely match, so
    // the search reaches its hand-off to `str::find`.
    texts.push("a".repeat(200));
    texts.push(format!("{}aab", "a".repeat(100)));
    texts.push("aXb".repeat(66));
    texts.push(format!("{}a_b{}a_b", "aXb".repeat(20), "aXb".repeat(5)));
    texts.push(format!("{}\u{1f980}x", "\u{1f980}y".repeat(20)));
    let separators: Vec<&str> = separators
        .iter()
        .copied()
        .chain([
            "aab",
            "a_b",
            "aaaaaaab",
            "\u{1f980}x",
            "\u{1f980}y\u{1f980}x",
        ])
        .collect();
    for text in &texts {
        let value = CheetahString::from(text.as_str());
        for &separator in &separators {
            let expected: Vec<&str> = text.split(separator).collect();
            let actual: Vec<&str> = value.split_str(separator).collect();
            assert_eq!(actual, expected, "split_str {text:?} {separator:?}");
        }
    }
}

#[test]
fn ordering_matches_str_across_storage_kinds() {
    let mut texts = haystacks();
    texts.extend(
        [
            "",
            "\u{0}",
            "a\u{0}",
            "a\u{0}\u{0}",
            "orders",
            "orders@group-a",
            "orders@group-b",
            "orders@group-a-with-a-long-shared-suffix",
            "exactly-twenty-three-b!",
            "exactly-twenty-three-b!!",
        ]
        .map(String::from),
    );
    // 23-byte inline values that differ in one byte only: where the first and
    // second words meet (7, 8), where the second and last words overlap (15,
    // 16), and the last byte (22), which the whole-buffer comparison must read.
    for at in [7usize, 8, 15, 16, 22] {
        for replacement in ["a", "z", "\u{0}"] {
            let mut text = "m".repeat(23);
            text.replace_range(at..at + 1, replacement);
            texts.push(text);
        }
    }
    texts.push("m".repeat(23));
    // Inline and shared values, plus static ones that are short enough to be
    // inline in the other forms.
    let mut values: Vec<CheetahString> = texts
        .iter()
        .map(|text| CheetahString::from(text.as_str()))
        .collect();
    values.extend(["", "a\u{0}", "orders", "orders@group-a"].map(CheetahString::from_static_str));
    for left in &values {
        for right in &values {
            assert_eq!(
                left.cmp(right),
                left.as_str().cmp(right.as_str()),
                "{left:?} vs {right:?}"
            );
            assert_eq!(left.partial_cmp(right), Some(left.cmp(right)));
        }
    }

    let mut sorted = values.clone();
    sorted.sort();
    let mut expected: Vec<&str> = values.iter().map(CheetahString::as_str).collect();
    expected.sort();
    assert_eq!(
        sorted.iter().map(CheetahString::as_str).collect::<Vec<_>>(),
        expected
    );

    let map: std::collections::BTreeMap<CheetahString, usize> =
        values.iter().cloned().zip(0..).collect();
    let keys: Vec<&str> = map.keys().map(CheetahString::as_str).collect();
    let mut unique = expected.clone();
    unique.dedup();
    assert_eq!(keys, unique);
}

fn assert_search_matches_str(text: &str, needle: &str) {
    let value = CheetahString::from(text);
    assert_eq!(
        value.find(needle),
        text.find(needle),
        "find {text:?} {needle:?}"
    );
    assert_eq!(
        value.rfind(needle),
        text.rfind(needle),
        "rfind {text:?} {needle:?}"
    );
    assert_eq!(
        value.contains(needle),
        text.contains(needle),
        "contains {text:?} {needle:?}"
    );
}

#[test]
fn search_matches_str_at_every_offset() {
    // Needles at every offset land on both sides of each 16-position block
    // boundary of the forward and reverse screens, in their scalar tails, and
    // on both sides of the 256-byte screening limit.
    let lengths: Vec<usize> = (0..=70)
        .chain([100, 127, 128, 129, 200, 255, 256, 257, 300])
        .collect();
    for needle in [
        "::",
        "a_b",
        "<-->",
        "\u{e9}\u{e9}",
        "exactly-thirty-two-bytes-long!!!",
    ] {
        for &len in &lengths {
            let base: String = "xy".chars().cycle().take(len).collect();
            assert_search_matches_str(&base, needle);
            for at in 0..=len.saturating_sub(needle.len()) {
                if at + needle.len() > len {
                    continue;
                }
                let mut text = base.clone();
                text.replace_range(at..at + needle.len(), needle);
                assert_search_matches_str(&text, needle);
                assert_search_matches_str(&format!("{text}{needle}"), needle);
            }
        }
    }
}

#[test]
fn search_hand_off_matches_str() {
    // More than four positions pass the first/last-byte screen without
    // matching, so `find`, `rfind`, and `contains` hand the rest of the text
    // to a Two-Way searcher; real matches sit before and after that region.
    let cases = [
        ("aXb".repeat(30), "a_b"),
        (format!("a_b{}", "aXb".repeat(30)), "a_b"),
        (format!("{}a_b", "aXb".repeat(30)), "a_b"),
        (
            format!("{}a_b{}", "aXb".repeat(10), "aXb".repeat(10)),
            "a_b",
        ),
        // Multi-byte needles, whose first byte is a UTF-8 lead byte; the
        // reverse hand-off cuts the text at a character boundary.
        ("\u{e9}X\u{e9}".repeat(20), "\u{e9}_\u{e9}"),
        (
            format!("\u{e9}_\u{e9}{}", "\u{e9}X\u{e9}".repeat(20)),
            "\u{e9}_\u{e9}",
        ),
        (
            format!("{}\u{e9}_\u{e9}", "\u{e9}X\u{e9}".repeat(20)),
            "\u{e9}_\u{e9}",
        ),
        (
            format!("\u{1f980}zy\u{1f980}{}", "\u{1f980}xy\u{1f980}".repeat(12)),
            "\u{1f980}zy\u{1f980}",
        ),
        (
            format!("{}\u{1f980}zy\u{1f980}", "\u{1f980}xy\u{1f980}".repeat(12)),
            "\u{1f980}zy\u{1f980}",
        ),
        // The last match ends exactly where the reverse hand-off cuts the
        // text: "aab" at 20 overlaps the fifth false candidate, "abb" at 21.
        (format!("{}aabb{}", "x".repeat(20), "aXb".repeat(4)), "aab"),
    ];
    for (text, needle) in &cases {
        assert_search_matches_str(text, needle);
    }
}

#[test]
fn split_str_hand_off_matches_str_split() {
    // Every piece is preceded by more than four positions that pass the
    // first/last-byte screen without matching, so the split hands the rest
    // of the text to `str::split` partway through.
    let cases = [
        (format!("{}tail", "aXbaXbaXbaXbaXbaXba_b".repeat(50)), "a_b"),
        // A multi-byte separator, whose first byte is a UTF-8 lead byte.
        (
            format!("{}end", "\u{e9}X\u{e9}".repeat(10) + "\u{e9}_\u{e9}").repeat(5),
            "\u{e9}_\u{e9}",
        ),
        (
            format!("head{}", "\u{1f980}xy\u{1f980}".repeat(12)),
            "\u{1f980}zy\u{1f980}",
        ),
    ];
    for (text, separator) in &cases {
        let value = CheetahString::from(text.as_str());
        let expected: Vec<&str> = text.split(separator).collect();
        assert_eq!(
            value.split_str(separator).collect::<Vec<_>>(),
            expected,
            "split_str {text:?} {separator:?}"
        );
        // Cloning and resuming at every point, before and after the hand-off,
        // yields the remaining pieces.
        for taken in 0..=expected.len() + 1 {
            let mut parts = value.split_str(separator);
            for _ in 0..taken {
                parts.next();
            }
            let resumed: Vec<&str> = parts.clone().collect();
            let remaining: Vec<&str> = expected.iter().skip(taken).copied().collect();
            assert_eq!(
                resumed, remaining,
                "resume after {taken} pieces of {separator:?}"
            );
            assert_eq!(parts.collect::<Vec<_>>(), remaining);
        }
    }
}

#[test]
fn split_str_matches_str_split_at_every_offset() {
    // Separators at every offset of texts up to 70 bytes land before, on, and
    // after each 16-position screening block boundary and in the scalar tail.
    for separator in ["::", "a_b", "<-->", "exactly-thirty-two-bytes-long!!!"] {
        for len in 0..70usize {
            for at in 0..=len.saturating_sub(separator.len()) {
                if at + separator.len() > len {
                    continue;
                }
                let mut text: String = "xy".chars().cycle().take(len).collect();
                text.replace_range(at..at + separator.len(), separator);
                for text in [
                    text.clone(),
                    format!("{text}{separator}"),
                    format!("{separator}{text}"),
                ] {
                    let value = CheetahString::from(text.as_str());
                    let expected: Vec<&str> = text.split(separator).collect();
                    let actual: Vec<&str> = value.split_str(separator).collect();
                    assert_eq!(actual, expected, "split_str {text:?} {separator:?}");
                }
            }
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
                "-",
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
