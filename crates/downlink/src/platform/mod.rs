//! OS-specific glue. Everything else in Downlink is platform-neutral.

pub mod launch_agent;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::{
    activate_app, hide_dock_icon, refresh_launch_at_login, set_launch_at_login, system_mode,
};

#[cfg(not(target_os = "macos"))]
mod other;
#[cfg(not(target_os = "macos"))]
pub use other::{
    activate_app, hide_dock_icon, refresh_launch_at_login, set_launch_at_login, system_mode,
};

use iced::theme::Mode;

/// Parses `defaults read -g AppleInterfaceStyle`: "Dark" in dark mode; the key is absent in light mode.
pub fn appearance_from_interface_style(output: Option<&str>) -> Mode {
    match output.map(str::trim) {
        Some(style) if style.eq_ignore_ascii_case("dark") => Mode::Dark,
        _ => Mode::Light,
    }
}
