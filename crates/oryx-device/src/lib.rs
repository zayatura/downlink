//! Talks to a ZSA keyboard's Oryx raw-HID interface: handshake, events, reconnect.

mod session;

pub use session::{HANDSHAKE_TIMEOUT, HEARTBEAT_IDLE, Session, UNSUPPORTED_REASON};

use oryx_protocol::{LayoutIds, MatrixPos, Model};

/// What the device layer reports to the rest of the app.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceEvent {
    /// Paired with a keyboard. `ids` is `None` when it didn't report a usable layout ID.
    Connected {
        model: Model,
        ids: Option<LayoutIds>,
        protocol: Option<u8>,
    },
    Layer(u8),
    KeyDown(MatrixPos),
    KeyUp(MatrixPos),
    Disconnected {
        reason: String,
    },
    /// The interface opened but the firmware never paired (no Oryx support).
    Unsupported {
        reason: String,
    },
    /// Found the keyboard but couldn't open it (permission denied, busy, …).
    OpenFailed {
        reason: String,
    },
}
