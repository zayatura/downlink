//! Rasterises Downlink's antenna glyph: a dot with two radio waves above it.

const SUBSAMPLES: u32 = 4;

/// The menu-bar glyph as straight RGBA: black with alpha (a macOS template image).
pub fn antenna_rgba(size: u32, connected: bool) -> Vec<u8> {
    let opacity = if connected { 1.0 } else { 0.35 };
    let mut rgba = vec![0u8; (size * size * 4) as usize];
    for y in 0..size {
        for x in 0..size {
            let coverage = coverage(x, y, size, inside_antenna);
            rgba[((y * size + x) * 4 + 3) as usize] = (coverage * opacity * 255.0).round() as u8;
        }
    }
    rgba
}

/// The application icon: an accent-coloured antenna on a dark rounded square.
pub fn app_icon_rgba(size: u32) -> Vec<u8> {
    let background = [0x1d as f32, 0x1f as f32, 0x24 as f32];
    let accent = [0x4c as f32, 0xc2 as f32, 0xff as f32];
    let mut rgba = vec![0u8; (size * size * 4) as usize];
    for y in 0..size {
        for x in 0..size {
            let shape = coverage(x, y, size, inside_rounded_square);
            let glyph = coverage(x, y, size, |u, v| {
                let (gu, gv) = ((u - 0.18) / 0.64, (v - 0.14) / 0.64);
                (0.0..=1.0).contains(&gu) && (0.0..=1.0).contains(&gv) && inside_antenna(gu, gv)
            });
            let i = ((y * size + x) * 4) as usize;
            for c in 0..3 {
                rgba[i + c] = (background[c] + (accent[c] - background[c]) * glyph).round() as u8;
            }
            rgba[i + 3] = (shape * 255.0).round() as u8;
        }
    }
    rgba
}

fn coverage(x: u32, y: u32, size: u32, inside: impl Fn(f32, f32) -> bool) -> f32 {
    let mut hits = 0;
    for sy in 0..SUBSAMPLES {
        for sx in 0..SUBSAMPLES {
            let u = (x as f32 + (sx as f32 + 0.5) / SUBSAMPLES as f32) / size as f32;
            let v = (y as f32 + (sy as f32 + 0.5) / SUBSAMPLES as f32) / size as f32;
            if inside(u, v) {
                hits += 1;
            }
        }
    }
    hits as f32 / (SUBSAMPLES * SUBSAMPLES) as f32
}

/// `u`, `v` in 0..1, origin top-left.
fn inside_antenna(u: f32, v: f32) -> bool {
    let (dx, dy) = (u - 0.5, v - 0.78);
    let distance = (dx * dx + dy * dy).sqrt();
    if distance <= 0.10 {
        return true;
    }
    // Waves only in the upward wedge within 50° of vertical.
    if dy >= 0.0 || dx.abs() > -dy * 50f32.to_radians().tan() {
        return false;
    }
    (distance - 0.30).abs() <= 0.04 || (distance - 0.52).abs() <= 0.04
}

fn inside_rounded_square(u: f32, v: f32) -> bool {
    const INSET: f32 = 0.06;
    const RADIUS: f32 = 0.2;
    if !(INSET..=1.0 - INSET).contains(&u) || !(INSET..=1.0 - INSET).contains(&v) {
        return false;
    }
    let cx = u.clamp(INSET + RADIUS, 1.0 - INSET - RADIUS);
    let cy = v.clamp(INSET + RADIUS, 1.0 - INSET - RADIUS);
    (u - cx).powi(2) + (v - cy).powi(2) <= RADIUS * RADIUS
}
