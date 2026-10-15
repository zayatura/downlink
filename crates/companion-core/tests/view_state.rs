use std::sync::Arc;

use companion_core::{Config, Engine, Input, TrayView};
use oryx_device::DeviceEvent;
use oryx_layout::{Action, Key, Layer, Layout, Modifiers};
use oryx_protocol::{LayoutIds, MatrixPos, Model};

fn ids() -> LayoutIds {
    LayoutIds::parse("abc12/rev99").unwrap()
}

fn connected_engine() -> Engine {
    let mut config = Config::default();
    config.layers.insert(0, "en".into());
    let mut e = Engine::new(config);
    e.handle(Input::Device(DeviceEvent::Connected {
        model: Model::Voyager,
        ids: Some(ids()),
        protocol: Some(5),
    }));
    e
}

fn with_layout(e: &mut Engine, titles: &[&str]) {
    load(e, test_layout(titles));
}

fn load(e: &mut Engine, layout: Layout) {
    e.handle(Input::LayoutLoaded {
        ids: ids(),
        result: Ok(Arc::new(layout)),
    });
}

fn test_layout(titles: &[&str]) -> Layout {
    Layout {
        title: "Test".into(),
        layout_id: "abc12".into(),
        revision_id: "rev99".into(),
        geometry: "voyager".into(),
        layers: titles
            .iter()
            .enumerate()
            .map(|(i, t)| Layer {
                index: i as u8,
                title: (*t).into(),
                keys: vec![Key::default(); 52],
            })
            .collect(),
    }
}

fn down(row: u8, col: u8) -> Input {
    Input::Device(DeviceEvent::KeyDown(MatrixPos { row, col }))
}

fn up(row: u8, col: u8) -> Input {
    Input::Device(DeviceEvent::KeyUp(MatrixPos { row, col }))
}

#[test]
fn key_down_and_up_track_oryx_indices() {
    let mut e = connected_engine();
    e.handle(down(0, 1));
    e.handle(down(11, 6));
    assert_eq!(e.pressed().iter().copied().collect::<Vec<_>>(), vec![0, 51]);
    e.handle(up(0, 1));
    assert_eq!(e.pressed().iter().copied().collect::<Vec<_>>(), vec![51]);
}

#[test]
fn positions_off_the_board_and_stray_key_ups_are_ignored() {
    let mut e = connected_engine();
    e.handle(down(4, 0));
    e.handle(down(13, 0));
    e.handle(up(2, 2));
    assert!(e.pressed().is_empty());
}

#[test]
fn a_held_key_stays_pressed_across_a_layer_change() {
    let mut e = connected_engine();
    e.handle(down(5, 0)); // left thumb, Oryx index 24
    e.handle(Input::Device(DeviceEvent::Layer(3)));
    assert!(e.pressed().contains(&24));
}

#[test]
fn disconnect_clears_pressed_keys() {
    let mut e = connected_engine();
    e.handle(down(0, 1));
    e.handle(Input::Device(DeviceEvent::Disconnected {
        reason: "gone".into(),
    }));
    assert!(e.pressed().is_empty());
}

#[test]
fn keys_before_connecting_are_ignored() {
    let mut e = Engine::new(Config::default());
    e.handle(down(0, 1));
    assert!(e.pressed().is_empty());
}

#[test]
fn preview_and_follow_live() {
    let mut e = connected_engine();
    e.handle(Input::Device(DeviceEvent::Layer(0)));
    assert!(e.is_following());
    assert_eq!(e.shown_layer(), Some(0));
    e.handle(Input::PreviewLayer(Some(2)));
    assert!(!e.is_following());
    assert_eq!(e.shown_layer(), Some(2));
    e.handle(Input::Device(DeviceEvent::Layer(1)));
    assert_eq!(
        e.shown_layer(),
        Some(2),
        "a pinned preview stays while the live layer changes"
    );
    e.handle(Input::PreviewLayer(Some(1)));
    assert!(
        e.is_following(),
        "clicking the live layer's chip follows again"
    );
    e.handle(Input::PreviewLayer(Some(3)));
    e.handle(Input::PreviewLayer(None));
    assert!(e.is_following());
}

#[test]
fn layer_titles_fall_back_to_numbers() {
    let mut e = connected_engine();
    assert_eq!(e.layer_title(0), "Layer 0", "no layout yet");
    with_layout(&mut e, &["Main", "  ", "HU"]);
    assert_eq!(e.layer_title(0), "Main");
    assert_eq!(e.layer_title(1), "Layer 1", "blank title");
    assert_eq!(e.layer_title(7), "Layer 7", "beyond the layout");
}

#[test]
fn tray_view_while_searching() {
    let e = Engine::new(Config::default());
    assert_eq!(
        e.tray_view(|_| None),
        TrayView {
            connected: false,
            status: "Waiting for Voyager…".into(),
            detail: String::new(),
            paused: false
        }
    );
}

#[test]
fn tray_view_when_connected_names_layer_and_source() {
    let mut e = connected_engine();
    with_layout(&mut e, &["Main", "RO"]);
    e.handle(Input::Device(DeviceEvent::Layer(0)));
    e.handle(Input::CurrentSourceChanged(Some("en".into())));
    e.handle(Input::SetPaused(true));
    let view = e.tray_view(|id| (id == "en").then(|| "English – Voyager".to_owned()));
    assert_eq!(
        view,
        TrayView {
            connected: true,
            status: "● Voyager connected".into(),
            detail: "Layer: Main · Input: English – Voyager".into(),
            paused: true,
        }
    );
}

#[test]
fn tray_view_falls_back_to_ids_and_dashes() {
    let mut e = connected_engine();
    assert_eq!(e.tray_view(|_| None).detail, "Layer: — · Input: —");
    e.handle(Input::CurrentSourceChanged(Some("com.example.x".into())));
    assert_eq!(
        e.tray_view(|_| None).detail,
        "Layer: — · Input: com.example.x"
    );
}

#[test]
fn tray_view_reports_failures() {
    let mut e = Engine::new(Config::default());
    e.handle(Input::Device(DeviceEvent::OpenFailed {
        reason: "busy".into(),
    }));
    assert_eq!(e.tray_view(|_| None).status, "Can't open keyboard: busy");
    e.handle(Input::Device(DeviceEvent::Unsupported {
        reason: "Firmware doesn't report live data".into(),
    }));
    assert_eq!(
        e.tray_view(|_| None).status,
        "Firmware doesn't report live data"
    );
}

fn layer(index: u8) -> Input {
    Input::Device(DeviceEvent::Layer(index))
}

/// Layers 0–7; the key at matrix (0, 1), Oryx index 0, moves Main → RO and RO → HU.
fn with_move_keys(e: &mut Engine) {
    let mut layout = test_layout(&[
        "Main", "RO", "HU", "Navig", "Num", "Sym R", "Sym L", "Mouse",
    ]);
    let move_to = |layer| Key {
        tap: Some(Action {
            code: "TO".into(),
            layer: Some(layer),
            modifiers: Modifiers::default(),
        }),
        ..Key::default()
    };
    layout.layers[0].keys[0] = move_to(1);
    layout.layers[1].keys[0] = move_to(2);
    load(e, layout);
}

#[test]
fn a_momentary_layer_falls_through_to_the_moved_to_layer() {
    let mut e = connected_engine();
    with_move_keys(&mut e);
    e.handle(layer(0));
    e.handle(down(0, 1));
    e.handle(layer(1));
    e.handle(up(0, 1));
    e.handle(layer(6));
    assert_eq!(e.layers_below(6), vec![1, 0]);
    e.handle(layer(1));
    assert_eq!(e.layers_below(1), vec![0]);
}

#[test]
fn moving_on_from_a_moved_to_layer_leaves_only_the_new_one() {
    let mut e = connected_engine();
    with_move_keys(&mut e);
    e.handle(down(0, 1));
    e.handle(layer(1));
    e.handle(up(0, 1));
    e.handle(down(0, 1)); // now RO's key: → HU
    e.handle(layer(2));
    e.handle(up(0, 1));
    e.handle(layer(6));
    assert_eq!(e.layers_below(6), vec![2, 0]);
}

#[test]
fn reconnecting_forgets_the_layer_stack() {
    let mut e = connected_engine();
    with_move_keys(&mut e);
    e.handle(down(0, 1));
    e.handle(layer(1));
    e.handle(Input::Device(DeviceEvent::Disconnected {
        reason: "unplugged".into(),
    }));
    e.handle(Input::Device(DeviceEvent::Connected {
        model: Model::Voyager,
        ids: Some(ids()),
        protocol: Some(5),
    }));
    e.handle(layer(6));
    assert_eq!(e.layers_below(6), vec![0]);
}
