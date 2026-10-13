//! Light and dark palettes (from the approved mockups) and system-mode resolution.

use companion_core::Appearance;
use iced::theme::{Mode, Palette};
use iced::{Color, Theme};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Colors {
    pub background: Color,
    pub panel: Color,
    pub key: Color,
    pub key_text: Color,
    pub sub_text: Color,
    pub accent: Color,
    pub on_accent: Color,
    pub press: Color,
    pub on_press: Color,
    pub border: Color,
    pub success: Color,
    pub danger: Color,
}

fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgb8(r, g, b)
}

pub fn dark() -> Colors {
    Colors {
        background: rgb(0x1d, 0x1f, 0x24),
        panel: rgb(0x25, 0x28, 0x2f),
        key: rgb(0x30, 0x34, 0x3c),
        key_text: rgb(0xe8, 0xea, 0xee),
        sub_text: rgb(0x9a, 0xa1, 0xad),
        accent: rgb(0x4c, 0xc2, 0xff),
        on_accent: rgb(0xff, 0xff, 0xff),
        press: rgb(0xff, 0xb5, 0x47),
        on_press: rgb(0x1d, 0x1f, 0x24),
        border: rgb(0x3a, 0x3f, 0x48),
        success: rgb(0x30, 0xd1, 0x58),
        danger: rgb(0xff, 0x45, 0x3a),
    }
}

pub fn light() -> Colors {
    Colors {
        background: rgb(0xf4, 0xf5, 0xf7),
        panel: rgb(0xff, 0xff, 0xff),
        key: rgb(0xe9, 0xeb, 0xef),
        key_text: rgb(0x1d, 0x20, 0x25),
        sub_text: rgb(0x6b, 0x72, 0x80),
        accent: rgb(0x0a, 0x84, 0xff),
        on_accent: rgb(0xff, 0xff, 0xff),
        press: rgb(0xff, 0x9f, 0x0a),
        on_press: rgb(0x1d, 0x1f, 0x24),
        border: rgb(0xd6, 0xd9, 0xdf),
        success: rgb(0x24, 0x8a, 0x3d),
        danger: rgb(0xd7, 0x00, 0x15),
    }
}

pub fn colors(dark_mode: bool) -> Colors {
    if dark_mode { dark() } else { light() }
}

/// `System` follows the OS; an unknown OS mode counts as light.
pub fn is_dark(appearance: Appearance, system: Mode) -> bool {
    match appearance {
        Appearance::Dark => true,
        Appearance::Light => false,
        Appearance::System => system == Mode::Dark,
    }
}

pub fn iced_theme(dark_mode: bool) -> Theme {
    let c = colors(dark_mode);
    Theme::custom(
        if dark_mode {
            "Downlink Dark"
        } else {
            "Downlink Light"
        },
        Palette {
            background: c.background,
            text: c.key_text,
            primary: c.accent,
            success: c.success,
            warning: c.press,
            danger: c.danger,
        },
    )
}
