use std::fs;
use std::path::Path;
use std::time::{Duration, UNIX_EPOCH};

use companion_core::config::{self, Appearance, Config};

const SPEC_EXAMPLE: &str = r#"
appearance = "system"
launch_at_login = true

[switching]
fallback = "com.apple.keylayout.US"

[switching.layers]
"0" = "com.example.keylayout.english"
"1" = "com.example.keylayout.romanian"
"2" = "com.example.keylayout.hungarian"
"#;

#[test]
fn defaults_are_system_appearance_and_no_switching() {
    let c = Config::default();
    assert_eq!(c.appearance, Appearance::System);
    assert!(!c.launch_at_login);
    assert_eq!(c.fallback, None);
    assert!(c.layers.is_empty());
}

#[test]
fn a_missing_file_gives_defaults_without_warnings() {
    let dir = tempfile::tempdir().unwrap();
    let loaded = config::load(&dir.path().join("config.toml"));
    assert_eq!(loaded.config, Config::default());
    assert!(loaded.warnings.is_empty());
}

#[test]
fn parses_the_spec_example() {
    let (c, warnings) = config::parse(SPEC_EXAMPLE).unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(c.appearance, Appearance::System);
    assert!(c.launch_at_login);
    assert_eq!(c.fallback.as_deref(), Some("com.apple.keylayout.US"));
    assert_eq!(
        c.layers.get(&0).map(String::as_str),
        Some("com.example.keylayout.english")
    );
    assert_eq!(
        c.layers.get(&2).map(String::as_str),
        Some("com.example.keylayout.hungarian")
    );
    assert_eq!(c.layers.len(), 3);
}

#[test]
fn save_then_load_round_trips() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nested").join("config.toml");
    let (mut original, _) = config::parse(SPEC_EXAMPLE).unwrap();
    original.appearance = Appearance::Dark;
    config::save(&path, &original).unwrap();
    let loaded = config::load(&path);
    assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
    assert_eq!(loaded.config, original);
}

#[test]
fn to_toml_output_parses_back() {
    let mut c = Config::default();
    c.layers.insert(10, "com.example.keylayout.x".into());
    let (back, warnings) = config::parse(&config::to_toml(&c)).unwrap();
    assert!(warnings.is_empty());
    assert_eq!(back, c);
}

#[test]
fn invalid_layer_entries_are_skipped_with_one_warning_each() {
    let text = r#"
[switching.layers]
"0" = "com.example.keylayout.english"
"abc" = "x"
"-1" = "y"
"300" = "z"
"4" = ""
"5" = 7
"#;
    let (c, warnings) = config::parse(text).unwrap();
    assert_eq!(c.layers.len(), 1);
    assert_eq!(
        c.layers.get(&0).map(String::as_str),
        Some("com.example.keylayout.english")
    );
    assert_eq!(warnings.len(), 5, "{warnings:?}");
}

#[test]
fn unknown_appearance_and_wrong_types_keep_the_valid_parts() {
    let text = r#"
appearance = "purple"
launch_at_login = "yes"

[switching]
fallback = "com.apple.keylayout.US"
"#;
    let (c, warnings) = config::parse(text).unwrap();
    assert_eq!(c.appearance, Appearance::System);
    assert!(!c.launch_at_login);
    assert_eq!(c.fallback.as_deref(), Some("com.apple.keylayout.US"));
    assert_eq!(warnings.len(), 2, "{warnings:?}");
}

#[test]
fn a_blank_fallback_means_no_change() {
    let (c, warnings) = config::parse("[switching]\nfallback = \"  \"\n").unwrap();
    assert_eq!(c.fallback, None);
    assert!(warnings.is_empty());
}

#[test]
fn switching_must_be_a_table() {
    let (c, warnings) = config::parse("switching = 3\n").unwrap();
    assert_eq!(c, Config::default());
    assert_eq!(warnings.len(), 1);
}

#[test]
fn a_corrupt_file_is_moved_aside_and_defaults_are_used() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    fs::write(&path, "appearance = [unclosed").unwrap();
    let loaded = config::load(&path);
    assert_eq!(loaded.config, Config::default());
    assert_eq!(loaded.warnings.len(), 1);
    assert!(
        loaded.warnings[0].contains("not valid TOML"),
        "{:?}",
        loaded.warnings
    );
    assert!(!path.exists(), "the corrupt file was moved");
    let backups: Vec<String> = fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with("config.toml.bad-"))
        .collect();
    assert_eq!(backups.len(), 1);
}

#[test]
fn a_non_utf8_file_is_moved_aside_with_its_bytes_intact() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    let original: &[u8] = &[0xff, 0xfe, b'a'];
    fs::write(&path, original).unwrap();
    let loaded = config::load(&path);
    assert_eq!(loaded.config, Config::default());
    assert_eq!(loaded.warnings.len(), 1, "{:?}", loaded.warnings);
    assert!(!path.exists(), "the non-UTF-8 file was moved");
    let backups: Vec<std::path::PathBuf> = fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| {
            p.file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with("config.toml.bad-"))
        })
        .collect();
    assert_eq!(backups.len(), 1);
    assert_eq!(fs::read(&backups[0]).unwrap(), original);
}

#[test]
fn backup_names_use_unix_seconds() {
    let backup = config::backup_path(
        Path::new("/x/config.toml"),
        UNIX_EPOCH + Duration::from_secs(1234),
    );
    assert_eq!(backup, Path::new("/x/config.toml.bad-1234"));
}
