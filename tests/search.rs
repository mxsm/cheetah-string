use cheetah_string::{CheetahFinder, CheetahString};

#[test]
fn empty_needle_matches_str_find_semantics() {
    let s = CheetahString::from("hello");

    assert_eq!(s.find(""), "hello".find(""));
    assert_eq!(s.rfind(""), "hello".rfind(""));
    assert!(s.contains(""));
}

#[test]
fn memmem_search_reports_byte_indices() {
    let s = CheetahString::from("cafe cafe");

    assert_eq!(s.find("fe"), Some(2));
    assert_eq!(s.rfind("fe"), Some(7));
    assert_eq!(s.find("missing"), None);
}

#[test]
fn unicode_search_matches_str_indices() {
    let s = CheetahString::from("éxé");

    assert_eq!(s.find("é"), "éxé".find("é"));
    assert_eq!(s.rfind("é"), "éxé".rfind("é"));
    assert_eq!(s.find("xé"), "éxé".find("xé"));
}

#[test]
fn reusable_finder_matches_repeated_needle() {
    let finder = CheetahFinder::new("route");
    let first = CheetahString::from("topic.route.alpha");
    let second = CheetahString::from("topic.name.beta");

    assert_eq!(finder.needle(), "route");
    assert_eq!(finder.find_in(&first), Some(6));
    assert_eq!(finder.find_in(&second), None);
    assert!(finder.is_match(&first));
    assert!(!finder.is_match(&second));
}

#[test]
fn reusable_empty_finder_matches_start() {
    let finder = CheetahFinder::new("");
    let s = CheetahString::from("payload");

    assert_eq!(finder.find_in(&s), Some(0));
    assert!(finder.is_match(&s));
}
