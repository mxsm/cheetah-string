use cheetah_string::{CheetahString, Error, Result as CheetahResult, StrPattern};
use core::str::Utf8Error;

fn assert_utf8_result(_: core::result::Result<CheetahString, Utf8Error>) {}

fn assert_substring_result(_: CheetahResult<CheetahString>) {}

#[test]
fn pattern_compatibility_method_remains_callable_downstream() {
    let character = 'x';
    let borrowed = "pattern";
    let owned = String::from("owned");

    let _ = StrPattern::as_str_pattern(&character);
    let _ = StrPattern::as_str_pattern(&borrowed);
    let _ = StrPattern::as_str_pattern(&&owned);

    let value = CheetahString::from("prefix-owned-suffix");
    let prefix = String::from("prefix");
    let suffix = String::from("suffix");
    let needle = String::from("owned");
    assert!(value.starts_with(&prefix));
    assert!(value.ends_with(&suffix));
    assert!(value.contains(&needle));
}

#[test]
fn range_error_variants_keep_their_display_and_source_contracts() {
    let cases = [
        (
            Error::IndexOutOfBounds { index: 8, len: 3 },
            "index 8 out of bounds (len: 3)",
        ),
        (
            Error::InvalidRange { start: 4, end: 2 },
            "range start 4 is greater than end 2",
        ),
        (
            Error::InvalidCharBoundary { index: 1 },
            "index 1 is not a char boundary",
        ),
    ];

    for (error, display) in cases {
        assert_eq!(error.to_string(), display);
        #[cfg(feature = "std")]
        {
            use std::error::Error as _;
            assert!(error.source().is_none());
        }
    }
}

#[test]
fn public_error_signatures_remain_precise_and_compatible() {
    assert_utf8_result(CheetahString::try_from_bytes(b"text"));
    assert_utf8_result(CheetahString::try_from_vec(b"text".to_vec()));
    assert_substring_result(CheetahString::from("text").try_substring(0, 2));

    let invalid = [0xff];
    let utf8 = core::str::from_utf8(std::hint::black_box(&invalid))
        .expect_err("input must be invalid UTF-8");
    let error = Error::from(utf8);
    assert!(matches!(error, Error::Utf8Error(_)));
    assert_eq!(
        error.to_string(),
        "UTF-8 error: invalid utf-8 sequence of 1 bytes from index 0"
    );

    #[cfg(feature = "std")]
    {
        use std::error::Error as _;
        assert!(error.source().is_some());
    }
}
