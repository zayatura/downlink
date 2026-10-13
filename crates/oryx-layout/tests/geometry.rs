use std::collections::HashSet;

use oryx_layout::geometry::{self, VOYAGER};
use oryx_protocol::{MatrixPos, Model};

fn pos(row: u8, col: u8) -> MatrixPos {
    MatrixPos { row, col }
}

#[test]
fn voyager_has_52_unique_matrix_positions() {
    assert_eq!(VOYAGER.len(), 52);
    let unique: HashSet<MatrixPos> = VOYAGER.iter().map(|k| k.matrix).collect();
    assert_eq!(unique.len(), 52);
}

#[test]
fn every_key_round_trips_through_oryx_index() {
    for (i, key) in VOYAGER.iter().enumerate() {
        assert_eq!(geometry::oryx_index(Model::Voyager, key.matrix), Some(i));
    }
}

#[test]
fn oryx_order_is_left_half_then_right_half() {
    // Left half: top row starts at matrix (0,1), at the left edge.
    assert_eq!(
        (VOYAGER[0].matrix, VOYAGER[0].x, VOYAGER[0].y),
        (pos(0, 1), 0.0, 0.5)
    );
    // Index 23 is the left half's bottom-right key, matrix (4,4).
    assert_eq!(
        (VOYAGER[23].matrix, VOYAGER[23].x, VOYAGER[23].y),
        (pos(4, 4), 5.0, 3.5)
    );
    // Left thumbs.
    assert_eq!(
        (VOYAGER[24].matrix, VOYAGER[24].x, VOYAGER[24].y),
        (pos(5, 0), 5.0, 4.5)
    );
    assert_eq!(
        (VOYAGER[25].matrix, VOYAGER[25].x, VOYAGER[25].y),
        (pos(5, 1), 6.0, 4.75)
    );
    // Right half starts at matrix (6,0), x = 10.
    assert_eq!(
        (VOYAGER[26].matrix, VOYAGER[26].x, VOYAGER[26].y),
        (pos(6, 0), 10.0, 0.5)
    );
    // Right half's bottom-left key is matrix (10,2).
    assert_eq!(
        (VOYAGER[44].matrix, VOYAGER[44].x, VOYAGER[44].y),
        (pos(10, 2), 10.0, 3.5)
    );
    // Right thumbs.
    assert_eq!(
        (VOYAGER[50].matrix, VOYAGER[50].x, VOYAGER[50].y),
        (pos(11, 5), 9.0, 4.75)
    );
    assert_eq!(
        (VOYAGER[51].matrix, VOYAGER[51].x, VOYAGER[51].y),
        (pos(11, 6), 10.0, 4.5)
    );
}

#[test]
fn positions_not_on_the_board_have_no_index() {
    for p in [pos(0, 0), pos(4, 0), pos(5, 2), pos(13, 0), pos(255, 255)] {
        assert_eq!(geometry::oryx_index(Model::Voyager, p), None, "{p:?}");
    }
}

#[test]
fn every_key_fits_on_the_board() {
    let (width, height) = geometry::board_size(Model::Voyager);
    assert_eq!((width, height), (16.0, 5.75));
    for key in geometry::keys(Model::Voyager) {
        assert!(key.x >= 0.0 && key.x + 1.0 <= width);
        assert!(key.y >= 0.0 && key.y + 1.0 <= height);
    }
}
