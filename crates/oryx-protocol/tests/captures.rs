use oryx_protocol::Event;

#[test]
fn every_phase0_capture_decodes_to_a_known_event() {
    let text = include_str!("fixtures/phase0-captures.txt");
    let mut count = 0;
    for line in text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
    {
        let bytes: Vec<u8> = line
            .split_whitespace()
            .map(|h| u8::from_str_radix(h, 16).expect("hex byte"))
            .collect();
        let event = Event::decode(&bytes).unwrap_or_else(|e| panic!("{line}: {e}"));
        assert!(
            !matches!(event, Event::Unknown(_)),
            "{line} decoded to {event:?}"
        );
        count += 1;
    }
    assert!(
        count >= 10,
        "expected at least 10 captured packets, found {count}"
    );
}
