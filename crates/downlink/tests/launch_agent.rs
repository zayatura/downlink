use std::path::Path;

use downlink::platform::launch_agent;

#[test]
fn plist_runs_the_executable_at_login_and_restarts_only_after_crashes() {
    let plist = launch_agent::plist(Path::new(
        "/Applications/Downlink.app/Contents/MacOS/downlink",
    ));
    assert!(plist.contains("<string>com.zayatura.downlink</string>"));
    assert!(plist.contains("<string>/Applications/Downlink.app/Contents/MacOS/downlink</string>"));
    assert!(plist.contains("<key>RunAtLoad</key>\n    <true/>"));
    assert!(plist.contains("<key>SuccessfulExit</key>\n        <false/>"));
}

#[test]
fn executable_paths_are_xml_escaped() {
    let plist = launch_agent::plist(Path::new("/Apps/A&B <x>.app/downlink"));
    assert!(plist.contains("<string>/Apps/A&amp;B &lt;x&gt;.app/downlink</string>"));
}

#[test]
fn install_and_remove_under_a_home_directory() {
    let home = tempfile::tempdir().unwrap();
    let exe = Path::new("/Applications/Downlink.app/Contents/MacOS/downlink");
    let path = launch_agent::install(home.path(), exe).unwrap();
    assert_eq!(
        path,
        home.path()
            .join("Library/LaunchAgents/com.zayatura.downlink.plist")
    );
    assert!(launch_agent::is_installed(home.path()));
    launch_agent::remove(home.path()).unwrap();
    assert!(!launch_agent::is_installed(home.path()));
    launch_agent::remove(home.path()).expect("removing twice is fine");
}

const BUNDLED: &str = "/Applications/Downlink.app/Contents/MacOS/downlink";
const DEV_BUILD: &str = "/tmp/checkout/target/debug/downlink";

#[test]
fn a_dev_build_never_refreshes_the_login_item() {
    let dev = Path::new(DEV_BUILD);
    let other = launch_agent::plist(Path::new(BUNDLED));
    assert!(!launch_agent::needs_refresh(None, dev));
    assert!(!launch_agent::needs_refresh(Some(&other), dev));
}

#[test]
fn a_bundled_app_installs_a_missing_login_item() {
    assert!(launch_agent::needs_refresh(None, Path::new(BUNDLED)));
}

#[test]
fn a_bundled_app_leaves_an_identical_login_item_alone() {
    let exe = Path::new(BUNDLED);
    let existing = launch_agent::plist(exe);
    assert!(!launch_agent::needs_refresh(Some(&existing), exe));
}

#[test]
fn a_moved_bundle_repoints_the_login_item() {
    let old = launch_agent::plist(Path::new("/Applications/Old.app/Contents/MacOS/downlink"));
    assert!(launch_agent::needs_refresh(Some(&old), Path::new(BUNDLED)));
}

#[test]
fn bundle_detection_needs_the_contents_macos_layout() {
    assert!(launch_agent::is_app_bundle_executable(Path::new(BUNDLED)));
    for not_a_bundle in [
        "/tmp/Downlink.app/downlink",
        "/tmp/x/Contents/MacOS/downlink",
        "/tmp/checkout/target/release/downlink",
    ] {
        assert!(
            !launch_agent::is_app_bundle_executable(Path::new(not_a_bundle)),
            "{not_a_bundle}"
        );
    }
}
