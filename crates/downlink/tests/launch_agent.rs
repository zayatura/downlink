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
