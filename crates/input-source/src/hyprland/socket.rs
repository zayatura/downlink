//! Finding Hyprland's IPC socket and sending it one request (spec §6.1).

use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;
use std::{env, fs};

use crate::InputSourceError;

const SOCKET_NAME: &str = ".socket.sock";
/// Hyprland answers in milliseconds; this only stops a hung compositor from freezing Downlink.
const TIMEOUT: Duration = Duration::from_secs(1);

fn platform(error: impl std::fmt::Display) -> InputSourceError {
    InputSourceError::Platform(format!("Hyprland IPC: {error}"))
}

/// The instance named by `signature` if its socket exists; otherwise the one instance under
/// `<runtime_dir>/hypr/` whose socket is `alive`. A shell started before a Hyprland restart
/// still names the old instance.
pub fn resolve_socket(
    runtime_dir: &Path,
    signature: Option<&str>,
    alive: impl Fn(&Path) -> bool,
) -> Result<PathBuf, InputSourceError> {
    let hypr = runtime_dir.join("hypr");
    if let Some(signature) = signature.filter(|s| !s.is_empty()) {
        let socket = hypr.join(signature).join(SOCKET_NAME);
        if socket.exists() {
            return Ok(socket);
        }
    }
    let live: Vec<PathBuf> = fs::read_dir(&hypr)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path().join(SOCKET_NAME))
        .filter(|socket| alive(socket))
        .collect();
    match live.as_slice() {
        [socket] => Ok(socket.clone()),
        [] => Err(InputSourceError::Platform("Hyprland is not running".into())),
        _ => Err(InputSourceError::Platform(
            "several Hyprland instances are running".into(),
        )),
    }
}

pub fn accepts_connections(path: &Path) -> bool {
    UnixStream::connect(path).is_ok()
}

/// One request per connection: write the command, read until Hyprland closes the socket.
pub fn request(socket: &Path, command: &str) -> Result<String, InputSourceError> {
    let mut stream = UnixStream::connect(socket).map_err(platform)?;
    stream.set_read_timeout(Some(TIMEOUT)).map_err(platform)?;
    stream.set_write_timeout(Some(TIMEOUT)).map_err(platform)?;
    stream.write_all(command.as_bytes()).map_err(platform)?;
    let mut reply = String::new();
    stream.read_to_string(&mut reply).map_err(platform)?;
    Ok(reply)
}

/// The running compositor's socket, from `XDG_RUNTIME_DIR` and `HYPRLAND_INSTANCE_SIGNATURE`.
pub fn system_socket() -> Result<PathBuf, InputSourceError> {
    let runtime = env::var_os("XDG_RUNTIME_DIR")
        .ok_or_else(|| InputSourceError::Platform("XDG_RUNTIME_DIR is not set".into()))?;
    let signature = env::var("HYPRLAND_INSTANCE_SIGNATURE").ok();
    resolve_socket(
        Path::new(&runtime),
        signature.as_deref(),
        accepts_connections,
    )
}
