use std::sync::Arc;

use companion_core::LayoutState;
use downlink::layout_view::overlay_message;
use oryx_layout::Layout;

fn ready() -> LayoutState {
    LayoutState::Ready(Arc::new(Layout {
        title: "Test".into(),
        layout_id: "abc12".into(),
        revision_id: "rev99".into(),
        geometry: "voyager".into(),
        layers: Vec::new(),
    }))
}

#[test]
fn an_unavailable_layout_that_will_be_retried_says_so() {
    let layout = LayoutState::Unavailable("timeout".into());
    assert_eq!(
        overlay_message(&layout, true, true).as_deref(),
        Some("Layout unavailable — will retry when online.")
    );
}

#[test]
fn an_unavailable_layout_that_is_not_retried_shows_the_reason() {
    let layout = LayoutState::Unavailable("timeout".into());
    assert_eq!(
        overlay_message(&layout, true, false).as_deref(),
        Some("Layout unavailable — timeout")
    );
}

#[test]
fn a_loading_layout_says_so() {
    assert_eq!(
        overlay_message(&LayoutState::Loading, true, false).as_deref(),
        Some("Loading layout…")
    );
}

#[test]
fn a_ready_layout_without_the_keyboard_waits_for_it() {
    assert_eq!(
        overlay_message(&ready(), false, false).as_deref(),
        Some("Waiting for Voyager…")
    );
}

#[test]
fn a_ready_layout_with_the_keyboard_has_no_overlay() {
    assert_eq!(overlay_message(&ready(), true, false), None);
}
