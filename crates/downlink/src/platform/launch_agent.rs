//! A per-user LaunchAgent that starts Downlink at login (spec §12).

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::{fs, io};

pub const LABEL: &str = "com.zayatura.downlink";

pub fn plist(executable: &Path) -> String {
    let exe = xml_escape(&executable.to_string_lossy());
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{LABEL}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{exe}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <dict>
        <key>SuccessfulExit</key>
        <false/>
    </dict>
    <key>ProcessType</key>
    <string>Interactive</string>
    <key>LimitLoadToSessionType</key>
    <string>Aqua</string>
</dict>
</plist>
"#
    )
}

fn xml_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// True for `…/Name.app/Contents/MacOS/<exe>`: only a bundled Downlink keeps the login item current.
pub fn is_app_bundle_executable(executable: &Path) -> bool {
    fn name_of(path: Option<&Path>) -> Option<&OsStr> {
        path.and_then(Path::file_name)
    }
    let macos = executable.parent();
    let contents = macos.and_then(Path::parent);
    let bundle = contents.and_then(Path::parent);
    name_of(macos) == Some("MacOS".as_ref())
        && name_of(contents) == Some("Contents".as_ref())
        && bundle.and_then(Path::extension) == Some("app".as_ref())
}

/// Whether startup should rewrite the login item: only from an app bundle, and only when the
/// plist on disk (`existing`, `None` if missing) differs from the one for `executable`.
pub fn needs_refresh(existing: Option<&str>, executable: &Path) -> bool {
    is_app_bundle_executable(executable) && existing != Some(plist(executable).as_str())
}

pub fn agent_path(home: &Path) -> PathBuf {
    home.join("Library/LaunchAgents")
        .join(format!("{LABEL}.plist"))
}

pub fn install(home: &Path, executable: &Path) -> io::Result<PathBuf> {
    let path = agent_path(home);
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(&path, plist(executable))?;
    Ok(path)
}

pub fn remove(home: &Path) -> io::Result<()> {
    match fs::remove_file(agent_path(home)) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

pub fn is_installed(home: &Path) -> bool {
    agent_path(home).exists()
}
