use std::path::Path;

use downlink::paths::Paths;

#[test]
fn macos_layout_matches_the_spec() {
    let p = Paths::macos(Path::new("/fake-home"));
    assert_eq!(
        p.config_file,
        Path::new("/fake-home/Library/Application Support/Downlink/config.toml")
    );
    assert_eq!(p.cache_dir, Path::new("/fake-home/Library/Caches/Downlink"));
    assert_eq!(p.log_dir, Path::new("/fake-home/Library/Logs/Downlink"));
    assert_eq!(
        p.lock_file,
        Path::new("/fake-home/Library/Caches/Downlink/downlink.lock")
    );
    assert_eq!(
        p.layout_cache_dir(),
        Path::new("/fake-home/Library/Caches/Downlink/layouts")
    );
}

#[test]
fn xdg_layout_prefers_the_runtime_dir_for_the_lock() {
    let p = Paths::xdg(
        Path::new("/x/config"),
        Path::new("/x/cache"),
        Path::new("/x/state"),
        Some(Path::new("/run/user/1000")),
    );
    assert_eq!(p.config_file, Path::new("/x/config/downlink/config.toml"));
    assert_eq!(p.cache_dir, Path::new("/x/cache/downlink"));
    assert_eq!(p.log_dir, Path::new("/x/state/downlink"));
    assert_eq!(p.lock_file, Path::new("/run/user/1000/downlink.lock"));
    let p = Paths::xdg(
        Path::new("/x/config"),
        Path::new("/x/cache"),
        Path::new("/x/state"),
        None,
    );
    assert_eq!(p.lock_file, Path::new("/x/cache/downlink/downlink.lock"));
}

#[test]
fn detect_finds_paths_on_this_machine() {
    assert!(Paths::detect().is_some());
}
