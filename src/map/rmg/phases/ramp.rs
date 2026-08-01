//! Substrate for carving a cliff stair: the per-shape tile tables and the
//! precheck every carve runs before it touches a cell.
//!
//! The carve routines themselves live in `carve`. What is here is the part
//! they all share, and the part that decides whether a carve happens at all:
//!
//! - [`RampRecord`] — the level-step and slope tables for the four straight
//!   ramp shapes, read out of the executable.
//! - [`rect_is_carveable`] — the go/no-go test. Its verdict is what makes the
//!   caller's hundred-attempt retry loop affordable, because a refusal costs
//!   nothing: no cell written, no random draw taken.
//!
//! Reached through `carve_driver`, which runs on the two island map types.

use crate::map::rmg::grid::RmgGrid;
use crate::map::rmg::preview::Playfield;
use crate::map::rmg::scratch::RmgScratch;
use crate::map::rmg::tiles::TileIds;

use super::connector::RampShape;

/// Cells along a ramp's stepped edge. The tables are all this wide.
pub const RAMP_STEPS: usize = 5;

/// The tile and level tables for one ramp shape.
///
/// `level_steps` is added to the region's own level and then dropped by 4, so
/// `[3, 2, 1, 0, 0]` walks a cell down one terrain step at a time and then
/// holds — the stair itself.
///
/// The two slope rows are variants chosen by the sign of the run between the
/// carve's endpoints, not by any random draw. A slope value `v` writes `v` to
/// the cell's slope byte and picks the tile `ramp_base + v - 1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RampRecord {
    pub level_steps: [i8; RAMP_STEPS],
    pub slope_descending: [u8; RAMP_STEPS],
    pub slope_ascending: [u8; RAMP_STEPS],
}

impl RampRecord {
    /// The slope row for a run of `delta`, which is the endpoint difference
    /// along the shape's own axis. Positive picks the ascending row.
    pub fn slopes(&self, delta: i32) -> &[u8; RAMP_STEPS] {
        if delta > 0 {
            &self.slope_ascending
        } else {
            &self.slope_descending
        }
    }
}

/// Every straight shape's level table is the same descending stair.
const LEVEL_STAIR: [i8; RAMP_STEPS] = [3, 2, 1, 0, 0];

/// Tables for the four straight ramp shapes, in the executable's own order.
///
/// Only the straight shapes have tables — the three corner routines stamp
/// their geometry differently and read none of this. The four slope triples
/// are the four cardinal ramp orientations: each row runs
/// `[head, body, body, body, tail]`.
const RAMP_RECORDS: [RampRecord; 4] = [
    // ClearSouth
    RampRecord {
        level_steps: LEVEL_STAIR,
        slope_descending: [11, 15, 15, 15, 7],
        slope_ascending: [12, 16, 16, 16, 8],
    },
    // ClearEast
    RampRecord {
        level_steps: LEVEL_STAIR,
        slope_descending: [10, 14, 14, 14, 6],
        slope_ascending: [11, 15, 15, 15, 7],
    },
    // ClearNorth
    RampRecord {
        level_steps: LEVEL_STAIR,
        slope_descending: [10, 14, 14, 14, 6],
        slope_ascending: [9, 13, 13, 13, 5],
    },
    // ClearWest
    RampRecord {
        level_steps: LEVEL_STAIR,
        slope_descending: [9, 13, 13, 13, 5],
        slope_ascending: [12, 16, 16, 16, 8],
    },
];

/// The table for a straight shape, or `None` for the corner shapes.
pub fn ramp_record(shape: RampShape) -> Option<&'static RampRecord> {
    let index = match shape {
        RampShape::ClearSouth => 0,
        RampShape::ClearEast => 1,
        RampShape::ClearNorth => 2,
        RampShape::ClearWest => 3,
        _ => return None,
    };
    Some(&RAMP_RECORDS[index])
}

/// The tile a slope value selects, relative to the theater's ramp base.
pub fn ramp_tile(ramp_base: i32, slope: u8) -> i32 {
    ramp_base + i32::from(slope) - 1
}

/// May a ramp be carved through this rect?
///
/// Two stages, and the whole thing bails on the first refusal:
///
/// 1. The four corners must be on the map.
/// 2. Every cell in the rect must be bare ground **and** belong either to the
///    carving region or to the lower one the ramp is reaching down to.
///
/// That second ownership term is why a ramp can cross the boundary between two
/// plateaus at all. Note the contrast with the ring-window quota test in
/// `connector`, which is handed the same lower-region id and ignores it — the
/// two look alike and mean different things.
///
/// The sweep itself is not diamond-guarded: the corner probes are what
/// establish the rect is on the map, exactly as in the original.
pub fn rect_is_carveable(
    grid: &mut RmgGrid,
    scratch: &RmgScratch,
    ids: &TileIds,
    playfield: &Playfield,
    rect: (i32, i32, i32, i32),
    region: i32,
    lower_region: i32,
) -> bool {
    let (rx, ry, w, h) = rect;
    let corners = [
        (rx, ry),
        (rx + w - 1, ry),
        (rx, ry + h - 1),
        (rx + w - 1, ry + h - 1),
    ];
    // The playfield, not the map diamond — they are not the same rectangle.
    // The playfield is inset by the map's local-size margins, so a rect out in
    // the border band passes the diamond and fails here, which is the whole
    // point: ramps do not get carved into the unplayable frame.
    //
    // Elevation-aware, because the carve asks for the raised form. A cell off
    // the grid has no level or slope to read, so it is refused outright rather
    // than probed at zero.
    for &(x, y) in &corners {
        let Ok(cx) = u16::try_from(x) else {
            return false;
        };
        let Ok(cy) = u16::try_from(y) else {
            return false;
        };
        let Some(cell) = grid.get(x, y) else {
            return false;
        };
        if !playfield.contains_raised(cx, cy, cell.level as i8, cell.slope) {
            return false;
        }
    }

    for row in 0..h {
        for col in 0..w {
            let (x, y) = (rx + col, ry + row);
            let owner = scratch.get(x, y).region;
            if owner != region && owner != lower_region {
                return false;
            }
            if !ids.is_clear(grid.cell_native(x, y).tile) {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::rmg::tiles::SpecialTerrain;

    fn ids() -> TileIds {
        TileIds {
            clear: 0,
            ramp_base: 200,
            ramp_smooth: 220,
            rough: -1,
            sand: -1,
            green: 100,
            rough_lat: -1,
            sand_lat: -1,
            green_lat: 110,
            pave_lat: -1,
            pave: -1,
            water_base: 500,
            shore: 400,
            water_bridge: -1,
            misc_pave: -1,
            paved_roads: -1,
            paved_road_ends: -1,
            medians: -1,
            special: SpecialTerrain::default(),
        }
    }

    /// A playfield covering the whole test diamond, so the corner probes
    /// only refuse for genuinely out-of-band cells.
    fn playfield() -> Playfield {
        Playfield::from_local_size(34, 0, 0, 34, 42)
    }

    /// Flat clear ground owned by `region`, with the whole diamond claimed.
    fn harness(region: i32) -> (RmgGrid, RmgScratch) {
        let (dmin, dmax) = (34, 34 + 2 * 42);
        let stride = (34 + 42 + 1) as usize;
        let mut grid = RmgGrid::new(stride, dmin, dmax);
        let mut scratch = RmgScratch::new(stride, dmin, dmax);
        for (x, y) in grid.native_cells().collect::<Vec<_>>() {
            grid.get_mut(x, y).expect("native cell").tile = 0;
            scratch.get_mut(x, y).region = region;
        }
        (grid, scratch)
    }

    #[test]
    fn every_straight_shape_walks_the_same_stair_down() {
        // The level table is what makes a ramp a ramp. All four shapes share
        // it; a shape that differed would step at a different rate.
        for shape in [
            RampShape::ClearSouth,
            RampShape::ClearEast,
            RampShape::ClearNorth,
            RampShape::ClearWest,
        ] {
            let record = ramp_record(shape).expect("straight shape has a table");
            assert_eq!(record.level_steps, [3, 2, 1, 0, 0], "{shape:?}");
        }
    }

    #[test]
    fn corner_shapes_have_no_table() {
        // The three corner routines stamp their own geometry and read none of
        // this. Handing them a straight shape's table would be silent nonsense.
        for shape in [
            RampShape::CornerNe,
            RampShape::CornerSe,
            RampShape::CornerSw,
            RampShape::CornerNw,
        ] {
            assert!(ramp_record(shape).is_none(), "{shape:?}");
        }
    }

    #[test]
    fn the_slope_row_is_chosen_by_the_sign_of_the_run() {
        // Not a random pick — the two rows are the same ramp seen from either
        // side, and a zero run takes the descending row.
        let record = ramp_record(RampShape::ClearSouth).expect("table");
        assert_eq!(record.slopes(1), &[12, 16, 16, 16, 8], "positive run");
        assert_eq!(record.slopes(-1), &[11, 15, 15, 15, 7], "negative run");
        assert_eq!(
            record.slopes(0),
            &[11, 15, 15, 15, 7],
            "zero takes descending"
        );
    }

    #[test]
    fn the_four_shapes_carry_four_distinct_orientations() {
        // Each row's head value is what selects the tile, so two shapes
        // sharing a whole row would carve the same-facing stair twice and
        // leave one orientation unreachable.
        let heads: Vec<(u8, u8)> = [
            RampShape::ClearSouth,
            RampShape::ClearEast,
            RampShape::ClearNorth,
            RampShape::ClearWest,
        ]
        .iter()
        .map(|&s| {
            let r = ramp_record(s).expect("table");
            (r.slope_descending[0], r.slope_ascending[0])
        })
        .collect();
        assert_eq!(heads, [(11, 12), (10, 11), (10, 9), (9, 12)]);
    }

    #[test]
    fn a_slope_value_picks_the_tile_one_below_it() {
        assert_eq!(ramp_tile(200, 1), 200);
        assert_eq!(ramp_tile(200, 16), 215);
    }

    #[test]
    fn flat_owned_ground_is_carveable() {
        let (mut grid, scratch) = harness(7);
        let ids = ids();
        assert!(rect_is_carveable(
            &mut grid,
            &scratch,
            &ids,
            &playfield(),
            (38, 48, 6, 4),
            7,
            7
        ));
    }

    #[test]
    fn the_lower_region_is_admitted_too() {
        // The whole point: a ramp reaches down into the neighbouring plateau,
        // so cells owned by the lower region must pass. A port that only
        // accepted the carving region would never carve a single ramp.
        let (mut grid, mut scratch) = harness(7);
        let ids = ids();
        for x in 40..44 {
            scratch.get_mut(x, 49).region = 3;
        }
        assert!(
            rect_is_carveable(
                &mut grid,
                &scratch,
                &ids,
                &playfield(),
                (38, 48, 6, 4),
                7,
                3
            ),
            "lower region accepted"
        );
        assert!(
            !rect_is_carveable(
                &mut grid,
                &scratch,
                &ids,
                &playfield(),
                (38, 48, 6, 4),
                7,
                9
            ),
            "a third region is not"
        );
    }

    #[test]
    fn a_non_clear_cell_refuses() {
        let (mut grid, scratch) = harness(7);
        let ids = ids();
        grid.get_mut(41, 49).expect("native cell").tile = 500;
        assert!(!rect_is_carveable(
            &mut grid,
            &scratch,
            &ids,
            &playfield(),
            (38, 48, 6, 4),
            7,
            7
        ));
    }

    #[test]
    fn a_rect_hanging_off_the_map_refuses_at_the_corners() {
        // The corner probes are the ONLY bounds check — the sweep has none —
        // so they have to catch this or the sweep reads off the map.
        //
        // To prove it is the probes doing the work, every slot the sweep will
        // touch is made to pass on its own terms: owned by the region and
        // clear, out-of-diamond ones included. Painting only the in-diamond
        // cells would let the ownership test refuse instead, and the probes
        // could be deleted without a single test noticing.
        let (mut grid, mut scratch) = harness(7);
        let ids = ids();
        for slot in scratch.cells_mut() {
            slot.region = 7;
        }
        assert!(!scratch.in_diamond(16, 16), "corner really is off");
        assert!(
            !rect_is_carveable(
                &mut grid,
                &scratch,
                &ids,
                &playfield(),
                (16, 16, 6, 4),
                7,
                7
            ),
            "the corner probes must refuse this"
        );
        // Same rect fully inside the diamond still passes, so the refusal
        // above is about position and not about the painting.
        assert!(rect_is_carveable(
            &mut grid,
            &scratch,
            &ids,
            &playfield(),
            (38, 48, 6, 4),
            7,
            7
        ));
    }
}
