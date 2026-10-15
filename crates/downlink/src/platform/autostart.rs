//! An XDG autostart entry that starts Downlink at login (spec §8.1). Pure, so it's tested on
//! every OS.

use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};
use std::{fs, io};

pub const FILE_NAME: &str = "downlink.desktop";

/// Characters that make the Desktop Entry spec require a quoted `Exec` argument.
const RESERVED: &[char] = &[
    ' ', '\t', '\n', '"', '\'', '\\', '>', '<', '~', '|', '&', ';', '$', '*', '?', '#', '(', ')',
    '`',
];

/// The executable as one `Exec=` argument. `%` is doubled (field codes); an argument with a
/// reserved character is quoted with `"`, `` ` ``, `$` and `\` backslash-escaped; then the string
/// escape doubles every backslash.
pub fn exec_arg(executable: &Path) -> String {
    let raw = executable.to_string_lossy().replace('%', "%%");
    let quoted = if raw.contains(RESERVED) {
        let mut out = String::from("\"");
        for c in raw.chars() {
            if matches!(c, '"' | '`' | '$' | '\\') {
                out.push('\\');
            }
            out.push(c);
        }
        out.push('"');
        out
    } else {
        raw
    };
    quoted.replace('\\', "\\\\")
}

pub fn desktop_entry(executable: &Path) -> String {
    format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=Downlink\n\
         Comment=Live layout and per-layer input switching for the ZSA Voyager\n\
         Exec={}\n\
         Icon=downlink\n\
         NoDisplay=true\n",
        exec_arg(executable)
    )
}

pub fn entry_path(config_dir: &Path) -> PathBuf {
    config_dir.join("autostart").join(FILE_NAME)
}

pub fn install(config_dir: &Path, executable: &Path) -> io::Result<PathBuf> {
    let path = entry_path(config_dir);
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(&path, desktop_entry(executable))?;
    Ok(path)
}

pub fn remove(config_dir: &Path) -> io::Result<()> {
    match fs::remove_file(entry_path(config_dir)) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

pub fn is_installed(config_dir: &Path) -> bool {
    entry_path(config_dir).exists()
}

/// True inside a cargo build directory: `…/target/debug/…`, `…/target/release/…`, or the same
/// under a target triple.
pub fn is_dev_build(executable: &Path) -> bool {
    let names: Vec<&OsStr> = executable
        .components()
        .filter_map(|c| match c {
            Component::Normal(name) => Some(name),
            _ => None,
        })
        .collect();
    names.iter().enumerate().any(|(i, name)| {
        *name == "target"
            && names[i + 1..]
                .iter()
                .take(2)
                .any(|next| *next == "debug" || *next == "release")
    })
}

/// Whether startup should rewrite the entry: never from a dev build, and only when the entry on
/// disk (`existing`, `None` if missing) differs from the one for `executable`.
pub fn needs_refresh(existing: Option<&str>, executable: &Path) -> bool {
    !is_dev_build(executable) && existing != Some(desktop_entry(executable).as_str())
}
