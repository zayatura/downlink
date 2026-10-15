use downlink::layout_view::{blend_over, key_paint, keycap_quad};
use downlink::theme;
use iced::{Color, Point, Rectangle, Size};
use oryx_layout::labels::{KeyLegend, LegendKind};

fn legend(kind: LegendKind) -> KeyLegend {
    KeyLegend {
        tap: "A".into(),
        hold: None,
        kind,
        inherited: false,
    }
}

#[test]
fn blending_over_the_background_gives_an_opaque_colour() {
    let ink = Color::from_rgb(0.2, 0.4, 0.6);
    assert_eq!(blend_over(ink, 1.0, Color::WHITE), ink);
    let half = blend_over(Color::BLACK, 0.5, Color::WHITE);
    assert_eq!(half, Color::from_rgb(0.5, 0.5, 0.5));
    assert_eq!(half.a, 1.0);
}

#[test]
fn a_translucent_colour_keeps_its_own_alpha_in_the_blend() {
    let faint = Color {
        a: 0.5,
        ..Color::BLACK
    };
    assert_eq!(
        blend_over(faint, 0.5, Color::WHITE),
        Color::from_rgb(0.75, 0.75, 0.75)
    );
}

#[test]
fn the_quad_has_the_edges_of_a_stroke_centred_on_the_key() {
    // A quad draws its border inside its bounds, so it grows by half the border width.
    let key = Rectangle::new(Point::new(10.0, 20.0), Size::new(40.0, 30.0));
    let (bounds, radius) = keycap_quad(key, 6.0, 2.0);
    assert_eq!(
        bounds,
        Rectangle::new(Point::new(9.0, 19.0), Size::new(42.0, 32.0))
    );
    assert_eq!(radius, 7.0);
}

#[test]
fn a_layer_key_gets_the_accent_border() {
    let c = theme::colors(false);
    let paint = key_paint(Some(&legend(LegendKind::Layer)), false, false, &c);
    assert_eq!(paint.border, c.accent);
    assert_eq!(paint.border_width, 1.5);
    let normal = key_paint(Some(&legend(LegendKind::Normal)), false, false, &c);
    assert_eq!(normal.border, c.border);
    assert_eq!(normal.border_width, 1.0);
}

#[test]
fn quiet_and_dimmed_keys_are_faded_into_opaque_colours() {
    let c = theme::colors(false);
    let blank = key_paint(Some(&legend(LegendKind::Blank)), false, false, &c);
    assert_eq!(blank.fill, blend_over(c.key, 0.55, c.background));
    assert_eq!(blank.border, blend_over(c.border, 0.55, c.background));
    let dimmed = key_paint(Some(&legend(LegendKind::Normal)), false, true, &c);
    assert_eq!(dimmed.fill, blend_over(c.key, 0.45, c.background));
    assert_eq!(key_paint(None, false, false, &c).fill, blank.fill);
}

#[test]
fn a_pressed_key_uses_the_press_colours() {
    let c = theme::colors(true);
    let paint = key_paint(Some(&legend(LegendKind::Blank)), true, false, &c);
    assert_eq!(paint.fill, c.press);
    assert_eq!(paint.ink, c.on_press);
}
