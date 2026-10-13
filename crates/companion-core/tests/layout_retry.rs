use std::sync::Arc;
use std::time::Duration;

use companion_core::{Config, Effect, Engine, Input, LayoutState};
use oryx_device::DeviceEvent;
use oryx_layout::{Key, Layer, Layout};
use oryx_protocol::{LayoutIds, Model};

fn ids() -> LayoutIds {
    LayoutIds::parse("abc12/rev99").unwrap()
}

fn connected() -> Input {
    Input::Device(DeviceEvent::Connected {
        model: Model::Voyager,
        ids: Some(ids()),
        protocol: Some(5),
    })
}

fn disconnected() -> Input {
    Input::Device(DeviceEvent::Disconnected {
        reason: "unplugged".into(),
    })
}

fn connected_engine() -> Engine {
    let mut engine = Engine::new(Config::default());
    engine.handle(connected());
    engine
}

fn test_layout() -> Arc<Layout> {
    Arc::new(Layout {
        title: "Test".into(),
        layout_id: "abc12".into(),
        revision_id: "rev99".into(),
        geometry: "voyager".into(),
        layers: vec![Layer {
            index: 0,
            title: "Main".into(),
            keys: vec![Key::default(); 52],
        }],
    })
}

fn fail(engine: &mut Engine) {
    engine.handle(Input::LayoutLoaded {
        ids: ids(),
        result: Err("offline".into()),
    });
}

fn secs(n: u64) -> Option<Duration> {
    Some(Duration::from_secs(n))
}

#[test]
fn a_failed_fetch_schedules_a_retry_that_fetches_again() {
    let mut e = connected_engine();
    fail(&mut e);
    assert_eq!(e.layout(), &LayoutState::Unavailable("offline".into()));
    assert_eq!(e.layout_retry_interval(), secs(30));

    let fx = e.handle(Input::RetryLayout);
    assert_eq!(
        fx,
        vec![Effect::FetchLayout {
            model: Model::Voyager,
            ids: ids(),
            force: false
        }]
    );
    assert_eq!(e.layout(), &LayoutState::Loading);
    assert_eq!(e.layout_retry_interval(), None);
}

#[test]
fn retries_back_off_to_ten_minutes() {
    let mut e = connected_engine();
    let mut seen = Vec::new();
    for _ in 0..7 {
        fail(&mut e);
        seen.push(e.layout_retry_interval());
        e.handle(Input::RetryLayout);
    }
    assert_eq!(
        seen,
        [30, 60, 120, 240, 480, 600, 600].map(|n| Some(Duration::from_secs(n)))
    );
}

#[test]
fn a_successful_load_resets_the_backoff() {
    let mut e = connected_engine();
    fail(&mut e);
    e.handle(Input::RetryLayout);
    fail(&mut e);
    assert_eq!(e.layout_retry_interval(), secs(60));
    e.handle(Input::RetryLayout);
    e.handle(Input::LayoutLoaded {
        ids: ids(),
        result: Ok(test_layout()),
    });
    assert_eq!(e.layout_retry_interval(), None);
    assert!(matches!(e.layout(), LayoutState::Ready(_)));

    e.handle(Input::RefreshLayout);
    fail(&mut e);
    assert_eq!(e.layout_retry_interval(), secs(30));
}

#[test]
fn a_manual_refresh_takes_over_from_the_scheduled_retry_but_keeps_the_backoff() {
    let mut e = connected_engine();
    fail(&mut e);
    assert_eq!(e.layout_retry_interval(), secs(30));
    let fx = e.handle(Input::RefreshLayout);
    assert_eq!(fx.len(), 1);
    assert_eq!(e.layout_retry_interval(), None);
    fail(&mut e);
    assert_eq!(e.layout_retry_interval(), secs(60));
}

#[test]
fn a_retry_without_a_pending_failure_does_nothing() {
    let mut e = Engine::new(Config::default());
    assert!(e.handle(Input::RetryLayout).is_empty(), "disconnected");
    assert_eq!(e.layout(), &LayoutState::None);

    e.handle(connected());
    assert!(e.handle(Input::RetryLayout).is_empty(), "loading");
    assert_eq!(e.layout(), &LayoutState::Loading);

    e.handle(Input::LayoutLoaded {
        ids: ids(),
        result: Ok(test_layout()),
    });
    assert!(e.handle(Input::RetryLayout).is_empty(), "ready");
    assert!(matches!(e.layout(), LayoutState::Ready(_)));
}

#[test]
fn disconnecting_cancels_the_retry() {
    let mut e = connected_engine();
    fail(&mut e);
    assert_eq!(e.layout_retry_interval(), secs(30));
    e.handle(disconnected());
    assert_eq!(e.layout_retry_interval(), None);
    assert!(e.handle(Input::RetryLayout).is_empty());
    assert_eq!(e.layout(), &LayoutState::Unavailable("offline".into()));
}

#[test]
fn a_late_failure_after_disconnecting_schedules_nothing() {
    let mut e = connected_engine();
    e.handle(disconnected());
    fail(&mut e);
    assert_eq!(e.layout_retry_interval(), None);
}

#[test]
fn reconnecting_restarts_the_backoff() {
    let mut e = connected_engine();
    fail(&mut e);
    e.handle(Input::RetryLayout);
    fail(&mut e);
    assert_eq!(e.layout_retry_interval(), secs(60));
    e.handle(disconnected());
    e.handle(connected());
    fail(&mut e);
    assert_eq!(e.layout_retry_interval(), secs(30));
}

#[test]
fn a_keyboard_without_ids_is_never_retried() {
    let mut e = Engine::new(Config::default());
    e.handle(Input::Device(DeviceEvent::Connected {
        model: Model::Voyager,
        ids: None,
        protocol: None,
    }));
    assert!(matches!(e.layout(), LayoutState::Unavailable(_)));
    assert_eq!(e.layout_retry_interval(), None);
    assert!(e.handle(Input::RetryLayout).is_empty());
}
