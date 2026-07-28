//! Ground checks for a low bridge deck.
//!
//! The deck placer tries up to two hundred spots. Each attempt starts by
//! rolling a cell out of [`pick_seed_cell`] — the only part of this file that
//! touches the random stream — and the ground it lands on is then offered to
//! [`deck_area_is_clear`] for the span and [`end_area_is_placeable`] for each
//! end before anything is stamped. A refusal just moves the placer on.
//!
//! **Those two ground checks look like the same check and are not.** They
//! disagree about the
//! swept extent, the order the reference level is read in, whether overlays
//! matter, and which tiles they will accept — see each one's own note. Sharing
//! a sweep between them would judge the wrong cells in both directions.

use crate::map::rmg::grid::RmgGrid;
use crate::map::rmg::rng::{RANGE_K_BITS, RmgRng};
use crate::map::rmg::scratch::RmgScratch;
use crate::map::rmg::tiles::TileIds;
use crate::map::rmg::x87::{self, TruncF64};

/// Turns an infinite native loop into a `None` and nothing else — see
/// [`pick_seed_cell`]. Not a retry budget: the original has no bound here.
const SEED_PICK_SPIN_LIMIT: u32 = 10_000_000;

/// One uniform index over the whole scratch array.
///
/// `rnd * span * K`, in that order, with `K` the original's own
/// `(1 + 2^-32) * 2^-32` rather than a folded-in division — the two round
/// differently at the last bit and are not interchangeable.
///
/// The bound is an **unsigned** compare in the original (`JA`), so this keeps
/// the cast rather than comparing as `i32`.
///
/// Whether the bound is `<=` or `<` cannot be observed, and the reason is worth
/// keeping. Tightening it only changes what happens to the single index
/// `span - 1`, which is the record at `(width-1, width-1)` — the far corner of
/// the square array. That corner is always outside the map diamond: being
/// inside needs `x + y <= diamond_max`, and `2*(width-1)` exceeds `diamond_max`
/// by the diamond's own minimum for every grid the generator builds. So the
/// record is never owned by a region and is always refused a step later. Either
/// spelling therefore spends exactly one draw on that value, and the stream is
/// identical.
fn draw_cell_index(rng: &mut RmgRng, span: i32) -> u32 {
    let scale = TruncF64::from_f64(f64::from_bits(RANGE_K_BITS));
    let span_f = TruncF64::from_f64(f64::from(span));
    loop {
        let value = x87::ftol(
            TruncF64::from_f64(f64::from(rng.next_u32()))
                .mul(span_f)
                .mul(scale)
                .to_f64(),
        ) as u32;
        if value <= (span - 1) as u32 {
            return value;
        }
    }
}

/// Pick the cell a low-bridge attempt starts from.
///
/// **This is the one place in the whole bridge and connector subtree where a
/// rejected draw is really thrown away.** Everywhere else the redraw loops are
/// arithmetic guards that cannot fire, because the generator truncates towards
/// zero and the largest possible draw lands exactly on the bound. Here two of
/// the three rejections are conditions on the map, they fire constantly, and
/// each one costs a fresh draw. Getting the count wrong by one desynchronises
/// every phase that follows.
///
/// The two real rejections:
///
/// - **The cell must belong to this region.** The draw ranges over the entire
///   scratch array, so on any ordinary map the overwhelming majority of draws
///   land outside the region and are spent.
/// - **The record's stored coordinate must not be (0, 0).** Records outside the
///   map diamond never had a coordinate stamped into them, so they still read
///   (0, 0) — that is how the original tells an unused slot apart from a real
///   one. It is not a test for the corner cell.
///
/// Returns `None` only where the original would spin forever, which is when no
/// cell in the scratch array satisfies both conditions. The spin limit is
/// VERA-internal; the original has no counter here. It cannot mask a real run:
/// with a single qualifying cell in a 256-wide scratch the chance of reaching
/// the limit anyway is about e^-153, and a bound that only trips where the
/// original never terminates cannot change any map the original produces.
pub fn pick_seed_cell(rng: &mut RmgRng, scratch: &RmgScratch, region: i32) -> Option<(i32, i32)> {
    let width = scratch.width() as i32;
    let span = width * width;
    for _ in 0..SEED_PICK_SPIN_LIMIT {
        let cell = scratch.cells()[draw_cell_index(rng, span) as usize];
        if cell.region != region {
            continue;
        }
        // Reading this as "not on the top row or left column" would be the
        // natural misreading, and it happens to be harmless: no stamped record
        // can sit on either. A record at x = 0 would need `diamond_min < y` and
        // `y < diamond_min` at once, and the same contradiction holds at
        // y = 0 — so the two spellings cannot be told apart. Kept as the
        // original's AND because the intent is "this slot was never filled in",
        // not a statement about the map edge.
        if cell.x == 0 && cell.y == 0 {
            continue;
        }
        return Some((i32::from(cell.x), i32::from(cell.y)));
    }
    None
}

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

    /// A scratch whose records are all unowned except the listed cells, which
    /// must be inside the diamond so that they carry a stamped coordinate.
    fn scratch_owning(cells: &[(i32, i32)], region: i32) -> RmgScratch {
        let (dmin, dmax) = (34, 34 + 2 * 42);
        let width = (34 + 42 + 1) as usize;
        let mut scratch = RmgScratch::new(width, dmin, dmax);
        for cell in scratch.cells_mut() {
            cell.region = -1;
        }
        for &(x, y) in cells {
            assert!(scratch.in_diamond(x, y), "fixture cell ({x},{y}) is inside");
            scratch.get_mut(x, y).region = region;
        }
        scratch
    }

    #[test]
    fn the_seed_cell_is_always_one_the_region_owns() {
        let owned = [(40, 48), (41, 48), (42, 49)];
        let scratch = scratch_owning(&owned, 7);
        let mut rng = RmgRng::new(4);
        for _ in 0..64 {
            let picked = pick_seed_cell(&mut rng, &scratch, 7).expect("a cell qualifies");
            assert!(owned.contains(&picked), "picked {picked:?}");
        }
    }

    #[test]
    fn a_record_that_never_got_a_coordinate_is_rejected_not_returned_as_zero_zero() {
        // The out-of-diamond record is given the region id too, so the region
        // test alone cannot refuse it. Only its unstamped (0, 0) coordinate
        // can. Drop that check and this returns (0, 0) about half the time.
        let mut scratch = scratch_owning(&[(40, 48)], 7);
        let unstamped = scratch.get_mut(16, 16);
        assert_eq!((unstamped.x, unstamped.y), (0, 0), "really unstamped");
        unstamped.region = 7;

        let mut rng = RmgRng::new(11);
        for _ in 0..64 {
            assert_eq!(pick_seed_cell(&mut rng, &scratch, 7), Some((40, 48)));
        }
    }

    #[test]
    fn every_rejection_costs_exactly_one_draw() {
        // The load-bearing property of this whole function. Counted against an
        // independent walk of the same stream whose accept condition is spelled
        // out here rather than borrowed from the code under test, so a dropped
        // filter shows up as a stream mismatch and not just a different cell.
        let scratch = scratch_owning(&[(40, 48)], 7);
        let span = (scratch.width() * scratch.width()) as i32;

        let mut probe = RmgRng::new(9);
        let mut rejections = 0;
        loop {
            let cell = scratch.cells()[draw_cell_index(&mut probe, span) as usize];
            if cell.region == 7 && (cell.x != 0 || cell.y != 0) {
                break;
            }
            rejections += 1;
        }
        assert!(
            rejections > 0,
            "the fixture has to reject before it accepts"
        );

        let mut rng = RmgRng::new(9);
        assert_eq!(pick_seed_cell(&mut rng, &scratch, 7), Some((40, 48)));
        assert_eq!(
            rng.next_u32(),
            probe.next_u32(),
            "the picker and the independent walk must leave the stream at the same place"
        );
    }

    #[test]
    fn an_empty_region_gives_up_where_the_original_would_spin_forever() {
        let scratch = scratch_owning(&[], 7);
        let mut rng = RmgRng::new(2);
        assert_eq!(pick_seed_cell(&mut rng, &scratch, 7), None);
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
