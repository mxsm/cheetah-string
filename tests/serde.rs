#![cfg(feature = "serde")]

use cheetah_string::{CheetahStr, CheetahString, Error};

#[test]
fn cheetah_str_serializes_and_deserializes_inline_values() {
    let value = CheetahStr::from("topic-a");

    let json = serde_json::to_string(&value).unwrap();
    let decoded: CheetahStr = serde_json::from_str(&json).unwrap();

    assert_eq!(json, "\"topic-a\"");
    assert_eq!(decoded, "topic-a");
}

#[test]
fn cheetah_str_serializes_and_deserializes_shared_values() {
    let value = CheetahStr::from("topic.".repeat(16));

    let json = serde_json::to_string(&value).unwrap();
    let decoded: CheetahStr = serde_json::from_str(&json).unwrap();

    assert_eq!(decoded, value);
}

#[test]
fn try_substring_reports_public_error_variants() {
    let value = CheetahString::from("hello");

    assert_eq!(value.try_substring(1, 4).unwrap(), "ell");

    assert!(matches!(
        value.try_substring(0, 6),
        Err(Error::IndexOutOfBounds { index: 6, len: 5 })
    ));
    assert!(matches!(
        value.try_substring(4, 2),
        Err(Error::InvalidRange { start: 4, end: 2 })
    ));

    let unicode = CheetahString::from("éx");
    assert!(matches!(
        unicode.try_substring(1, 2),
        Err(Error::InvalidCharBoundary { index: 1 })
    ));
}
