//! Runs the blocking device loop on its own thread and streams its events into iced.

use std::sync::mpsc;

use futures::channel::mpsc as async_mpsc;
use futures::{SinkExt, Stream, StreamExt};
use oryx_device::{DeviceCommand, DeviceEvent, HidDiscovery, Timing};

#[derive(Debug, Clone)]
pub struct DeviceHandle(mpsc::Sender<DeviceCommand>);

impl DeviceHandle {
    pub fn shutdown(&self) {
        let _ = self.0.send(DeviceCommand::Shutdown);
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

pub fn events() -> impl Stream<Item = DeviceMessage> {
    iced::stream::channel(64, async |mut output: async_mpsc::Sender<DeviceMessage>| {
        let (command_tx, command_rx) = mpsc::channel();
        let (event_tx, mut event_rx) = async_mpsc::unbounded();
        if output
            .send(DeviceMessage::Ready(DeviceHandle(command_tx)))
            .await
            .is_err()
        {
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
            return;
        }
        while let Some(event) = event_rx.next().await {
            if output.send(DeviceMessage::Event(event)).await.is_err() {
                break;
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use oryx_device::DeviceEvent;
    use oryx_protocol::{MatrixPos, Model};

    use super::loggable;

    #[test]
    fn key_events_are_never_logged() {
        let pos = MatrixPos { row: 1, col: 2 };
        assert!(!loggable(&DeviceEvent::KeyDown(pos)));
        assert!(!loggable(&DeviceEvent::KeyUp(pos)));
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
