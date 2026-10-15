//! Touches the live compositor. Run by hand: cargo test -p input-source -- --ignored
#![cfg(target_os = "linux")]

use input_source::system_backend;

#[test]
#[ignore = "talks to the running Hyprland"]
fn lists_layouts_including_the_current_one() {
    let backend = system_backend();
    let sources = backend.list().expect("list");
    assert!(!sources.is_empty());
    let current = backend.current().expect("current");
    assert!(
        sources.iter().any(|s| s.id == current),
        "{current} not in {sources:?}"
    );
}

#[test]
#[ignore = "talks to the running Hyprland"]
fn reselecting_the_current_layout_succeeds() {
    let backend = system_backend();
    let current = backend.current().expect("current");
    backend.select(&current).expect("select current");
    assert_eq!(backend.current().unwrap(), current);
}

#[test]
#[ignore = "talks to the running Hyprland"]
fn selecting_an_unknown_layout_is_not_found() {
    let backend = system_backend();
    assert_eq!(
        backend.select("no-such-layout"),
        Err(input_source::InputSourceError::NotFound(
            "no-such-layout".into()
        ))
    );
}
