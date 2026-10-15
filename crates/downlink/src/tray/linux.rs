//! The tray item on Linux (spec §7): a StatusNotifierItem over D-Bus, shown by Omarchy's bar.

use companion_core::TrayView;
use ksni::blocking::{Handle, TrayMethods};
use ksni::menu::{CheckmarkItem, StandardItem};
use ksni::{Icon, MenuItem, OfflineReason, ToolTip};

use super::menu_model::{MenuEntry, dbusmenu_label, sni_entries};
use super::menu_sender;
use crate::icon;

/// Glyph colours: light on a dark appearance, dark on a light one (the palettes' key text).
const ON_DARK: [u8; 3] = [0xe8, 0xea, 0xee];
const ON_LIGHT: [u8; 3] = [0x1d, 0x20, 0x25];
/// One logical size at 1× and 2×; the host picks the closest.
const ICON_SIZES: [u32; 2] = [22, 44];

/// Nothing to register: menu items send clicks straight into the menu channel.
pub fn install_event_handlers() {}

/// Linux has no app menu: the compositor closes windows (for example Meta+W).
pub struct AppMenu;

impl AppMenu {
    pub fn install() -> Result<AppMenu, String> {
        Ok(AppMenu)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct State {
    view: TrayView,
    dark: bool,
}

struct DownlinkTray {
    state: State,
    /// Counts updates, so every update reshapes the menu (see `reshaped`).
    generation: u64,
}

impl ksni::Tray for DownlinkTray {
    /// A left click opens the menu, as on macOS.
    const MENU_ON_ACTIVATE: bool = true;

    fn id(&self) -> String {
        "downlink".into()
    }

    fn title(&self) -> String {
        "Downlink".into()
    }

    fn icon_pixmap(&self) -> Vec<Icon> {
        let rgb = if self.state.dark { ON_DARK } else { ON_LIGHT };
        ICON_SIZES
            .iter()
            .map(|&px| Icon {
                width: px as i32,
                height: px as i32,
                data: icon::menubar_argb(px, self.state.view.connected, rgb),
            })
            .collect()
    }

    fn tool_tip(&self) -> ToolTip {
        ToolTip {
            title: "Downlink".into(),
            description: self.state.view.status.clone(),
            ..Default::default()
        }
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        sni_entries(&self.state.view, self.generation)
            .into_iter()
            .map(to_ksni)
            .collect()
    }

    fn watcher_offline(&self, reason: OfflineReason) -> bool {
        tracing::warn!(
            ?reason,
            "the tray host went away; the icon returns when it's back"
        );
        true
    }
}

fn to_ksni(entry: MenuEntry) -> MenuItem<DownlinkTray> {
    match entry {
        MenuEntry::Label(text) => StandardItem {
            label: dbusmenu_label(&text),
            enabled: false,
            ..Default::default()
        }
        .into(),
        MenuEntry::Item { id, label } => StandardItem {
            label: label.into(),
            activate: Box::new(move |_: &mut DownlinkTray| send(id)),
            ..Default::default()
        }
        .into(),
        MenuEntry::Check { id, label, checked } => CheckmarkItem {
            label: label.into(),
            checked,
            activate: Box::new(move |_: &mut DownlinkTray| send(id)),
            ..Default::default()
        }
        .into(),
        MenuEntry::Separator => MenuItem::Separator,
        MenuEntry::Hidden => StandardItem {
            visible: false,
            enabled: false,
            ..Default::default()
        }
        .into(),
    }
}

fn send(id: &str) {
    let _ = menu_sender().unbounded_send(id.to_owned());
}

pub struct Tray {
    handle: Handle<DownlinkTray>,
    state: State,
    generation: u64,
}

impl Tray {
    /// Spawns the D-Bus service on its own thread. With `assume_sni_available`, a bar that
    /// isn't up yet (autostart racing it at login) is waited for instead of failing.
    pub fn new() -> Result<Tray, String> {
        let state = State {
            view: TrayView {
                connected: false,
                status: "Waiting for Voyager…".into(),
                detail: String::new(),
                paused: false,
            },
            dark: false,
        };
        let handle = DownlinkTray {
            state: state.clone(),
            generation: 0,
        }
        .assume_sni_available(true)
        .spawn()
        .map_err(|e| e.to_string())?;
        Ok(Tray {
            handle,
            state,
            generation: 0,
        })
    }

    pub fn apply(&mut self, view: &TrayView, dark: bool) {
        let state = State {
            view: view.clone(),
            dark,
        };
        if state == self.state {
            return;
        }
        self.state = state.clone();
        self.generation += 1;
        let generation = self.generation;
        let update = move |tray: &mut DownlinkTray| {
            tray.state = state;
            tray.generation = generation;
        };
        if self.handle.update(update).is_none() {
            tracing::warn!("the tray service stopped; the menu is no longer updated");
        }
    }
}
