//! Oryx raw-HID protocol: 32-byte packets exchanged with a ZSA keyboard.
//!
//! Written from the protocol's observable behaviour (byte codes and packet
//! layout); no firmware code is copied.

mod command;
mod event;
mod ids;
mod model;

pub use command::Command;
pub use event::{DecodeError, Event, MatrixPos};
pub use ids::LayoutIds;
pub use model::{Model, RAW_HID_USAGE, RAW_HID_USAGE_PAGE, VOYAGER_PRODUCT_ID, ZSA_VENDOR_ID};

/// Size of every raw-HID report, in both directions.
pub const PACKET_LEN: usize = 32;

/// Marks the end of the meaningful bytes in a packet.
pub const STOP: u8 = 0xFE;
