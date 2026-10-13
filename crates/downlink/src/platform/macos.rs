use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};

use super::launch_agent;

/// Menu-bar-only app: no Dock icon, no app menu. Call on the main thread.
pub fn hide_dock_icon() {
    let Some(mtm) = MainThreadMarker::new() else {
        tracing::warn!("hide_dock_icon must run on the main thread");
        return;
    };
    let app = NSApplication::sharedApplication(mtm);
    #[allow(unused_unsafe)]
    let changed = unsafe { app.setActivationPolicy(NSApplicationActivationPolicy::Accessory) };
    if !changed {
        tracing::warn!("macOS refused the accessory activation policy");
    }
}

/// Brings Downlink's windows to the front (needed for accessory apps).
pub fn activate_app() {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let app = NSApplication::sharedApplication(mtm);
    #[allow(unused_unsafe, deprecated)]
    unsafe {
        app.activateIgnoringOtherApps(true)
    };
}

pub fn set_launch_at_login(enabled: bool) -> Result<(), String> {
    let home = dirs::home_dir().ok_or("could not find the home directory")?;
    if enabled {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        launch_agent::install(&home, &exe)
            .map(|_| ())
            .map_err(|e| e.to_string())
    } else {
        launch_agent::remove(&home).map_err(|e| e.to_string())
    }
}

/// The current macOS appearance. iced can't tell before a window exists (Phase 0).
pub fn system_mode() -> iced::theme::Mode {
    let output = std::process::Command::new("/usr/bin/defaults")
        .args(["read", "-g", "AppleInterfaceStyle"])
        .output();
    let style = match &output {
        Ok(out) if out.status.success() => Some(String::from_utf8_lossy(&out.stdout).into_owned()),
        _ => None, // the key is absent in light mode
    };
    super::appearance_from_interface_style(style.as_deref())
}
