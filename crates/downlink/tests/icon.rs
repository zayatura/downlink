use downlink::icon::{app_icon_rgba, menubar_argb, menubar_rgba};

fn px(rgba: &[u8], size: u32, x: u32, y: u32) -> [u8; 4] {
    let i = ((y * size + x) * 4) as usize;
    [rgba[i], rgba[i + 1], rgba[i + 2], rgba[i + 3]]
}

#[test]
fn menubar_icon_draws_the_keycap_and_its_arrow() {
    let icon = menubar_rgba(36, true);
    assert_eq!(icon.len(), 36 * 36 * 4);
    assert_eq!(px(&icon, 36, 0, 0)[3], 0, "corner is transparent");
    assert_eq!(px(&icon, 36, 5, 18)[3], 255, "keycap rim");
    assert_eq!(px(&icon, 36, 10, 12)[3], 0, "keycap face (cut out)");
    assert_eq!(px(&icon, 36, 18, 13)[3], 255, "arrow shaft");
}

#[test]
fn disconnected_menubar_icon_is_dimmed() {
    let alpha = px(&menubar_rgba(36, false), 36, 18, 13)[3];
    assert!(alpha > 60 && alpha < 120, "dimmed alpha was {alpha}");
}

#[test]
fn menubar_icon_is_a_black_template_image() {
    assert!(
        menubar_rgba(36, true)
            .chunks(4)
            .all(|p| p[0] == 0 && p[1] == 0 && p[2] == 0)
    );
}

#[test]
fn menubar_icon_scales_to_any_size() {
    let icon = menubar_rgba(18, true);
    assert_eq!(icon.len(), 18 * 18 * 4);
    assert!(icon.chunks(4).any(|p| p[3] == 255));
}

#[test]
fn app_icon_is_the_accent_glyph_on_a_dark_square() {
    let icon = app_icon_rgba(64);
    assert_eq!(icon.len(), 64 * 64 * 4);
    assert_eq!(
        px(&icon, 64, 0, 0)[3],
        0,
        "corner outside the rounded square"
    );
    let [r, g, b, a] = px(&icon, 64, 32, 32);
    assert_eq!(a, 255);
    assert!(
        b > 200 && b > r,
        "centre is the accent-blue arrow, got {r},{g},{b}"
    );
    let [r, g, b, a] = px(&icon, 64, 11, 11);
    assert_eq!(a, 255);
    assert!(
        r < 0x40 && g < 0x40 && b < 0x40,
        "background is dark, got {r},{g},{b}"
    );
}

#[test]
fn tray_pixmap_is_argb_in_the_requested_colour() {
    let rgba = menubar_rgba(22, true);
    let argb = menubar_argb(22, true, [0xe8, 0xea, 0xee]);
    assert_eq!(argb.len(), rgba.len());
    for (a, r) in argb.chunks(4).zip(rgba.chunks(4)) {
        assert_eq!(a[0], r[3], "alpha comes first, unchanged");
        assert_eq!(
            &a[1..],
            &[0xe8, 0xea, 0xee],
            "every pixel has the glyph colour"
        );
    }
}

#[test]
fn disconnected_tray_pixmap_is_dimmed() {
    let argb = menubar_argb(36, false, [0xff, 0xff, 0xff]);
    let alpha = argb[((13 * 36 + 18) * 4) as usize];
    assert!(alpha > 60 && alpha < 120, "dimmed alpha was {alpha}");
}
