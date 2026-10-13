use std::io;
use std::sync::{Arc, Mutex};

use tracing_subscriber::fmt::MakeWriter;

#[derive(Clone, Default)]
struct Captured(Arc<Mutex<Vec<u8>>>);

impl io::Write for Captured {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for Captured {
    type Writer = Captured;
    fn make_writer(&'a self) -> Captured {
        self.clone()
    }
}

#[test]
fn every_config_warning_is_logged_and_the_valid_parts_are_kept() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(
        &path,
        "[switching.layers]\n\"abc\" = \"com.example.en\"\n\"1\" = \"com.example.ro\"\n",
    )
    .unwrap();
    let log = Captured::default();
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .with_writer(log.clone())
        .finish();

    let config =
        tracing::subscriber::with_default(subscriber, || downlink::load_config_logged(&path));

    assert_eq!(
        config.layers.get(&1).map(String::as_str),
        Some("com.example.ro")
    );
    let text = String::from_utf8(log.0.lock().unwrap().clone()).unwrap();
    assert!(text.contains("WARN"), "{text}");
    assert!(
        text.contains("abc"),
        "the invalid layer key is named in the warning: {text}"
    );
}

#[test]
fn a_corrupt_file_is_moved_aside_and_the_warning_is_logged() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, "this is = = not toml").unwrap();
    let log = Captured::default();
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .with_writer(log.clone())
        .finish();

    let config =
        tracing::subscriber::with_default(subscriber, || downlink::load_config_logged(&path));

    assert_eq!(config, companion_core::Config::default());
    assert!(!path.exists());
    let text = String::from_utf8(log.0.lock().unwrap().clone()).unwrap();
    assert!(
        text.contains("config.toml.bad-"),
        "the backup name is the only record: {text}"
    );
}

#[test]
fn first_run_writes_the_defaults() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nested/config.toml");

    let config = downlink::load_or_create_config(&path);

    assert_eq!(config, companion_core::Config::default());
    assert!(path.is_file(), "the settings file is created");
    let loaded = companion_core::config::load(&path);
    assert_eq!(loaded.config, companion_core::Config::default());
    assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
}

#[test]
fn an_existing_file_is_not_rewritten() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    let text = "# my notes\n[switching]\nfallback = \"com.example.en\"\n";
    std::fs::write(&path, text).unwrap();

    let config = downlink::load_or_create_config(&path);

    assert_eq!(config.fallback.as_deref(), Some("com.example.en"));
    assert_eq!(std::fs::read(&path).unwrap(), text.as_bytes());
}

#[test]
fn a_corrupt_file_is_moved_aside_and_not_replaced() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, "this is = = not toml").unwrap();

    let config = downlink::load_or_create_config(&path);

    assert_eq!(config, companion_core::Config::default());
    assert!(!path.exists(), "no fresh file over a file the user wrote");
    let backups: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with("config.toml.bad-"))
        .collect();
    assert_eq!(backups.len(), 1, "{backups:?}");
}
