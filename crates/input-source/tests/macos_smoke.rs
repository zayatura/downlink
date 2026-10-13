//! Touches the real system. Run by hand: cargo test -p input-source -- --ignored
#![cfg(target_os = "macos")]

use input_source::system_backend;

#[test]
#[ignore = "talks to the real macOS input-source system"]
fn lists_sources_including_the_current_one() {
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
#[ignore = "talks to the real macOS input-source system"]
fn reselecting_the_current_source_succeeds() {
    let backend = system_backend();
    let current = backend.current().expect("current");
    backend.select(&current).expect("select current");
    assert_eq!(backend.current().unwrap(), current);
}

#[test]
#[ignore = "talks to the real macOS input-source system"]
fn concurrent_use_from_several_threads_is_safe() {
    // Text Input Sources is not thread-safe; the backend must serialize its own calls.
    // This only ever re-selects the source that is already current.
    let threads: Vec<_> = (0..4)
        .map(|_| {
            std::thread::spawn(|| {
                let backend = system_backend();
                for _ in 0..25 {
                    let current = backend.current().expect("current");
                    backend.list().expect("list");
                    backend.select(&current).expect("select current");
                }
            })
        })
        .collect();
    for thread in threads {
        thread.join().expect("no thread panicked");
    }
}
