//! The menu-bar item (spec §6.1).

use companion_core::TrayView;
use futures::channel::mpsc;
use futures::{SinkExt, Stream, StreamExt};
use tray_icon::menu::accelerator::Accelerator;
use tray_icon::menu::{
    CheckMenuItem, IsMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu,
};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

use crate::icon;

pub const SHOW_LAYOUT: &str = "show-layout";
pub const SETTINGS: &str = "settings";
pub const PAUSE: &str = "pause";
pub const QUIT: &str = "quit";
/// App-menu items, active while a Downlink window is focused.
pub const APP_SETTINGS: &str = "app-settings";
pub const CLOSE_WINDOW: &str = "app-close-window";
pub const CLOSE_WINDOW_Q: &str = "app-close-window-q";

/// What a clicked menu item (or its shortcut) asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    ShowLayout,
    ShowSettings,
    TogglePause,
    Quit,
    CloseWindow,
}

/// Only the menu-bar "Quit Downlink" quits; ⌘W and ⌘Q in a window close it (Phase 0, option B).
pub fn menu_action(id: &str) -> Option<MenuAction> {
    match id {
        SHOW_LAYOUT => Some(MenuAction::ShowLayout),
        SETTINGS | APP_SETTINGS => Some(MenuAction::ShowSettings),
        PAUSE => Some(MenuAction::TogglePause),
        QUIT => Some(MenuAction::Quit),
        CLOSE_WINDOW | CLOSE_WINDOW_Q => Some(MenuAction::CloseWindow),
        _ => None,
    }
}

/// The app's main menu. On macOS it replaces winit's default menu, whose ⌘Q item would call
/// `NSApp terminate:` and end the process outside Downlink's Quit path. An accessory app never
/// shows this menu, but macOS still routes its shortcuts while a Downlink window is focused.
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
                CLOSE_WINDOW_Q,
                "Close Window",
                true,
                accelerator("CmdOrCtrl+Q"),
            ),
        ] {
            app.append(item).map_err(|e| e.to_string())?;
        }
        menu.append(&app).map_err(|e| e.to_string())?;
        #[cfg(target_os = "macos")]
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

    pub fn apply(&mut self, view: &TrayView) {
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

/// Menu item IDs as they are clicked.
pub fn menu_events() -> impl Stream<Item = String> {
    iced::stream::channel(16, async |mut output: mpsc::Sender<String>| {
        let (tx, mut rx) = mpsc::unbounded::<String>();
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            let _ = tx.unbounded_send(event.id.0);
        }));
        while let Some(id) = rx.next().await {
            if output.send(id).await.is_err() {
                break;
            }
        }
    })
}
