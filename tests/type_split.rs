use cheetah_string::{CheetahBuilder, CheetahStr, CheetahString};
use std::collections::HashMap;

#[test]
fn cheetah_str_keeps_long_clones_shared() {
    let value = CheetahStr::from("topic.".repeat(32));
    let cloned = value.clone();

    assert_eq!(value, cloned);
    assert_eq!(value.as_bytes().as_ptr(), cloned.as_bytes().as_ptr());
}

#[test]
fn cheetah_str_works_as_hash_map_key() {
    let mut routes = HashMap::new();
    routes.insert(CheetahStr::from_static_str("topic-a"), 7);

    assert_eq!(routes.get("topic-a"), Some(&7));
}

#[test]
fn builder_finishes_to_mutable_string_with_spare_capacity() {
    let mut builder = CheetahBuilder::with_capacity(128);
    builder.push_str("hello");
    let before = builder.as_str().as_bytes().as_ptr();

    let mut value = builder.finish_string();
    value.push_str(" world");

    assert_eq!(value, "hello world");
    assert_eq!(value.as_bytes().as_ptr(), before);
}

#[test]
fn builder_finishes_to_clone_cheap_str() {
    let mut builder = CheetahBuilder::new();
    builder.push_str(&"broker-".repeat(32));

    let value = builder.finish_str();
    let cloned = value.clone();

    assert_eq!(value, cloned);
    assert_eq!(value.as_bytes().as_ptr(), cloned.as_bytes().as_ptr());
}

#[test]
fn cheetah_string_can_be_compacted_into_cheetah_str() {
    let mut value = CheetahString::with_capacity(64);
    value.push_str("consumer-group");

    let compact = CheetahStr::from(value);

    assert_eq!(compact, "consumer-group");
}
