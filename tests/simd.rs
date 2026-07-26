use cheetah_string::CheetahString;

const SIZES: [usize; 15] = [0, 1, 2, 7, 15, 16, 17, 22, 23, 24, 31, 32, 64, 1024, 4096];

fn deterministic_ascii(len: usize) -> String {
    (0..len)
        .map(|index| char::from(b'a' + ((index * 17 + 11) % 26) as u8))
        .collect()
}

#[test]
fn equality_matches_slice_semantics_across_sizes_and_mismatches() {
    for size in SIZES {
        let source = deterministic_ascii(size);
        let value = CheetahString::from(source.as_str());
        let same = CheetahString::from(source.as_str());
        assert_eq!(value == same, source.as_bytes() == source.as_bytes());

        if size > 0 {
            for index in [0, size / 2, size - 1] {
                let mut mismatch = source.clone().into_bytes();
                mismatch[index] = if mismatch[index] == b'~' { b'!' } else { b'~' };
                let mismatch = String::from_utf8(mismatch).expect("ASCII replacement");
                let other = CheetahString::from(mismatch.as_str());
                assert_eq!(value == other, source.as_bytes() == mismatch.as_bytes());
            }
        }
    }
}

#[test]
fn prefix_and_suffix_match_str_semantics_across_boundaries() {
    for size in SIZES {
        let source = deterministic_ascii(size);
        let value = CheetahString::from(source.as_str());

        for needle_len in [0, size / 2, size, size.saturating_add(1)] {
            let prefix_len = needle_len.min(size);
            let prefix = &source[..prefix_len];
            assert_eq!(value.starts_with(prefix), source.starts_with(prefix));
            assert_eq!(value.ends_with(prefix), source.ends_with(prefix));

            let absent = "~".repeat(needle_len);
            assert_eq!(
                value.starts_with(absent.as_str()),
                source.starts_with(absent.as_str())
            );
            assert_eq!(
                value.ends_with(absent.as_str()),
                source.ends_with(absent.as_str())
            );
        }
    }
}

#[test]
fn unicode_and_unaligned_inputs_keep_identical_behavior() {
    for offset in 0..16 {
        let source = format!("{}火水風{}", "x".repeat(offset), "界".repeat(64));
        let value = CheetahString::from(source.as_str());
        for needle in ["", "火", "火水風", "界", "不存在"] {
            assert_eq!(value.starts_with(needle), source.starts_with(needle));
            assert_eq!(value.ends_with(needle), source.ends_with(needle));
        }
        assert_eq!(value, source);
    }
}
