//! Cutting a cliff stair into the ground.
//!
//! [`carve_straight_ramp_clear_south`] is the first of the original's seven
//! carve routines. It takes the two endpoints a ramp site chose and turns the
//! ground between them into a walkable stair: two rectangles handed to the
//! lower plateau, two to the upper, a tile block stamped at each end, then the
//! stepped edge itself and the flat run that leads onto it.
//!
//! Two things here are easy to get wrong and expensive to notice:
//!
//! - **The four fill loops split both the region id and the level**, in matched
//!   pairs. Each half of the corridor is handed to the plateau it belongs to.
//!   Tagging the whole corridor with one id compiles, looks right, and hands
//!   half the ramp to the wrong plateau — which only shows up much later, when
//!   starts and tiberium read those ids.
//! - **The end blocks come from a different tileset base than the stepped
//!   edge.** Conflating them stamps plausible-looking wrong tiles.
//!
//! Unwired, like the rest of the carve layer — see `connector`.

// Unwired until all seven routines land and the driver is written.
#![allow(dead_code)]

use crate::map::rmg::grid::RmgGrid;
use crate::map::rmg::preview::Playfield;
use crate::map::rmg::rng::RmgRng;
use crate::map::rmg::scratch::RmgScratch;
use crate::map::rmg::tiles::TileIds;

use super::connector::RampShape;
use super::ramp::{RAMP_STEPS, ramp_record, ramp_tile, rect_is_carveable};
use super::shore::TileBlocks;

/// Everything a carve borrows.
pub struct CarveCtx<'a> {
    pub grid: &'a mut RmgGrid,
    pub scratch: &'a mut RmgScratch,
    pub ids: &'a TileIds,
    pub blocks: &'a dyn TileBlocks,
    pub rng: &'a mut RmgRng,
    pub playfield: &'a Playfield,
    /// Tileset base for the two blocks stamped at a ramp's ends.
    ///
    /// Supplied by the caller rather than read off `TileIds`: in the original
    /// this is a global whose writer has not been traced, so there is no
    /// justified theater key to resolve it from yet. Threading it in keeps the
    /// unknown at the boundary instead of inventing a mapping.
    pub ramp_end_block: i32,
}

/// The two plateaus a carve joins.
///
/// `lower_level` is looked up from the region list on every cell in the
/// original. Nothing in the carve mutates that list, so resolving it once up
/// front is the same value every time — but it is a *lookup*, not the caller's
/// own level, and the two differ.
#[derive(Debug, Clone, Copy)]
pub struct CarveRegions {
    pub region: i32,
    pub level: u8,
    pub lower_region: i32,
    pub lower_level: u8,
}

/// Clearance the ramp needs along its run beyond the endpoint offset.
const RUN_CLEARANCE: i32 = 5;
/// Cells the flat run is deep.
const FLAT_RUN_DEPTH: i32 = 4;
/// Slope byte and tile offset the flat run stamps.
const FLAT_RUN_SLOPE: u8 = 4;
const FLAT_RUN_TILE_OFFSET: i32 = 3;

/// Stamp a tile block, writing every sub-tile that the block defines.
///
/// The unconditional stamper: unlike the shore tiler's, it asks nothing about
/// what is already in the cell. A `level_base` of `None` leaves the cell's
/// level alone — the low-bridge deck uses that; a ramp never does.
pub(crate) fn stamp_iso_block(
    ctx: &mut CarveCtx<'_>,
    tile: i32,
    origin: (i32, i32),
    scratch_id: i32,
    level_base: Option<u8>,
) {
    let Some(block) = ctx.blocks.block(tile) else {
        // Unknown block: the original stamps nothing and says nothing.
        return;
    };
    let block = block.clone();
    for row in 0..block.height {
        for col in 0..block.width {
            let (x, y) = (origin.0 + col, origin.1 + row);
            if !ctx.scratch.in_diamond(x, y) {
                continue;
            }
            let index = (block.width * row + col) as usize;
            let Some(sub) = block.subtiles.get(index).copied().flatten() else {
                continue;
            };
            let cell = ctx.grid.cell_native_mut(x, y);
            cell.sub_tile = index as u8;
            cell.tile = tile;
            if let Some(base) = level_base {
                cell.level = sub.height.wrapping_add(base).wrapping_sub(4);
            }
            cell.slope = sub.slope;
            ctx.scratch.get_mut(x, y).region = scratch_id;
        }
    }
}

/// Paint one fill rectangle: claim each in-bounds cell for `region` and set its
/// level.
fn fill(ctx: &mut CarveCtx<'_>, xs: (i32, i32), ys: (i32, i32), region: i32, level: u8) {
    for y in ys.0..ys.1 {
        for x in xs.0..xs.1 {
            if !ctx.scratch.in_diamond(x, y) {
                continue;
            }
            ctx.scratch.get_mut(x, y).region = region;
            ctx.grid.cell_native_mut(x, y).level = level;
        }
    }
}

/// Carve the east-west stair whose south side is clear.
///
/// `a` is the west endpoint and `b` the east one; the shape guarantees
/// `b.x > a.x`. Returns whether anything was carved — a refusal writes no cell
/// and takes no draw, which is what lets the caller retry cheaply.
pub fn carve_straight_ramp_clear_south(
    ctx: &mut CarveCtx<'_>,
    regions: CarveRegions,
    a: (i32, i32),
    b: (i32, i32),
) -> bool {
    let (min_y, max_y) = (a.1.min(b.1), a.1.max(b.1));
    let rect = (a.0 - 5, min_y - 3, (b.0 - a.0) + 9, (max_y - min_y) + 11);
    if !rect_is_carveable(
        ctx.grid,
        ctx.scratch,
        ctx.ids,
        ctx.playfield,
        rect,
        regions.region,
        regions.lower_region,
    ) {
        return false;
    }

    let dy = b.1 - a.1;
    let run = dy.abs();
    // Not enough length to fit the stair's rise. Refused before any draw.
    if (b.0 - a.0) - RUN_CLEARANCE < run {
        return false;
    }

    let half = ((b.0 - a.0) + 1) / 2 + 3;
    let (rx, ry, _, rh) = rect;
    let bottom = ry + rh + 2;

    // The lower plateau takes the two southern rectangles, with its own id and
    // its own level; the upper plateau takes the two northern ones. Both axes
    // split together — see the module note.
    fill(
        ctx,
        (rx, rx + 2 + half),
        (a.1 + 1, bottom),
        regions.lower_region,
        regions.lower_level,
    );
    fill(
        ctx,
        (b.0 - half + 3, b.0 + 5),
        (b.1 + 1, bottom),
        regions.lower_region,
        regions.lower_level,
    );
    fill(
        ctx,
        (rx + 4, rx + 2 + half),
        (ry + 2, a.1 + 1),
        regions.region,
        regions.level,
    );
    fill(
        ctx,
        (b.0 - half + 2, b.0 + 1),
        (ry + 2, b.1 + 1),
        regions.region,
        regions.level,
    );

    // The two end blocks. Consecutive tiles from the ramp-end base, which is
    // NOT the base the stepped edge below uses.
    let end_base = ctx.ramp_end_block;
    stamp_iso_block(ctx, end_base, a, regions.lower_region, Some(regions.level));
    stamp_iso_block(
        ctx,
        end_base + 1,
        (b.0 - 2, b.1),
        regions.lower_region,
        Some(regions.level),
    );

    let record = ramp_record(RampShape::ClearSouth).expect("straight shape has a table");
    let mut shift = (0i32, 0i32);

    if dy != 0 {
        // The one draw. It decides whether the flat run is displaced to meet
        // the stair, or the stair is slid along to meet the run.
        let coin = super::connector::jitter(ctx.rng);
        let ascending = dy > 0;
        let y_adjust = if dy >= 0 { 0 } else { -1 };
        let mut x_slide = 0;
        if coin == 1 {
            shift = (run, dy);
        } else {
            x_slide = (b.0 - run - a.0) - RUN_CLEARANCE;
        }
        let slopes = record.slopes(dy);

        for i in 0..run {
            // x advances with the outer counter and y with the inner — the
            // stair walks east one column at a time, each column a full run of
            // steps top to bottom.
            let lift = if ascending { i } else { -i };
            for k in 0..RAMP_STEPS as i32 {
                let x = a.0 + 3 + i + x_slide;
                let y = a.1 + k + y_adjust + lift;
                let slope = slopes[k as usize];
                let cell = ctx.grid.cell_native_mut(x, y);
                cell.level = (record.level_steps[k as usize])
                    .wrapping_add(regions.level as i8)
                    .wrapping_sub(4) as u8;
                cell.slope = slope;
                cell.sub_tile = 0;
                cell.tile = ramp_tile(ctx.ids.ramp_base, slope);
            }
        }
    }

    // The flat run that leads onto the stair. Its scratch id is written
    // directly rather than through the claim helper, and it is the lower
    // plateau's — the run belongs to the ground you walk in from.
    let flat_len = (b.0 - a.0) - run - RUN_CLEARANCE;
    for i in 0..flat_len {
        for k in 0..FLAT_RUN_DEPTH {
            let x = a.0 + 3 + i + shift.0;
            let y = a.1 + k + shift.1;
            if ctx.scratch.in_diamond(x, y) {
                ctx.scratch.get_mut(x, y).region = regions.lower_region;
            }
            let cell = ctx.grid.cell_native_mut(x, y);
            cell.slope = FLAT_RUN_SLOPE;
            cell.level = (regions.level as i8).wrapping_sub(k as i8).wrapping_sub(1) as u8;
            cell.tile = ctx.ids.ramp_base + FLAT_RUN_TILE_OFFSET;
            cell.sub_tile = 0;
        }
    }

    true
}

/// Carve the north-south stair whose east side is clear.
///
/// The mirror of [`carve_straight_ramp_clear_south`] across the isometric
/// diagonal: the run is along Y rather than X, so `a` is the **south** endpoint
/// and `b` the north one, and the stepped edge advances its column with the
/// inner counter instead of the outer.
///
/// Two constants genuinely differ rather than just swapping axes, and both were
/// transcribed rather than assumed: the half-span carries **no `+3`** here, and
/// the end blocks are the *next pair* of tiles after the east-west routine's.
pub fn carve_straight_ramp_clear_east(
    ctx: &mut CarveCtx<'_>,
    regions: CarveRegions,
    a: (i32, i32),
    b: (i32, i32),
) -> bool {
    let (min_x, max_x) = (a.0.min(b.0), a.0.max(b.0));
    let rect = (min_x - 3, b.1 - 4, (max_x - min_x) + 11, (a.1 - b.1) + 9);
    if !rect_is_carveable(
        ctx.grid,
        ctx.scratch,
        ctx.ids,
        ctx.playfield,
        rect,
        regions.region,
        regions.lower_region,
    ) {
        return false;
    }

    let dx = b.0 - a.0;
    let run = dx.abs();
    let length = a.1 - b.1;
    if length - RUN_CLEARANCE < run {
        return false;
    }

    // No `+ 3` — the east-west routine adds one and this one does not.
    let half = (length + 1) / 2;
    let (rx, ry, _, _) = rect;

    // Same pairing as the east-west routine, whose split was read out of the
    // assembly: the two rectangles that take the lower plateau's level take its
    // id as well, and the other two take the caller's.
    fill(
        ctx,
        (a.0 + 1, a.0 + 8),
        (a.1 - half, a.1 + 5),
        regions.lower_region,
        regions.lower_level,
    );
    fill(
        ctx,
        (b.0 + 1, b.0 + 8),
        (ry, b.1 + half),
        regions.lower_region,
        regions.lower_level,
    );
    fill(
        ctx,
        (rx + 2, a.0 + 1),
        (a.1 - half, a.1 + 1),
        regions.region,
        regions.level,
    );
    fill(
        ctx,
        (rx + 2, b.0 + 1),
        (b.1, b.1 + half),
        regions.region,
        regions.level,
    );

    // The next pair of end-block tiles after the east-west routine's, not the
    // same two.
    let end_base = ctx.ramp_end_block;
    stamp_iso_block(
        ctx,
        end_base + 2,
        (a.0, a.1 - 2),
        regions.lower_region,
        Some(regions.level),
    );
    stamp_iso_block(
        ctx,
        end_base + 3,
        b,
        regions.lower_region,
        Some(regions.level),
    );

    let record = ramp_record(RampShape::ClearEast).expect("straight shape has a table");
    let mut shift = (0i32, 0i32);

    if dx != 0 {
        let coin = super::connector::jitter(ctx.rng);
        let eastward = dx > 0;
        let x_adjust = if dx >= 0 { 0 } else { -1 };
        let mut y_slide = 0;
        if coin == 1 {
            shift = (dx, -run);
        } else {
            y_slide = -((a.1 - run - b.1) - RUN_CLEARANCE);
        }
        let slopes = record.slopes(dx);

        for i in 0..run {
            // Mirrored from the east-west routine: here y advances with the
            // outer counter and x with the inner, so each ROW of the stair is a
            // full run of steps.
            let lift = if eastward { i } else { -i };
            for k in 0..RAMP_STEPS as i32 {
                let x = a.0 + k + x_adjust + lift;
                let y = a.1 - 3 - i + y_slide;
                let slope = slopes[k as usize];
                let cell = ctx.grid.cell_native_mut(x, y);
                cell.level = (record.level_steps[k as usize])
                    .wrapping_add(regions.level as i8)
                    .wrapping_sub(4) as u8;
                cell.slope = slope;
                cell.sub_tile = 0;
                cell.tile = ramp_tile(ctx.ids.ramp_base, slope);
            }
        }
    }

    // The flat run. Its slope byte and tile offset are one lower than the
    // east-west routine's — a different face of the same stair set.
    let flat_len = length - run - RUN_CLEARANCE;
    for i in 0..flat_len {
        for k in 0..FLAT_RUN_DEPTH {
            let x = a.0 + k + shift.0;
            let y = a.1 - 3 - i + shift.1;
            if ctx.scratch.in_diamond(x, y) {
                ctx.scratch.get_mut(x, y).region = regions.lower_region;
            }
            let cell = ctx.grid.cell_native_mut(x, y);
            cell.slope = FLAT_RUN_SLOPE - 1;
            cell.level = (regions.level as i8).wrapping_sub(k as i8).wrapping_sub(1) as u8;
            cell.tile = ctx.ids.ramp_base + FLAT_RUN_TILE_OFFSET - 1;
            cell.sub_tile = 0;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::rmg::phases::shore::{SubTile, TileBlock};
    use crate::map::rmg::tiles::SpecialTerrain;
    use std::cell::RefCell;

    const REGION: i32 = 7;
    const REGION_LEVEL: u8 = 8;
    const LOWER: i32 = 3;
    const LOWER_LEVEL: u8 = 4;
    const END_BASE: i32 = 900;

    /// A one-cell block for every tile, recording which tiles were asked for.
    struct RecordingBlocks {
        block: TileBlock,
        seen: RefCell<Vec<i32>>,
    }

    impl RecordingBlocks {
        fn new() -> Self {
            Self {
                block: TileBlock {
                    width: 1,
                    height: 1,
                    subtiles: vec![Some(SubTile {
                        height: 0,
                        terrain: 0,
                        slope: 0,
                    })],
                },
                seen: RefCell::new(Vec::new()),
            }
        }
    }

    impl TileBlocks for RecordingBlocks {
        fn block(&self, tile: i32) -> Option<&TileBlock> {
            self.seen.borrow_mut().push(tile);
            Some(&self.block)
        }
    }

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

    /// Flat clear ground, every cell owned by the upper region at its level.
    fn harness() -> (RmgGrid, RmgScratch) {
        let (dmin, dmax) = (34, 34 + 2 * 42);
        let stride = (34 + 42 + 1) as usize;
        let mut grid = RmgGrid::new(stride, dmin, dmax);
        let mut scratch = RmgScratch::new(stride, dmin, dmax);
        for (x, y) in grid.native_cells().collect::<Vec<_>>() {
            let cell = grid.get_mut(x, y).expect("native cell");
            cell.tile = 0;
            cell.level = REGION_LEVEL;
            scratch.get_mut(x, y).region = REGION;
        }
        (grid, scratch)
    }

    fn regions() -> CarveRegions {
        CarveRegions {
            region: REGION,
            level: REGION_LEVEL,
            lower_region: LOWER,
            lower_level: LOWER_LEVEL,
        }
    }

    #[test]
    fn the_fill_rectangles_split_between_the_two_plateaus() {
        // The bug this exists to catch: handing the whole corridor to one
        // region. Both the id AND the level split, in matched pairs, so a cell
        // in the southern fill must carry the LOWER region id and level while
        // one in the northern fill carries the upper one.
        let (mut grid, mut scratch) = harness();
        let ids = ids();
        let blocks = RecordingBlocks::new();
        let mut rng = RmgRng::new(1);
        let playfield = Playfield::from_local_size(34, 0, 0, 34, 42);
        let mut ctx = CarveCtx {
            grid: &mut grid,
            scratch: &mut scratch,
            ids: &ids,
            blocks: &blocks,
            rng: &mut rng,
            playfield: &playfield,
            ramp_end_block: END_BASE,
        };
        assert!(carve_straight_ramp_clear_south(
            &mut ctx,
            regions(),
            (40, 50),
            (52, 50)
        ));

        // (38, 55) is in the southern fill only.
        assert_eq!(ctx.scratch.get(38, 55).region, LOWER, "south id");
        assert_eq!(
            ctx.grid.cell_native(38, 55).level,
            LOWER_LEVEL,
            "south level"
        );
        // (40, 49) is in the northern fill only.
        assert_eq!(ctx.scratch.get(40, 49).region, REGION, "north id");
        assert_eq!(
            ctx.grid.cell_native(40, 49).level,
            REGION_LEVEL,
            "north level"
        );
    }

    #[test]
    fn the_two_end_blocks_are_consecutive_tiles_from_the_ramp_end_base() {
        // Not the stepped edge base -- a different tileset entirely.
        let (mut grid, mut scratch) = harness();
        let ids = ids();
        let blocks = RecordingBlocks::new();
        let mut rng = RmgRng::new(1);
        let playfield = Playfield::from_local_size(34, 0, 0, 34, 42);
        let mut ctx = CarveCtx {
            grid: &mut grid,
            scratch: &mut scratch,
            ids: &ids,
            blocks: &blocks,
            rng: &mut rng,
            playfield: &playfield,
            ramp_end_block: END_BASE,
        };
        assert!(carve_straight_ramp_clear_south(
            &mut ctx,
            regions(),
            (40, 50),
            (52, 50)
        ));
        assert_eq!(
            blocks.seen.borrow().as_slice(),
            &[END_BASE, END_BASE + 1],
            "two stamps, consecutive, in order"
        );
        assert_ne!(END_BASE, ids.ramp_base, "the fixture keeps the bases apart");
    }

    #[test]
    fn a_level_run_takes_no_draw_and_a_sloped_one_takes_exactly_one() {
        // The draw is conditional on the endpoints differing in row. Making it
        // unconditional, or dropping it, desynchronises every later phase.
        for (b, expected_draws) in [((52, 50), 0u32), ((52, 52), 1u32)] {
            let (mut grid, mut scratch) = harness();
            let ids = ids();
            let blocks = RecordingBlocks::new();
            let mut rng = RmgRng::new(1);
            let playfield = Playfield::from_local_size(34, 0, 0, 34, 42);
            let mut ctx = CarveCtx {
                grid: &mut grid,
                scratch: &mut scratch,
                ids: &ids,
                blocks: &blocks,
                rng: &mut rng,
                playfield: &playfield,
                ramp_end_block: END_BASE,
            };
            assert!(carve_straight_ramp_clear_south(
                &mut ctx,
                regions(),
                (40, 50),
                b
            ));
            let mut probe = RmgRng::new(1);
            for _ in 0..expected_draws {
                probe.next_u32();
            }
            assert_eq!(
                ctx.rng.next_u32(),
                probe.next_u32(),
                "b = {b:?} should take {expected_draws} draw(s)"
            );
        }
    }

    #[test]
    fn both_refusals_write_nothing_and_draw_nothing() {
        // A cheap refusal is what makes the caller hundred-attempt loop
        // affordable. If either path drew, the retry loop would burn the
        // stream.
        // The third case sits ONE cell past the length limit. Both other cases
        // are far from it, so on their own they leave the comparison free to
        // slide by one in either direction; a stair that is one step too steep
        // for its run would then get carved.
        for (label, blocker, b) in [
            ("rect not carveable", true, (52, 50)),
            ("run far too short for the rise", false, (47, 58)),
            ("run exactly one past the limit", false, (46, 52)),
        ] {
            let (mut grid, mut scratch) = harness();
            let ids = ids();
            if blocker {
                grid.get_mut(41, 49).expect("native cell").tile = 500;
            }
            let blocks = RecordingBlocks::new();
            let mut rng = RmgRng::new(1);
            let playfield = Playfield::from_local_size(34, 0, 0, 34, 42);
            let mut ctx = CarveCtx {
                grid: &mut grid,
                scratch: &mut scratch,
                ids: &ids,
                blocks: &blocks,
                rng: &mut rng,
                playfield: &playfield,
                ramp_end_block: END_BASE,
            };
            assert!(
                !carve_straight_ramp_clear_south(&mut ctx, regions(), (40, 50), b),
                "{label}: must refuse"
            );
            let mut fresh = RmgRng::new(1);
            assert_eq!(ctx.rng.next_u32(), fresh.next_u32(), "{label}: no draw");
            assert!(blocks.seen.borrow().is_empty(), "{label}: no stamp");
            assert_eq!(
                ctx.scratch.get(38, 55).region,
                REGION,
                "{label}: no cell claimed"
            );
        }
    }

    #[test]
    fn the_stepped_edge_descends_down_a_column_not_along_a_row() {
        // The axis assignment: x advances with the OUTER counter and y with the
        // inner, so each column of the stair is a full run of steps top to
        // bottom. Swapping them lays the stair on its side and is invisible in
        // any test that only checks which cells were touched.
        //
        // Endpoints chosen so the flat run is empty and the slide is zero,
        // which puts the stepped edge in one known column whatever the coin
        // comes up as.
        let (mut grid, mut scratch) = harness();
        let ids = ids();
        let blocks = RecordingBlocks::new();
        let mut rng = RmgRng::new(1);
        let playfield = Playfield::from_local_size(34, 0, 0, 34, 42);
        let mut ctx = CarveCtx {
            grid: &mut grid,
            scratch: &mut scratch,
            ids: &ids,
            blocks: &blocks,
            rng: &mut rng,
            playfield: &playfield,
            ramp_end_block: END_BASE,
        };
        assert!(carve_straight_ramp_clear_south(
            &mut ctx,
            regions(),
            (40, 50),
            (46, 51)
        ));
        // Going DOWN the column, the stair steps 3,2,1,0,0 above level-4.
        let expected = [7u8, 6, 5, 4, 4];
        for (k, want) in expected.iter().enumerate() {
            assert_eq!(
                ctx.grid.cell_native(43, 50 + k as i32).level,
                *want,
                "column cell {k}"
            );
        }
        // And the ascending slope row, tiles one below the slope value.
        let slopes = [12u8, 16, 16, 16, 8];
        for (k, slope) in slopes.iter().enumerate() {
            let cell = ctx.grid.cell_native(43, 50 + k as i32);
            assert_eq!(cell.slope, *slope, "slope {k}");
            assert_eq!(cell.tile, 200 + i32::from(*slope) - 1, "tile {k}");
        }
    }

    /// Build a context over a fresh flat harness.
    macro_rules! ctx_over {
        ($grid:ident, $scratch:ident, $ids:ident, $blocks:ident, $rng:ident, $pf:ident) => {
            CarveCtx {
                grid: &mut $grid,
                scratch: &mut $scratch,
                ids: &$ids,
                blocks: &$blocks,
                rng: &mut $rng,
                playfield: &$pf,
                ramp_end_block: END_BASE,
            }
        };
    }

    #[test]
    fn the_north_south_stair_splits_ownership_the_same_way() {
        let (mut grid, mut scratch) = harness();
        let ids = ids();
        let blocks = RecordingBlocks::new();
        let mut rng = RmgRng::new(1);
        let pf = Playfield::from_local_size(34, 0, 0, 34, 42);
        let mut ctx = ctx_over!(grid, scratch, ids, blocks, rng, pf);
        assert!(carve_straight_ramp_clear_east(
            &mut ctx,
            regions(),
            (45, 52),
            (45, 40)
        ));
        // (51, 54) falls in the southern fill only.
        assert_eq!(ctx.scratch.get(51, 54).region, LOWER, "south id");
        assert_eq!(
            ctx.grid.cell_native(51, 54).level,
            LOWER_LEVEL,
            "south level"
        );
        // (44, 51) falls in the western fill only.
        assert_eq!(ctx.scratch.get(44, 51).region, REGION, "upper id");
        assert_eq!(
            ctx.grid.cell_native(44, 51).level,
            REGION_LEVEL,
            "upper level"
        );
    }

    #[test]
    fn the_north_south_stair_uses_the_second_pair_of_end_blocks() {
        // Four straight routines, eight consecutive end-block tiles, two each.
        // Reusing the east-west pair here would stamp the wrong faces.
        let (mut grid, mut scratch) = harness();
        let ids = ids();
        let blocks = RecordingBlocks::new();
        let mut rng = RmgRng::new(1);
        let pf = Playfield::from_local_size(34, 0, 0, 34, 42);
        let mut ctx = ctx_over!(grid, scratch, ids, blocks, rng, pf);
        assert!(carve_straight_ramp_clear_east(
            &mut ctx,
            regions(),
            (45, 52),
            (45, 40)
        ));
        assert_eq!(
            blocks.seen.borrow().as_slice(),
            &[END_BASE + 2, END_BASE + 3],
            "the second pair, in order"
        );
    }

    #[test]
    fn the_north_south_stepped_edge_runs_along_a_row_not_down_a_column() {
        // The mirror of the east-west routine: here y advances with the OUTER
        // counter and x with the inner, so the stair steps across a row. If the
        // two routines shared an axis assignment, one of them would be lying
        // on its side.
        //
        // Endpoints chosen so the flat run is empty and the slide is zero
        // whatever the coin gives.
        let (mut grid, mut scratch) = harness();
        let ids = ids();
        let blocks = RecordingBlocks::new();
        let mut rng = RmgRng::new(1);
        let pf = Playfield::from_local_size(34, 0, 0, 34, 42);
        let mut ctx = ctx_over!(grid, scratch, ids, blocks, rng, pf);
        assert!(carve_straight_ramp_clear_east(
            &mut ctx,
            regions(),
            (45, 52),
            (46, 46)
        ));
        let expected = [7u8, 6, 5, 4, 4];
        for (k, want) in expected.iter().enumerate() {
            assert_eq!(
                ctx.grid.cell_native(45 + k as i32, 49).level,
                *want,
                "row cell {k}"
            );
        }
        // Record 1 ascending row -- a different set from the east-west stair.
        let slopes = [11u8, 15, 15, 15, 7];
        for (k, slope) in slopes.iter().enumerate() {
            let cell = ctx.grid.cell_native(45 + k as i32, 49);
            assert_eq!(cell.slope, *slope, "slope {k}");
            assert_eq!(cell.tile, 200 + i32::from(*slope) - 1, "tile {k}");
        }
    }

    #[test]
    fn the_north_south_stair_draws_once_and_only_when_it_leans() {
        for (b, draws) in [((45, 40), 0u32), ((47, 40), 1u32)] {
            let (mut grid, mut scratch) = harness();
            let ids = ids();
            let blocks = RecordingBlocks::new();
            let mut rng = RmgRng::new(1);
            let pf = Playfield::from_local_size(34, 0, 0, 34, 42);
            let mut ctx = ctx_over!(grid, scratch, ids, blocks, rng, pf);
            assert!(carve_straight_ramp_clear_east(
                &mut ctx,
                regions(),
                (45, 52),
                b
            ));
            let mut probe = RmgRng::new(1);
            for _ in 0..draws {
                probe.next_u32();
            }
            assert_eq!(
                ctx.rng.next_u32(),
                probe.next_u32(),
                "b = {b:?} should take {draws} draw(s)"
            );
        }
    }

    #[test]
    fn the_north_south_refusal_boundary_is_pinned_on_both_sides() {
        // Exactly at the limit must carve; one past it must not. Neither the
        // length nor the lean is free to slide a cell.
        // Sited well inside the map so the length rule is what decides.
        // Nearer the edge the rect check fires first and the test passes for
        // the wrong reason.
        let cases = [((46, 46), true), ((47, 46), false)];
        for (b, should_carve) in cases {
            let (mut grid, mut scratch) = harness();
            let ids = ids();
            let blocks = RecordingBlocks::new();
            let mut rng = RmgRng::new(1);
            let pf = Playfield::from_local_size(34, 0, 0, 34, 42);
            let mut ctx = ctx_over!(grid, scratch, ids, blocks, rng, pf);
            let carved = carve_straight_ramp_clear_east(&mut ctx, regions(), (45, 52), b);
            assert_eq!(carved, should_carve, "b = {b:?}");
            if !carved {
                let mut fresh = RmgRng::new(1);
                assert_eq!(ctx.rng.next_u32(), fresh.next_u32(), "refusal took a draw");
                assert!(blocks.seen.borrow().is_empty(), "refusal stamped");
            }
        }
    }

    #[test]
    fn the_north_south_half_span_carries_no_extra_three() {
        // The two routines compute their half-span differently -- the
        // east-west one adds three and this one does not. In most geometries
        // the difference is invisible, because the four fill rectangles overlap
        // and the pairs write identical values. It shows only where a LATER
        // rectangle would reach a cell an earlier one already claimed for the
        // other plateau.
        //
        // Leaning stair with an empty flat run, so nothing downstream can
        // repaint the probe: (47, 50) is claimed by the southern fill at the
        // real half-span, and by the upper-plateau fill at the inflated one.
        let (mut grid, mut scratch) = harness();
        let ids = ids();
        let blocks = RecordingBlocks::new();
        let mut rng = RmgRng::new(1);
        let pf = Playfield::from_local_size(34, 0, 0, 34, 42);
        let mut ctx = ctx_over!(grid, scratch, ids, blocks, rng, pf);
        assert!(carve_straight_ramp_clear_east(
            &mut ctx,
            regions(),
            (45, 52),
            (47, 45)
        ));
        assert_eq!(ctx.scratch.get(47, 50).region, LOWER, "half-span id");
        assert_eq!(
            ctx.grid.cell_native(47, 50).level,
            LOWER_LEVEL,
            "half-span level"
        );
    }

    #[test]
    fn each_flat_run_stamps_its_own_face_of_the_stair() {
        // The two routines lay different faces: the east-west run takes slope 4
        // and the tile three above the base, the north-south run takes slope 3
        // and the tile two above. Sharing either constant puts one stair's
        // approach on the wrong face, which reads as a seam in the ground.
        let pf = Playfield::from_local_size(34, 0, 0, 34, 42);

        let (mut grid, mut scratch) = harness();
        let ids1 = ids();
        let blocks = RecordingBlocks::new();
        let mut rng = RmgRng::new(1);
        let mut ctx = ctx_over!(grid, scratch, ids1, blocks, rng, pf);
        assert!(carve_straight_ramp_clear_south(
            &mut ctx,
            regions(),
            (40, 50),
            (52, 50)
        ));
        let cell = ctx.grid.cell_native(45, 51);
        assert_eq!(cell.slope, 4, "east-west flat slope");
        assert_eq!(cell.tile, 203, "east-west flat tile");
        assert_eq!(cell.level, 6, "east-west flat level");

        let (mut grid, mut scratch) = harness();
        let ids2 = ids();
        let blocks = RecordingBlocks::new();
        let mut rng = RmgRng::new(1);
        let mut ctx = ctx_over!(grid, scratch, ids2, blocks, rng, pf);
        assert!(carve_straight_ramp_clear_east(
            &mut ctx,
            regions(),
            (45, 52),
            (45, 40)
        ));
        let cell = ctx.grid.cell_native(46, 47);
        assert_eq!(cell.slope, 3, "north-south flat slope");
        assert_eq!(cell.tile, 202, "north-south flat tile");
        assert_eq!(cell.level, 6, "north-south flat level");
    }
}
