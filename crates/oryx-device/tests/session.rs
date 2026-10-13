use std::time::{Duration, Instant};

use oryx_device::{DeviceEvent, HANDSHAKE_TIMEOUT, HEARTBEAT_IDLE, Session, UNSUPPORTED_REASON};
use oryx_protocol::{Command, LayoutIds, MatrixPos, Model};

fn pkt(bytes: &[u8]) -> Vec<u8> {
    let mut p = bytes.to_vec();
    p.resize(32, 0);
    p
}

fn fw(version: &str) -> Vec<u8> {
    let mut p = vec![0x00];
    p.extend_from_slice(version.as_bytes());
    p.push(0xFE);
    pkt(&p)
}

fn paired() -> Vec<u8> {
    pkt(&[0x04, 0xFE])
}

fn layer(l: u8) -> Vec<u8> {
    pkt(&[0x05, l, 0xFE])
}

fn protocol(v: u8) -> Vec<u8> {
    pkt(&[0xFE, v, 0xFE])
}

fn connected(ids: &str, protocol: Option<u8>) -> DeviceEvent {
    DeviceEvent::Connected {
        model: Model::Voyager,
        ids: LayoutIds::parse(ids),
        protocol,
    }
}

#[test]
fn start_asks_for_ids_and_protocol_before_pairing() {
    let (_session, commands) = Session::start(Model::Voyager, None, Instant::now());
    assert_eq!(
        commands,
        vec![
            Command::GetFwVersion,
            Command::GetProtocolVersion,
            Command::PairingInit
        ]
    );
}

#[test]
fn connected_after_pairing_carries_ids_and_protocol() {
    let t0 = Instant::now();
    let (mut s, _) = Session::start(Model::Voyager, None, t0);
    assert!(s.on_packet(&fw("eVAVm/m57mrj"), t0).is_empty());
    assert!(s.on_packet(&protocol(5), t0).is_empty());
    assert_eq!(
        s.on_packet(&paired(), t0),
        vec![connected("eVAVm/m57mrj", Some(5))]
    );
    assert_eq!(s.on_packet(&layer(2), t0), vec![DeviceEvent::Layer(2)]);
}

#[test]
fn serial_number_is_used_when_the_firmware_version_never_arrives() {
    let t0 = Instant::now();
    let (mut s, _) = Session::start(Model::Voyager, Some("abc12/rev99"), t0);
    assert_eq!(
        s.on_packet(&paired(), t0),
        vec![connected("abc12/rev99", None)]
    );
}

#[test]
fn firmware_version_overrides_the_serial_number() {
    let t0 = Instant::now();
    let (mut s, _) = Session::start(Model::Voyager, Some("old11/old22"), t0);
    s.on_packet(&fw("new11/new22"), t0);
    assert_eq!(
        s.on_packet(&paired(), t0),
        vec![connected("new11/new22", None)]
    );
}

#[test]
fn garbage_ids_are_dropped_not_trusted() {
    let t0 = Instant::now();
    let (mut s, _) = Session::start(Model::Voyager, Some("not a serial"), t0);
    s.on_packet(&fw("../../etc/passwd"), t0);
    assert_eq!(
        s.on_packet(&paired(), t0),
        vec![DeviceEvent::Connected {
            model: Model::Voyager,
            ids: None,
            protocol: None
        }]
    );
}

#[test]
fn a_layer_seen_before_pairing_is_replayed_after_connected() {
    let t0 = Instant::now();
    let (mut s, _) = Session::start(Model::Voyager, Some("abc12/rev99"), t0);
    assert!(s.on_packet(&layer(1), t0).is_empty());
    assert_eq!(
        s.on_packet(&paired(), t0),
        vec![connected("abc12/rev99", None), DeviceEvent::Layer(1)]
    );
}

#[test]
fn key_events_are_forwarded_only_once_connected() {
    let t0 = Instant::now();
    let (mut s, _) = Session::start(Model::Voyager, None, t0);
    assert!(s.on_packet(&pkt(&[0x06, 3, 1, 0xFE]), t0).is_empty());
    s.on_packet(&paired(), t0);
    assert_eq!(
        s.on_packet(&pkt(&[0x06, 3, 1, 0xFE]), t0),
        vec![DeviceEvent::KeyDown(MatrixPos { row: 1, col: 3 })]
    );
    assert_eq!(
        s.on_packet(&pkt(&[0x07, 3, 1, 0xFE]), t0),
        vec![DeviceEvent::KeyUp(MatrixPos { row: 1, col: 3 })]
    );
}

#[test]
fn repeated_pairing_replies_do_not_reconnect() {
    let t0 = Instant::now();
    let (mut s, _) = Session::start(Model::Voyager, None, t0);
    s.on_packet(&paired(), t0);
    assert!(s.on_packet(&paired(), t0).is_empty());
}

#[test]
fn malformed_and_unknown_packets_are_ignored() {
    let t0 = Instant::now();
    let (mut s, _) = Session::start(Model::Voyager, None, t0);
    s.on_packet(&paired(), t0);
    assert!(s.on_packet(&[], t0).is_empty());
    assert!(s.on_packet(&[0x05], t0).is_empty());
    assert!(s.on_packet(&pkt(&[0x42, 1, 2]), t0).is_empty());
    assert!(s.on_packet(&pkt(&[0xFF, 0x01]), t0).is_empty());
}

#[test]
fn unsupported_once_the_handshake_times_out() {
    let t0 = Instant::now();
    let (mut s, _) = Session::start(Model::Voyager, None, t0);
    let (events, _) = s.on_tick(t0 + HANDSHAKE_TIMEOUT - Duration::from_millis(1));
    assert!(events.is_empty());
    let (events, _) = s.on_tick(t0 + HANDSHAKE_TIMEOUT);
    assert_eq!(
        events,
        vec![DeviceEvent::Unsupported {
            reason: UNSUPPORTED_REASON.into()
        }]
    );
    let (events, _) = s.on_tick(t0 + HANDSHAKE_TIMEOUT + Duration::from_millis(500));
    assert!(events.is_empty(), "reported once only");
}

#[test]
fn an_unsupported_keyboard_recovers_when_pairing_arrives_later() {
    let t0 = Instant::now();
    let (mut s, _) = Session::start(Model::Voyager, Some("abc12/rev99"), t0);
    s.on_tick(t0 + HANDSHAKE_TIMEOUT);
    let later = t0 + Duration::from_secs(6);
    assert_eq!(
        s.on_packet(&paired(), later),
        vec![connected("abc12/rev99", None)]
    );
}

#[test]
fn heartbeat_repairs_after_idle_and_not_more_often() {
    let t0 = Instant::now();
    let (mut s, _) = Session::start(Model::Voyager, None, t0);
    s.on_packet(&paired(), t0);
    let (_, commands) = s.on_tick(t0 + HEARTBEAT_IDLE - Duration::from_millis(1));
    assert!(commands.is_empty());
    let (_, commands) = s.on_tick(t0 + HEARTBEAT_IDLE);
    assert_eq!(commands, vec![Command::PairingInit]);
    let (_, commands) = s.on_tick(t0 + HEARTBEAT_IDLE + Duration::from_secs(1));
    assert!(commands.is_empty(), "a ping was just sent");
    let (_, commands) = s.on_tick(t0 + HEARTBEAT_IDLE * 2);
    assert_eq!(commands, vec![Command::PairingInit]);
}

#[test]
fn traffic_postpones_the_heartbeat() {
    let t0 = Instant::now();
    let (mut s, _) = Session::start(Model::Voyager, None, t0);
    s.on_packet(&paired(), t0);
    s.on_packet(&layer(0), t0 + Duration::from_secs(4));
    let (_, commands) = s.on_tick(t0 + Duration::from_secs(6));
    assert!(commands.is_empty());
    let (_, commands) = s.on_tick(t0 + Duration::from_secs(9));
    assert_eq!(commands, vec![Command::PairingInit]);
}
