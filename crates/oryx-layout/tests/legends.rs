use oryx_layout::labels::{KeyLegend, LegendKind, ModifierStyle, code_label, key_legend};
use oryx_layout::{Action, Key, Modifiers};

fn act(code: &str) -> Option<Action> {
    Some(Action {
        code: code.into(),
        layer: None,
        modifiers: Modifiers::default(),
    })
}

fn to_layer(code: &str, layer: u8) -> Option<Action> {
    Some(Action {
        code: code.into(),
        layer: Some(layer),
        modifiers: Modifiers::default(),
    })
}

fn titles() -> Vec<String> {
    ["Main", "RO", "HU", "Navig"].map(String::from).to_vec()
}

fn legend(key: Key) -> KeyLegend {
    key_legend(&key, &titles(), ModifierStyle::Symbols)
}

#[test]
fn transparent_keys_show_a_marker() {
    assert_eq!(
        legend(Key::default()),
        KeyLegend {
            tap: "▽".into(),
            hold: None,
            kind: LegendKind::Transparent
        }
    );
}

#[test]
fn kc_no_is_blank() {
    assert_eq!(
        legend(Key {
            tap: act("KC_NO"),
            ..Key::default()
        })
        .kind,
        LegendKind::Blank
    );
}

#[test]
fn letters_digits_and_function_keys_use_their_name() {
    assert_eq!(code_label("KC_A", ModifierStyle::Symbols), "A");
    assert_eq!(code_label("KC_7", ModifierStyle::Symbols), "7");
    assert_eq!(code_label("KC_F11", ModifierStyle::Symbols), "F11");
    assert_eq!(code_label("KC_KP_SLASH", ModifierStyle::Symbols), "/");
}

#[test]
fn common_codes_missing_from_oryx_labels_are_covered() {
    for (code, label) in [
        ("KC_BSPC", "Bksp"),
        ("KC_SCLN", ";"),
        ("KC_LBRC", "["),
        ("KC_BSLS", "\\"),
        ("KC_PGDN", "PgDn"),
        ("KC_PAGE_UP", "PgUp"),
        ("KC_CAPS", "Caps"),
        ("CW_TOGG", "Caps W"),
        ("KC_LEFT", "←"),
        ("KC_AUDIO_VOL_UP", "Vol+"),
        ("KC_COLN", ":"),
        ("KC_SPACE", "Space"),
    ] {
        assert_eq!(code_label(code, ModifierStyle::Symbols), label, "{code}");
    }
}

#[test]
fn modifier_codes_follow_the_style() {
    assert_eq!(code_label("KC_LEFT_CTRL", ModifierStyle::Symbols), "⌃");
    assert_eq!(code_label("KC_RIGHT_GUI", ModifierStyle::Symbols), "⌘");
    assert_eq!(code_label("KC_LEFT_CTRL", ModifierStyle::Words), "Ctrl");
    assert_eq!(code_label("KC_RIGHT_GUI", ModifierStyle::Words), "Super");
}

#[test]
fn unknown_codes_fall_back_to_a_readable_name() {
    assert_eq!(
        code_label("NAVIGATOR_AIM", ModifierStyle::Symbols),
        "NAVIGATOR AIM"
    );
    assert_eq!(
        code_label("KC_WEIRD_THING", ModifierStyle::Symbols),
        "WEIRD THING"
    );
}

#[test]
fn layer_tap_shows_tap_and_target_layer_title() {
    let key = Key {
        tap: act("KC_SPACE"),
        hold: to_layer("MO", 3),
        ..Key::default()
    };
    assert_eq!(
        legend(key),
        KeyLegend {
            tap: "Space".into(),
            hold: Some("Navig".into()),
            kind: LegendKind::Normal
        }
    );
}

#[test]
fn mod_tap_shows_the_modifier_as_hold() {
    let key = Key {
        tap: act("KC_A"),
        hold: act("KC_LEFT_CTRL"),
        ..Key::default()
    };
    assert_eq!(legend(key).hold.as_deref(), Some("⌃"));
}

#[test]
fn layer_switch_keys_are_marked_and_named() {
    assert_eq!(
        legend(Key {
            tap: to_layer("TO", 2),
            ..Key::default()
        }),
        KeyLegend {
            tap: "→ HU".into(),
            hold: None,
            kind: LegendKind::Layer
        }
    );
    assert_eq!(
        legend(Key {
            tap: to_layer("TG", 1),
            ..Key::default()
        })
        .tap,
        "⇆ RO"
    );
    assert_eq!(
        legend(Key {
            tap: to_layer("OSL", 3),
            ..Key::default()
        })
        .tap,
        "1× Navig"
    );
    assert_eq!(
        legend(Key {
            tap: to_layer("MO", 3),
            ..Key::default()
        })
        .tap,
        "Navig"
    );
}

#[test]
fn a_layer_beyond_the_known_titles_falls_back_to_its_number() {
    assert_eq!(
        legend(Key {
            tap: to_layer("TO", 7),
            ..Key::default()
        })
        .tap,
        "→ L7"
    );
}

#[test]
fn custom_labels_win() {
    let mut tap = act("KC_S").unwrap();
    tap.modifiers.alt = true;
    let key = Key {
        tap: Some(tap),
        custom_label: Some("Ș".into()),
        ..Key::default()
    };
    assert_eq!(legend(key).tap, "Ș");
}

#[test]
fn wrapped_modifiers_prefix_the_label() {
    let mut tap = act("KC_F").unwrap();
    tap.modifiers.alt = true;
    tap.modifiers.shift = true;
    let key = Key {
        tap: Some(tap),
        ..Key::default()
    };
    assert_eq!(
        key_legend(&key, &titles(), ModifierStyle::Symbols).tap,
        "⌥⇧F"
    );
    assert_eq!(
        key_legend(&key, &titles(), ModifierStyle::Words).tap,
        "Alt+Shift+F"
    );
}
