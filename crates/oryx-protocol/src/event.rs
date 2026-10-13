use crate::STOP;

/// A key's position in the firmware matrix, as reported by KEYDOWN/KEYUP.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MatrixPos {
    pub row: u8,
    pub col: u8,
}

/// A keyboard → host message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    FwVersion(String),
    PairingSuccess,
    PairingFailed,
    Layer(u8),
    KeyDown(MatrixPos),
    KeyUp(MatrixPos),
    ProtocolVersion(u8),
    Error(u8),
    Unknown(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DecodeError {
    #[error("empty packet")]
    Empty,
    #[error("event 0x{code:02x} is truncated")]
    Truncated { code: u8 },
}

impl Event {
    pub fn decode(packet: &[u8]) -> Result<Event, DecodeError> {
        let (&code, params) = packet.split_first().ok_or(DecodeError::Empty)?;
        let param = |i: usize| {
            params
                .get(i)
                .copied()
                .ok_or(DecodeError::Truncated { code })
        };
        Ok(match code {
            0x00 => Event::FwVersion(text_until_stop(params)),
            0x03 => Event::PairingFailed,
            0x04 => Event::PairingSuccess,
            0x05 => Event::Layer(param(0)?),
            0x06 => Event::KeyDown(MatrixPos {
                col: param(0)?,
                row: param(1)?,
            }),
            0x07 => Event::KeyUp(MatrixPos {
                col: param(0)?,
                row: param(1)?,
            }),
            0xFE => Event::ProtocolVersion(param(0)?),
            0xFF => Event::Error(param(0)?),
            other => Event::Unknown(other),
        })
    }
}

fn text_until_stop(bytes: &[u8]) -> String {
    let end = bytes
        .iter()
        .position(|&b| b == STOP || b == 0)
        .unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}
