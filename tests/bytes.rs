#![cfg(feature = "bytes")]

use cheetah_string::{CheetahBytes, CheetahString, FromUtf8BytesError};

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

    let raw = bytes::Bytes::from(vec![0xFF, 0xFE]);
    let payload_pointer = raw.as_ptr();
    let invalid = CheetahBytes::from(raw);
    let result: Result<CheetahString, FromUtf8BytesError> = CheetahString::try_from(invalid);
    let error = result.unwrap_err();

    assert_eq!(error.bytes().as_ref(), &[0xFF, 0xFE]);
    assert_eq!(error.bytes().as_ptr(), payload_pointer);
    assert_eq!(error.utf8_error().valid_up_to(), 0);

    let recovered = error.into_bytes();
    assert_eq!(recovered.as_ref(), &[0xFF, 0xFE]);
    assert_eq!(recovered.as_ptr(), payload_pointer);
}

#[test]
fn cheetah_bytes_roundtrips_bytes_crate_type() {
    let raw = bytes::Bytes::from_static(b"payload");
    let payload_pointer = raw.as_ptr();
    let cheetah = CheetahBytes::from(raw.clone());
    assert_eq!(cheetah.as_bytes().as_ptr(), payload_pointer);
    let roundtrip: bytes::Bytes = cheetah.into();

    assert_eq!(roundtrip, raw);
    assert_eq!(roundtrip.as_ptr(), payload_pointer);
}

#[test]
fn invalid_utf8_error_recovers_original_bytes_buffer() {
    let raw = bytes::Bytes::from(vec![0xFF, 0xFE, 0xFD]);
    let payload_pointer = raw.as_ptr();

    let result: Result<CheetahString, FromUtf8BytesError> = CheetahString::try_from(raw);
    let error = result.unwrap_err();
    assert_eq!(error.bytes().as_ref(), &[0xFF, 0xFE, 0xFD]);
    assert_eq!(error.bytes().as_ptr(), payload_pointer);
    assert_eq!(error.utf8_error().valid_up_to(), 0);

    let recovered = error.into_bytes();
    assert_eq!(recovered.as_ptr(), payload_pointer);
    assert_eq!(recovered.as_ref(), &[0xFF, 0xFE, 0xFD]);
}

#[test]
fn bytes_to_string_apis_are_explicit_copy_boundaries() {
    let raw = bytes::Bytes::from(vec![b'x'; 1024]);
    let payload_pointer = raw.as_ptr();
    let copied = CheetahString::try_copy_from_bytes(raw).unwrap();
    assert_eq!(copied, "x".repeat(1024));
    assert_ne!(copied.as_bytes().as_ptr(), payload_pointer);

    let cheetah_bytes = CheetahBytes::from(bytes::Bytes::from(vec![b'y'; 1024]));
    let source_pointer = cheetah_bytes.as_bytes().as_ptr();
    let copied = cheetah_bytes.try_copy_to_cheetah_string().unwrap();
    assert_eq!(copied, "y".repeat(1024));
    assert_ne!(copied.as_bytes().as_ptr(), source_pointer);
    assert_eq!(cheetah_bytes.as_bytes().as_ptr(), source_pointer);
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
