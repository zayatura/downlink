use std::{fs, io};

use objc2::MainThreadMarker;
use objc2_app_kit::{
    NSAppearanceNameAqua, NSAppearanceNameDarkAqua, NSApplication, NSApplicationActivationPolicy,
};
use objc2_foundation::NSArray;

use super::launch_agent;

/// Menu-bar-only app: no Dock icon, no app menu. Call on the main thread.
pub fn hide_dock_icon() {
    let Some(mtm) = MainThreadMarker::new() else {
        tracing::warn!("hide_dock_icon must run on the main thread");
        return;
    };
    let app = NSApplication::sharedApplication(mtm);
    // The bundle's LSUIElement already starts Downlink as an accessory app, and macOS answers
    // a request for the policy it already has with "refused".
    if app.activationPolicy() == NSApplicationActivationPolicy::Accessory {
        return;
    }
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

/// Startup check (spec §12 says the Settings toggle writes the login item, so a launch only
/// repairs it): from an app bundle, rewrite the LaunchAgent if it is missing or points elsewhere.
/// Returns whether it was rewritten.
pub fn refresh_launch_at_login() -> Result<bool, String> {
    let home = dirs::home_dir().ok_or("could not find the home directory")?;
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let existing = match fs::read_to_string(launch_agent::agent_path(&home)) {
        Ok(text) => Some(text),
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => return Err(error.to_string()),
    };
    if !launch_agent::needs_refresh(existing.as_deref(), &exe) {
        return Ok(false);
    }
    launch_agent::install(&home, &exe).map_err(|e| e.to_string())?;
    Ok(true)
}

/// The current macOS appearance, `Mode::None` off the main thread. iced only learns of a change
/// through a window whose appearance it hasn't pinned yet, so Downlink asks AppKit directly: at
/// startup, when it opens a window, and on every poll while one is open.
pub fn system_mode() -> iced::theme::Mode {
    let Some(mtm) = MainThreadMarker::new() else {
        return iced::theme::Mode::None;
    };
    let appearance = NSApplication::sharedApplication(mtm).effectiveAppearance();
    // SAFETY: AppKit's appearance-name constants are immutable statics.
    let (aqua, dark) = unsafe { (NSAppearanceNameAqua, NSAppearanceNameDarkAqua) };
    match appearance.bestMatchFromAppearancesWithNames(&NSArray::from_slice(&[aqua, dark])) {
        Some(name) if &*name == dark => iced::theme::Mode::Dark,
        _ => iced::theme::Mode::Light,
    }
}
