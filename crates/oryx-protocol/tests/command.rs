use oryx_protocol::{Command, PACKET_LEN, STOP};

fn packet(prefix: &[u8]) -> [u8; PACKET_LEN] {
    let mut p = [0u8; PACKET_LEN];
    p[..prefix.len()].copy_from_slice(prefix);
    p
}

#[test]
fn simple_commands_are_code_then_stop_byte() {
    assert_eq!(Command::GetFwVersion.encode(), packet(&[0x00, STOP]));
    assert_eq!(Command::PairingInit.encode(), packet(&[0x01, STOP]));
    assert_eq!(Command::GetProtocolVersion.encode(), packet(&[0xFE, STOP]));
}

#[test]
fn set_layer_carries_on_flag_and_layer() {
    assert_eq!(
        Command::SetLayer { on: true, layer: 2 }.encode(),
        packet(&[0x04, 1, 2, STOP])
    );
    assert_eq!(
        Command::SetLayer {
            on: false,
            layer: 7
        }
        .encode(),
        packet(&[0x04, 0, 7, STOP])
    );
}

#[test]
fn codes_match_the_firmware() {
    assert_eq!(Command::GetFwVersion.code(), 0x00);
    assert_eq!(Command::PairingInit.code(), 0x01);
    assert_eq!(Command::SetLayer { on: true, layer: 0 }.code(), 0x04);
    assert_eq!(Command::GetProtocolVersion.code(), 0xFE);
}
