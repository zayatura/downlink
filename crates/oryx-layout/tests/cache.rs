use oryx_layout::LayoutCache;
use oryx_protocol::LayoutIds;

fn ids(text: &str) -> LayoutIds {
    LayoutIds::parse(text).expect("valid ids")
}

#[test]
fn stores_and_loads_raw_json() {
    let dir = tempfile::tempdir().unwrap();
    let cache = LayoutCache::new(dir.path().join("layouts"));
    let ids = ids("abc12/rev99");
    assert_eq!(cache.load(&ids), None);
    assert_eq!(cache.modified(&ids), None);
    cache
        .store(&ids, r#"{"data":{}}"#)
        .expect("store creates the directory");
    assert_eq!(cache.load(&ids).as_deref(), Some(r#"{"data":{}}"#));
    assert!(cache.modified(&ids).is_some());
}

#[test]
fn different_revisions_are_different_files() {
    let dir = tempfile::tempdir().unwrap();
    let cache = LayoutCache::new(dir.path());
    cache.store(&ids("abc12/rev1"), "one").unwrap();
    cache.store(&ids("abc12/rev2"), "two").unwrap();
    assert_eq!(cache.load(&ids("abc12/rev1")).as_deref(), Some("one"));
    assert_eq!(cache.load(&ids("abc12/rev2")).as_deref(), Some("two"));
}

#[test]
fn cache_files_always_live_directly_inside_the_cache_dir() {
    let dir = tempfile::tempdir().unwrap();
    let cache = LayoutCache::new(dir.path());
    let path = cache.path_for(&ids("abc12/rev99"));
    assert_eq!(path.parent(), Some(dir.path()));
    assert_eq!(path.file_name().unwrap(), "abc12-rev99.json");
}
