//! Linux glue (spec §8). Appearance comes from iced's portal watcher, and the compositor
//! handles windows, so most of this does nothing.

use std::path::PathBuf;
use std::{fs, io};

use iced::window::settings::PlatformSpecific;

use super::autostart;

pub fn hide_dock_icon() {}

pub fn activate_app() {}

fn config_dir() -> Result<PathBuf, String> {
    dirs::config_dir().ok_or_else(|| "could not find the config directory".to_owned())
}

pub fn set_launch_at_login(enabled: bool) -> Result<(), String> {
    let config = config_dir()?;
    if enabled {
        let exe = std::env::current_exe()
            .map(autostart::live_executable)
            .map_err(|e| e.to_string())?;
        autostart::install(&config, &exe)
            .map(|_| ())
            .map_err(|e| e.to_string())
    } else {
        autostart::remove(&config).map_err(|e| e.to_string())
    }
}

/// Startup check: rewrite the autostart entry if it's missing or points elsewhere, except from a
/// dev build. Returns whether it was rewritten.
pub fn refresh_launch_at_login() -> Result<bool, String> {
    let config = config_dir()?;
    let exe = std::env::current_exe()
        .map(autostart::live_executable)
        .map_err(|e| e.to_string())?;
    let existing = match fs::read_to_string(autostart::entry_path(&config)) {
        Ok(text) => Some(text),
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => return Err(error.to_string()),
    };
    if !autostart::needs_refresh(existing.as_deref(), &exe) {
        return Ok(false);
    }
    autostart::install(&config, &exe).map_err(|e| e.to_string())?;
    Ok(true)
}

/// Unknown here; iced's own detection (the XDG portal, through mundy) supplies it.
pub fn system_mode() -> iced::theme::Mode {
    iced::theme::Mode::None
}

/// The Wayland `app_id`, matching `downlink.desktop`, so Hyprland and the launcher know the
/// windows.
pub fn window_platform_specific() -> PlatformSpecific {
    PlatformSpecific {
        application_id: "downlink".into(),
        ..PlatformSpecific::default()
    }
}
