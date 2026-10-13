use downlink::icon::{antenna_rgba, app_icon_rgba};

fn alpha(rgba: &[u8], size: u32, x: u32, y: u32) -> u8 {
    rgba[((y * size + x) * 4 + 3) as usize]
}

#[test]
fn antenna_has_the_right_size_and_a_solid_dot() {
    let icon = antenna_rgba(36, true);
    assert_eq!(icon.len(), 36 * 36 * 4);
    assert_eq!(alpha(&icon, 36, 18, 28), 255, "dot centre");
    assert_eq!(alpha(&icon, 36, 0, 0), 0, "corner is transparent");
    assert!(alpha(&icon, 36, 18, 17) > 200, "inner wave");
}

#[test]
fn disconnected_antenna_is_dimmed() {
    let icon = antenna_rgba(36, false);
    let dot = alpha(&icon, 36, 18, 28);
    assert!(dot > 60 && dot < 120, "dimmed dot alpha was {dot}");
}

#[test]
fn template_icon_is_black_with_alpha_only() {
    let icon = antenna_rgba(36, true);
    assert!(
        icon.chunks(4)
            .all(|px| px[0] == 0 && px[1] == 0 && px[2] == 0)
    );
}

#[test]
fn app_icon_is_opaque_in_the_middle_and_clear_in_the_corner() {
    let icon = app_icon_rgba(64);
    assert_eq!(icon.len(), 64 * 64 * 4);
    assert_eq!(alpha(&icon, 64, 0, 0), 0);
    assert_eq!(alpha(&icon, 64, 32, 32), 255);
}
