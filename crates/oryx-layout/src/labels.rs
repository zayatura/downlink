//! Turns key actions into short display legends.

use crate::{Action, Key, Layout, Modifiers};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModifierStyle {
    /// ⌃ ⌥ ⇧ ⌘ (macOS).
    Symbols,
    /// Ctrl Alt Shift Super (Linux).
    Words,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegendKind {
    Normal,
    /// The tap action switches layers; drawn with the accent border.
    Layer,
    /// Falls through to the layer below; has no legend of its own.
    Transparent,
    /// Does nothing (`KC_NO`).
    Blank,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyLegend {
    pub tap: String,
    pub hold: Option<String>,
    pub kind: LegendKind,
    /// Borrowed from a layer beneath, because the shown layer leaves the key transparent.
    pub inherited: bool,
}

const LAYER_CODES: [&str; 7] = ["TO", "DF", "TG", "OSL", "MO", "TT", "LM"];

pub fn key_legend(key: &Key, layer_titles: &[String], style: ModifierStyle) -> KeyLegend {
    if key.is_transparent() {
        return KeyLegend {
            tap: String::new(),
            hold: None,
            kind: LegendKind::Transparent,
            inherited: false,
        };
    }
    let hold = key
        .hold
        .as_ref()
        .map(|action| hold_label(action, layer_titles, style));
    let (tap, kind) = match (key.custom_label.as_deref(), key.tap.as_ref()) {
        (Some(label), tap) => {
            let kind = if tap.is_some_and(is_layer_action) {
                LegendKind::Layer
            } else {
                LegendKind::Normal
            };
            (label.to_owned(), kind)
        }
        (None, Some(action)) => tap_label(action, layer_titles, style),
        (None, None) => (String::new(), LegendKind::Normal),
    };
    let kind = if tap.is_empty() && hold.is_none() {
        LegendKind::Blank
    } else {
        kind
    };
    KeyLegend {
        tap,
        hold,
        kind,
        inherited: false,
    }
}

/// The legends for every key of layer `shown`, with `below` (highest first) the active layers
/// its transparent keys fall through to. A key no layer sets is blank. Empty if `shown` is not
/// in the layout.
pub fn layer_legends(
    layout: &Layout,
    shown: u8,
    below: &[u8],
    style: ModifierStyle,
) -> Vec<KeyLegend> {
    let Some(layer) = layout.layer(shown) else {
        return Vec::new();
    };
    let titles: Vec<String> = layout.layers.iter().map(|l| l.title.clone()).collect();
    let order = || std::iter::once(shown).chain(below.iter().copied());
    (0..layer.keys.len())
        .map(|index| match layout.resolve_key(order(), index) {
            Some((from, key)) => KeyLegend {
                inherited: from != shown,
                ..key_legend(key, &titles, style)
            },
            None => KeyLegend {
                tap: String::new(),
                hold: None,
                kind: LegendKind::Blank,
                inherited: false,
            },
        })
        .collect()
}

fn is_layer_action(action: &Action) -> bool {
    action.layer.is_some() && LAYER_CODES.contains(&action.code.as_str())
}

fn tap_label(action: &Action, titles: &[String], style: ModifierStyle) -> (String, LegendKind) {
    if let Some(layer) = action.layer.filter(|_| is_layer_action(action)) {
        let title = layer_title(titles, layer);
        let text = match action.code.as_str() {
            "TO" | "DF" => format!("→ {title}"),
            "TG" => format!("⇆ {title}"),
            "OSL" => format!("1× {title}"),
            _ => title,
        };
        return (text, LegendKind::Layer);
    }
    (
        with_modifiers(&code_label(&action.code, style), action.modifiers, style),
        LegendKind::Normal,
    )
}

fn hold_label(action: &Action, titles: &[String], style: ModifierStyle) -> String {
    match action.layer {
        Some(layer) if is_layer_action(action) => layer_title(titles, layer),
        _ => with_modifiers(&code_label(&action.code, style), action.modifiers, style),
    }
}

fn layer_title(titles: &[String], layer: u8) -> String {
    titles
        .get(usize::from(layer))
        .map(|title| title.trim())
        .filter(|title| !title.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| format!("L{layer}"))
}

fn with_modifiers(label: &str, m: Modifiers, style: ModifierStyle) -> String {
    if !m.any() {
        return label.to_owned();
    }
    let parts = [
        (m.ctrl, "⌃", "Ctrl"),
        (m.alt, "⌥", "Alt"),
        (m.shift, "⇧", "Shift"),
        (m.gui, "⌘", "Super"),
    ];
    match style {
        ModifierStyle::Symbols => {
            let prefix: String = parts.iter().filter(|p| p.0).map(|p| p.1).collect();
            format!("{prefix}{label}")
        }
        ModifierStyle::Words => {
            let prefix: Vec<&str> = parts.iter().filter(|p| p.0).map(|p| p.2).collect();
            format!("{}+{label}", prefix.join("+"))
        }
    }
}

/// A short legend for a keycode.
pub fn code_label(code: &str, style: ModifierStyle) -> String {
    if let Some(label) = builtin(code, style) {
        return label.to_owned();
    }
    let bare = code.strip_prefix("KC_").unwrap_or(code);
    if let Some(rest) = bare.strip_prefix("KP_")
        && rest.chars().count() == 1
    {
        return rest.to_owned();
    }
    if bare.chars().count() == 1 || is_function_key(bare) {
        return bare.to_owned();
    }
    bare.replace('_', " ")
}

fn is_function_key(name: &str) -> bool {
    name.strip_prefix('F')
        .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
}

fn sym(style: ModifierStyle, symbol: &'static str, word: &'static str) -> &'static str {
    match style {
        ModifierStyle::Symbols => symbol,
        ModifierStyle::Words => word,
    }
}

fn builtin(code: &str, style: ModifierStyle) -> Option<&'static str> {
    Some(match code {
        "KC_NO" => "",
        "KC_TRANSPARENT" | "KC_TRNS" => "",
        "KC_ENTER" | "KC_ENT" | "KC_KP_ENTER" => "Enter",
        "KC_ESCAPE" | "KC_ESC" => "Esc",
        "KC_BSPC" | "KC_BACKSPACE" => "Bksp",
        "KC_DELETE" | "KC_DEL" => "Del",
        "KC_TAB" => "Tab",
        "KC_SPACE" | "KC_SPC" => "Space",
        "KC_MINUS" | "KC_MINS" | "KC_KP_MINUS" => "-",
        "KC_EQUAL" | "KC_EQL" => "=",
        "KC_LBRC" | "KC_LEFT_BRACKET" => "[",
        "KC_RBRC" | "KC_RIGHT_BRACKET" => "]",
        "KC_BSLS" | "KC_BACKSLASH" => "\\",
        "KC_SCLN" | "KC_SEMICOLON" => ";",
        "KC_QUOTE" | "KC_QUOT" => "'",
        "KC_GRAVE" | "KC_GRV" => "`",
        "KC_COMMA" | "KC_COMM" => ",",
        "KC_DOT" | "KC_KP_DOT" => ".",
        "KC_SLASH" | "KC_SLSH" | "KC_KP_SLASH" => "/",
        "KC_EXLM" => "!",
        "KC_AT" => "@",
        "KC_HASH" => "#",
        "KC_DLR" => "$",
        "KC_PERC" => "%",
        "KC_CIRC" => "^",
        "KC_AMPR" => "&",
        "KC_ASTR" | "KC_KP_ASTERISK" => "*",
        "KC_LPRN" => "(",
        "KC_RPRN" => ")",
        "KC_UNDS" => "_",
        "KC_PLUS" | "KC_KP_PLUS" => "+",
        "KC_LCBR" => "{",
        "KC_RCBR" => "}",
        "KC_PIPE" => "|",
        "KC_COLN" => ":",
        "KC_DQUO" => "\"",
        "KC_TILD" => "~",
        "KC_LABK" => "<",
        "KC_RABK" => ">",
        "KC_QUES" => "?",
        "KC_LEFT" => "←",
        "KC_RIGHT" => "→",
        "KC_UP" => "↑",
        "KC_DOWN" => "↓",
        "KC_HOME" => "Home",
        "KC_END" => "End",
        "KC_PAGE_UP" | "KC_PGUP" => "PgUp",
        "KC_PAGE_DOWN" | "KC_PGDN" => "PgDn",
        "KC_INSERT" | "KC_INS" => "Ins",
        "KC_CAPS" | "KC_CAPS_LOCK" => "Caps",
        "KC_PSCR" | "KC_PRINT_SCREEN" => "PrtSc",
        "KC_SCRL" | "KC_SCROLL_LOCK" => "ScrLk",
        "KC_PAUSE" => "Pause",
        "KC_APPLICATION" | "KC_APP" => "Menu",
        "KC_LEFT_CTRL" | "KC_LCTL" | "KC_RIGHT_CTRL" | "KC_RCTL" => sym(style, "⌃", "Ctrl"),
        "KC_LEFT_ALT" | "KC_LALT" | "KC_RIGHT_ALT" | "KC_RALT" => sym(style, "⌥", "Alt"),
        "KC_LEFT_SHIFT" | "KC_LSFT" | "KC_RIGHT_SHIFT" | "KC_RSFT" => sym(style, "⇧", "Shift"),
        "KC_LEFT_GUI" | "KC_LGUI" | "KC_RIGHT_GUI" | "KC_RGUI" => sym(style, "⌘", "Super"),
        "KC_AUDIO_MUTE" => "Mute",
        "KC_AUDIO_VOL_UP" => "Vol+",
        "KC_AUDIO_VOL_DOWN" => "Vol−",
        "KC_MEDIA_PLAY_PAUSE" => "Play",
        "KC_MEDIA_NEXT_TRACK" => "Next",
        "KC_MEDIA_PREV_TRACK" => "Prev",
        "KC_MEDIA_STOP" => "Stop",
        "KC_BRIGHTNESS_UP" => "Bri+",
        "KC_BRIGHTNESS_DOWN" => "Bri−",
        "KC_MS_BTN1" => "Click",
        "KC_MS_BTN2" => "RClick",
        "KC_MS_BTN3" => "MClick",
        "KC_MS_UP" => "M↑",
        "KC_MS_DOWN" => "M↓",
        "KC_MS_LEFT" => "M←",
        "KC_MS_RIGHT" => "M→",
        "KC_MS_WH_UP" => "Wh↑",
        "KC_MS_WH_DOWN" => "Wh↓",
        "CW_TOGG" => "Caps W",
        "QK_LLCK" => "Lock",
        "QK_BOOT" => "Boot",
        "QK_REP" | "QK_REPEAT_KEY" => "Rep",
        _ => return None,
    })
}
