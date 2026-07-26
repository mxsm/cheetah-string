use cheetah_string::{CheetahBuilder, CheetahString};

#[test]
fn builder_mutations_match_string() {
    let mut builder = CheetahBuilder::with_capacity(4);
    let mut string = String::with_capacity(4);

    for part in ["alpha", "::", "βeta", "", "::tail"] {
        builder.push_str(part);
        string.push_str(part);
        assert_eq!(builder.as_str(), string);
        assert!(builder.capacity() >= builder.len());
    }

    builder.reserve(128);
    string.reserve(128);
    assert_eq!(builder.as_str(), string);
    assert!(builder.capacity() >= builder.len() + 128);

    builder.clear();
    string.clear();
    builder.push('火');
    string.push('火');
    assert_eq!(builder.as_str(), string);
}

#[test]
fn builder_finish_and_into_string_have_distinct_contracts() {
    let long = "canonical-".repeat(32);
    let mut canonical_builder = CheetahBuilder::with_capacity(long.len());
    canonical_builder.push_str(&long);
    let canonical = canonical_builder.finish();
    let cloned = canonical.clone();
    assert_eq!(canonical.as_bytes().as_ptr(), cloned.as_bytes().as_ptr());

    let mut mutable_builder = CheetahBuilder::with_capacity(512);
    mutable_builder.push_str("mutable");
    let pointer = mutable_builder.as_str().as_ptr();
    let capacity = mutable_builder.capacity();
    let mutable = mutable_builder.into_string();
    assert_eq!(mutable.as_ptr(), pointer);
    assert_eq!(mutable.capacity(), capacity);
}

#[test]
#[allow(deprecated)]
fn v2_builder_finish_string_remains_a_cheetah_string_compatibility_shim() {
    let mut builder = CheetahBuilder::new();
    builder.push_str("topic");

    let value: CheetahString = builder.finish_string();
    assert_eq!(value, "topic");
}

#[test]
fn all_long_construction_sources_clone_by_sharing_payload() {
    const STATIC: &str =
        "a static string longer than the twenty-three-byte inline representation boundary";
    let long = "shared-value-".repeat(64);

    for value in [
        CheetahString::from_slice(&long),
        CheetahString::from_string(long.clone()),
        CheetahString::from_static_str(STATIC),
    ] {
        let cloned = value.clone();
        assert_eq!(value, cloned);
        assert_eq!(value.as_bytes().as_ptr(), cloned.as_bytes().as_ptr());
    }
}

#[test]
fn concatenation_produces_a_new_immutable_value() {
    let left = CheetahString::from("hello");
    let original = left.clone();
    let combined = left + " world";

    assert_eq!(original, "hello");
    assert_eq!(combined, "hello world");
}
