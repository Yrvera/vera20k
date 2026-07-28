//! Ground checks for a low bridge deck.
//!
//! The deck placer tries up to two hundred spots; each one is offered to
//! [`deck_area_is_clear`] before anything is stamped, and a refusal just moves
//! the placer on to its next try. No randomness here.

use crate::map::rmg::grid::RmgGrid;
use crate::map::rmg::preview::Playfield;
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
/// The four corners are checked first, so the sweep never runs on an area that
/// hangs off the map.
pub fn deck_area_is_clear(
    grid: &mut RmgGrid,
    scratch: &RmgScratch,
    ids: &TileIds,
    playfield: &Playfield,
    rect: (i32, i32, i32, i32),
) -> bool {
    let (rx, ry, w, h) = rect;

    // Read first, exactly as the original does — the origin's level is the bar
    // every other cell is held to, even if the corner checks then reject.
    let Some(origin) = grid.get(rx, ry) else {
        return false;
    };
    let reference_level = origin.level;

    // NOT YET COVERED BY A TEST. Every fixture tried so far is refused earlier
    // — by the origin read above, which already rejects a rect starting off the
    // grid. Distinguishing these needs a rect whose ORIGIN is valid but whose
    // far corner leaves the diamond. Recorded rather than assumed covered.
    for &(x, y) in &[
        (rx, ry),
        (rx + w - 1, ry),
        (rx, ry + h - 1),
        (rx + w - 1, ry + h - 1),
    ] {
        if !scratch.in_diamond(x, y) {
            return false;
        }
    }
    let _ = playfield;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::rmg::tiles::SpecialTerrain;

    fn ids() -> TileIds {
        TileIds {
            clear: 0,
            ramp_base: 200,
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

    fn playfield() -> Playfield {
        Playfield::from_local_size(34, 0, 0, 34, 42)
    }

    #[test]
    fn flat_clear_ground_takes_a_deck() {
        let (mut grid, scratch) = harness();
        assert!(deck_area_is_clear(
            &mut grid,
            &scratch,
            &ids(),
            &playfield(),
            (40, 48, 6, 3)
        ));
    }

    #[test]
    fn water_is_accepted_because_that_is_what_a_deck_spans() {
        let (mut grid, scratch) = harness();
        for x in 41..45 {
            grid.get_mut(x, 49).expect("native cell").tile = 500;
        }
        assert!(deck_area_is_clear(
            &mut grid,
            &scratch,
            &ids(),
            &playfield(),
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
                !deck_area_is_clear(&mut grid, &scratch, &ids(), &playfield(), (40, 48, 6, 3)),
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
            &playfield(),
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
            &playfield(),
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
            &playfield(),
            (40, 48, 6, 3)
        ));
    }

    #[test]
    fn a_rect_hanging_off_the_map_refuses_at_the_corners() {
        // Every OTHER condition is made to pass over the swept area first, so
        // the corner probes are the only thing left that can refuse. Without
        // that the level check does the work and the probes could be deleted
        // with this test still green.
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
        assert!(!deck_area_is_clear(
            &mut grid,
            &scratch,
            &ids(),
            &playfield(),
            (16, 16, 6, 3)
        ));
    }
}
