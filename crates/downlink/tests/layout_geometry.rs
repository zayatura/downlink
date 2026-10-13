use downlink::layout_view::key_rects;
use iced::Size;
use oryx_layout::geometry;
use oryx_protocol::Model;

fn board() -> Size {
    let (width, height) = geometry::board_size(Model::Voyager);
    Size::new(width, height)
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-3
}

#[test]
fn an_exact_fit_uses_100_pixel_keys_with_gaps() {
    let rects = key_rects(
        Size::new(1600.0, 575.0),
        geometry::keys(Model::Voyager),
        board(),
    );
    assert_eq!(rects.len(), 52);
    let first = rects[0];
    assert!(close(first.x, 4.0) && close(first.y, 54.0), "{first:?}");
    assert!(close(first.width, 92.0) && close(first.height, 92.0));
}

#[test]
fn wide_windows_centre_the_board() {
    let rects = key_rects(
        Size::new(3200.0, 575.0),
        geometry::keys(Model::Voyager),
        board(),
    );
    assert!(close(rects[0].x, 804.0), "{:?}", rects[0]);
}

#[test]
fn every_key_stays_inside_the_window() {
    for (w, h) in [
        (640.0, 300.0),
        (980.0, 440.0),
        (2000.0, 300.0),
        (500.0, 900.0),
    ] {
        for r in key_rects(Size::new(w, h), geometry::keys(Model::Voyager), board()) {
            assert!(
                r.x >= 0.0 && r.y >= 0.0 && r.x + r.width <= w + 1e-3 && r.y + r.height <= h + 1e-3,
                "{r:?} in {w}x{h}"
            );
        }
    }
}

#[test]
fn degenerate_sizes_draw_nothing() {
    assert!(key_rects(Size::ZERO, geometry::keys(Model::Voyager), board()).is_empty());
    assert!(
        key_rects(
            Size::new(800.0, 300.0),
            geometry::keys(Model::Voyager),
            Size::ZERO
        )
        .is_empty()
    );
}
