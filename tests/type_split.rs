use cheetah_string::{CheetahBuilder, CheetahString};
use std::collections::HashMap;

#[test]
fn cheetah_string_keeps_long_clones_shared() {
    let value = CheetahString::from("topic.".repeat(32));
    let cloned = value.clone();

    assert_eq!(value, cloned);
    assert_eq!(value.as_bytes().as_ptr(), cloned.as_bytes().as_ptr());
}

#[test]
fn cheetah_string_works_as_hash_map_key() {
    let mut routes = HashMap::new();
    routes.insert(CheetahString::from_static_str("topic-a"), 7);

    assert_eq!(routes.get("topic-a"), Some(&7));
}

#[test]
fn builder_into_string_preserves_spare_capacity_for_mutation() {
    let mut builder = CheetahBuilder::with_capacity(128);
    builder.push_str("hello");
    let before = builder.as_str().as_bytes().as_ptr();

    let mut value = builder.into_string();
    value.push_str(" world");

    assert_eq!(value, "hello world");
    assert_eq!(value.as_bytes().as_ptr(), before);
}

#[test]
fn builder_finish_freezes_to_clone_cheap_canonical_value() {
    let mut builder = CheetahBuilder::new();
    builder.push_str(&"broker-".repeat(32));

    let value = builder.finish();
    let cloned = value.clone();

    assert_eq!(value, cloned);
    assert_eq!(value.as_bytes().as_ptr(), cloned.as_bytes().as_ptr());
}

#[test]
fn builder_freezes_into_canonical_cheetah_string() {
    let mut builder = CheetahBuilder::with_capacity(64);
    builder.push_str("consumer-group");
    let compact = builder.finish();

    assert_eq!(compact, "consumer-group");
}

#[test]
#[allow(deprecated)]
fn deprecated_cheetah_str_alias_has_identical_layout_and_shared_payload() {
    use cheetah_string::CheetahStr;

    assert_eq!(
        core::mem::size_of::<CheetahStr>(),
        core::mem::size_of::<CheetahString>()
    );
    let value = CheetahStr::from("alias.".repeat(64));
    let canonical: CheetahString = value;
    let cloned = canonical.clone();
    assert_eq!(canonical.as_bytes().as_ptr(), cloned.as_bytes().as_ptr());
}

#[test]
#[allow(deprecated)]
fn v2_split_entry_preserves_concrete_iterator_capabilities() {
    let value = CheetahString::from_static_str("a::b::c");

    assert_eq!(value.split(',').rev().collect::<Vec<_>>(), vec!["a::b::c"]);
    assert_eq!(value.split("::").collect::<Vec<_>>(), vec!["a", "b", "c"]);
}

#[test]
fn builder_keeps_short_contents_inline_and_spills_on_growth() {
    use std::fmt::Write;

    let mut builder = CheetahBuilder::new();
    assert_eq!(builder.capacity(), 23);
    builder.push_str("orders");
    builder.push('@');
    write!(builder, "group-{}", 7).unwrap();
    assert_eq!(builder.as_str(), "orders@group-7");
    assert_eq!(builder.len(), 14);
    assert_eq!(builder.capacity(), 23);

    builder.push_str("-and-a-long-tail-\u{1f980}");
    assert_eq!(builder.as_str(), "orders@group-7-and-a-long-tail-\u{1f980}");
    assert!(builder.capacity() >= builder.len());
    let long = builder.finish();
    assert_eq!(long, "orders@group-7-and-a-long-tail-\u{1f980}");

    let mut builder = CheetahBuilder::with_capacity(8);
    builder.extend(['a', '\u{e9}', 'b']);
    builder.extend(["-x", "-y"]);
    assert_eq!(builder.as_str(), "a\u{e9}b-x-y");
    builder.clear();
    assert!(builder.is_empty());
    builder.reserve(64);
    assert!(builder.capacity() >= 64);
    builder.push_str("after-reserve");
    assert_eq!(builder.clone().into_string(), "after-reserve");
    assert!(format!("{builder:?}").contains("after-reserve"));

    let mut builder = CheetahBuilder::with_capacity(100);
    assert!(builder.capacity() >= 100);
    builder.push_str("heap");
    builder.clear();
    assert!(builder.capacity() >= 100);
    assert_eq!(CheetahBuilder::from("short").into_string(), "short");
    assert_eq!(CheetahBuilder::default().finish(), "");
}
