use std::path::Path;

use downlink::platform::autostart;

const INSTALLED: &str = "/opt/downlink/bin/downlink";

#[test]
fn the_entry_starts_the_executable_hidden_from_launchers() {
    let entry = autostart::desktop_entry(Path::new(INSTALLED));
    assert!(entry.starts_with("[Desktop Entry]\n"));
    for line in [
        "Type=Application",
        "Name=Downlink",
        "Exec=/opt/downlink/bin/downlink",
        "Icon=downlink",
        "NoDisplay=true",
    ] {
        assert!(
            entry.lines().any(|l| l == line),
            "missing {line:?} in\n{entry}"
        );
    }
}

#[test]
fn exec_quotes_and_escapes_reserved_characters() {
    assert_eq!(autostart::exec_arg(Path::new(INSTALLED)), INSTALLED);
    assert_eq!(
        autostart::exec_arg(Path::new("/opt/My Apps/downlink")),
        r#""/opt/My Apps/downlink""#
    );
    assert_eq!(
        autostart::exec_arg(Path::new("/opt/50%/downlink")),
        "/opt/50%%/downlink"
    );
    // Quoting escapes `$` with one backslash; the string escape then doubles that backslash.
    assert_eq!(
        autostart::exec_arg(Path::new("/opt/a$b/downlink")),
        r#""/opt/a\\$b/downlink""#
    );
    assert_eq!(
        autostart::exec_arg(Path::new(r#"/opt/say "hi"/downlink"#)),
        r#""/opt/say \\"hi\\"/downlink""#
    );
}

#[test]
fn install_and_remove_under_a_config_directory() {
    let config = tempfile::tempdir().unwrap();
    let path = autostart::install(config.path(), Path::new(INSTALLED)).unwrap();
    assert_eq!(path, config.path().join("autostart/downlink.desktop"));
    assert_eq!(path, autostart::entry_path(config.path()));
    assert!(autostart::is_installed(config.path()));
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        autostart::desktop_entry(Path::new(INSTALLED))
    );
    autostart::remove(config.path()).unwrap();
    assert!(!autostart::is_installed(config.path()));
    autostart::remove(config.path()).expect("removing twice is fine");
}

#[test]
fn cargo_build_directories_are_dev_builds() {
    for dev in [
        "/opt/src/downlink/target/debug/downlink",
        "/opt/src/downlink/target/release/downlink",
        "/opt/src/downlink/target/x86_64-unknown-linux-gnu/release/downlink",
    ] {
        assert!(autostart::is_dev_build(Path::new(dev)), "{dev}");
    }
    for installed in [INSTALLED, "/usr/local/bin/downlink", "/opt/target/downlink"] {
        assert!(
            !autostart::is_dev_build(Path::new(installed)),
            "{installed}"
        );
    }
}

#[test]
fn a_dev_build_never_refreshes_the_login_item() {
    let dev = Path::new("/opt/src/downlink/target/debug/downlink");
    assert!(!autostart::needs_refresh(None, dev));
    let other = autostart::desktop_entry(Path::new(INSTALLED));
    assert!(!autostart::needs_refresh(Some(&other), dev));
}

#[test]
fn an_installed_binary_writes_a_missing_or_different_entry_only() {
    let exe = Path::new(INSTALLED);
    assert!(autostart::needs_refresh(None, exe));
    let same = autostart::desktop_entry(exe);
    assert!(!autostart::needs_refresh(Some(&same), exe));
    let moved = autostart::desktop_entry(Path::new("/usr/local/bin/downlink"));
    assert!(autostart::needs_refresh(Some(&moved), exe));
}
