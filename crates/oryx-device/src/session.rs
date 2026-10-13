use std::time::{Duration, Instant};

use oryx_protocol::{Command, Event, LayoutIds, Model};

use crate::DeviceEvent;

/// How long to wait for `PAIRING_SUCCESS` before calling the firmware unsupported.
pub const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(2);

/// After this long without any packet, pair again. The firmware drops its
/// paired state whenever a send fails, for example while the host sleeps.
pub const HEARTBEAT_IDLE: Duration = Duration::from_secs(5);

pub const UNSUPPORTED_REASON: &str = "Firmware doesn't report live data";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Handshaking,
    Connected,
    Unsupported,
}

/// Handshake, heartbeat and event filtering for one open interface. No I/O:
/// callers feed packets and clock ticks in and send the returned commands.
#[derive(Debug)]
pub struct Session {
    model: Model,
    phase: Phase,
    started: Instant,
    last_rx: Instant,
    last_ping: Instant,
    ids: Option<LayoutIds>,
    protocol: Option<u8>,
    pending_layer: Option<u8>,
}

impl Session {
    pub fn start(model: Model, serial: Option<&str>, now: Instant) -> (Session, Vec<Command>) {
        let session = Session {
            model,
            phase: Phase::Handshaking,
            started: now,
            last_rx: now,
            last_ping: now,
            ids: serial.and_then(LayoutIds::parse),
            protocol: None,
            pending_layer: None,
        };
        // Replies arrive in this order, so the IDs are known before pairing completes.
        (
            session,
            vec![
                Command::GetFwVersion,
                Command::GetProtocolVersion,
                Command::PairingInit,
            ],
        )
    }

    pub fn on_packet(&mut self, packet: &[u8], now: Instant) -> Vec<DeviceEvent> {
        self.last_rx = now;
        let event = match Event::decode(packet) {
            Ok(event) => event,
            Err(error) => {
                tracing::debug!(%error, "ignoring malformed packet");
                return Vec::new();
            }
        };
        match event {
            Event::FwVersion(version) => {
                match LayoutIds::parse(&version) {
                    Some(ids) => self.ids = Some(ids),
                    None => tracing::warn!(%version, "unrecognised firmware version string"),
                }
                Vec::new()
            }
            Event::ProtocolVersion(version) => {
                if !(4..=5).contains(&version) {
                    tracing::warn!(
                        version,
                        "untested Oryx protocol version; only core events are used"
                    );
                }
                self.protocol = Some(version);
                Vec::new()
            }
            Event::PairingSuccess => self.become_connected(),
            Event::Layer(layer) if self.phase == Phase::Connected => {
                vec![DeviceEvent::Layer(layer)]
            }
            Event::Layer(layer) => {
                self.pending_layer = Some(layer);
                Vec::new()
            }
            Event::KeyDown(pos) if self.phase == Phase::Connected => {
                vec![DeviceEvent::KeyDown(pos)]
            }
            Event::KeyUp(pos) if self.phase == Phase::Connected => vec![DeviceEvent::KeyUp(pos)],
            Event::Error(code) => {
                tracing::warn!(code, "keyboard reported an error");
                Vec::new()
            }
            // Key positions are never logged, at any level: a debug log would be a keystroke log.
            Event::KeyDown(_) | Event::KeyUp(_) => Vec::new(),
            other => {
                tracing::debug!(?other, "ignoring event");
                Vec::new()
            }
        }
    }

    pub fn on_tick(&mut self, now: Instant) -> (Vec<DeviceEvent>, Vec<Command>) {
        let mut events = Vec::new();
        let mut commands = Vec::new();
        if self.phase == Phase::Handshaking && now.duration_since(self.started) >= HANDSHAKE_TIMEOUT
        {
            self.phase = Phase::Unsupported;
            events.push(DeviceEvent::Unsupported {
                reason: UNSUPPORTED_REASON.into(),
            });
        }
        if now.duration_since(self.last_rx) >= HEARTBEAT_IDLE
            && now.duration_since(self.last_ping) >= HEARTBEAT_IDLE
        {
            self.last_ping = now;
            commands.push(Command::PairingInit);
        }
        (events, commands)
    }

    fn become_connected(&mut self) -> Vec<DeviceEvent> {
        if self.phase == Phase::Connected {
            return Vec::new(); // a heartbeat's reply
        }
        self.phase = Phase::Connected;
        let mut events = vec![DeviceEvent::Connected {
            model: self.model,
            ids: self.ids.clone(),
            protocol: self.protocol,
        }];
        if let Some(layer) = self.pending_layer.take() {
            events.push(DeviceEvent::Layer(layer));
        }
        events
    }
}
