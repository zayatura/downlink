//! Prints live keyboard events. Run with: cargo run -p oryx-device --example watch
fn main() {
    let (_commands, rx) = std::sync::mpsc::channel();
    println!("Watching for a ZSA Voyager… (Ctrl+C to stop)");
    oryx_device::run(
        oryx_device::HidDiscovery::new(),
        oryx_device::Timing::default(),
        rx,
        |event| {
            println!("{event:?}");
            true
        },
    );
}
