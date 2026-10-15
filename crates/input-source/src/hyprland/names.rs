//! Human-readable layout names, read from the compiled keymap (spec §6.2).

use xkbcommon::xkb;

/// Each layout's XKB name, e.g. "English (US)", in `layout` order. Options are left out: they
/// don't change names, and a custom option the system rules don't know would fail the compile.
/// libxkbcommon also searches `~/.config/xkb`, so custom layouts get their names too.
pub fn layout_names(rules: &str, model: &str, layout: &str, variant: &str) -> Vec<String> {
    let context = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
    let Some(keymap) = xkb::Keymap::new_from_names(
        &context,
        rules,
        model,
        layout,
        variant,
        None,
        xkb::KEYMAP_COMPILE_NO_FLAGS,
    ) else {
        return Vec::new();
    };
    (0..keymap.num_layouts())
        .map(|index| keymap.layout_get_name(index).to_owned())
        .collect()
}
