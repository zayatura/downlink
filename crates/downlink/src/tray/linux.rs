//! The tray item on Linux. Placeholder until the StatusNotifierItem tray lands.

use companion_core::TrayView;

/// Nothing to register: the Linux tray sends clicks straight into the menu channel.
pub fn install_event_handlers() {}

/// Linux has no app menu: the compositor closes windows (for example Meta+W).
pub struct AppMenu;

impl AppMenu {
    pub fn install() -> Result<AppMenu, String> {
        Ok(AppMenu)
    }
}

pub struct Tray;

impl Tray {
    pub fn new() -> Result<Tray, String> {
        Err("the Linux tray is not available yet".into())
    }

    pub fn apply(&mut self, _view: &TrayView, _dark: bool) {}
}
