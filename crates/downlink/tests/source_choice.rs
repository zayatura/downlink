use downlink::source_choice::SourceChoice;
use input_source::InputSource;

fn sources() -> Vec<InputSource> {
    vec![
        InputSource {
            id: "com.example.en".into(),
            name: "English".into(),
        },
        InputSource {
            id: "com.example.ro".into(),
            name: "Romanian".into(),
        },
    ]
}

#[test]
fn options_start_with_no_change() {
    let options = SourceChoice::options(&sources());
    assert_eq!(options[0], SourceChoice::NoChange);
    assert_eq!(options.len(), 3);
    assert_eq!(options[2].to_string(), "Romanian");
}

#[test]
fn current_choice_resolves_installed_and_missing_sources() {
    assert_eq!(
        SourceChoice::current(None, &sources()),
        SourceChoice::NoChange
    );
    assert_eq!(
        SourceChoice::current(Some("com.example.en"), &sources()),
        SourceChoice::Source {
            id: "com.example.en".into(),
            name: "English".into()
        }
    );
    let missing = SourceChoice::current(Some("com.example.gone"), &sources());
    assert!(missing.is_missing());
    assert_eq!(missing.to_string(), "⚠ com.example.gone (not installed)");
    assert_eq!(
        missing.id(),
        Some("com.example.gone"),
        "keeps the mapping until the user changes it"
    );
}

#[test]
fn no_change_has_no_id() {
    assert_eq!(SourceChoice::NoChange.id(), None);
    assert_eq!(SourceChoice::NoChange.to_string(), "No change");
}
