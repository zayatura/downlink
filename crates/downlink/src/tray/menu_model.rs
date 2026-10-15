//! The tray menu as data, built from `TrayView`. The Linux tray turns it into DBusMenu items.

use companion_core::TrayView;

use super::{PAUSE, QUIT, SETTINGS, SHOW_LAYOUT};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuEntry {
    /// A greyed-out line of text: the status and the detail.
    Label(String),
    Item {
        id: &'static str,
        label: &'static str,
    },
    Check {
        id: &'static str,
        label: &'static str,
        checked: bool,
    },
    Separator,
    /// An invisible entry; see `reshaped`.
    Hidden,
}

/// The same items as the macOS menu (spec §7). The detail row is left out while it's empty.
pub fn menu_entries(view: &TrayView) -> Vec<MenuEntry> {
    let mut entries = vec![MenuEntry::Label(view.status.clone())];
    if !view.detail.is_empty() {
        entries.push(MenuEntry::Label(view.detail.clone()));
    }
    entries.extend([
        MenuEntry::Separator,
        MenuEntry::Item {
            id: SHOW_LAYOUT,
            label: "Show Layout",
        },
        MenuEntry::Item {
            id: SETTINGS,
            label: "Settings…",
        },
        MenuEntry::Separator,
        MenuEntry::Check {
            id: PAUSE,
            label: "Pause Input Switching",
            checked: view.paused,
        },
        MenuEntry::Separator,
        MenuEntry::Item {
            id: QUIT,
            label: "Quit Downlink",
        },
    ]);
    entries
}

/// DBusMenu reads `_` as a mnemonic marker; a literal underscore is written twice.
pub fn dbusmenu_label(text: &str) -> String {
    text.replace('_', "__")
}

/// quickshell 0.3, Omarchy's bar, reads an `ItemsPropertiesUpdated` signal as an item's full
/// property set, so a partial update blanks the label, re-enables a greyed row and drops the
/// checkmark. ksni sends `LayoutUpdated` instead, which quickshell handles, only when the menu
/// changes shape: a hidden entry on every other update changes it each time.
pub fn reshaped(mut entries: Vec<MenuEntry>, generation: u64) -> Vec<MenuEntry> {
    if generation % 2 == 1 {
        entries.push(MenuEntry::Hidden);
    }
    entries
}

/// What the Linux tray shows: the menu without separators, which Omarchy's bar draws 1 or 2
/// pixels thick at a fractional scale, reshaped for every update.
pub fn sni_entries(view: &TrayView, generation: u64) -> Vec<MenuEntry> {
    let entries = menu_entries(view)
        .into_iter()
        .filter(|entry| *entry != MenuEntry::Separator)
        .collect();
    reshaped(entries, generation)
}
