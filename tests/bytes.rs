#![cfg(feature = "bytes")]

use cheetah_string::{CheetahBytes, CheetahString};

#[test]
fn cheetah_bytes_accepts_invalid_utf8() {
    let bytes = CheetahBytes::from_vec(vec![0, 159, 146, 150, 255]);

    assert_eq!(bytes.len(), 5);
    assert_eq!(bytes.as_bytes(), &[0, 159, 146, 150, 255]);
}

#[test]
fn cheetah_bytes_try_into_string_validates_utf8() {
    let valid = CheetahBytes::from_vec(b"hello".to_vec());
    let s = CheetahString::try_from(valid).unwrap();
    assert_eq!(s, "hello");

    let invalid = CheetahBytes::from_vec(vec![0xFF, 0xFE]);
    assert!(CheetahString::try_from(invalid).is_err());
}

#[test]
fn cheetah_bytes_roundtrips_bytes_crate_type() {
    let raw = bytes::Bytes::from_static(b"payload");
    let cheetah = CheetahBytes::from(raw.clone());
    let roundtrip: bytes::Bytes = cheetah.into();

    assert_eq!(roundtrip, raw);
}

#[test]
fn unsafe_bytes_to_string_conversion_is_explicit() {
    let bytes = CheetahBytes::from_vec(b"hello".to_vec());

    // SAFETY: The test input is valid UTF-8.
    let s = unsafe { bytes.into_string_unchecked() };

    assert_eq!(s, "hello");
}

#[cfg(feature = "serde")]
#[test]
fn serde_uses_bytes_semantics_for_cheetah_bytes() {
    let bytes = CheetahBytes::from_vec(vec![0, 1, 255]);
    let json = serde_json::to_string(&bytes).unwrap();
    assert_eq!(json, "[0,1,255]");

    let decoded: CheetahBytes = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded.as_bytes(), &[0, 1, 255]);
}
