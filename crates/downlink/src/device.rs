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
