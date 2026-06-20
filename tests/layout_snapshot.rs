use cheetah_string::CheetahString;
use std::env;
use std::fs;
use std::mem::{align_of, size_of};
use std::path::PathBuf;

fn target_dir() -> PathBuf {
    env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target"))
}

fn layout_entry<T>(name: &str) -> String {
    format!(
        r#"{{"type":"{}","size":{},"align":{}}}"#,
        name,
        size_of::<T>(),
        align_of::<T>()
    )
}

#[test]
fn layout_snapshot() {
    let layouts = [
        layout_entry::<CheetahString>("CheetahString"),
        layout_entry::<Option<CheetahString>>("Option<CheetahString>"),
        layout_entry::<String>("String"),
        layout_entry::<Option<String>>("Option<String>"),
        layout_entry::<&str>("&str"),
        layout_entry::<Option<&str>>("Option<&str>"),
        layout_entry::<std::sync::Arc<str>>("Arc<str>"),
        layout_entry::<Option<std::sync::Arc<str>>>("Option<Arc<str>>"),
    ];

    let snapshot = format!(
        concat!(
            "{{\n",
            "  \"crate\":\"cheetah-string\",\n",
            "  \"profile\":\"test\",\n",
            "  \"target_arch\":\"{}\",\n",
            "  \"target_os\":\"{}\",\n",
            "  \"pointer_width\":\"{}\",\n",
            "  \"layouts\":[\n    {}\n  ]\n",
            "}}\n"
        ),
        env::consts::ARCH,
        env::consts::OS,
        std::mem::size_of::<usize>() * 8,
        layouts.join(",\n    ")
    );

    let artifact_dir = target_dir().join("layout-artifacts");
    fs::create_dir_all(&artifact_dir).expect("create layout artifact directory");
    fs::write(artifact_dir.join("layout-snapshot.json"), &snapshot)
        .expect("write layout snapshot artifact");

    println!("{snapshot}");

    assert!(size_of::<CheetahString>() >= size_of::<usize>());
    assert!(align_of::<CheetahString>() >= align_of::<usize>());
    assert!(size_of::<Option<CheetahString>>() >= size_of::<CheetahString>());
}
