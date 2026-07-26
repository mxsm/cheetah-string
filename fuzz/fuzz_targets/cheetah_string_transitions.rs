#![no_main]

use cheetah_string::{CheetahBuilder, CheetahString};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(input) = core::str::from_utf8(data) else {
        return;
    };

    let borrowed = CheetahString::from_slice(input);
    let owned = CheetahString::from_string(input.to_owned());
    assert_eq!(borrowed, input);
    assert_eq!(owned, input);

    let borrowed_clone = borrowed.clone();
    let owned_clone = owned.clone();
    assert_eq!(borrowed_clone, borrowed);
    assert_eq!(owned_clone, owned);
    if input.len() > 23 {
        assert_eq!(borrowed_clone.as_bytes().as_ptr(), borrowed.as_bytes().as_ptr());
        assert_eq!(owned_clone.as_bytes().as_ptr(), owned.as_bytes().as_ptr());
    }

    let split_at = input
        .char_indices()
        .nth(input.chars().count() / 2)
        .map_or(input.len(), |(index, _)| index);
    let (left, right) = input.split_at(split_at);
    let mut builder = CheetahBuilder::with_capacity(input.len());
    builder.push_str(left);
    builder.push_str(right);
    let frozen = builder.finish();
    assert_eq!(frozen, input);

    let appended = frozen + input;
    let mut expected = input.to_owned();
    expected.push_str(input);
    assert_eq!(appended, expected);
});
