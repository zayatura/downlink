//! Downlink's icons, rasterised from the SVG sources in `assets/`: one drawing for every size.

use resvg::tiny_skia::{Pixmap, Transform};
use resvg::usvg::{Options, Tree};

const MENUBAR_SVG: &str = include_str!("../assets/menubar-icon.svg");
const APP_SVG: &str = include_str!("../assets/app-icon.svg");
const DISCONNECTED_OPACITY: f32 = 0.35;

/// The menu-bar glyph as straight RGBA: black with alpha (a macOS template image).
pub fn menubar_rgba(size: u32, connected: bool) -> Vec<u8> {
    let mut rgba = render(MENUBAR_SVG, size);
    if !connected {
        for pixel in rgba.as_chunks_mut::<4>().0 {
            pixel[3] = (f32::from(pixel[3]) * DISCONNECTED_OPACITY).round() as u8;
        }
    }
    rgba
}

/// The application icon as straight RGBA.
pub fn app_icon_rgba(size: u32) -> Vec<u8> {
    render(APP_SVG, size)
}

fn render(svg: &str, size: u32) -> Vec<u8> {
    let tree = Tree::from_str(svg, &Options::default()).expect("the bundled SVG is valid");
    let mut pixmap = Pixmap::new(size, size).expect("icon size is non-zero");
    let scale = size as f32 / tree.size().width();
    resvg::render(
        &tree,
        Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    pixmap
        .pixels()
        .iter()
        .flat_map(|p| {
            let c = p.demultiply();
            [c.red(), c.green(), c.blue(), c.alpha()]
        })
        .collect()
}
