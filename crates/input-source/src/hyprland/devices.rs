//! Hyprland's keyboard list (`j/devices`) and the decisions Downlink makes from it (spec §6.2–6.4).

use serde::Deserialize;

use crate::{InputSource, InputSourceError};

/// Devices Hyprland lists as keyboards that nobody types on. The same list as Omarchy's
/// `keyboard-layout-cycle`, so Downlink and `SUPER + Y` read the same keyboard.
const UNTYPED_PREFIXES: [&str; 5] = [
    "hl-virtual-keyboard",
    "power-button",
    "sleep-button",
    "lid-switch",
    "video-bus",
];

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Keyboard {
    pub name: String,
    #[serde(default)]
    pub rules: String,
    #[serde(default)]
    pub model: String,
    pub layout: String,
    #[serde(default)]
    pub variant: String,
    #[serde(default)]
    pub options: String,
    #[serde(default)]
    pub active_layout_index: usize,
    #[serde(default)]
    pub main: bool,
}

#[derive(Deserialize)]
struct Devices {
    keyboards: Vec<Keyboard>,
}

impl Keyboard {
    pub fn is_typed(&self) -> bool {
        !UNTYPED_PREFIXES.iter().any(|p| self.name.starts_with(p))
    }

    /// This keyboard's layout IDs, in `kb_layout` order.
    pub fn layout_ids(&self) -> Vec<String> {
        layout_ids(&self.layout, &self.variant)
    }

    fn same_layouts(&self, other: &Keyboard) -> bool {
        self.layout == other.layout && self.variant == other.variant
    }
}

pub fn no_keyboards() -> InputSourceError {
    InputSourceError::Platform("no keyboards".into())
}

pub fn parse_keyboards(json: &str) -> Result<Vec<Keyboard>, InputSourceError> {
    serde_json::from_str::<Devices>(json)
        .map(|devices| devices.keyboards)
        .map_err(|e| InputSourceError::Platform(format!("unexpected reply from Hyprland: {e}")))
}

/// `layout`, or `layout(variant)` when it has one. Variants pair with layouts by position; a
/// missing variant counts as empty.
pub fn layout_ids(layout: &str, variant: &str) -> Vec<String> {
    if layout.trim().is_empty() {
        return Vec::new();
    }
    let mut variants = variant.split(',');
    layout
        .split(',')
        .map(|layout| {
            let layout = layout.trim();
            match variants.next().map(str::trim).unwrap_or("") {
                "" => layout.to_owned(),
                variant => format!("{layout}({variant})"),
            }
        })
        .collect()
}

/// The keyboard being typed on: the typed one furthest through the list, since only the active
/// keyboard moves through it. Without typed keyboards, the main one, then any.
pub fn reading_keyboard(keyboards: &[Keyboard]) -> Option<&Keyboard> {
    keyboards
        .iter()
        .filter(|k| k.is_typed())
        .max_by_key(|k| k.active_layout_index)
        .or_else(|| keyboards.iter().find(|k| k.main))
        .or_else(|| keyboards.first())
}

pub fn current_id(keyboards: &[Keyboard]) -> Result<String, InputSourceError> {
    let reader = reading_keyboard(keyboards).ok_or_else(no_keyboards)?;
    let ids = reader.layout_ids();
    ids.get(reader.active_layout_index).cloned().ok_or_else(|| {
        InputSourceError::Platform(format!(
            "{} reports layout {} of {}",
            reader.name,
            reader.active_layout_index,
            ids.len()
        ))
    })
}

/// One `switchxkblayout` per keyboard holding the reading keyboard's list, untyped ones included,
/// so none of them drifts (spec §6.3).
pub fn switch_commands(keyboards: &[Keyboard], id: &str) -> Result<Vec<String>, InputSourceError> {
    let reader = reading_keyboard(keyboards).ok_or_else(no_keyboards)?;
    let index = reader
        .layout_ids()
        .iter()
        .position(|layout| layout == id)
        .ok_or_else(|| InputSourceError::NotFound(id.to_owned()))?;
    Ok(keyboards
        .iter()
        .filter(|k| k.same_layouts(reader))
        .map(|k| format!("switchxkblayout {} {index}", k.name))
        .collect())
}

pub fn batch(commands: &[String]) -> String {
    format!("[[BATCH]]{}", commands.join(" ; "))
}

/// Hyprland answers a batch with one reply per command, separated by `\n\n\n`.
pub fn check_batch_reply(reply: &str, commands: usize) -> Result<(), InputSourceError> {
    let replies: Vec<&str> = reply.split("\n\n\n").map(str::trim).collect();
    if replies.len() == commands && replies.iter().all(|r| *r == "ok") {
        return Ok(());
    }
    let reply = reply.trim();
    Err(InputSourceError::Platform(format!(
        "Hyprland did not switch the layout: {}",
        if reply.is_empty() {
            "no reply".to_owned()
        } else {
            reply.replace("\n\n\n", "; ")
        }
    )))
}

/// The selectable layouts: the reading keyboard's list, named by index (an empty or missing name
/// falls back to the ID), deduplicated and sorted by name. The sort ignores case, so a lowercase
/// fallback ID sits among the names rather than after them all.
pub fn sources(reader: &Keyboard, names: &[String]) -> Vec<InputSource> {
    let mut sources: Vec<InputSource> = Vec::new();
    for (index, id) in reader.layout_ids().into_iter().enumerate() {
        if sources.iter().any(|s| s.id == id) {
            continue;
        }
        let name = names
            .get(index)
            .filter(|name| !name.is_empty())
            .cloned()
            .unwrap_or_else(|| id.clone());
        sources.push(InputSource { id, name });
    }
    sources.sort_by(|a, b| (a.name.to_lowercase(), &a.id).cmp(&(b.name.to_lowercase(), &b.id)));
    sources
}
