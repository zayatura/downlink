//! Physical key positions, in Oryx key order.
//!
//! The numbers are physical facts about the ZSA Voyager: matrix positions and
//! key coordinates in key units, as listed in QMK's
//! `keyboards/zsa/voyager/keyboard.json`. Only the numeric data is used here.
//! Oryx numbers keys differently from QMK's `LAYOUT` macro: the left half
//! first (four rows of six, then two thumb keys), then the right half the same way.

use oryx_protocol::{MatrixPos, Model};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KeyGeom {
    /// Left edge, in key units.
    pub x: f32,
    /// Top edge, in key units.
    pub y: f32,
    pub matrix: MatrixPos,
}

const fn k(row: u8, col: u8, x: f32, y: f32) -> KeyGeom {
    KeyGeom {
        x,
        y,
        matrix: MatrixPos { row, col },
    }
}

#[rustfmt::skip]
pub static VOYAGER: [KeyGeom; 52] = [
    // Left half, rows 0–3.
    k(0, 1, 0.0, 0.5), k(0, 2, 1.0, 0.5), k(0, 3, 2.0, 0.25), k(0, 4, 3.0, 0.0), k(0, 5, 4.0, 0.25), k(0, 6, 5.0, 0.5),
    k(1, 1, 0.0, 1.5), k(1, 2, 1.0, 1.5), k(1, 3, 2.0, 1.25), k(1, 4, 3.0, 1.0), k(1, 5, 4.0, 1.25), k(1, 6, 5.0, 1.5),
    k(2, 1, 0.0, 2.5), k(2, 2, 1.0, 2.5), k(2, 3, 2.0, 2.25), k(2, 4, 3.0, 2.0), k(2, 5, 4.0, 2.25), k(2, 6, 5.0, 2.5),
    k(3, 1, 0.0, 3.5), k(3, 2, 1.0, 3.5), k(3, 3, 2.0, 3.25), k(3, 4, 3.0, 3.0), k(3, 5, 4.0, 3.25), k(4, 4, 5.0, 3.5),
    // Left thumbs.
    k(5, 0, 5.0, 4.5), k(5, 1, 6.0, 4.75),
    // Right half, rows 0–3.
    k(6, 0, 10.0, 0.5), k(6, 1, 11.0, 0.25), k(6, 2, 12.0, 0.0), k(6, 3, 13.0, 0.25), k(6, 4, 14.0, 0.5), k(6, 5, 15.0, 0.5),
    k(7, 0, 10.0, 1.5), k(7, 1, 11.0, 1.25), k(7, 2, 12.0, 1.0), k(7, 3, 13.0, 1.25), k(7, 4, 14.0, 1.5), k(7, 5, 15.0, 1.5),
    k(8, 0, 10.0, 2.5), k(8, 1, 11.0, 2.25), k(8, 2, 12.0, 2.0), k(8, 3, 13.0, 2.25), k(8, 4, 14.0, 2.5), k(8, 5, 15.0, 2.5),
    k(10, 2, 10.0, 3.5), k(9, 1, 11.0, 3.25), k(9, 2, 12.0, 3.0), k(9, 3, 13.0, 3.25), k(9, 4, 14.0, 3.5), k(9, 5, 15.0, 3.5),
    // Right thumbs.
    k(11, 5, 9.0, 4.75), k(11, 6, 10.0, 4.5),
];

pub fn keys(model: Model) -> &'static [KeyGeom] {
    match model {
        Model::Voyager => &VOYAGER,
    }
}

/// Board width and height in key units.
pub fn board_size(model: Model) -> (f32, f32) {
    match model {
        Model::Voyager => (16.0, 5.75),
    }
}

/// The Oryx key index for a matrix position, or `None` if no key sits there.
pub fn oryx_index(model: Model, pos: MatrixPos) -> Option<usize> {
    keys(model).iter().position(|key| key.matrix == pos)
}
