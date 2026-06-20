#![cfg(all(
    feature = "experimental-packed",
    target_pointer_width = "64",
    target_endian = "little"
))]

use cheetah_string::packed::{PackedCheetahString, INLINE_CAPACITY};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::mem::{align_of, size_of};

#[test]
fn packed_layout_is_24_bytes() {
    assert_eq!(size_of::<PackedCheetahString>(), 24);
    assert_eq!(align_of::<PackedCheetahString>(), 8);
    assert!(size_of::<Option<PackedCheetahString>>() >= size_of::<PackedCheetahString>());
}

#[test]
fn inline_string_roundtrips() {
    let s = PackedCheetahString::from("hello");

    assert!(s.is_inline());
    assert_eq!(s.len(), 5);
    assert_eq!(s.as_str(), "hello");
}

#[test]
fn heap_string_roundtrips() {
    let value = "a".repeat(INLINE_CAPACITY + 1);
    let s = PackedCheetahString::from(value.clone());

    assert!(!s.is_inline());
    assert_eq!(s.as_str(), value);
}

#[test]
fn push_str_promotes_inline_to_heap() {
    let mut s = PackedCheetahString::from("a".repeat(INLINE_CAPACITY).as_str());

    s.push_str("b");

    assert!(!s.is_inline());
    assert_eq!(s.as_str(), format!("{}b", "a".repeat(INLINE_CAPACITY)));
}

#[test]
fn clone_is_independent_for_heap_storage() {
    let value = "route-key".repeat(16);
    let original = PackedCheetahString::from(value.clone());
    let mut cloned = original.clone();

    cloned.push_str("-copy");

    assert_eq!(original.as_str(), value);
    assert_eq!(cloned.as_str(), format!("{value}-copy"));
}

#[test]
fn hash_matches_str_semantics() {
    let packed = PackedCheetahString::from("hello");
    let mut packed_hasher = DefaultHasher::new();
    packed.hash(&mut packed_hasher);

    let mut str_hasher = DefaultHasher::new();
    "hello".hash(&mut str_hasher);

    assert_eq!(packed_hasher.finish(), str_hasher.finish());
}
