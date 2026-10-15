//! Lets a second launch reach the running Downlink (spec §8.5). The instance holding the
//! single-instance lock listens on a Unix socket next to the lock; a later launch sends it one
//! request line and exits.

use std::ffi::OsStr;
use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::time::Duration;
use std::{fs, thread};

/// Asks the running instance to open, or bring forward, its Layout window.
pub const SHOW_LAYOUT: &str = "show-layout";
/// The launcher entry passes it, so starting Downlink from the app launcher shows a window.
const SHOW_LAYOUT_FLAG: &str = "--show-layout";
/// A sender writes one short line; this only stops a stuck one from blocking the listener.
const TIMEOUT: Duration = Duration::from_secs(1);

pub fn socket_path(lock_file: &Path) -> PathBuf {
    lock_file.with_extension("sock")
}

/// Whether the command-line arguments (without the program name) ask for the Layout window.
pub fn wants_show_layout<I>(args: I) -> bool
where
    I: IntoIterator,
    I::Item: AsRef<OsStr>,
{
    args.into_iter().any(|arg| arg.as_ref() == SHOW_LAYOUT_FLAG)
}

/// Binds the socket and hands each request line to `on_request` from a background thread.
/// Call it only while holding the single-instance lock: a socket file already at `path` is left
/// from an instance that has exited, so it is replaced.
pub fn listen(path: &Path, on_request: impl Fn(String) + Send + 'static) -> io::Result<()> {
    match fs::remove_file(path) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => return Err(error),
        _ => {}
    }
    let listener = UnixListener::bind(path)?;
    thread::Builder::new()
        .name("instance-channel".into())
        .spawn(move || {
            for stream in listener.incoming() {
                match stream.and_then(read_request) {
                    Ok(request) => on_request(request),
                    Err(error) => tracing::debug!(%error, "ignoring a bad instance request"),
                }
            }
        })?;
    Ok(())
}

fn read_request(stream: UnixStream) -> io::Result<String> {
    stream.set_read_timeout(Some(TIMEOUT))?;
    let mut line = String::new();
    BufReader::new(stream).read_line(&mut line)?;
    Ok(line.trim().to_owned())
}

/// Sends one request to the running instance.
pub fn send(path: &Path, request: &str) -> io::Result<()> {
    let mut stream = UnixStream::connect(path)?;
    stream.set_write_timeout(Some(TIMEOUT))?;
    stream.write_all(format!("{request}\n").as_bytes())
}
