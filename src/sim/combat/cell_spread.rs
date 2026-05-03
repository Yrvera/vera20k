//! CellSpread offset table recovered from gamemd's AoE CellOffsetsDxDyTable.
//!
//! Used by combat to iterate cells affected by a warhead detonation for
//! ore destruction (and future wall/bridge/radiation effects).
//!
//! Source: `RA2-GAME.EXE-IDB/evidence/derived/aoe_dxdy_yr_1001_static_recovered_20260503.tsv`.

/// CellSpreadTable counts per integer radius, matching gamemd (0x007ed3d0).
/// Index = integer CellSpread radius, value = total cells to process.
const CELL_SPREAD_COUNTS: [usize; 12] = [1, 9, 21, 37, 61, 89, 121, 161, 205, 253, 309, 369];

/// Maximum supported CellSpread radius (index into CELL_SPREAD_COUNTS).
const MAX_SPREAD_RADIUS: usize = 11;

/// Exact AoE offset order for radius indices 0-11.
#[rustfmt::skip]
const SPREAD_OFFSETS: [(i16, i16); 369] = [
    (0, 0), (1, -1), (0, -1), (-1, -1), (-1, 0), (1, 0),
    (-1, 1), (0, 1), (1, 1), (-1, -2), (0, -2), (1, -2),
    (-2, -1), (2, -1), (-2, 0), (2, 0), (-2, 1), (2, 1),
    (-1, 2), (0, 2), (1, 2), (-1, -3), (0, -3), (1, -3),
    (-2, -2), (2, -2), (-3, -1), (3, -1), (-3, 0), (3, 0),
    (-3, 1), (3, 1), (-2, 2), (2, 2), (-1, 3), (0, 3),
    (1, 3), (-1, -4), (0, -4), (1, -4), (-3, -3), (-2, -3),
    (2, -3), (3, -3), (-3, -2), (3, -2), (-4, -1), (4, -1),
    (-4, 0), (4, 0), (-4, 1), (4, 1), (-3, 2), (3, 2),
    (-3, 3), (-2, 3), (2, 3), (3, 3), (-1, 4), (0, 4),
    (1, 4), (-1, -5), (0, -5), (1, -5), (-3, -4), (-2, -4),
    (2, -4), (3, -4), (-4, -3), (4, -3), (-4, -2), (4, -2),
    (-5, -1), (5, -1), (-5, 0), (5, 0), (-5, 1), (5, 1),
    (-4, 2), (4, 2), (-4, 3), (4, 3), (-3, 4), (-2, 4),
    (2, 4), (3, 4), (-1, 5), (0, 5), (1, 5), (-1, -6),
    (0, -6), (1, -6), (-3, -5), (-2, -5), (2, -5), (3, -5),
    (-4, -4), (4, -4), (-5, -3), (5, -3), (-5, -2), (5, -2),
    (-6, -1), (6, -1), (-6, 0), (6, 0), (-6, 1), (6, 1),
    (-5, 2), (5, 2), (-5, 3), (5, 3), (-4, 4), (4, 4),
    (-3, 5), (-2, 5), (2, 5), (3, 5), (-1, 6), (0, 6),
    (1, 6), (-1, -7), (0, -7), (1, -7), (-3, -6), (-2, -6),
    (2, -6), (3, -6), (-5, -5), (-4, -5), (4, -5), (5, -5),
    (-5, -4), (5, -4), (-6, -3), (6, -3), (-6, -2), (6, -2),
    (-7, -1), (7, -1), (-7, 0), (7, 0), (-7, 1), (7, 1),
    (-6, 2), (6, 2), (-6, 3), (6, 3), (-5, 4), (5, 4),
    (-5, 5), (-4, 5), (4, 5), (5, 5), (-3, 6), (-2, 6),
    (2, 6), (3, 6), (-1, 7), (0, 7), (1, 7), (-1, -8),
    (0, -8), (1, -8), (-3, -7), (-2, -7), (2, -7), (3, -7),
    (-5, -6), (-4, -6), (4, -6), (5, -6), (-6, -5), (6, -5),
    (-6, -4), (6, -4), (-7, -3), (7, -3), (-7, -2), (7, -2),
    (-8, -1), (8, -1), (-8, 0), (8, 0), (-8, 1), (8, 1),
    (-7, 2), (7, 2), (-7, 3), (7, 3), (-6, 4), (6, 4),
    (-6, 5), (6, 5), (-5, 6), (-4, 6), (4, 6), (5, 6),
    (-3, 7), (-2, 7), (2, 7), (3, 7), (-1, 8), (0, 8),
    (1, 8), (-1, -9), (0, -9), (1, -9), (-3, -8), (-2, -8),
    (2, -8), (3, -8), (-5, -7), (-4, -7), (4, -7), (5, -7),
    (-6, -6), (6, -6), (-7, -5), (7, -5), (-7, -4), (7, -4),
    (-8, -3), (8, -3), (-8, -2), (8, -2), (-9, -1), (9, -1),
    (-9, 0), (9, 0), (-9, 1), (9, 1), (-8, 2), (8, 2),
    (-8, 3), (8, 3), (-7, 4), (7, 4), (-7, 5), (7, 5),
    (-6, 6), (6, 6), (-5, 7), (-4, 7), (4, 7), (5, 7),
    (-3, 8), (-2, 8), (2, 8), (3, 8), (-1, 9), (0, 9),
    (1, 9), (-1, -10), (0, -10), (1, -10), (-3, -9), (-2, -9),
    (2, -9), (3, -9), (-5, -8), (-4, -8), (4, -8), (5, -8),
    (-7, -7), (-6, -7), (6, -7), (7, -7), (-7, -6), (7, -6),
    (-8, -5), (8, -5), (-8, -4), (8, -4), (-9, -3), (9, -3),
    (-9, -2), (9, -2), (-10, -1), (10, -1), (-10, 0), (10, 0),
    (-10, 1), (10, 1), (-9, 2), (9, 2), (-9, 3), (9, 3),
    (-8, 4), (8, 4), (-8, 5), (8, 5), (-7, 6), (7, 6),
    (-7, 7), (-6, 7), (6, 7), (7, 7), (-5, 8), (-4, 8),
    (4, 8), (5, 8), (-3, 9), (-2, 9), (2, 9), (3, 9),
    (-1, 10), (0, 10), (1, 10), (0, 11), (0, -11), (-1, 11),
    (1, 11), (-1, -11), (1, -11), (-2, 11), (2, 11), (-2, -11),
    (2, -11), (-3, 11), (3, 11), (-3, -11), (-3, 11), (-4, 9),
    (4, 9), (-4, -9), (4, -9), (-5, 9), (5, 9), (-5, -9),
    (5, -9), (-6, 8), (6, 8), (-6, -8), (6, -8), (-7, 8),
    (7, 8), (-7, -8), (7, -8), (-8, 7), (8, 7), (-8, -7),
    (8, -7), (-8, 6), (8, 6), (-8, -6), (8, -6), (-9, 5),
    (9, 5), (-9, -5), (9, -5), (-9, 4), (9, 4), (-9, -4),
    (9, -4), (-10, 3), (10, 3), (-10, -3), (10, -3), (-10, 2),
    (10, 2), (-10, -2), (10, -2), (-11, 1), (11, 1), (-11, -1),
    (11, -1), (11, 0), (-11, 0),
];

/// Returns cell offsets for a given integer spread radius (0–11).
///
/// Index 0 is always `(0, 0)` — the center cell. Radii beyond 11 are
/// clamped to 11. Counts match gamemd's CellSpreadTable:
/// `[1, 9, 21, 37, 61, 89, 121, 161, 205, 253, 309, 369]`.
pub fn cells_in_spread(radius: u32) -> &'static [(i16, i16)] {
    let r = (radius as usize).min(MAX_SPREAD_RADIUS);
    let count = CELL_SPREAD_COUNTS[r];
    &SPREAD_OFFSETS[..count]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_match_gamemd_cell_spread_table() {
        let expected = [1, 9, 21, 37, 61, 89, 121, 161, 205, 253, 309, 369];
        for (radius, &expected_count) in expected.iter().enumerate() {
            let offsets = cells_in_spread(radius as u32);
            assert_eq!(
                offsets.len(),
                expected_count,
                "radius {radius}: expected {expected_count} cells, got {}",
                offsets.len()
            );
        }
    }

    #[test]
    fn center_cell_always_first() {
        for radius in 0..=11u32 {
            let offsets = cells_in_spread(radius);
            assert_eq!(
                offsets[0],
                (0, 0),
                "radius {radius}: first offset must be (0,0)"
            );
        }
    }

    #[test]
    fn radius_11_preserves_recovered_order_anomaly() {
        let offsets = cells_in_spread(11);
        assert_eq!(offsets[319], (-3, 11));
        assert_eq!(offsets[320], (3, 11));
        assert_eq!(offsets[321], (-3, -11));
        assert_eq!(offsets[322], (-3, 11));
        assert!(!offsets.contains(&(3, -11)));
    }

    #[test]
    fn radius_beyond_max_clamped() {
        let r11 = cells_in_spread(11);
        let r99 = cells_in_spread(99);
        assert_eq!(r11.len(), r99.len());
    }

    #[test]
    fn radius_zero_is_center_only() {
        let offsets = cells_in_spread(0);
        assert_eq!(offsets.len(), 1);
        assert_eq!(offsets[0], (0, 0));
    }
}
