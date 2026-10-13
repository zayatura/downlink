use std::sync::Arc;

use companion_core::{Config, Connection, Effect, Engine, Input, LayoutState};
use oryx_device::DeviceEvent;
use oryx_layout::{Key, Layer, Layout};
use oryx_protocol::{LayoutIds, Model};

fn ids() -> LayoutIds {
    LayoutIds::parse("abc12/rev99").unwrap()
}

fn config(layers: &[(u8, &str)], fallback: Option<&str>) -> Config {
    Config {
        layers: layers
            .iter()
            .map(|(l, id)| (*l, (*id).to_owned()))
            .collect(),
        fallback: fallback.map(str::to_owned),
        ..Config::default()
    }
}

fn voyager_config() -> Config {
    config(&[(0, "en"), (1, "ro"), (2, "hu")], Some("en"))
}

fn connected() -> Input {
    Input::Device(DeviceEvent::Connected {
        model: Model::Voyager,
        ids: Some(ids()),
        protocol: Some(5),
    })
}

fn layer(l: u8) -> Input {
    Input::Device(DeviceEvent::Layer(l))
}

fn disconnected() -> Input {
    Input::Device(DeviceEvent::Disconnected {
        reason: "unplugged".into(),
    })
}

fn selects(effects: &[Effect]) -> Vec<String> {
    effects
        .iter()
        .filter_map(|e| match e {
            Effect::SelectInputSource(id) => Some(id.clone()),
            _ => None,
        })
        .collect()
}

fn connected_engine(config: Config) -> Engine {
    let mut engine = Engine::new(config);
    engine.handle(connected());
    engine
}

fn test_layout() -> Arc<Layout> {
    Arc::new(Layout {
        title: "Test".into(),
        layout_id: "abc12".into(),
        revision_id: "rev99".into(),
        geometry: "voyager".into(),
        layers: ["Main", "RO", "HU", "Navig"]
            .iter()
            .enumerate()
            .map(|(i, title)| Layer {
                index: i as u8,
                title: (*title).into(),
                keys: vec![Key::default(); 52],
            })
            .collect(),
    })
}

#[test]
fn a_mapped_layer_selects_its_source() {
    let mut e = connected_engine(voyager_config());
    assert_eq!(selects(&e.handle(layer(0))), vec!["en"]);
    assert_eq!(e.active_layer(), Some(0));
}

#[test]
fn a_transient_layer_keeps_the_current_source() {
    let mut e = connected_engine(voyager_config());
    e.handle(layer(0));
    assert!(selects(&e.handle(layer(3))).is_empty());
    assert!(
        selects(&e.handle(layer(0))).is_empty(),
        "already selected en"
    );
}

#[test]
fn switching_between_mapped_layers() {
    let mut e = connected_engine(voyager_config());
    e.handle(layer(0));
    assert_eq!(selects(&e.handle(layer(1))), vec!["ro"]);
    assert_eq!(selects(&e.handle(layer(2))), vec!["hu"]);
    assert_eq!(selects(&e.handle(layer(0))), vec!["en"]);
}

#[test]
fn a_repeated_layer_event_does_nothing() {
    let mut e = connected_engine(voyager_config());
    e.handle(layer(1));
    assert!(e.handle(layer(1)).is_empty());
}

#[test]
fn flapping_between_transient_and_mapped_layers_switches_once() {
    let mut e = connected_engine(voyager_config());
    let mut all = selects(&e.handle(layer(0)));
    for _ in 0..10 {
        all.extend(selects(&e.handle(layer(3))));
        all.extend(selects(&e.handle(layer(0))));
    }
    assert_eq!(all, vec!["en"]);
}

#[test]
fn an_unmapped_high_layer_does_nothing() {
    let mut e = connected_engine(voyager_config());
    e.handle(layer(1));
    assert!(selects(&e.handle(layer(31))).is_empty());
    assert_eq!(e.active_layer(), Some(31));
}

#[test]
fn disconnect_selects_the_fallback_and_forgets_the_last_source() {
    let mut e = connected_engine(voyager_config());
    e.handle(layer(1));
    assert_eq!(selects(&e.handle(disconnected())), vec!["en"]);
    assert_eq!(e.connection(), &Connection::Searching);
    assert_eq!(e.active_layer(), None);
    e.handle(connected());
    assert_eq!(
        selects(&e.handle(layer(1))),
        vec!["ro"],
        "switches again after reconnect"
    );
}

#[test]
fn without_a_fallback_disconnect_selects_nothing() {
    let mut e = connected_engine(config(&[(0, "en")], None));
    e.handle(layer(0));
    assert!(selects(&e.handle(disconnected())).is_empty());
}

#[test]
fn repeated_open_failures_never_select() {
    let mut e = Engine::new(voyager_config());
    for _ in 0..3 {
        let fx = e.handle(Input::Device(DeviceEvent::OpenFailed {
            reason: "busy".into(),
        }));
        assert!(selects(&fx).is_empty());
    }
    assert_eq!(
        e.connection(),
        &Connection::OpenFailed {
            reason: "busy".into()
        }
    );
}

#[test]
fn becoming_unsupported_selects_the_fallback_once() {
    let mut e = Engine::new(voyager_config());
    let unsupported = || {
        Input::Device(DeviceEvent::Unsupported {
            reason: "no live data".into(),
        })
    };
    assert_eq!(selects(&e.handle(unsupported())), vec!["en"]);
    assert!(selects(&e.handle(unsupported())).is_empty());
}

#[test]
fn quit_selects_the_fallback_then_exits() {
    let mut e = connected_engine(voyager_config());
    e.handle(layer(1));
    assert_eq!(
        e.handle(Input::Quit),
        vec![Effect::SelectInputSource("en".into()), Effect::Exit]
    );
}

#[test]
fn pausing_blocks_layer_disconnect_and_quit_switching() {
    let mut e = connected_engine(voyager_config());
    e.handle(Input::SetPaused(true));
    assert!(e.paused());
    assert!(selects(&e.handle(layer(1))).is_empty());
    assert!(selects(&e.handle(disconnected())).is_empty());
    assert_eq!(e.handle(Input::Quit), vec![Effect::Exit]);
}

#[test]
fn resuming_waits_for_the_next_layer_change() {
    let mut e = connected_engine(voyager_config());
    e.handle(Input::SetPaused(true));
    e.handle(layer(1));
    assert!(selects(&e.handle(Input::SetPaused(false))).is_empty());
    assert_eq!(selects(&e.handle(layer(2))), vec!["hu"]);
}

#[test]
fn a_failed_select_is_retried_on_the_next_change() {
    let mut e = connected_engine(voyager_config());
    e.handle(layer(0));
    e.handle(Input::SelectFailed("en".into()));
    e.handle(layer(3));
    assert_eq!(selects(&e.handle(layer(0))), vec!["en"]);
}

#[test]
fn a_mapping_change_applies_to_the_current_layer() {
    let mut e = connected_engine(voyager_config());
    e.handle(layer(1));
    let mut new = voyager_config();
    new.layers.insert(1, "ro-2".into());
    let fx = e.handle(Input::ConfigChanged(new));
    assert!(fx.contains(&Effect::PersistConfig));
    assert_eq!(selects(&fx), vec!["ro-2"]);
}

#[test]
fn making_the_current_layer_transient_does_not_switch() {
    let mut e = connected_engine(voyager_config());
    e.handle(layer(1));
    let mut new = voyager_config();
    new.layers.remove(&1);
    assert!(selects(&e.handle(Input::ConfigChanged(new))).is_empty());
}

#[test]
fn connecting_requests_the_layout() {
    let mut e = Engine::new(voyager_config());
    let fx = e.handle(connected());
    assert!(fx.contains(&Effect::FetchLayout {
        model: Model::Voyager,
        ids: ids(),
        force: false
    }));
    assert_eq!(e.layout(), &LayoutState::Loading);
}

#[test]
fn reconnecting_with_the_same_revision_keeps_the_layout_without_refetching() {
    let mut e = connected_engine(voyager_config());
    e.handle(Input::LayoutLoaded {
        ids: ids(),
        result: Ok(test_layout()),
    });
    e.handle(disconnected());
    let fx = e.handle(connected());
    assert!(!fx.iter().any(|f| matches!(f, Effect::FetchLayout { .. })));
    assert!(matches!(e.layout(), LayoutState::Ready(_)));
}

#[test]
fn a_keyboard_without_layout_ids_still_switches() {
    let mut e = Engine::new(voyager_config());
    let fx = e.handle(Input::Device(DeviceEvent::Connected {
        model: Model::Voyager,
        ids: None,
        protocol: None,
    }));
    assert!(!fx.iter().any(|f| matches!(f, Effect::FetchLayout { .. })));
    assert!(matches!(e.layout(), LayoutState::Unavailable(_)));
    assert_eq!(selects(&e.handle(layer(1))), vec!["ro"]);
}

#[test]
fn layout_results_for_another_revision_are_ignored() {
    let mut e = connected_engine(voyager_config());
    let other = LayoutIds::parse("abc12/old11").unwrap();
    e.handle(Input::LayoutLoaded {
        ids: other,
        result: Ok(test_layout()),
    });
    assert_eq!(e.layout(), &LayoutState::Loading);
}

#[test]
fn a_failed_layout_load_is_unavailable() {
    let mut e = connected_engine(voyager_config());
    e.handle(Input::LayoutLoaded {
        ids: ids(),
        result: Err("offline".into()),
    });
    assert_eq!(e.layout(), &LayoutState::Unavailable("offline".into()));
}

#[test]
fn refresh_forces_a_fetch_when_connected() {
    let mut e = Engine::new(voyager_config());
    assert!(
        e.handle(Input::RefreshLayout).is_empty(),
        "nothing to refresh while disconnected"
    );
    e.handle(connected());
    e.handle(Input::LayoutLoaded {
        ids: ids(),
        result: Ok(test_layout()),
    });
    assert_eq!(
        e.handle(Input::RefreshLayout),
        vec![Effect::FetchLayout {
            model: Model::Voyager,
            ids: ids(),
            force: true
        }]
    );
    assert_eq!(e.layout(), &LayoutState::Loading);
}

#[test]
fn the_current_source_is_tracked_for_display() {
    let mut e = Engine::new(voyager_config());
    assert_eq!(
        e.handle(Input::CurrentSourceChanged(Some("ro".into()))),
        vec![Effect::RefreshTray]
    );
    assert_eq!(e.current_source(), Some("ro"));
    assert!(
        e.handle(Input::CurrentSourceChanged(Some("ro".into())))
            .is_empty()
    );
}
