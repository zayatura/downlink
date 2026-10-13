use input_source::{FakeBackend, InputSource, InputSourceBackend, InputSourceError};

fn sources() -> Vec<InputSource> {
    vec![
        InputSource {
            id: "com.example.keylayout.english".into(),
            name: "English".into(),
        },
        InputSource {
            id: "com.example.keylayout.romanian".into(),
            name: "Romanian".into(),
        },
    ]
}

#[test]
fn selecting_a_known_source_makes_it_current() {
    let backend = FakeBackend::new(sources(), "com.example.keylayout.english");
    backend.select("com.example.keylayout.romanian").unwrap();
    assert_eq!(backend.current().unwrap(), "com.example.keylayout.romanian");
    assert_eq!(
        backend.selections(),
        vec!["com.example.keylayout.romanian".to_owned()]
    );
    assert_eq!(backend.list().unwrap(), sources());
}

#[test]
fn selecting_an_unknown_source_fails_and_changes_nothing() {
    let backend = FakeBackend::new(sources(), "com.example.keylayout.english");
    let error = backend.select("com.example.missing").unwrap_err();
    assert!(matches!(error, InputSourceError::NotFound(id) if id == "com.example.missing"));
    assert_eq!(backend.current().unwrap(), "com.example.keylayout.english");
    assert!(backend.selections().is_empty());
}
