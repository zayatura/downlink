use companion_core::TrayView;
use downlink::tray::menu_model::{MenuEntry, menu_entries};
use downlink::tray::{MenuAction, PAUSE, QUIT, SETTINGS, SHOW_LAYOUT, menu_action};

fn view(connected: bool, paused: bool) -> TrayView {
    TrayView {
        connected,
        status: if connected {
            "● Voyager connected".into()
        } else {
            "Waiting for Voyager…".into()
        },
        detail: if connected {
            "Layer: Main · Input: English (US)".into()
        } else {
            String::new()
        },
        paused,
    }
}

#[test]
fn a_connected_menu_matches_the_macos_one() {
    assert_eq!(
        menu_entries(&view(true, false)),
        vec![
            MenuEntry::Label("● Voyager connected".into()),
            MenuEntry::Label("Layer: Main · Input: English (US)".into()),
            MenuEntry::Separator,
            MenuEntry::Item {
                id: SHOW_LAYOUT,
                label: "Show Layout"
            },
            MenuEntry::Item {
                id: SETTINGS,
                label: "Settings…"
            },
            MenuEntry::Separator,
            MenuEntry::Check {
                id: PAUSE,
                label: "Pause Input Switching",
                checked: false
            },
            MenuEntry::Separator,
            MenuEntry::Item {
                id: QUIT,
                label: "Quit Downlink"
            },
        ]
    );
}

#[test]
fn a_disconnected_menu_has_no_detail_row() {
    let entries = menu_entries(&view(false, false));
    assert_eq!(entries[0], MenuEntry::Label("Waiting for Voyager…".into()));
    assert_eq!(entries[1], MenuEntry::Separator);
    assert_eq!(
        entries
            .iter()
            .filter(|e| matches!(e, MenuEntry::Label(_)))
            .count(),
        1
    );
}

#[test]
fn pausing_checks_the_pause_item() {
    assert!(menu_entries(&view(true, true)).contains(&MenuEntry::Check {
        id: PAUSE,
        label: "Pause Input Switching",
        checked: true,
    }));
}

#[test]
fn every_clickable_entry_maps_to_an_action_and_only_quit_quits() {
    for entry in menu_entries(&view(true, false)) {
        let id = match entry {
            MenuEntry::Item { id, .. } | MenuEntry::Check { id, .. } => id,
            MenuEntry::Label(_) | MenuEntry::Separator | MenuEntry::Hidden => continue,
        };
        let action = menu_action(id).unwrap_or_else(|| panic!("{id} has no action"));
        assert_eq!(action == MenuAction::Quit, id == QUIT, "{id}");
    }
}

use downlink::tray::menu_model::{dbusmenu_label, reshaped};

#[test]
fn underscores_in_labels_are_not_mnemonics() {
    assert_eq!(
        dbusmenu_label("Layer: Sym_L · Input: us"),
        "Layer: Sym__L · Input: us"
    );
    assert_eq!(dbusmenu_label("Settings…"), "Settings…");
}

#[test]
fn consecutive_updates_change_the_menu_shape_but_not_what_is_shown() {
    let entries = menu_entries(&view(true, true));
    let even = reshaped(entries.clone(), 4);
    let odd = reshaped(entries.clone(), 5);
    assert_ne!(
        even.len(),
        odd.len(),
        "the shape changes, so the host reloads the layout"
    );
    let shown = |list: &[MenuEntry]| {
        list.iter()
            .filter(|e| **e != MenuEntry::Hidden)
            .cloned()
            .collect::<Vec<_>>()
    };
    assert_eq!(shown(&even), entries);
    assert_eq!(shown(&odd), entries);
}

#[test]
fn the_linux_tray_shows_the_items_without_separators() {
    use downlink::tray::menu_model::sni_entries;
    let view = view(true, false);
    let entries = sni_entries(&view, 2);
    assert!(!entries.contains(&MenuEntry::Separator));
    let expected: Vec<MenuEntry> = menu_entries(&view)
        .into_iter()
        .filter(|e| *e != MenuEntry::Separator)
        .collect();
    assert_eq!(entries, expected);
    assert_eq!(
        sni_entries(&view, 3).len(),
        expected.len() + 1,
        "still reshaped"
    );
}
