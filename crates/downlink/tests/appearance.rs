use downlink::platform::{appearance_from_interface_style, system_mode};
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

#[cfg(target_os = "macos")]
#[test]
fn macos_always_reports_a_definite_mode() {
    assert_ne!(system_mode(), Mode::None);
}
