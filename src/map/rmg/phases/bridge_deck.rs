//! Ground checks for a low bridge deck.
//!
//! The deck placer tries up to two hundred spots. Each one is offered to
//! [`deck_area_is_clear`] before anything is stamped, and the ground under each
//! of the bridge's end pieces is offered to [`end_area_is_placeable`]; a
//! refusal just moves the placer on to its next try. No randomness in either.
//!
//! **The two look like the same check and are not.** They disagree about the
//! swept extent, the order the reference level is read in, whether overlays
//! matter, and which tiles they will accept — see each one's own note. Sharing
//! a sweep between them would judge the wrong cells in both directions.

use crate::map::rmg::grid::RmgGrid;
use crate::map::rmg::scratch::RmgScratch;
use crate::map::rmg::tiles::TileIds;

/// Is this area clear enough to lay a bridge deck across?
///
/// Three things have to hold, and two of them are easy to get subtly wrong:
///
/// - **The swept area is one row and one column LARGER than the deck.** The
///   walk runs inclusive on both axes, so a deck of `w x h` is judged on
///   `(w+1) x (h+1)` cells. That margin is why decks never end up flush
///   against something they should not touch, and shrinking the sweep to the
///   deck itself would let them.
/// - **The tile rule is "clear or water", not "clear or bridge".** The
///   original's second term is the shore/water family test; the name it
///   carries in the disassembly says bridge, and that name is wrong. Porting
///   it from the name would accept cells that already hold a bridge and reject
///   the open water a deck is for.
/// - The reference level is read from the deck's own origin **before** the
///   corner checks, and every swept cell must match it exactly.
///
/// The four corners are checked after that read, so the sweep never runs on an
/// area that hangs off the map.
pub fn deck_area_is_clear(
    grid: &mut RmgGrid,
    scratch: &RmgScratch,
    ids: &TileIds,
    rect: (i32, i32, i32, i32),
) -> bool {
    let (rx, ry, w, h) = rect;

    // Read first, exactly as the original does — the origin's level is the bar
    // every other cell is held to, even if the corner checks then reject.
    let Some(origin) = grid.get(rx, ry) else {
        return false;
    };
    let reference_level = origin.level;

    if !corners_in_diamond(scratch, rect) {
        return false;
    }

    // Inclusive on both axes: one row and one column more than the deck.
    for y in ry..=(ry + h) {
        for x in rx..=(rx + w) {
            let cell = *grid.cell_native(x, y);
            if cell.overlay != -1 {
                return false;
            }
            if cell.level != reference_level {
                return false;
            }
            // Clear ground, or the water family a deck spans.
            //
            // UNVERIFIED CORRESPONDENCE: the port's water-family predicate
            // covers the water span and shore pieces. The original's also
            // takes in the waterfall sets. A deck offered a waterfall cell
            // would be refused here and accepted there — recorded rather than
            // widened on a guess, since widening it wrongly would let decks
            // stamp over terrain they must not.
            if !ids.is_clear(cell.tile) && !ids.is_bridge_absorbable(cell.tile) {
                return false;
            }
        }
    }
    true
}

/// Can a bridge's end piece anchor on this ground?
///
/// Read as a twin of [`deck_area_is_clear`] and it will mislead you. Every one
/// of these differences flips verdicts on ordinary ground:
///
/// - **This walks exactly `w x h`** — no inclusive margin. The deck validator
///   walks one row and one column further.
/// - **The corners are probed before the reference level is read**, the
///   opposite order. Here that ordering is load-bearing rather than cosmetic:
///   with the probes first, a rect hanging off the map never reaches a cell
///   read at all.
/// - **Overlays are not looked at.** A cell already carrying one can still
///   anchor an end piece, where it would refuse a deck.
/// - **Water refuses.** The deck spans water; its ends have to stand on land.
///
/// The tile rule is the paving rule the start-placement window uses: a paved
/// road or a road-end refuses outright, and what remains must be clear ground,
/// misc-pave or pave.
///
/// The original carries a byte parameter that would make roads and road-ends
/// acceptable instead of refusing. **All four call sites zero it**, so that
/// branch cannot be reached in play and is folded away here rather than
/// modelled as a flag no caller sets.
pub fn end_area_is_placeable(
    grid: &mut RmgGrid,
    scratch: &RmgScratch,
    ids: &TileIds,
    rect: (i32, i32, i32, i32),
) -> bool {
    let (rx, ry, w, h) = rect;

    if !corners_in_diamond(scratch, rect) {
        return false;
    }

    let reference_level = grid.cell_native(rx, ry).level;

    // Exclusive on both axes: exactly the rect, nothing around it.
    for y in ry..(ry + h) {
        for x in rx..(rx + w) {
            let cell = *grid.cell_native(x, y);
            if cell.level != reference_level {
                return false;
            }
            if ids.is_paved_road(cell.tile) || ids.is_paved_road_end(cell.tile) {
                return false;
            }
            if !ids.is_clear(cell.tile) && !ids.is_misc_pave(cell.tile) && !ids.is_pave(cell.tile) {
                return false;
            }
        }
    }
    true
}

/// The four corners of `rect`, each tested against the map diamond.
///
/// The diamond is convex, so four corners inside it put the whole rectangle
/// inside — which is what lets both sweeps read cells without re-testing.
fn corners_in_diamond(scratch: &RmgScratch, rect: (i32, i32, i32, i32)) -> bool {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::rmg::tiles::SpecialTerrain;

    /// Tile bases the fixtures address families by. Spaced far enough apart
    /// that no span reaches the next.
    const WATER: i32 = 500;
    const GREEN: i32 = 100;
    const PAVE: i32 = 600;
    const MISC_PAVE: i32 = 620;
    const PAVED_ROAD: i32 = 640;
    const PAVED_ROAD_END: i32 = 660;

    fn ids() -> TileIds {
        TileIds {
            clear: 0,
            ramp_base: 200,
            rough: -1,
            sand: -1,
            green: GREEN,
            rough_lat: -1,
            sand_lat: -1,
            green_lat: 110,
            pave_lat: -1,
            pave: PAVE,
            water_base: WATER,
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

    /// A rect that sits inside the grid array but outside the map diamond,
    /// painted so that every check except the corner probes would pass.
    fn off_diamond_rect() -> ((i32, i32, i32, i32), RmgGrid, RmgScratch) {
        let rect = (16, 16, 6, 3);
        let (mut grid, scratch) = harness();
        for y in 16..=19 {
            for x in 16..=22 {
                let cell = grid.cell_native_mut(x, y);
                cell.tile = 0;
                cell.level = 4;
                cell.overlay = -1;
            }
        }
        assert!(!scratch.in_diamond(16, 16), "the rect really does hang off");
        (rect, grid, scratch)
    }

    #[test]
    fn flat_clear_ground_takes_a_deck() {
        let (mut grid, scratch) = harness();
        assert!(deck_area_is_clear(
            &mut grid,
            &scratch,
            &ids(),
            (40, 48, 6, 3)
        ));
    }

    #[test]
    fn water_is_accepted_because_that_is_what_a_deck_spans() {
        let (mut grid, scratch) = harness();
        for x in 41..45 {
            grid.get_mut(x, 49).expect("native cell").tile = WATER;
        }
        assert!(deck_area_is_clear(
            &mut grid,
            &scratch,
            &ids(),
            (40, 48, 6, 3)
        ));
    }

    #[test]
    fn the_sweep_reaches_one_past_the_deck_on_both_axes() {
        // The row below and the column right of the deck are still judged. A
        // sweep confined to the deck itself would miss both, which is what the
        // margin exists to prevent.
        for (bx, by, label) in [
            (46, 49, "one column past"),
            (43, 51, "one row past"),
            (46, 51, "the far corner"),
        ] {
            let (mut grid, scratch) = harness();
            grid.get_mut(bx, by).expect("native cell").level = 9;
            assert!(
                !deck_area_is_clear(&mut grid, &scratch, &ids(), (40, 48, 6, 3)),
                "{label} must be swept"
            );
        }
        // And one further out is genuinely outside, so it must NOT refuse —
        // otherwise the test above would pass for a sweep of any size.
        let (mut grid, scratch) = harness();
        grid.get_mut(47, 52).expect("native cell").level = 9;
        assert!(deck_area_is_clear(
            &mut grid,
            &scratch,
            &ids(),
            (40, 48, 6, 3)
        ));
    }

    #[test]
    fn an_existing_overlay_refuses() {
        let (mut grid, scratch) = harness();
        grid.get_mut(42, 49).expect("native cell").overlay = 0x4A;
        assert!(!deck_area_is_clear(
            &mut grid,
            &scratch,
            &ids(),
            (40, 48, 6, 3)
        ));
    }

    #[test]
    fn a_step_in_the_ground_refuses() {
        let (mut grid, scratch) = harness();
        grid.get_mut(42, 49).expect("native cell").level = 8;
        assert!(!deck_area_is_clear(
            &mut grid,
            &scratch,
            &ids(),
            (40, 48, 6, 3)
        ));
    }

    #[test]
    fn a_rect_hanging_off_the_map_refuses_at_the_corners() {
        // Every OTHER condition is made to pass over the swept area first, so
        // the corner probes are the only thing left that can refuse. Without
        // that the level check does the work and the probes could be deleted
        // with this test still green.
        let (rect, mut grid, scratch) = off_diamond_rect();
        assert!(!deck_area_is_clear(&mut grid, &scratch, &ids(), rect));
    }

    #[test]
    fn flat_clear_ground_anchors_an_end_piece() {
        let (mut grid, scratch) = harness();
        assert!(end_area_is_placeable(
            &mut grid,
            &scratch,
            &ids(),
            (40, 48, 6, 3)
        ));
    }

    #[test]
    fn the_end_area_sweep_stops_at_the_rect() {
        // The sharp contrast with the deck validator: one cell past the rect on
        // either axis is NOT judged here. Giving this walk the deck's inclusive
        // margin would refuse end pieces the original places.
        for (bx, by, label) in [
            (46, 49, "one column past"),
            (43, 51, "one row past"),
            (46, 51, "the far corner"),
        ] {
            let (mut grid, scratch) = harness();
            grid.get_mut(bx, by).expect("native cell").level = 9;
            assert!(
                end_area_is_placeable(&mut grid, &scratch, &ids(), (40, 48, 6, 3)),
                "{label} is outside the end area"
            );
        }
        // The last cell that IS inside, so the pair pins the boundary rather
        // than just proving the sweep is small.
        let (mut grid, scratch) = harness();
        grid.get_mut(45, 50).expect("native cell").level = 9;
        assert!(!end_area_is_placeable(
            &mut grid,
            &scratch,
            &ids(),
            (40, 48, 6, 3)
        ));
    }

    #[test]
    fn a_step_in_the_ground_refuses_an_end_piece() {
        let (mut grid, scratch) = harness();
        grid.get_mut(42, 49).expect("native cell").level = 8;
        assert!(!end_area_is_placeable(
            &mut grid,
            &scratch,
            &ids(),
            (40, 48, 6, 3)
        ));
    }

    #[test]
    fn paved_roads_and_road_ends_refuse_an_end_piece() {
        // Two separate tile families, each with its own base — testing one
        // would leave the other free to be dropped.
        for (base, label) in [(PAVED_ROAD, "paved road"), (PAVED_ROAD_END, "road end")] {
            let (mut grid, scratch) = harness();
            grid.get_mut(42, 49).expect("native cell").tile = base;
            assert!(
                !end_area_is_placeable(&mut grid, &scratch, &ids(), (40, 48, 6, 3)),
                "{label} must refuse"
            );
        }
    }

    #[test]
    fn a_road_tile_refuses_even_when_it_is_also_inside_the_pave_span() {
        // Why the refusal is a separate step and not just an absence from the
        // accept list: every one of these spans is a hardcoded length, not the
        // tileset's real size, so a short set lets the span run on into its
        // neighbour and a tile can land in two of them at once. Where that
        // happens the refusal has to win. With disjoint bases the ordering is
        // invisible, which is exactly why it needs its own fixture.
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
        assert!(!end_area_is_placeable(
            &mut grid,
            &scratch,
            &overlapping,
            (40, 48, 6, 3)
        ));
    }

    #[test]
    fn pave_and_misc_pave_still_anchor_an_end_piece() {
        // The accept list is wider than clear ground alone.
        for (base, label) in [(PAVE, "pave"), (MISC_PAVE, "misc pave")] {
            let (mut grid, scratch) = harness();
            grid.get_mut(42, 49).expect("native cell").tile = base;
            assert!(
                end_area_is_placeable(&mut grid, &scratch, &ids(), (40, 48, 6, 3)),
                "{label} must be accepted"
            );
        }
    }

    #[test]
    fn water_refuses_an_end_piece_even_though_a_deck_spans_it() {
        // Same cell, same tile, opposite verdicts from the two checks. A bridge
        // crosses water; its ends have to stand on land.
        let (mut grid, scratch) = harness();
        grid.get_mut(42, 49).expect("native cell").tile = WATER;
        assert!(!end_area_is_placeable(
            &mut grid,
            &scratch,
            &ids(),
            (40, 48, 6, 3)
        ));
        let (mut grid, scratch) = harness();
        grid.get_mut(42, 49).expect("native cell").tile = WATER;
        assert!(deck_area_is_clear(
            &mut grid,
            &scratch,
            &ids(),
            (40, 48, 6, 3)
        ));
    }

    #[test]
    fn ordinary_terrain_refuses_an_end_piece() {
        let (mut grid, scratch) = harness();
        grid.get_mut(42, 49).expect("native cell").tile = GREEN;
        assert!(!end_area_is_placeable(
            &mut grid,
            &scratch,
            &ids(),
            (40, 48, 6, 3)
        ));
    }

    #[test]
    fn an_overlay_does_not_stop_an_end_piece() {
        // The other direction of the same contrast: this check never looks at
        // overlays, so copying the deck validator's rule here would refuse
        // ground the original accepts.
        let (mut grid, scratch) = harness();
        grid.get_mut(42, 49).expect("native cell").overlay = 0x4A;
        assert!(end_area_is_placeable(
            &mut grid,
            &scratch,
            &ids(),
            (40, 48, 6, 3)
        ));
    }

    #[test]
    fn an_end_area_hanging_off_the_map_refuses_at_the_corners() {
        // Nothing else can refuse: the cells are real, uniform and clear. With
        // the probes gone this returns true, which is what makes the fixture
        // worth having — the corner test is the only thing standing between the
        // placer and an end piece off the edge of the map.
        let (rect, mut grid, scratch) = off_diamond_rect();
        assert!(!end_area_is_placeable(&mut grid, &scratch, &ids(), rect));
    }
}
