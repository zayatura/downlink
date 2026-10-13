pub fn hide_dock_icon() {}

pub fn activate_app() {}

pub fn set_launch_at_login(_enabled: bool) -> Result<(), String> {
    Err("Launch at login is not supported on this platform yet".into())
}

/// Unknown here; iced's own detection (mundy on Linux) supplies it.
pub fn system_mode() -> iced::theme::Mode {
    iced::theme::Mode::None
}
