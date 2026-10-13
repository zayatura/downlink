use std::sync::mpsc::{Receiver, RecvTimeoutError, TryRecvError};
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
                if pause(&commands, timing.poll_interval) {
                    return;
                }
                continue;
            }
            Err(error) => {
                tracing::warn!(%error, "keyboard enumeration failed");
                if pause(&commands, timing.poll_interval) {
                    return;
                }
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
                if pause(&commands, backoff) {
                    return;
                }
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
                // The interface can keep enumerating while it fails, so pace the
                // next attempt to the enumeration interval (spec §8).
                if pause(&commands, timing.poll_interval) {
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

/// Waits up to `duration` while still watching `commands`, so a shutdown is not
/// held up by a sleep. Returns `true` when the loop must stop: on `Shutdown`, or
/// when the sender has been dropped. Host commands are dropped, since no
/// keyboard is connected while pausing.
fn pause(commands: &Receiver<DeviceCommand>, duration: Duration) -> bool {
    let deadline = Instant::now() + duration;
    loop {
        match commands.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
            Ok(DeviceCommand::Shutdown) | Err(RecvTimeoutError::Disconnected) => return true,
            Ok(DeviceCommand::Send(command)) => {
                tracing::debug!(?command, "dropping command: no keyboard connected");
            }
            Err(RecvTimeoutError::Timeout) => return false,
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
