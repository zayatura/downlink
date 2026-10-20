//! The menu-bar item and app menu on macOS (spec §6.1): tray-icon and muda.

use companion_core::TrayView;
use tray_icon::menu::accelerator::Accelerator;
use tray_icon::menu::{
    CheckMenuItem, IsMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu,
};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder, TrayIconEvent};

use super::{
    APP_SETTINGS, CLOSE_ALL_WINDOWS, CLOSE_WINDOW, PAUSE, QUIT, SETTINGS, SHOW_LAYOUT, menu_sender,
};
use crate::icon;

/// The app's main menu. On macOS it replaces winit's default menu, whose ⌘Q item would call
/// `NSApp terminate:` and end the process outside Downlink's Quit path. macOS shows it while a
/// Downlink window is open, since Downlink is then a regular app.
pub struct AppMenu {
    _menu: Menu,
}

impl AppMenu {
    /// Call from `update` (main thread, event loop running), like `Tray::new`.
    pub fn install() -> Result<AppMenu, String> {
        let menu = Menu::new();
        let app = Submenu::new("Downlink", true);
        for item in [
            &MenuItem::with_id(APP_SETTINGS, "Settings…", true, accelerator("CmdOrCtrl+,"))
                as &dyn IsMenuItem,
            &PredefinedMenuItem::separator(),
            &MenuItem::with_id(
                CLOSE_WINDOW,
                "Close Window",
                true,
                accelerator("CmdOrCtrl+W"),
            ),
            &MenuItem::with_id(
                CLOSE_ALL_WINDOWS,
                "Close All Windows",
                true,
                accelerator("CmdOrCtrl+Q"),
            ),
        ] {
            app.append(item).map_err(|e| e.to_string())?;
        }
        menu.append(&app).map_err(|e| e.to_string())?;
        menu.init_for_nsapp();
        Ok(AppMenu { _menu: menu })
    }
}

/// 18 pt at 2× scale.
const ICON_PX: u32 = 36;

pub struct Tray {
    icon: TrayIcon,
    status: MenuItem,
    detail: MenuItem,
    pause: CheckMenuItem,
    connected: bool,
}

impl Tray {
    /// Must run on the main thread after the event loop starts: call it from `update`.
    pub fn new() -> Result<Tray, String> {
        let status = MenuItem::new("Waiting for Voyager…", false, None);
        let detail = MenuItem::new(" ", false, None);
        let show = MenuItem::with_id(SHOW_LAYOUT, "Show Layout", true, accelerator("CmdOrCtrl+L"));
        let settings = MenuItem::with_id(SETTINGS, "Settings…", true, accelerator("CmdOrCtrl+,"));
        let pause = CheckMenuItem::with_id(PAUSE, "Pause Input Switching", true, false, None);
        let quit = MenuItem::with_id(QUIT, "Quit Downlink", true, accelerator("CmdOrCtrl+Q"));
        let menu = Menu::new();
        for item in [
            &status as &dyn IsMenuItem,
            &detail,
            &PredefinedMenuItem::separator(),
            &show,
            &settings,
            &PredefinedMenuItem::separator(),
            &pause,
            &PredefinedMenuItem::separator(),
            &quit,
        ] {
            menu.append(item).map_err(|e| e.to_string())?;
        }
        let icon = TrayIconBuilder::new()
            .with_icon_templated(tray_icon_image(true)?)
            .with_menu(Box::new(menu))
            .with_tooltip("Downlink")
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Tray {
            icon,
            status,
            detail,
            pause,
            connected: true,
        })
    }

    /// `_dark` is unused: macOS recolours the template icon for the menu bar itself.
    pub fn apply(&mut self, view: &TrayView, _dark: bool) {
        self.status.set_text(&view.status);
        self.detail.set_text(if view.detail.is_empty() {
            " "
        } else {
            view.detail.as_str()
        });
        self.pause.set_checked(view.paused);
        if view.connected != self.connected {
            match tray_icon_image(view.connected) {
                Ok(image) => {
                    if let Err(error) = self.icon.set_icon_templated(Some(image)) {
                        tracing::warn!(%error, "could not update the menu-bar icon");
                    }
                }
                Err(error) => tracing::warn!(%error, "could not render the menu-bar icon"),
            }
            self.connected = view.connected;
        }
    }
}

fn accelerator(spec: &str) -> Option<Accelerator> {
    spec.parse().ok()
}

fn tray_icon_image(connected: bool) -> Result<Icon, String> {
    Icon::from_rgba(icon::menubar_rgba(ICON_PX, connected), ICON_PX, ICON_PX)
        .map_err(|e| e.to_string())
}

/// Register both event handlers. Call it once, first thing in `App::init`, before any menu or
/// tray icon exists.
///
/// tray-icon and muda keep their handler in a write-once cell. An event that arrives while no
/// handler is set fills the cell with "no handler" for good, so a handler registered later from
/// inside a subscription future could be silently ignored. With no handler, tray-icon also
/// queues every Enter/Move/Leave/Click over the status item in a global unbounded channel that
/// nothing reads, so the memory only grows.
pub fn install_event_handlers() {
    // Downlink doesn't use tray events; the handler only stops them piling up.
    TrayIconEvent::set_event_handler(Some(|_event: TrayIconEvent| {}));
    let tx = menu_sender().clone();
    MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        let _ = tx.unbounded_send(event.id.0);
    }));
}
