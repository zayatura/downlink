//! Runs the blocking device loop on its own thread and streams its events into iced.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};

use futures::channel::mpsc as async_mpsc;
use futures::{SinkExt, Stream, StreamExt};
use oryx_device::{DeviceCommand, DeviceEvent, HidDiscovery, Timing};

#[derive(Debug, Clone)]
pub struct DeviceHandle {
    commands: mpsc::Sender<DeviceCommand>,
    /// Set before the shutdown command, so the stream can tell Quit from a dead thread.
    stopping: Arc<AtomicBool>,
}

impl DeviceHandle {
    pub fn shutdown(&self) {
        self.stopping.store(true, Ordering::SeqCst);
        let _ = self.commands.send(DeviceCommand::Shutdown);
    }
}

#[derive(Debug, Clone)]
pub enum DeviceMessage {
    Ready(DeviceHandle),
    Event(DeviceEvent),
}

/// Whether the app may log this event. Key presses never are, at any level: a debug log would
/// otherwise be a keystroke log.
pub fn loggable(event: &DeviceEvent) -> bool {
    !matches!(event, DeviceEvent::KeyDown(_) | DeviceEvent::KeyUp(_))
}

/// What the app hears when the keyboard thread is gone: nothing during Quit, otherwise an
/// open failure, so the tray stops claiming a connection and the fallback source is selected.
pub fn thread_stopped_event(stopping: bool) -> Option<DeviceEvent> {
    (!stopping).then(|| DeviceEvent::OpenFailed {
        reason: "the keyboard thread stopped; restart Downlink".into(),
    })
}

pub fn events() -> impl Stream<Item = DeviceMessage> {
    iced::stream::channel(64, async |mut output: async_mpsc::Sender<DeviceMessage>| {
        let (command_tx, command_rx) = mpsc::channel();
        let (event_tx, mut event_rx) = async_mpsc::unbounded();
        let stopping = Arc::new(AtomicBool::new(false));
        let handle = DeviceHandle {
            commands: command_tx,
            stopping: stopping.clone(),
        };
        if output.send(DeviceMessage::Ready(handle)).await.is_err() {
            return;
        }
        let spawned = std::thread::Builder::new()
            .name("oryx-device".into())
            .spawn(move || {
                oryx_device::run(
                    HidDiscovery::new(),
                    Timing::default(),
                    command_rx,
                    |event| event_tx.unbounded_send(event).is_ok(),
                );
            });
        if let Err(error) = spawned {
            tracing::error!(%error, "could not start the keyboard thread");
            if let Some(event) = thread_stopped_event(stopping.load(Ordering::SeqCst)) {
                let _ = output.send(DeviceMessage::Event(event)).await;
            }
            return;
        }
        while let Some(event) = event_rx.next().await {
            if output.send(DeviceMessage::Event(event)).await.is_err() {
                return;
            }
        }
        // The thread dropped its sender: Quit asked it to stop, or it died (a panic).
        if let Some(event) = thread_stopped_event(stopping.load(Ordering::SeqCst)) {
            tracing::error!("the keyboard thread stopped unexpectedly");
            let _ = output.send(DeviceMessage::Event(event)).await;
        }
    })
}

#[cfg(test)]
mod tests {
    use oryx_device::DeviceEvent;
    use oryx_protocol::{MatrixPos, Model};

    use super::{loggable, thread_stopped_event};

    #[test]
    fn key_events_are_never_logged() {
        let pos = MatrixPos { row: 1, col: 2 };
        assert!(!loggable(&DeviceEvent::KeyDown(pos)));
        assert!(!loggable(&DeviceEvent::KeyUp(pos)));
    }

    #[test]
    fn quitting_stops_the_thread_silently() {
        assert_eq!(thread_stopped_event(true), None);
    }

    #[test]
    fn an_unexpected_stop_reports_an_open_failure() {
        let event = thread_stopped_event(false).expect("an unexpected stop is reported");
        let DeviceEvent::OpenFailed { reason } = &event else {
            panic!("expected an open failure, got {event:?}");
        };
        assert!(reason.contains("restart"), "{reason}");
        assert!(loggable(&event));
    }

    #[test]
    fn connection_and_layer_events_are_logged() {
        let events = [
            DeviceEvent::Connected {
                model: Model::Voyager,
                ids: None,
                protocol: None,
            },
            DeviceEvent::Layer(1),
            DeviceEvent::Disconnected {
                reason: "unplugged".into(),
            },
            DeviceEvent::Unsupported {
                reason: "no pairing".into(),
            },
            DeviceEvent::OpenFailed {
                reason: "busy".into(),
            },
        ];
        for event in events {
            assert!(loggable(&event), "{event:?}");
        }
    }
}
