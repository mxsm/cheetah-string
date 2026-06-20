use cheetah_string::CheetahString;

#[test]
fn inline_push_str_appends_in_place() {
    let mut s = CheetahString::from("hello");
    let before = s.as_bytes().as_ptr();

    s.push_str(" world");

    assert_eq!(s, "hello world");
    assert_eq!(s.as_bytes().as_ptr(), before);
}

#[test]
fn owned_push_str_reuses_spare_capacity() {
    let mut s = CheetahString::with_capacity(128);
    s.push_str("hello");
    let before = s.as_bytes().as_ptr();

    s.push_str(" world");

    assert_eq!(s, "hello world");
    assert_eq!(s.as_bytes().as_ptr(), before);
}

#[test]
fn add_reuses_owned_lhs_capacity() {
    let mut s = CheetahString::with_capacity(128);
    s.push_str("hello");
    let before = s.as_bytes().as_ptr();

    let s = s + " world";

    assert_eq!(s, "hello world");
    assert_eq!(s.as_bytes().as_ptr(), before);
}

#[test]
fn from_string_owned_preserves_spare_capacity_for_push() {
    let mut raw = String::with_capacity(128);
    raw.push_str("hello");

    let mut s = CheetahString::from_string_owned(raw);
    let before = s.as_bytes().as_ptr();

    s.push_str(" world");

    assert_eq!(s, "hello world");
    assert_eq!(s.as_bytes().as_ptr(), before);
}

#[test]
fn from_string_shared_keeps_long_immutable_inputs_clone_cheap() {
    let s = CheetahString::from_string_shared("a".repeat(128));
    let cloned = s.clone();

    assert_eq!(s.as_str(), cloned.as_str());
    assert_eq!(s.as_bytes().as_ptr(), cloned.as_bytes().as_ptr());
}

#[test]
fn reserve_zero_keeps_existing_buffer() {
    let mut s = CheetahString::with_capacity(128);
    s.push_str("hello");
    let before = s.as_bytes().as_ptr();

    s.reserve(0);

    assert_eq!(s, "hello");
    assert_eq!(s.as_bytes().as_ptr(), before);
}

#[test]
fn add_assign_uses_push_str_path() {
    let mut s = CheetahString::with_capacity(128);
    s.push_str("hello");
    let before = s.as_bytes().as_ptr();

    s += " world";

    assert_eq!(s, "hello world");
    assert_eq!(s.as_bytes().as_ptr(), before);
}
