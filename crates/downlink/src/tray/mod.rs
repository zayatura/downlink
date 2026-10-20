//! The tray item (spec §6.1): menu IDs and the click channel shared by both OSes, with one
//! implementation per OS.

use std::sync::{Mutex, OnceLock, PoisonError};

use futures::channel::mpsc;
use futures::{SinkExt, Stream, StreamExt};

pub mod menu_model;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::{AppMenu, Tray, install_event_handlers};

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::{AppMenu, Tray, install_event_handlers};

pub const SHOW_LAYOUT: &str = "show-layout";
pub const SETTINGS: &str = "settings";
pub const PAUSE: &str = "pause";
pub const QUIT: &str = "quit";
/// App-menu items, active while a Downlink window is focused (macOS only).
pub const APP_SETTINGS: &str = "app-settings";
pub const CLOSE_WINDOW: &str = "app-close-window";
pub const CLOSE_ALL_WINDOWS: &str = "app-close-all-windows";

/// What a clicked menu item (or its shortcut) asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    ShowLayout,
    ShowSettings,
    TogglePause,
    Quit,
    CloseWindow,
    CloseAllWindows,
}

/// Only the menu-bar "Quit Downlink" quits (Phase 0, option B). In a window, ⌘W closes it and ⌘Q
/// closes every window, which also takes Downlink out of the Dock.
pub fn menu_action(id: &str) -> Option<MenuAction> {
    match id {
        SHOW_LAYOUT => Some(MenuAction::ShowLayout),
        SETTINGS | APP_SETTINGS => Some(MenuAction::ShowSettings),
        PAUSE => Some(MenuAction::TogglePause),
        QUIT => Some(MenuAction::Quit),
        CLOSE_WINDOW => Some(MenuAction::CloseWindow),
        CLOSE_ALL_WINDOWS => Some(MenuAction::CloseAllWindows),
        _ => None,
    }
}

/// The process-wide channel that carries clicked menu item IDs from the tray to the
/// `menu_events` stream: the sender, and the receiver until the stream takes it.
type MenuChannel = (
    mpsc::UnboundedSender<String>,
    Mutex<Option<mpsc::UnboundedReceiver<String>>>,
);

static MENU_CHANNEL: OnceLock<MenuChannel> = OnceLock::new();

fn menu_channel() -> &'static MenuChannel {
    MENU_CHANNEL.get_or_init(|| {
        let (tx, rx) = mpsc::unbounded();
        (tx, Mutex::new(Some(rx)))
    })
}

/// Where a tray implementation sends the ID of a clicked item.
pub(crate) fn menu_sender() -> &'static mpsc::UnboundedSender<String> {
    &menu_channel().0
}

/// Menu item IDs as they are clicked.
pub fn menu_events() -> impl Stream<Item = String> {
    iced::stream::channel(16, async |mut output: mpsc::Sender<String>| {
        let receiver = menu_channel()
            .1
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
        let Some(mut rx) = receiver else {
            tracing::error!("the menu event stream was started twice; menu clicks are not handled");
            return;
        };
        while let Some(id) = rx.next().await {
            if output.send(id).await.is_err() {
                break;
            }
        }
    })
}
