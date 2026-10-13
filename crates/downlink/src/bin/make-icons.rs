//! Writes the PNGs of a macOS .iconset: cargo run -p downlink --bin make-icons -- <dir>.iconset
use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;

const SIZES: [(&str, u32); 10] = [
    ("icon_16x16", 16),
    ("icon_16x16@2x", 32),
    ("icon_32x32", 32),
    ("icon_32x32@2x", 64),
    ("icon_128x128", 128),
    ("icon_128x128@2x", 256),
    ("icon_256x256", 256),
    ("icon_256x256@2x", 512),
    ("icon_512x512", 512),
    ("icon_512x512@2x", 1024),
];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = PathBuf::from(
        std::env::args()
            .nth(1)
            .ok_or("usage: make-icons <output .iconset directory>")?,
    );
    std::fs::create_dir_all(&dir)?;
    for (name, size) in SIZES {
        let rgba = downlink::icon::app_icon_rgba(size);
        let file = BufWriter::new(File::create(dir.join(format!("{name}.png")))?);
        let mut encoder = png::Encoder::new(file, size, size);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.write_header()?.write_image_data(&rgba)?;
    }
    Ok(())
}
