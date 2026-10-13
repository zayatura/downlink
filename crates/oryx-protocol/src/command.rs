use crate::{PACKET_LEN, STOP};

/// A host → keyboard request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    GetFwVersion,
    PairingInit,
    GetProtocolVersion,
    /// `on = false` turns `layer` off; `on = true` moves to `layer`.
    SetLayer {
        on: bool,
        layer: u8,
    },
}

impl Command {
    pub fn code(self) -> u8 {
        match self {
            Command::GetFwVersion => 0x00,
            Command::PairingInit => 0x01,
            Command::SetLayer { .. } => 0x04,
            Command::GetProtocolVersion => 0xFE,
        }
    }

    pub fn encode(self) -> [u8; PACKET_LEN] {
        let mut packet = [0u8; PACKET_LEN];
        packet[0] = self.code();
        let mut len = 1;
        if let Command::SetLayer { on, layer } = self {
            packet[1] = u8::from(on);
            packet[2] = layer;
            len = 3;
        }
        packet[len] = STOP;
        packet
    }
}
