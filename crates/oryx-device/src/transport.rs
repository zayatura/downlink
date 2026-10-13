use std::time::Duration;

use oryx_protocol::{Model, PACKET_LEN};

/// A keyboard interface that was found but not opened yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    pub model: Model,
    pub serial: Option<String>,
    /// Platform path used to open the interface.
    pub path: String,
}

/// An open raw-HID interface.
pub trait Transport {
    /// Reads one report into `buf`, waiting up to `timeout`. `Ok(0)` means nothing arrived.
    fn read(&mut self, buf: &mut [u8; PACKET_LEN], timeout: Duration) -> Result<usize, String>;
    fn write(&mut self, packet: &[u8; PACKET_LEN]) -> Result<(), String>;
}

/// Finds and opens supported keyboards.
pub trait Discovery {
    type Transport: Transport;
    fn find(&mut self) -> Result<Option<Found>, String>;
    fn open(&mut self, found: &Found) -> Result<Self::Transport, String>;
}
