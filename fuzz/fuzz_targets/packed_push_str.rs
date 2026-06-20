#![no_main]

use cheetah_string::packed::PackedCheetahString;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let split_at = data.len() / 2;
    let (left, right) = data.split_at(split_at);
    let (Ok(left), Ok(right)) = (core::str::from_utf8(left), core::str::from_utf8(right)) else {
        return;
    };

    let mut packed = PackedCheetahString::from(left);
    packed.push_str(right);

    let mut expected = String::from(left);
    expected.push_str(right);

    assert_eq!(packed.as_str(), expected);
});
