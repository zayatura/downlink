#![cfg(target_os = "linux")]

use input_source::hyprland::names::layout_names;

#[test]
fn stock_layouts_get_their_xkb_names_in_order() {
    assert_eq!(
        layout_names("", "", "us,de", ",nodeadkeys"),
        ["English (US)", "German (no dead keys)"]
    );
}

#[test]
fn a_keymap_that_does_not_compile_gives_no_names() {
    assert!(layout_names("", "", "no-such-layout-xyz", "").is_empty());
}
