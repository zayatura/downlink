use downlink::platform::appearance_from_interface_style;
use iced::theme::Mode;

#[test]
fn dark_style_means_dark_mode() {
    assert_eq!(appearance_from_interface_style(Some("Dark\n")), Mode::Dark);
}

#[test]
fn a_missing_or_other_style_means_light_mode() {
    assert_eq!(appearance_from_interface_style(None), Mode::Light);
    assert_eq!(appearance_from_interface_style(Some("")), Mode::Light);
}

/// Smoke test: runs the real `/usr/bin/defaults` call (read-only) and expects a definite mode.
#[cfg(target_os = "macos")]
#[test]
fn macos_system_mode_is_light_or_dark() {
    use downlink::platform::system_mode;

    assert!(matches!(system_mode(), Mode::Light | Mode::Dark));
}
