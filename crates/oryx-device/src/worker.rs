use std::sync::mpsc::{Receiver, TryRecvError};
use std::thread::sleep;
use std::time::{Duration, Instant};

use oryx_protocol::{Command, PACKET_LEN};

use crate::{DeviceEvent, Discovery, Found, Session, Transport};

/// Requests from the app to the device thread.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceCommand {
    Send(Command),
    Shutdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timing {
    pub poll_interval: Duration,
    pub read_timeout: Duration,
    pub backoff_start: Duration,
    pub backoff_max: Duration,
}

impl Default for Timing {
    fn default() -> Self {
        Timing {
            poll_interval: Duration::from_secs(1),
            read_timeout: Duration::from_millis(100),
            backoff_start: Duration::from_secs(1),
            backoff_max: Duration::from_secs(10),
        }
    }
}

enum Exit {
    Shutdown,
    Lost(String),
    ReceiverGone,
}

/// Finds, opens and drives the keyboard until shut down. Blocking: run it on
/// its own thread. Stops when `emit` returns `false`.
pub fn run<D: Discovery>(
    mut discovery: D,
    timing: Timing,
    commands: Receiver<DeviceCommand>,
    mut emit: impl FnMut(DeviceEvent) -> bool,
) {
    let mut backoff = timing.backoff_start;
    loop {
        if shutdown_requested(&commands) {
            return;
        }
        let found = match discovery.find() {
            Ok(Some(found)) => found,
            Ok(None) => {
                sleep(timing.poll_interval);
                continue;
            }
            Err(error) => {
                tracing::warn!(%error, "keyboard enumeration failed");
                sleep(timing.poll_interval);
                continue;
            }
        };
        let mut transport = match discovery.open(&found) {
            Ok(transport) => {
                backoff = timing.backoff_start;
                transport
            }
            Err(reason) => {
                tracing::warn!(%reason, "could not open the keyboard");
                if !emit(DeviceEvent::OpenFailed { reason }) {
                    return;
                }
                sleep(backoff);
                backoff = (backoff * 2).min(timing.backoff_max);
                continue;
            }
        };
        match drive(&mut transport, &found, &timing, &commands, &mut emit) {
            Exit::Shutdown | Exit::ReceiverGone => return,
            Exit::Lost(reason) => {
                if !emit(DeviceEvent::Disconnected { reason }) {
                    return;
                }
            }
        }
    }
}

fn shutdown_requested(commands: &Receiver<DeviceCommand>) -> bool {
    loop {
        match commands.try_recv() {
            Ok(DeviceCommand::Shutdown) | Err(TryRecvError::Disconnected) => return true,
            Ok(DeviceCommand::Send(command)) => {
                tracing::debug!(?command, "dropping command: no keyboard connected");
            }
            Err(TryRecvError::Empty) => return false,
        }
    }
}

fn drive<T: Transport>(
    transport: &mut T,
    found: &Found,
    timing: &Timing,
    commands: &Receiver<DeviceCommand>,
    emit: &mut impl FnMut(DeviceEvent) -> bool,
) -> Exit {
    let (mut session, hello) = Session::start(found.model, found.serial.as_deref(), Instant::now());
    for command in hello {
        if let Err(error) = transport.write(&command.encode()) {
            return Exit::Lost(error);
        }
    }
    let mut buf = [0u8; PACKET_LEN];
    loop {
        loop {
            match commands.try_recv() {
                Ok(DeviceCommand::Shutdown) | Err(TryRecvError::Disconnected) => {
                    return Exit::Shutdown;
                }
                Ok(DeviceCommand::Send(command)) => {
                    if let Err(error) = transport.write(&command.encode()) {
                        return Exit::Lost(error);
                    }
                }
                Err(TryRecvError::Empty) => break,
            }
        }
        match transport.read(&mut buf, timing.read_timeout) {
            Ok(0) => {}
            Ok(n) => {
                for event in session.on_packet(&buf[..n.min(PACKET_LEN)], Instant::now()) {
                    if !emit(event) {
                        return Exit::ReceiverGone;
                    }
                }
            }
            Err(error) => return Exit::Lost(error),
        }
        let (events, to_send) = session.on_tick(Instant::now());
        for event in events {
            if !emit(event) {
                return Exit::ReceiverGone;
            }
        }
        for command in to_send {
            if let Err(error) = transport.write(&command.encode()) {
                return Exit::Lost(error);
            }
        }
    }
}
