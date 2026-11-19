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

#[test]
fn unsafe_bytes_buffer_constructor_round_trips_valid_utf8() {
    let bytes = bytes::Bytes::from_static("validated bytes ✓".as_bytes());

    // SAFETY: the buffer is the UTF-8 encoding of a string literal.
    let value = unsafe { CheetahString::from_utf8_unchecked_bytes_buf(bytes) };

    assert_eq!(value, "validated bytes ✓");
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

mod text {
    use bytes::Bytes;
    use cheetah_string::{CheetahBytes, CheetahString, CheetahText, Error};
    use std::collections::HashMap;
    use std::sync::Arc;

    #[test]
    fn validation_keeps_the_buffer_and_rejects_invalid_utf8() {
        let raw = Bytes::from(String::from("topic=orders;tag=\u{1f980}"));
        let pointer = raw.as_ptr();
        let text = CheetahText::try_from_bytes(raw).expect("valid UTF-8");
        assert_eq!(text, "topic=orders;tag=\u{1f980}");
        assert_eq!(text.as_bytes().as_ptr(), pointer);

        let invalid = Bytes::from(vec![b'a', 0xFF]);
        let invalid_pointer = invalid.as_ptr();
        let error = CheetahText::try_from_bytes(invalid).unwrap_err();
        assert_eq!(error.utf8_error().valid_up_to(), 1);
        assert_eq!(error.into_bytes().as_ptr(), invalid_pointer);

        let from_cheetah_bytes = CheetahText::try_from(CheetahBytes::from_static(b"ok")).unwrap();
        assert_eq!(from_cheetah_bytes, "ok");
    }

    #[test]
    // The inverted range is the case under test.
    #[allow(clippy::reversed_empty_ranges)]
    fn slices_share_the_buffer_and_respect_char_boundaries() {
        let text = CheetahText::from(String::from("ab\u{e9}cd"));
        let base = text.as_bytes().as_ptr();

        let middle = text.slice(1..5);
        assert_eq!(middle, "b\u{e9}c");
        assert_eq!(middle.as_bytes().as_ptr(), base.wrapping_add(1));
        assert_eq!(text.slice(..), text);
        assert_eq!(text.slice(..=1), "ab");
        assert_eq!(text.slice(4..), "cd");
        assert_eq!(text.slice(6..6), "");

        assert_eq!(
            text.try_slice(3..4),
            Err(Error::InvalidCharBoundary { index: 3 })
        );
        assert_eq!(
            text.try_slice(1..3),
            Err(Error::InvalidCharBoundary { index: 3 })
        );
        assert_eq!(
            text.try_slice(4..2),
            Err(Error::InvalidRange { start: 4, end: 2 })
        );
        assert_eq!(
            text.try_slice(0..9),
            Err(Error::IndexOutOfBounds { index: 9, len: 6 })
        );
        assert_eq!(
            text.try_slice(0..=usize::MAX),
            Err(Error::IndexOutOfBounds {
                index: usize::MAX,
                len: 6
            })
        );
    }

    #[test]
    #[should_panic(expected = "UTF-8 character boundaries")]
    fn slice_panics_inside_a_character() {
        let _ = CheetahText::from_static("\u{e9}").slice(1..);
    }

    #[test]
    fn slice_ref_turns_str_results_into_owned_values() {
        let text =
            CheetahText::try_from_bytes(Bytes::from_static(b"KEYS=order-1\nTAGS=paid\n")).unwrap();
        let mut properties = HashMap::new();
        for line in text.lines() {
            let (key, value) = line.split_once('=').expect("key=value");
            properties.insert(text.slice_ref(key), text.slice_ref(value));
        }
        assert_eq!(
            properties.get("KEYS").map(CheetahText::as_str),
            Some("order-1")
        );
        assert_eq!(
            properties.get("TAGS").map(CheetahText::as_str),
            Some("paid")
        );
        let key = properties.keys().find(|key| *key == "KEYS").unwrap();
        assert_eq!(key.as_bytes().as_ptr(), text.as_bytes().as_ptr());

        assert!(text.slice_ref("").is_empty());
    }

    #[test]
    #[should_panic]
    fn slice_ref_rejects_text_from_another_buffer() {
        let text = CheetahText::from(String::from("inside"));
        let other = String::from("outside");
        let _ = text.slice_ref(&other);
    }

    #[test]
    fn equality_ordering_and_hashing_match_str() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        fn hash_of<T: Hash + ?Sized>(value: &T) -> u64 {
            let mut hasher = DefaultHasher::new();
            value.hash(&mut hasher);
            hasher.finish()
        }

        let a = CheetahText::from_static("alpha");
        let b = CheetahText::copy_from_str("beta");
        assert_eq!(hash_of(&a), hash_of("alpha"));
        assert!(a < b);
        assert_eq!(a, "alpha");
        assert_eq!("alpha", a);
        assert_eq!(a, String::from("alpha"));
        assert_eq!(String::from("alpha"), a);
        assert_eq!(a, CheetahString::from("alpha"));
        assert_eq!(CheetahString::from("alpha"), a);
        assert_eq!(format!("{a}|{a:>7}|{a:?}"), "alpha|  alpha|\"alpha\"");
        assert_eq!(CheetahText::new(), "");
        assert_eq!(CheetahText::default(), "");
    }

    #[test]
    fn conversions_avoid_copies_where_the_storage_allows() {
        let owned = String::from("adopted allocation");
        let pointer = owned.as_ptr();
        let text = CheetahText::from(owned);
        assert_eq!(text.as_bytes().as_ptr(), pointer);

        let boxed: Box<str> = Box::from("boxed allocation");
        let pointer = boxed.as_ptr();
        assert_eq!(CheetahText::from(boxed).as_bytes().as_ptr(), pointer);

        let static_value = CheetahString::from_static_str("a static value");
        let from_static = CheetahText::from(&static_value);
        assert_eq!(
            from_static.as_bytes().as_ptr(),
            static_value.as_bytes().as_ptr()
        );

        let shared: Arc<str> = Arc::from("a shared value that is longer than the inline limit");
        let shared_value = CheetahString::from(Arc::clone(&shared));
        let from_shared = CheetahText::from(shared_value);
        assert_eq!(from_shared.as_bytes().as_ptr(), shared.as_ptr());
        assert_eq!(from_shared, &*shared);
        drop(from_shared);
        assert_eq!(Arc::strong_count(&shared), 1);

        let inline = CheetahString::from("inline");
        assert_eq!(CheetahText::from(&inline), "inline");

        let text = CheetahText::from_static("payload");
        let pointer = text.as_bytes().as_ptr();
        assert_eq!(Bytes::from(text.clone()).as_ptr(), pointer);
        assert_eq!(
            CheetahBytes::from(text.clone()).as_bytes().as_ptr(),
            pointer
        );
        assert_eq!(text.clone().into_bytes().as_ptr(), pointer);
        assert_eq!(text.bytes().as_ptr(), pointer);
    }

    #[test]
    fn to_cheetah_string_copies_and_releases_the_buffer() {
        let frame = Bytes::from(String::from(
            "short;a value that is longer than twenty-three bytes",
        ));
        let text = CheetahText::try_from_bytes(frame).unwrap();
        let short = text.slice(..5).to_cheetah_string();
        let long = CheetahString::from(text.slice(6..));
        assert_eq!(short, "short");
        assert_eq!(long, "a value that is longer than twenty-three bytes");
        assert_ne!(long.as_bytes().as_ptr(), text.as_bytes()[6..].as_ptr());
    }
}
