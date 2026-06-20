#![no_main]

use cheetah_string::packed::PackedCheetahString;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = core::str::from_utf8(data) {
        let packed = PackedCheetahString::from(s);
        assert_eq!(packed.as_str(), s);
    }
});
