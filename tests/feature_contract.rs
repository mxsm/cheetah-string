#[cfg(feature = "experimental-packed")]
#[test]
fn retired_experimental_packed_feature_is_noop() {
    let value = cheetah_string::CheetahString::from("stable-value");
    assert_eq!(value, "stable-value");
}
