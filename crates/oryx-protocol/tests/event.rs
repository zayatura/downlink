use oryx_protocol::{DecodeError, Event, MatrixPos, PACKET_LEN};

fn packet(prefix: &[u8]) -> Vec<u8> {
    let mut p = prefix.to_vec();
    p.resize(PACKET_LEN, 0);
    p
}

#[test]
fn firmware_version_is_the_text_before_the_stop_byte() {
    let mut raw = vec![0x00];
    raw.extend_from_slice(b"eVAVm/m57mrj");
    raw.push(0xFE);
    assert_eq!(
        Event::decode(&packet(&raw)),
        Ok(Event::FwVersion("eVAVm/m57mrj".into()))
    );
}

#[test]
fn firmware_version_without_stop_byte_ends_at_padding() {
    let mut raw = vec![0x00];
    raw.extend_from_slice(b"abc12/rev99");
    assert_eq!(
        Event::decode(&packet(&raw)),
        Ok(Event::FwVersion("abc12/rev99".into()))
    );
}

#[test]
fn bytes_after_the_stop_byte_are_ignored() {
    // Real firmware leaves uninitialised buffer bytes after the stop byte (Phase 0 captures).
    let mut raw = vec![0x00];
    raw.extend_from_slice(b"eVAVm/m57mrj");
    raw.extend_from_slice(&[0xFE, 0x00, 0x20, 0x41, 0x42]);
    assert_eq!(
        Event::decode(&packet(&raw)),
        Ok(Event::FwVersion("eVAVm/m57mrj".into()))
    );
    assert_eq!(
        Event::decode(&packet(&[0x05, 0x00, 0xFE, 0x00, 0x4f, 0xdb])),
        Ok(Event::Layer(0))
    );
}

#[test]
fn layer_and_key_events() {
    assert_eq!(
        Event::decode(&packet(&[0x05, 2, 0xFE])),
        Ok(Event::Layer(2))
    );
    assert_eq!(
        Event::decode(&packet(&[0x06, 3, 1, 0xFE])),
        Ok(Event::KeyDown(MatrixPos { row: 1, col: 3 }))
    );
    assert_eq!(
        Event::decode(&packet(&[0x07, 6, 11, 0xFE])),
        Ok(Event::KeyUp(MatrixPos { row: 11, col: 6 }))
    );
}

#[test]
fn pairing_protocol_and_error_events() {
    assert_eq!(
        Event::decode(&packet(&[0x04, 0xFE])),
        Ok(Event::PairingSuccess)
    );
    assert_eq!(
        Event::decode(&packet(&[0x03, 0xFE])),
        Ok(Event::PairingFailed)
    );
    assert_eq!(
        Event::decode(&packet(&[0xFE, 5, 0xFE])),
        Ok(Event::ProtocolVersion(5))
    );
    assert_eq!(
        Event::decode(&packet(&[0xFF, 0xFF])),
        Ok(Event::Error(0xFF))
    );
}

#[test]
fn unknown_codes_are_reported_not_rejected() {
    assert_eq!(Event::decode(&packet(&[0x42])), Ok(Event::Unknown(0x42)));
}

#[test]
fn empty_and_truncated_packets_are_errors() {
    assert_eq!(Event::decode(&[]), Err(DecodeError::Empty));
    assert_eq!(
        Event::decode(&[0x05]),
        Err(DecodeError::Truncated { code: 0x05 })
    );
    assert_eq!(
        Event::decode(&[0x06, 3]),
        Err(DecodeError::Truncated { code: 0x06 })
    );
}
