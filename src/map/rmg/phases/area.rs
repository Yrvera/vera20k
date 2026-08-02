//! The area tests that decide whether something may be placed on a patch of
//! ground.
//!
//! Two phases ask the same question in the same words. The start placer offers
//! a 6x6 window around each candidate waypoint; the low-bridge placer offers a
//! 3-cell-wide strip as it walks a corridor outward from its seed. Both go
//! through **one routine in the original**, so they are one routine here — two
//! copies of a native predicate is how the copies start disagreeing.
//!
//! The routine takes two override bytes that would let paved roads and
//! road-ends through instead of refusing them. Both call sites pass zero for
//! both, so the overrides are folded away and the rule is simply: a road
//! refuses, and what is left has to be clear ground, misc-pave or pave.

use crate::map::rmg::grid::RmgGrid;
use crate::map::rmg::scratch::RmgScratch;
use crate::map::rmg::tiles::TileIds;

/// The four corners of `rect`, each tested against the map diamond.
///
/// The diamond is convex, so four corners inside it put the whole rectangle
/// inside — which is what lets the sweeps that follow read cells without
/// re-testing every one.
pub fn corners_in_diamond(scratch: &RmgScratch, rect: (i32, i32, i32, i32)) -> bool {
    let (rx, ry, w, h) = rect;
    [
        (rx, ry),
        (rx + w - 1, ry),
        (rx, ry + h - 1),
        (rx + w - 1, ry + h - 1),
    ]
    .iter()
    .all(|&(x, y)| scratch.in_diamond(x, y))
}

/// May something stand on this tile?
///
/// The refusal is a separate step rather than just an absence from the accept
/// list, and that matters: these spans are hardcoded lengths, not the real size
/// of each tileset, so a short set lets a span run on into its neighbour and a
/// tile can land in two families at once. Where that happens the refusal wins.
pub fn tile_is_placeable(ids: &TileIds, tile: i32) -> bool {
    if ids.is_paved_road(tile) || ids.is_paved_road_end(tile) {
        return false;
    }
    ids.is_clear(tile) || ids.is_misc_pave(tile) || ids.is_pave(tile)
}

/// Is every cell of `rect` clear of roads and standable?
///
/// Corners first, then **exactly `w x h`** — no inclusive margin. There is no
/// level test here; a rect spanning a step in the ground still passes. The
/// low-bridge end-piece check is this test plus level uniformity, and the deck
/// validator is a different sweep again, so none of the three may share a walk.
pub fn area_is_paved_clear(
    grid: &RmgGrid,
    scratch: &RmgScratch,
    ids: &TileIds,
    rect: (i32, i32, i32, i32),
) -> bool {
    let (rx, ry, w, h) = rect;

    if !corners_in_diamond(scratch, rect) {
        return false;
    }

    for y in ry..(ry + h) {
        for x in rx..(rx + w) {
            // The corner probes put the whole rect inside the diamond, and the
            // diamond is inside the array, so this never actually falls out.
            let Some(cell) = grid.get(x, y) else {
                return false;
            };
            if !tile_is_placeable(ids, cell.tile) {
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

    const GREEN: i32 = 100;
    const PAVE: i32 = 600;
    const MISC_PAVE: i32 = 620;
    const PAVED_ROAD: i32 = 640;
    const PAVED_ROAD_END: i32 = 660;

    fn ids() -> TileIds {
        TileIds {
            clear: 0,
            ramp_base: 200,
            ramp_smooth: 220,
            rough: -1,
            sand: -1,
            green: GREEN,
            rough_lat: -1,
            sand_lat: -1,
            green_lat: 110,
            pave_lat: -1,
            pave: PAVE,
            water_base: 500,
            shore: 400,
            water_bridge: -1,
            misc_pave: MISC_PAVE,
            paved_roads: PAVED_ROAD,
            paved_road_ends: PAVED_ROAD_END,
            medians: -1,
            special: SpecialTerrain::default(),
        }
    }

    fn harness() -> (RmgGrid, RmgScratch) {
        let (dmin, dmax) = (34, 34 + 2 * 42);
        let stride = (34 + 42 + 1) as usize;
        let mut grid = RmgGrid::new(stride, dmin, dmax);
        let scratch = RmgScratch::new(stride, dmin, dmax);
        for (x, y) in grid.native_cells().collect::<Vec<_>>() {
            let cell = grid.get_mut(x, y).expect("native cell");
            cell.tile = 0;
            cell.level = 4;
            cell.overlay = -1;
        }
        (grid, scratch)
    }

    #[test]
    fn clear_ground_is_placeable() {
        let (grid, scratch) = harness();
        assert!(area_is_paved_clear(&grid, &scratch, &ids(), (40, 48, 6, 6)));
    }

    #[test]
    fn roads_and_road_ends_refuse() {
        for (base, label) in [(PAVED_ROAD, "paved road"), (PAVED_ROAD_END, "road end")] {
            let (mut grid, scratch) = harness();
            grid.get_mut(42, 49).expect("native cell").tile = base;
            assert!(
                !area_is_paved_clear(&grid, &scratch, &ids(), (40, 48, 6, 6)),
                "{label} must refuse"
            );
        }
    }

    #[test]
    fn pave_and_misc_pave_are_accepted() {
        for (base, label) in [(PAVE, "pave"), (MISC_PAVE, "misc pave")] {
            let (mut grid, scratch) = harness();
            grid.get_mut(42, 49).expect("native cell").tile = base;
            assert!(
                area_is_paved_clear(&grid, &scratch, &ids(), (40, 48, 6, 6)),
                "{label} must be accepted"
            );
        }
    }

    #[test]
    fn other_terrain_refuses() {
        let (mut grid, scratch) = harness();
        grid.get_mut(42, 49).expect("native cell").tile = GREEN;
        assert!(!area_is_paved_clear(
            &grid,
            &scratch,
            &ids(),
            (40, 48, 6, 6)
        ));
    }

    #[test]
    fn a_road_tile_refuses_even_when_it_is_also_inside_the_pave_span() {
        let overlapping = TileIds {
            pave: PAVED_ROAD - 2,
            ..ids()
        };
        assert!(
            overlapping.is_pave(PAVED_ROAD) && overlapping.is_paved_road(PAVED_ROAD),
            "the fixture really does put one tile in both spans"
        );
        let (mut grid, scratch) = harness();
        grid.get_mut(42, 49).expect("native cell").tile = PAVED_ROAD;
        assert!(!area_is_paved_clear(
            &grid,
            &scratch,
            &overlapping,
            (40, 48, 6, 6)
        ));
    }

    #[test]
    fn a_step_in_the_ground_does_not_refuse() {
        // No level test here, unlike the bridge end-piece check. Adding one
        // would refuse start positions the original accepts.
        let (mut grid, scratch) = harness();
        grid.get_mut(42, 49).expect("native cell").level = 9;
        assert!(area_is_paved_clear(&grid, &scratch, &ids(), (40, 48, 6, 6)));
    }

    #[test]
    fn the_sweep_stops_at_the_rect() {
        // One past the rect on either axis is not judged; the last cell inside
        // is. The pair pins the extent rather than just proving it is small.
        for (bx, by, label) in [(46, 49, "one column past"), (43, 54, "one row past")] {
            let (mut grid, scratch) = harness();
            grid.get_mut(bx, by).expect("native cell").tile = GREEN;
            assert!(
                area_is_paved_clear(&grid, &scratch, &ids(), (40, 48, 6, 6)),
                "{label} is outside"
            );
        }
        let (mut grid, scratch) = harness();
        grid.get_mut(45, 53).expect("native cell").tile = GREEN;
        assert!(!area_is_paved_clear(
            &grid,
            &scratch,
            &ids(),
            (40, 48, 6, 6)
        ));
    }

    #[test]
    fn a_rect_hanging_off_the_map_refuses_at_the_corners() {
        // Everything else over the swept area is made to pass, so only the
        // corner probes can refuse.
        let (mut grid, scratch) = harness();
        for y in 16..=22 {
            for x in 16..=22 {
                let cell = grid.cell_native_mut(x, y);
                cell.tile = 0;
                cell.level = 4;
            }
        }
        assert!(!scratch.in_diamond(16, 16), "the rect really does hang off");
        assert!(!area_is_paved_clear(
            &grid,
            &scratch,
            &ids(),
            (16, 16, 6, 6)
        ));
    }
}
