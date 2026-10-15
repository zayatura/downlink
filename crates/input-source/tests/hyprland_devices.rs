#![cfg(target_os = "linux")]

use input_source::hyprland::devices::{
    Keyboard, batch, check_batch_reply, current_id, layout_ids, parse_keyboards, reading_keyboard,
    sources, switch_commands,
};
use input_source::{InputSource, InputSourceError};

const LAYOUT: &str = "zd,hu,ro,us";
const VARIANT: &str = ",102_qwerty_dot_nodead,std,";

fn kb(name: &str, index: usize) -> Keyboard {
    Keyboard {
        name: name.into(),
        rules: String::new(),
        model: String::new(),
        layout: LAYOUT.into(),
        variant: VARIANT.into(),
        options: "compose:ralt".into(),
        active_layout_index: index,
        main: false,
    }
}

fn seat(voyager_index: usize) -> Vec<Keyboard> {
    let mut virtual_kb = kb("hl-virtual-keyboard-fcitx5", 0);
    virtual_kb.main = true;
    vec![
        kb("power-button", 0),
        kb("video-bus", 0),
        kb("at-translated-set-2-keyboard", 0),
        kb("zsa-technology-labs-voyager", voyager_index),
        virtual_kb,
    ]
}

#[test]
fn parses_hyprlands_reply_ignoring_unknown_fields() {
    let json = r#"{"mice": [], "keyboards": [{
        "address": "0x55a8fc65b1a0", "name": "hl-virtual-keyboard-fcitx5",
        "rules": "", "model": "", "layout": "zd,hu,ro,us",
        "variant": ",102_qwerty_dot_nodead,std,", "options": "compose:ralt",
        "active_layout_index": 3, "active_keymap": "English (US)",
        "capsLock": false, "numLock": true, "main": true
    }], "tablets": [], "touch": [], "switches": []}"#;
    let keyboards = parse_keyboards(json).unwrap();
    assert_eq!(keyboards.len(), 1);
    assert_eq!(keyboards[0].name, "hl-virtual-keyboard-fcitx5");
    assert_eq!(keyboards[0].active_layout_index, 3);
    assert!(keyboards[0].main);
}

#[test]
fn parses_the_captured_reply() {
    let json = include_str!("fixtures/hyprland-devices.json");
    let keyboards = parse_keyboards(json).unwrap();
    assert!(keyboards.iter().any(|k| k.is_typed()));
    assert!(current_id(&keyboards).is_ok());
}

#[test]
fn a_garbled_reply_is_a_platform_error() {
    assert!(matches!(
        parse_keyboards("unknown request"),
        Err(InputSourceError::Platform(_))
    ));
}

#[test]
fn layout_ids_pair_layouts_with_variants_by_position() {
    assert_eq!(
        layout_ids(LAYOUT, VARIANT),
        ["zd", "hu(102_qwerty_dot_nodead)", "ro(std)", "us"]
    );
}

#[test]
fn a_short_or_empty_variant_list_leaves_the_rest_plain() {
    assert_eq!(layout_ids(LAYOUT, ""), ["zd", "hu", "ro", "us"]);
    assert_eq!(
        layout_ids("us,de", ",nodeadkeys,extra"),
        ["us", "de(nodeadkeys)"]
    );
    assert_eq!(layout_ids(" us , de ", ""), ["us", "de"]);
    assert!(layout_ids("", "").is_empty());
}

#[test]
fn untyped_keyboards_are_recognised_by_name() {
    for name in [
        "hl-virtual-keyboard-fcitx5",
        "power-button",
        "sleep-button",
        "lid-switch",
        "video-bus",
    ] {
        assert!(!kb(name, 0).is_typed(), "{name}");
    }
    assert!(kb("zsa-technology-labs-voyager", 0).is_typed());
}

#[test]
fn the_furthest_typed_keyboard_is_read() {
    assert_eq!(current_id(&seat(3)).unwrap(), "us");
    assert_eq!(
        reading_keyboard(&seat(3)).unwrap().name,
        "zsa-technology-labs-voyager"
    );
}

#[test]
fn untyped_keyboards_are_not_read_while_a_typed_one_exists() {
    let mut keyboards = seat(0);
    keyboards[0].active_layout_index = 3; // power-button drifted
    assert_eq!(current_id(&keyboards).unwrap(), "zd");
}

#[test]
fn without_typed_keyboards_the_main_one_is_read() {
    let mut virtual_kb = kb("hl-virtual-keyboard-fcitx5", 2);
    virtual_kb.main = true;
    let keyboards = vec![kb("power-button", 0), virtual_kb];
    assert_eq!(current_id(&keyboards).unwrap(), "ro(std)");
}

#[test]
fn no_keyboards_is_a_platform_error() {
    assert_eq!(
        current_id(&[]),
        Err(InputSourceError::Platform("no keyboards".into()))
    );
}

#[test]
fn an_out_of_range_index_is_a_platform_error() {
    let keyboards = vec![kb("at-translated-set-2-keyboard", 9)];
    assert!(matches!(
        current_id(&keyboards),
        Err(InputSourceError::Platform(_))
    ));
}

#[test]
fn switching_covers_every_keyboard_with_the_same_list_untyped_included() {
    assert_eq!(
        switch_commands(&seat(0), "us").unwrap(),
        [
            "switchxkblayout power-button 3",
            "switchxkblayout video-bus 3",
            "switchxkblayout at-translated-set-2-keyboard 3",
            "switchxkblayout zsa-technology-labs-voyager 3",
            "switchxkblayout hl-virtual-keyboard-fcitx5 3",
        ]
    );
}

#[test]
fn keyboards_with_a_different_list_are_left_alone() {
    let mut keyboards = seat(0);
    keyboards[2].layout = "zd,hu,ro".into();
    let commands = switch_commands(&keyboards, "zd").unwrap();
    assert!(
        !commands
            .iter()
            .any(|c| c.contains("at-translated-set-2-keyboard"))
    );
    assert_eq!(commands.len(), 4);
}

#[test]
fn an_unknown_id_is_not_found() {
    assert_eq!(
        switch_commands(&seat(0), "de"),
        Err(InputSourceError::NotFound("de".into()))
    );
}

#[test]
fn commands_are_sent_as_one_batch() {
    let commands = [
        "switchxkblayout a 3".to_owned(),
        "switchxkblayout b 3".to_owned(),
    ];
    assert_eq!(
        batch(&commands),
        "[[BATCH]]switchxkblayout a 3 ; switchxkblayout b 3"
    );
}

#[test]
fn a_batch_succeeds_only_when_every_command_says_ok() {
    assert_eq!(check_batch_reply("ok\n\n\nok", 2), Ok(()));
    assert_eq!(check_batch_reply("ok", 1), Ok(()));
    let failed = check_batch_reply("ok\n\n\ndevice not found", 2).unwrap_err();
    assert!(
        matches!(&failed, InputSourceError::Platform(m) if m.contains("device not found")),
        "{failed:?}"
    );
    assert!(check_batch_reply("ok", 2).is_err(), "a missing reply");
    assert!(check_batch_reply("", 1).is_err(), "no reply at all");
}

#[test]
fn sources_are_named_by_index_deduplicated_and_sorted() {
    let mut reader = kb("at-translated-set-2-keyboard", 0);
    reader.layout = "zd,us,us,hu".into();
    reader.variant = String::new();
    let names = [
        "ZD (US with HU/RO alt keys)".to_owned(),
        "English (US)".to_owned(),
    ];
    assert_eq!(
        sources(&reader, &names),
        [
            InputSource {
                id: "us".into(),
                name: "English (US)".into()
            },
            InputSource {
                id: "hu".into(),
                name: "hu".into()
            },
            InputSource {
                id: "zd".into(),
                name: "ZD (US with HU/RO alt keys)".into()
            },
        ]
    );
}

#[test]
fn the_captured_stale_virtual_keyboard_is_left_out_of_a_switch() {
    // Captured right after a config reload (Phase 0): fcitx5's virtual keyboard still held the
    // old three-layout list, and the eleven other keyboards the new four-layout one.
    let keyboards = parse_keyboards(include_str!("fixtures/hyprland-devices.json")).unwrap();
    let commands = switch_commands(&keyboards, "us").unwrap();
    assert_eq!(commands.len(), 11);
    assert!(commands.iter().all(|c| c.ends_with(" 3")));
    assert!(!commands.iter().any(|c| c.contains("hl-virtual-keyboard")));
}
