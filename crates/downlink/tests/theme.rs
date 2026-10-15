use companion_core::Appearance;
use downlink::theme::{self, colors, iced_theme, is_dark, tray_is_dark};
use iced::Color;
use iced::theme::Mode;

#[test]
fn system_appearance_follows_the_os() {
    assert!(is_dark(Appearance::System, Mode::Dark));
    assert!(!is_dark(Appearance::System, Mode::Light));
    assert!(
        !is_dark(Appearance::System, Mode::None),
        "unknown OS mode falls back to light"
    );
}

#[test]
fn forced_appearance_ignores_the_os() {
    assert!(is_dark(Appearance::Dark, Mode::Light));
    assert!(!is_dark(Appearance::Light, Mode::Dark));
}

#[test]
fn palettes_use_the_mockup_colours() {
    assert_eq!(theme::dark().background, Color::from_rgb8(0x1d, 0x1f, 0x24));
    assert_eq!(theme::dark().accent, Color::from_rgb8(0x4c, 0xc2, 0xff));
    assert_eq!(theme::dark().press, Color::from_rgb8(0xff, 0xb5, 0x47));
    assert_eq!(
        theme::light().background,
        Color::from_rgb8(0xf4, 0xf5, 0xf7)
    );
    assert_eq!(theme::light().accent, Color::from_rgb8(0x0a, 0x84, 0xff));
    assert_eq!(colors(true), theme::dark());
    assert_eq!(colors(false), theme::light());
}

#[test]
fn iced_theme_carries_the_palette() {
    assert_eq!(
        iced_theme(true).palette().background,
        theme::dark().background
    );
    assert_eq!(iced_theme(false).palette().primary, theme::light().accent);
}

#[test]
fn the_tray_icon_follows_the_system_not_the_window_override() {
    // The bar is drawn in the system appearance, whatever Downlink's windows use.
    assert!(!tray_is_dark(Appearance::Dark, Mode::Light));
    assert!(tray_is_dark(Appearance::Light, Mode::Dark));
    assert!(tray_is_dark(Appearance::System, Mode::Dark));
    // Before the system mode is known, the effective appearance is the best guess.
    assert!(tray_is_dark(Appearance::Dark, Mode::None));
    assert!(!tray_is_dark(Appearance::System, Mode::None));
}
