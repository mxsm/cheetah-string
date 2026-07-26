#![no_main]

use cheetah_string::CheetahString;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Some((&selector, bytes)) = data.split_first() else {
        return;
    };
    let Ok(input) = core::str::from_utf8(bytes) else {
        return;
    };
    let value = CheetahString::from_slice(input);

    let delimiter = char::from(selector);
    let expected_char: Vec<&str> = input.split(delimiter).collect();
    let actual_char: Vec<&str> = value.split_char(delimiter).collect();
    assert_eq!(actual_char, expected_char);

    let pattern_end = input
        .char_indices()
        .nth((selector as usize) % (input.chars().count().saturating_add(1)))
        .map_or(input.len(), |(index, _)| index);
    let pattern = &input[..pattern_end];
    let expected_str: Vec<&str> = input.split(pattern).collect();
    let actual_str: Vec<&str> = value.split_str(pattern).collect();
    assert_eq!(actual_str, expected_str);
});
