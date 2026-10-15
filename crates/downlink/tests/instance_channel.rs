use std::os::unix::net::UnixListener;
use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;

use downlink::instance_channel::{self, SHOW_LAYOUT};

fn listen_into(path: &Path) -> mpsc::Receiver<String> {
    let (tx, rx) = mpsc::channel();
    instance_channel::listen(path, move |request| {
        let _ = tx.send(request);
    })
    .unwrap();
    rx
}

#[test]
fn the_socket_sits_next_to_the_lock() {
    assert_eq!(
        instance_channel::socket_path(Path::new("/run/user/1000/downlink.lock")),
        Path::new("/run/user/1000/downlink.sock")
    );
}

#[test]
fn a_second_launch_reaches_the_running_instance() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("downlink.sock");
    let requests = listen_into(&path);
    instance_channel::send(&path, SHOW_LAYOUT).unwrap();
    assert_eq!(
        requests.recv_timeout(Duration::from_secs(2)).unwrap(),
        SHOW_LAYOUT
    );
    instance_channel::send(&path, SHOW_LAYOUT).unwrap();
    assert_eq!(
        requests.recv_timeout(Duration::from_secs(2)).unwrap(),
        SHOW_LAYOUT,
        "it keeps listening after the first request"
    );
}

#[test]
fn a_stale_socket_from_a_crashed_instance_is_replaced() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("downlink.sock");
    drop(UnixListener::bind(&path).unwrap()); // leaves the socket file behind
    let requests = listen_into(&path);
    instance_channel::send(&path, SHOW_LAYOUT).unwrap();
    assert_eq!(
        requests.recv_timeout(Duration::from_secs(2)).unwrap(),
        SHOW_LAYOUT
    );
}

#[test]
fn sending_without_a_running_instance_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    assert!(instance_channel::send(&dir.path().join("downlink.sock"), SHOW_LAYOUT).is_err());
}

#[test]
fn only_the_launcher_flag_asks_for_the_layout() {
    assert!(instance_channel::wants_show_layout(["--show-layout"]));
    assert!(!instance_channel::wants_show_layout(Vec::<&str>::new()));
    assert!(!instance_channel::wants_show_layout(["--other"]));
}
