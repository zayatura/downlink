#![cfg(target_os = "linux")]

use std::fs;
use std::io::{Read, Write};
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};

use input_source::InputSourceError;
use input_source::hyprland::socket::{request, resolve_socket};

fn instance(runtime: &Path, name: &str) -> PathBuf {
    let dir = runtime.join("hypr").join(name);
    fs::create_dir_all(&dir).unwrap();
    let socket = dir.join(".socket.sock");
    fs::write(&socket, "").unwrap();
    socket
}

fn never_alive(_: &Path) -> bool {
    false
}

#[test]
fn the_signature_from_the_environment_wins() {
    let runtime = tempfile::tempdir().unwrap();
    let wanted = instance(runtime.path(), "abc_1");
    instance(runtime.path(), "def_2");
    assert_eq!(
        resolve_socket(runtime.path(), Some("abc_1"), never_alive).unwrap(),
        wanted
    );
}

#[test]
fn a_stale_signature_falls_back_to_the_live_instance() {
    let runtime = tempfile::tempdir().unwrap();
    let live = instance(runtime.path(), "live_2");
    let alive = |p: &Path| p.starts_with(runtime.path().join("hypr/live_2"));
    assert_eq!(
        resolve_socket(runtime.path(), Some("gone_1"), alive).unwrap(),
        live
    );
    assert_eq!(resolve_socket(runtime.path(), None, alive).unwrap(), live);
}

#[test]
fn no_live_instance_is_an_error() {
    let runtime = tempfile::tempdir().unwrap();
    assert_eq!(
        resolve_socket(runtime.path(), None, never_alive),
        Err(InputSourceError::Platform("Hyprland is not running".into()))
    );
    instance(runtime.path(), "dead_1");
    assert_eq!(
        resolve_socket(runtime.path(), Some("gone"), never_alive),
        Err(InputSourceError::Platform("Hyprland is not running".into()))
    );
}

#[test]
fn several_live_instances_are_an_error() {
    let runtime = tempfile::tempdir().unwrap();
    instance(runtime.path(), "a_1");
    instance(runtime.path(), "b_2");
    assert_eq!(
        resolve_socket(runtime.path(), None, |_| true),
        Err(InputSourceError::Platform(
            "several Hyprland instances are running".into()
        ))
    );
}

#[test]
fn a_request_writes_the_command_and_reads_the_reply_to_the_end() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".socket.sock");
    let listener = UnixListener::bind(&path).unwrap();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut buf = [0u8; 256];
        let n = stream.read(&mut buf).unwrap();
        stream.write_all(b"ok\n\n\nok").unwrap();
        String::from_utf8_lossy(&buf[..n]).into_owned()
    });
    let reply = request(&path, "[[BATCH]]switchxkblayout a 0 ; switchxkblayout b 0").unwrap();
    assert_eq!(reply, "ok\n\n\nok");
    assert_eq!(
        server.join().unwrap(),
        "[[BATCH]]switchxkblayout a 0 ; switchxkblayout b 0"
    );
}

#[test]
fn a_request_to_a_missing_socket_is_a_platform_error() {
    let dir = tempfile::tempdir().unwrap();
    assert!(matches!(
        request(&dir.path().join(".socket.sock"), "j/devices"),
        Err(InputSourceError::Platform(_))
    ));
}
