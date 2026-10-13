//! Key positions must never reach the log, at any level: a debug log would be a keystroke log.

use std::io;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use oryx_device::Session;
use oryx_protocol::Model;
use tracing_subscriber::fmt::MakeWriter;

#[derive(Clone, Default)]
struct Captured(Arc<Mutex<Vec<u8>>>);

impl io::Write for Captured {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for Captured {
    type Writer = Captured;
    fn make_writer(&'a self) -> Captured {
        self.clone()
    }
}

fn pkt(bytes: &[u8]) -> Vec<u8> {
    let mut p = bytes.to_vec();
    p.resize(32, 0);
    p
}

#[test]
fn key_events_are_not_logged_before_or_after_pairing() {
    let log = Captured::default();
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .with_ansi(false)
        .with_writer(log.clone())
        .finish();
    tracing::subscriber::with_default(subscriber, || {
        let t0 = Instant::now();
        let (mut session, _) = Session::start(Model::Voyager, None, t0);
        // Still handshaking: the session ignores these.
        session.on_packet(&pkt(&[0x06, 3, 1, 0xFE]), t0);
        session.on_packet(&pkt(&[0x07, 3, 1, 0xFE]), t0);
        session.on_packet(&pkt(&[0x04, 0xFE]), t0);
        // Connected: the session forwards these as events, and still must not log them.
        session.on_packet(&pkt(&[0x06, 5, 2, 0xFE]), t0);
        session.on_packet(&pkt(&[0x07, 5, 2, 0xFE]), t0);
    });
    let text = String::from_utf8(log.0.lock().unwrap().clone()).unwrap();
    assert!(
        !text.contains("KeyDown") && !text.contains("KeyUp") && !text.contains("MatrixPos"),
        "{text}"
    );
}
