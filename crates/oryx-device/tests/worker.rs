use std::collections::VecDeque;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use oryx_device::{DeviceCommand, DeviceEvent, Discovery, Found, Timing, Transport, run};
use oryx_protocol::{Command, LayoutIds, Model, PACKET_LEN};

enum Read {
    Packet(Vec<u8>),
    Fail(&'static str),
}

enum Step {
    Absent,
    Found(Vec<Read>),
    OpenFails(&'static str),
}

type Writes = Arc<Mutex<Vec<Vec<u8>>>>;

struct FakeDiscovery {
    steps: VecDeque<Step>,
    pending: Option<Result<Vec<Read>, &'static str>>,
    writes: Writes,
}

impl FakeDiscovery {
    fn new(steps: Vec<Step>) -> (FakeDiscovery, Writes) {
        let writes = Writes::default();
        (
            FakeDiscovery {
                steps: steps.into(),
                pending: None,
                writes: writes.clone(),
            },
            writes,
        )
    }
}

fn found() -> Found {
    Found {
        model: Model::Voyager,
        serial: Some("abc12/rev99".into()),
        path: "fake".into(),
    }
}

impl Discovery for FakeDiscovery {
    type Transport = FakeTransport;

    fn find(&mut self) -> Result<Option<Found>, String> {
        match self.steps.pop_front() {
            None | Some(Step::Absent) => Ok(None),
            Some(Step::Found(reads)) => {
                self.pending = Some(Ok(reads));
                Ok(Some(found()))
            }
            Some(Step::OpenFails(reason)) => {
                self.pending = Some(Err(reason));
                Ok(Some(found()))
            }
        }
    }

    fn open(&mut self, _found: &Found) -> Result<FakeTransport, String> {
        match self.pending.take() {
            Some(Ok(reads)) => Ok(FakeTransport {
                reads: reads.into(),
                writes: self.writes.clone(),
            }),
            Some(Err(reason)) => Err(reason.to_owned()),
            None => Err("nothing to open".to_owned()),
        }
    }
}

struct FakeTransport {
    reads: VecDeque<Read>,
    writes: Writes,
}

impl Transport for FakeTransport {
    fn read(&mut self, buf: &mut [u8; PACKET_LEN], timeout: Duration) -> Result<usize, String> {
        match self.reads.pop_front() {
            Some(Read::Packet(p)) => {
                buf.fill(0);
                buf[..p.len()].copy_from_slice(&p);
                Ok(PACKET_LEN)
            }
            Some(Read::Fail(reason)) => Err(reason.to_owned()),
            None => {
                std::thread::sleep(timeout);
                Ok(0)
            }
        }
    }

    fn write(&mut self, packet: &[u8; PACKET_LEN]) -> Result<(), String> {
        self.writes.lock().unwrap().push(packet.to_vec());
        Ok(())
    }
}

fn fast() -> Timing {
    Timing {
        poll_interval: Duration::from_millis(1),
        read_timeout: Duration::from_millis(1),
        backoff_start: Duration::from_millis(1),
        backoff_max: Duration::from_millis(4),
    }
}

fn paired() -> Read {
    Read::Packet(vec![0x04, 0xFE])
}

fn layer(l: u8) -> Read {
    Read::Packet(vec![0x05, l, 0xFE])
}

fn connected() -> DeviceEvent {
    DeviceEvent::Connected {
        model: Model::Voyager,
        ids: LayoutIds::parse("abc12/rev99"),
        protocol: None,
    }
}

#[test]
fn reconnects_after_the_keyboard_goes_away() {
    let (discovery, writes) = FakeDiscovery::new(vec![
        Step::Found(vec![paired(), layer(1), Read::Fail("gone")]),
        Step::Absent,
        Step::Found(vec![paired(), layer(2)]),
    ]);
    let (_tx, rx) = mpsc::channel();
    let mut events = Vec::new();
    run(discovery, fast(), rx, |event| {
        let done = event == DeviceEvent::Layer(2);
        events.push(event);
        !done
    });
    assert_eq!(
        events,
        vec![
            connected(),
            DeviceEvent::Layer(1),
            DeviceEvent::Disconnected {
                reason: "gone".into()
            },
            connected(),
            DeviceEvent::Layer(2),
        ]
    );
    let codes: Vec<u8> = writes
        .lock()
        .unwrap()
        .iter()
        .take(3)
        .map(|p| p[0])
        .collect();
    assert_eq!(codes, vec![0x00, 0xFE, 0x01], "handshake order");
}

#[test]
fn open_failures_are_reported_and_retried() {
    let (discovery, _) = FakeDiscovery::new(vec![
        Step::OpenFails("busy"),
        Step::OpenFails("busy"),
        Step::Found(vec![paired()]),
    ]);
    let (_tx, rx) = mpsc::channel();
    let mut events = Vec::new();
    run(discovery, fast(), rx, |event| {
        let done = matches!(event, DeviceEvent::Connected { .. });
        events.push(event);
        !done
    });
    assert_eq!(
        events,
        vec![
            DeviceEvent::OpenFailed {
                reason: "busy".into()
            },
            DeviceEvent::OpenFailed {
                reason: "busy".into()
            },
            connected(),
        ]
    );
}

#[test]
fn shutdown_before_start_returns_immediately() {
    let (discovery, _) = FakeDiscovery::new(vec![Step::Found(vec![paired()])]);
    let (tx, rx) = mpsc::channel();
    tx.send(DeviceCommand::Shutdown).unwrap();
    let mut events = Vec::new();
    run(discovery, fast(), rx, |event| {
        events.push(event);
        true
    });
    assert!(events.is_empty());
}

#[test]
fn shutdown_while_connected_returns() {
    let (discovery, _) = FakeDiscovery::new(vec![Step::Found(vec![paired()])]);
    let (tx, rx) = mpsc::channel();
    let mut events = Vec::new();
    run(discovery, fast(), rx, |event| {
        if matches!(event, DeviceEvent::Connected { .. }) {
            tx.send(DeviceCommand::Shutdown).unwrap();
        }
        events.push(event);
        true
    });
    assert_eq!(events, vec![connected()]);
}

#[test]
fn dropping_the_command_sender_stops_the_loop() {
    let (discovery, _) = FakeDiscovery::new(vec![Step::Absent, Step::Absent]);
    let (tx, rx) = mpsc::channel::<DeviceCommand>();
    drop(tx);
    run(discovery, fast(), rx, |_| true);
}

#[test]
fn host_commands_are_written_to_the_keyboard() {
    let (discovery, writes) = FakeDiscovery::new(vec![Step::Found(vec![paired()])]);
    let (tx, rx) = mpsc::channel();
    run(discovery, fast(), rx, |event| {
        if matches!(event, DeviceEvent::Connected { .. }) {
            tx.send(DeviceCommand::Send(Command::SetLayer {
                on: true,
                layer: 3,
            }))
            .unwrap();
            tx.send(DeviceCommand::Shutdown).unwrap();
        }
        true
    });
    let written = writes.lock().unwrap();
    assert!(
        written.iter().any(|p| p[..4] == [0x04, 1, 3, 0xFE]),
        "SetLayer was written"
    );
}
