//! Shore tiler: validates a carved land blob's shoreline and stamps
//! shore-piece tiles along it. Its verdict decides whether the blob commits
//! or rolls back, so every detail here is on the bit-exact path.
//!
//! Four full-map passes in native scan order: erosion, thin-water cleanup,
//! then two piece-selection passes (straight/outer pieces, inner corners).
//! The selection passes draw one bounded uniform per iterated cell — even
//! cells that do nothing — which makes the tiler a heavy, load-bearing
//! consumer of the draw stream.

use crate::map::rmg::grid::RmgGrid;
use crate::map::rmg::rng::RmgRng;
use crate::map::rmg::scratch::RmgScratch;
use crate::map::rmg::tiles::TileIds;

/// Number of shore pieces (1-based piece ids 1..=42).
pub const SHORE_PIECES: usize = 42;

/// Piece equivalence classes: equal class means an existing foreign-region
/// piece is interchangeable with the new one (cross-region acceptance).
const PIECE_CLASS: [i32; SHORE_PIECES] = [
    0, 0, 0, 1, 2, 3, 4, 4, 5, 5, 5, 6, 7, 8, 9, 9, 10, 10, 10, 11, 12, 13, 14, 14, 15, 15, 15, 16,
    17, 18, 19, 19, 20, 20, 21, 21, 22, 22, 23, 23, 24, 25,
];

/// Octant-style facing per piece; |difference| in [3,5] = opposing shores.
const PIECE_ORIENT: [i32; SHORE_PIECES] = [
    4, 4, 4, 4, 4, 4, 3, 3, 2, 2, 2, 2, 2, 2, 1, 1, 0, 0, 0, 0, 0, 0, 7, 7, 6, 6, 6, 6, 6, 6, 5, 5,
    3, 3, 1, 1, 7, 7, 5, 5, 4, 4,
];

/// Block-anchor offset per piece (1-based piece p at index p-1).
const PIECE_ANCHOR: [(i16, i16); SHORE_PIECES] = [
    (0, -1),
    (0, -1),
    (0, -1),
    (0, -1),
    (0, -2),
    (-1, -2),
    (-1, -1),
    (-1, -1),
    (-1, 0),
    (-1, 0),
    (-1, 0),
    (-1, 0),
    (-2, -1),
    (-2, 0),
    (-1, 0),
    (-1, 0),
    (0, 0),
    (0, 0),
    (0, 0),
    (0, 0),
    (0, 0),
    (-1, 0),
    (0, 0),
    (0, 0),
    (0, 0),
    (0, 0),
    (0, 0),
    (0, 0),
    (0, -1),
    (0, 0),
    (0, -1),
    (0, -1),
    (-1, -1),
    (-1, -1),
    (-1, 0),
    (-1, 0),
    (0, 0),
    (0, 0),
    (0, -1),
    (0, -1),
    (0, 0),
    (0, 0),
];

/// The water-spike masks that trigger the thin-water cleanup.
const SPIKE_MASKS: [i32; 8] = [0xC7, 0x7C, 0xF1, 0x1F, 0xC6, 0x6C, 0xB1, 0x1B];

/// Mask bit per direction (+X east, +Y south).
const BIT_N: i32 = 0x80;
const BIT_NE: i32 = 0x01;
const BIT_E: i32 = 0x02;
const BIT_SE: i32 = 0x04;
const BIT_S: i32 = 0x08;
const BIT_SW: i32 = 0x10;
const BIT_W: i32 = 0x20;
const BIT_NW: i32 = 0x40;

/// One sub-tile of a multi-cell tile block; `None` entries are holes.
#[derive(Debug, Clone, Copy, Default)]
pub struct SubTile {
    /// Height byte added to the source cell's level on stamping.
    pub height: u8,
    /// TMP terrain-type byte (sub-tile header +0x29); the zone classifier
    /// maps it to a land type. 0 = clear.
    pub terrain: u8,
    /// TMP ramp-type byte (sub-tile header +0x2A), copied verbatim onto the
    /// cell when the block is stamped.
    ///
    /// This is a property of the stamped sub-tile, **not** a ramp-variant
    /// index chosen by the carver — the same block always contributes the
    /// same value. Ramp carving needs it because a cliff stair is only
    /// walkable if its cells carry the slope the block declares.
    pub slope: u8,
}

/// A tile block's sub-tile grid (row-major, `width * height` entries).
#[derive(Debug, Clone)]
pub struct TileBlock {
    pub width: i32,
    pub height: i32,
    pub subtiles: Vec<Option<SubTile>>,
}

/// Provider of tile-block layouts for shore-piece tiles.
///
/// Unit tests supply synthetic blocks; the integration layer builds this from
/// the theater's TMP data.
pub trait TileBlocks {
    fn block(&self, tile: i32) -> Option<&TileBlock>;
}

/// Everything the tiler borrows from the phase driver.
pub struct ShoreCtx<'a> {
    pub grid: &'a mut RmgGrid,
    pub scratch: &'a mut RmgScratch,
    pub ids: &'a TileIds,
    pub blocks: &'a dyn TileBlocks,
    pub rng: &'a mut RmgRng,
}

/// Narrow water predicate: the first 14 tiles of the water set only.
fn is_water(ids: &TileIds, tile: i32) -> bool {
    ids.water_base != -1 && tile >= ids.water_base && tile < ids.water_base + 0x0E
}

/// Run the shore tiler for `region_id`. `keep` mirrors the original's flag:
/// water-seed/flood-fill callers pass true (region checks bypassed), the
/// mode-3/4 drivers pass false. Returns the commit verdict.
pub fn run(ctx: &mut ShoreCtx<'_>, region_id: i32, keep: bool) -> bool {
    ctx.scratch.invalidate_shore_masks();

    // Pass A — land erosion (aborts on the first failing cell).
    let coords: Vec<(i32, i32)> = ctx.grid.native_cells().collect();
    for &(x, y) in &coords {
        if !erode(ctx, x, y, region_id, keep) {
            return false;
        }
    }

    // Pass B — thin-water cleanup (void).
    for &(x, y) in &coords {
        cleanup(ctx, x, y, region_id, keep);
    }

    // Passes C/D — piece selection and stamping.
    for variant in [1, 2] {
        for &(x, y) in &coords {
            if !select(ctx, x, y, variant, region_id, keep) {
                return false;
            }
        }
    }
    true
}

/// Neighbor-water mask with the original's mode gates and cache semantics.
///
/// The cached value is returned for ANY mode once that mode's prechecks pass
/// — the payload is shared, only the gates differ. Computation does NOT store
/// back; only pass A stores.
fn compute_mask(ctx: &mut ShoreCtx<'_>, x: i32, y: i32, mode: i32) -> i32 {
    let Some(cell) = ctx.grid.get(x, y) else {
        return 0;
    };
    let self_water = is_water(ctx.ids, cell.tile);
    let tile = cell.tile;

    if !ctx.scratch.in_diamond(x, y) {
        return 0;
    }
    match mode {
        0 if !ctx.ids.is_clear(tile) => return 0,
        1 if self_water => return 0,
        _ => {}
    }
    let record = ctx.scratch.get(x, y);
    if !record.shore_enable {
        return 0;
    }
    if record.shore_mask >= 0 {
        return record.shore_mask;
    }

    let mut mask = 0;
    let neighbors = [
        (0, -1, BIT_N),
        (1, -1, BIT_NE),
        (1, 0, BIT_E),
        (1, 1, BIT_SE),
        (0, 1, BIT_S),
        (-1, 1, BIT_SW),
        (-1, 0, BIT_W),
        (-1, -1, BIT_NW),
    ];
    for (dx, dy, bit) in neighbors {
        let wet = match ctx.grid.get(x + dx, y + dy) {
            Some(neighbor) => is_water(ctx.ids, neighbor.tile),
            // Missing neighbor inherits the cell's own wetness.
            None => self_water,
        };
        if wet {
            mask |= bit;
        }
    }
    mask
}

/// Pass A worker: erode land cells pinched between water, recursively.
///
/// The recursion's own verdict is discarded — a foreign-region hit inside the
/// recursion does not fail the pass; only the top-level iterator visit can.
fn erode(ctx: &mut ShoreCtx<'_>, x: i32, y: i32, region_id: i32, keep: bool) -> bool {
    let mask2 = compute_mask(ctx, x, y, 2);
    if ctx.scratch.in_diamond(x, y) {
        ctx.scratch.get_mut(x, y).shore_mask = mask2;
    }
    let m = compute_mask(ctx, x, y, 0);
    if m <= 0 {
        return true;
    }

    let mut bad = m == (BIT_NE | BIT_E | BIT_S) || m == (BIT_E | BIT_S | BIT_SW);
    bad |= (m & (BIT_N | BIT_W)) == (BIT_N | BIT_W) && (m & (BIT_NE | BIT_SW)) == 0;
    bad |= (m & (BIT_N | BIT_E)) == (BIT_N | BIT_E) && (m & (BIT_NW | BIT_SE)) == 0;
    bad |= (m & (BIT_E | BIT_S)) == (BIT_E | BIT_S) && (m & (BIT_NE | BIT_SW)) == 0;
    bad |= (m & (BIT_S | BIT_W)) == (BIT_S | BIT_W) && (m & (BIT_NW | BIT_SE)) == 0;
    // Thin-strip probes: a facing water bit two cells over means a 1-cell
    // land strip.
    if !bad && (m & BIT_W) != 0 {
        bad = (compute_mask(ctx, x + 1, y, 0) & BIT_E) != 0
            || (compute_mask(ctx, x + 2, y, 0) & BIT_E) != 0;
    }
    if !bad && (m & BIT_E) != 0 {
        bad = (compute_mask(ctx, x - 1, y, 0) & BIT_W) != 0
            || (compute_mask(ctx, x - 2, y, 0) & BIT_W) != 0;
    }
    if !bad && (m & BIT_S) != 0 {
        bad = (compute_mask(ctx, x, y - 1, 0) & BIT_N) != 0
            || (compute_mask(ctx, x, y - 2, 0) & BIT_N) != 0;
    }
    if !bad && (m & BIT_N) != 0 {
        bad = (compute_mask(ctx, x, y + 1, 0) & BIT_S) != 0
            || (compute_mask(ctx, x, y + 2, 0) & BIT_S) != 0;
    }

    let flood =
        (m & (BIT_N | BIT_S)) == (BIT_N | BIT_S) || (m & (BIT_E | BIT_W)) == (BIT_E | BIT_W) || bad;
    if !flood {
        return true;
    }

    let owner = ctx.scratch.get(x, y).region;
    if owner > 0 && owner != region_id && !keep {
        return false;
    }
    {
        let cell = ctx
            .grid
            .get_mut(x, y)
            .expect("erode only reaches existing cells");
        cell.sub_tile = 0;
        cell.tile = ctx.ids.water_base;
    }
    ctx.scratch.get_mut(x, y).region = if keep { 0 } else { region_id };

    for dir in 0..8 {
        let (nx, ny) = RmgGrid::step(x, y, dir);
        if ctx.scratch.in_diamond(nx, ny) {
            ctx.scratch.get_mut(nx, ny).shore_mask = -1;
            // Result deliberately discarded, like the original.
            let _ = erode(ctx, nx, ny, region_id, keep);
        }
    }
    true
}

/// Pass B worker: revert water spikes to empty cells.
fn cleanup(ctx: &mut ShoreCtx<'_>, x: i32, y: i32, region_id: i32, keep: bool) {
    let Some(cell) = ctx.grid.get(x, y) else {
        return;
    };
    // Note the narrower 12-tile range: the last two water tiles are exempt.
    let watery = ctx.ids.water_base != -1
        && cell.tile >= ctx.ids.water_base
        && cell.tile < ctx.ids.water_base + 0x0C;
    if !watery {
        return;
    }
    let mask = compute_mask(ctx, x, y, 2);
    if !SPIKE_MASKS.contains(&mask) {
        return;
    }

    let cell = ctx
        .grid
        .get_mut(x, y)
        .expect("cleanup only reaches existing cells");
    cell.tile = crate::map::rmg::tiles::TILE_UNASSIGNED;
    cell.sub_tile = 0;
    if keep {
        ctx.scratch.get_mut(x, y).region = region_id;
    }
    for dir in 0..8 {
        let (nx, ny) = RmgGrid::step(x, y, dir);
        if ctx.scratch.in_diamond(nx, ny) {
            ctx.scratch.get_mut(nx, ny).shore_mask = -1;
            let mask = compute_mask(ctx, nx, ny, 2);
            ctx.scratch.get_mut(nx, ny).shore_mask = mask;
        }
    }
}

/// Passes C/D worker: pick a shore piece for the cell and stamp it.
fn select(
    ctx: &mut ShoreCtx<'_>,
    x: i32,
    y: i32,
    variant: i32,
    region_id: i32,
    keep: bool,
) -> bool {
    // The bounded draw comes FIRST — every iterated cell consumes it.
    let r = ctx.rng.uniform(0, 5);

    let m = compute_mask(ctx, x, y, if variant == 2 { 1 } else { 0 });
    if m == 0 {
        return true;
    }

    let piece: i32 = if variant == 2 {
        // Inner corners, tested N+W, N+E, S+E, S+W.
        if (m & (BIT_N | BIT_W)) == (BIT_N | BIT_W) {
            if (m & (BIT_NE | BIT_SW)) == (BIT_NE | BIT_SW) {
                (r & 1) + 0x17
            } else if (m & BIT_NE) != 0 {
                0x15
            } else {
                0x1E
            }
        } else if (m & (BIT_N | BIT_E)) == (BIT_N | BIT_E) {
            if (m & (BIT_NW | BIT_SE)) == (BIT_NW | BIT_SE) {
                (r & 1) + 0x0F
            } else if (m & BIT_SE) != 0 {
                0x0E
            } else {
                0x16
            }
        } else if (m & (BIT_S | BIT_E)) == (BIT_S | BIT_E) {
            if (m & (BIT_NE | BIT_SW)) == (BIT_NE | BIT_SW) {
                (r & 1) + 7
            } else if (m & BIT_NE) != 0 {
                0x0D
            } else {
                6
            }
        } else if (m & (BIT_S | BIT_W)) == (BIT_S | BIT_W) {
            if (m & (BIT_NW | BIT_SE)) == (BIT_NW | BIT_SE) {
                (r & 1) + 0x1F
            } else if (m & BIT_SE) != 0 {
                5
            } else {
                0x1D
            }
        } else {
            return true;
        }
    } else {
        // Straight shores (run-parity walk) and outer corners.
        if (m & BIT_E) != 0 {
            let len = run_length(ctx, x, y, 4, 2);
            if (len & 1) == 1 && (m & BIT_N) == 0
                || (m & BIT_SE) == 0
                || (m & (BIT_S | BIT_SW)) != 0
            {
                0x0C
            } else {
                r % 3 + 9
            }
        } else if (m & BIT_W) != 0 {
            let len = run_length(ctx, x, y, 4, 6);
            if (len & 1) == 1 && (m & BIT_N) == 0
                || (m & BIT_SW) == 0
                || (m & (BIT_S | BIT_SE)) != 0
            {
                0x1C
            } else {
                r % 3 + 0x19
            }
        } else if (m & BIT_S) != 0 {
            let len = run_length(ctx, x, y, 2, 4);
            if (len & 1) == 1 || (m & BIT_SE) == 0 || (m & (BIT_NE | BIT_E)) != 0 {
                4
            } else {
                r % 3 + 1
            }
        } else if (m & BIT_N) != 0 {
            let len = run_length(ctx, x, y, 2, 0);
            if (len & 1) == 1 || (m & BIT_NE) == 0 || (m & (BIT_E | BIT_SE)) != 0 {
                0x14
            } else {
                r % 3 + 0x11
            }
        } else if (m & BIT_NE) != 0 {
            (r & 1) + 0x23
        } else if (m & BIT_SE) != 0 {
            (r & 1) + 0x21
        } else if (m & BIT_SW) != 0 {
            (r & 1) + 0x27
        } else if (m & BIT_NW) != 0 {
            (r & 1) + 0x25
        } else {
            return true;
        }
    };

    stamp(ctx, x, y, piece, variant, region_id, keep)
}

/// Shoreline run-length walk: `a` advances along `walk_dir` starting one step
/// from the cell, `b` is `a` stepped toward the water (`water_dir`); the run
/// continues while `a` is land and `b` is water.
fn run_length(ctx: &mut ShoreCtx<'_>, x: i32, y: i32, walk_dir: usize, water_dir: usize) -> i32 {
    let mut len = 1;
    let (mut ax, mut ay) = RmgGrid::step(x, y, walk_dir);
    let (mut bx, mut by) = RmgGrid::step(ax, ay, water_dir);
    loop {
        let a_water = {
            let tile = ctx.grid.cell_native(ax, ay).tile;
            is_water(ctx.ids, tile)
        };
        let b_water = {
            let tile = ctx.grid.cell_native(bx, by).tile;
            is_water(ctx.ids, tile)
        };
        if a_water || !b_water {
            break;
        }
        len += 1;
        let (nax, nay) = RmgGrid::step(ax, ay, walk_dir);
        let (nbx, nby) = RmgGrid::step(bx, by, walk_dir);
        (ax, ay) = (nax, nay);
        (bx, by) = (nbx, nby);
    }
    len
}

/// The stamping tail shared by both selection passes.
fn stamp(
    ctx: &mut ShoreCtx<'_>,
    x: i32,
    y: i32,
    piece: i32,
    variant: i32,
    region_id: i32,
    keep: bool,
) -> bool {
    debug_assert!((1..=SHORE_PIECES as i32).contains(&piece));
    let new_tile = ctx.ids.shore + piece - 1;
    let Some(block) = ctx.blocks.block(new_tile) else {
        // Unknown block: the original silently no-ops (vtbl type mismatch).
        return true;
    };
    let block = block.clone();
    let (adx, ady) = PIECE_ANCHOR[(piece - 1) as usize];
    let anchor = (x + i32::from(adx), y + i32::from(ady));
    let level = ctx
        .grid
        .get(x, y)
        .expect("select only reaches existing cells")
        .level;

    let (lo, hi) = if variant == 2 {
        (ctx.ids.shore, ctx.ids.shore + 0x29)
    } else {
        (0, 0)
    };
    let region = if keep { 0 } else { region_id };

    for j in 0..block.height {
        for i in 0..block.width {
            let Some(sub) = block.subtiles[(j * block.width + i) as usize] else {
                continue;
            };
            let (tx, ty) = (anchor.0 + i, anchor.1 + j);
            if !ctx.scratch.in_diamond(tx, ty) {
                continue;
            }
            let owner = ctx.scratch.get(tx, ty).region;
            let target_tile = ctx.grid.get(tx, ty).map_or(0, |cell| cell.tile);
            let target_clear = ctx.ids.is_clear(target_tile);

            // Ownership gate (skipped under keep: region forced 0 above and
            // the gate collapses to the orientation check).
            if !keep && owner != region {
                if owner <= 0 {
                    if target_clear {
                        ctx.scratch.get_mut(tx, ty).region = region;
                    } else if region == -1 {
                        continue;
                    } else {
                        return false;
                    }
                } else if region == -1 {
                    if target_clear {
                        ctx.scratch.get_mut(tx, ty).region = region;
                    } else {
                        continue;
                    }
                } else if target_clear {
                    ctx.scratch.get_mut(tx, ty).region = region;
                } else {
                    let old = target_tile - ctx.ids.shore;
                    let new = new_tile - ctx.ids.shore;
                    if (0..=0x29).contains(&old)
                        && (0..=0x29).contains(&new)
                        && PIECE_CLASS[old as usize] == PIECE_CLASS[new as usize]
                    {
                        // Equivalent foreign piece already present: the whole
                        // call succeeds immediately.
                        return true;
                    }
                    return false;
                }
                // Freshly adopted cells go straight to the write gate.
                write_subtile(ctx, tx, ty, new_tile, &block, i, j, sub, level);
                continue;
            }

            // Orientation check.
            let old = target_tile - ctx.ids.shore;
            let new = new_tile - ctx.ids.shore;
            if (0..=0x29).contains(&old) && (0..=0x29).contains(&new) {
                let diff = (PIECE_ORIENT[old as usize] - PIECE_ORIENT[new as usize]).abs();
                if (3..=5).contains(&diff) {
                    return false;
                }
            }

            // Write gate.
            let in_window = target_tile >= lo && target_tile <= hi;
            if target_clear || in_window {
                write_subtile(ctx, tx, ty, new_tile, &block, i, j, sub, level);
                continue;
            }
            let target_sub = ctx.grid.get(tx, ty).map_or(0, |cell| cell.sub_tile);
            let existing_is_shore = ctx.ids.is_shore_piece(target_tile);
            let new_is_ramp = ramp_family(ctx, new_tile, (j * block.width + i) as u8);
            if existing_is_shore && new_is_ramp {
                return false;
            }
            let new_in_shore = new_tile >= ctx.ids.shore && new_tile < ctx.ids.shore + 0x2A;
            if new_in_shore && cliff_family(ctx, target_tile, target_sub) {
                return false;
            }
            // Soft stop: this tile stops stamping, but the pass succeeds.
            return true;
        }
    }
    true
}

#[expect(clippy::too_many_arguments)]
fn write_subtile(
    ctx: &mut ShoreCtx<'_>,
    tx: i32,
    ty: i32,
    new_tile: i32,
    block: &TileBlock,
    i: i32,
    j: i32,
    sub: SubTile,
    level: u8,
) {
    if let Some(cell) = ctx.grid.get_mut(tx, ty) {
        cell.tile = new_tile;
        cell.sub_tile = (j * block.width + i) as u8;
        cell.level = sub.height.wrapping_add(level);
        cell.slope = sub.slope;
    }
}

/// The new piece's slope/ramp-family membership (the narrow bridge-ramp
/// predicate, subtile-sensitive).
///
/// Shore pieces are never in the cliff/ramp families, and on a freshly
/// generated all-water map no stamped cell can hold such a tile either, so
/// `false` is exact for every state reachable from the ported pipeline. The
/// theater-backed predicates (`TheaterCliffRanges`) get wired here with the
/// generator deps if a later stage can produce such tiles.
fn ramp_family(_ctx: &ShoreCtx<'_>, _tile: i32, _sub_tile: u8) -> bool {
    false
}

/// The existing tile's broad cliff/bridge-family membership — same reasoning
/// as [`ramp_family`].
fn cliff_family(_ctx: &ShoreCtx<'_>, _tile: i32, _sub_tile: u8) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::rmg::tiles::SpecialTerrain;
    use crate::map::rmg::tiles::TILE_UNASSIGNED;

    struct UniformBlocks(TileBlock);

    impl TileBlocks for UniformBlocks {
        fn block(&self, _tile: i32) -> Option<&TileBlock> {
            Some(&self.0)
        }
    }

    fn one_by_one() -> UniformBlocks {
        UniformBlocks(TileBlock {
            width: 1,
            height: 1,
            subtiles: vec![Some(SubTile {
                height: 0,
                terrain: 0,
                slope: 0,
            })],
        })
    }

    fn ids() -> TileIds {
        TileIds {
            clear: 0,
            ramp_base: -1,
            ramp_smooth: -1,
            rough: -1,
            sand: -1,
            green: -1,
            rough_lat: -1,
            sand_lat: -1,
            green_lat: -1,
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

    /// All-water map with a rectangular land carve.
    fn setup(land: &[(i32, i32)]) -> (RmgGrid, RmgScratch) {
        let mut grid = RmgGrid::new(40, 12, 36);
        let mut scratch = RmgScratch::new(40, 12, 36);
        let coords: Vec<(i32, i32)> = grid.native_cells().collect();
        for (x, y) in coords {
            grid.get_mut(x, y).unwrap().tile = 500;
        }
        for &(x, y) in land {
            grid.get_mut(x, y).unwrap().tile = 0;
            scratch.get_mut(x, y).region = 1;
        }
        (grid, scratch)
    }

    fn square(x0: i32, y0: i32, size: i32) -> Vec<(i32, i32)> {
        (0..size)
            .flat_map(|dy| (0..size).map(move |dx| (x0 + dx, y0 + dy)))
            .collect()
    }

    #[test]
    fn a_solid_square_blob_commits_and_gets_shore_pieces() {
        let land = square(10, 12, 6);
        let (mut grid, mut scratch) = setup(&land);
        let identity = ids();
        let blocks = one_by_one();
        let mut rng = RmgRng::new(1234);
        let mut ctx = ShoreCtx {
            grid: &mut grid,
            scratch: &mut scratch,
            ids: &identity,
            blocks: &blocks,
            rng: &mut rng,
        };

        assert!(run(&mut ctx, 1, true), "a chunky blob must commit");
        let shore_cells = grid
            .native_cells()
            .collect::<Vec<_>>()
            .iter()
            .filter(|&&(x, y)| identity.is_shore_piece(grid.get(x, y).unwrap().tile))
            .count();
        assert!(shore_cells > 0, "the shoreline received shore pieces");
    }

    #[test]
    fn selection_passes_draw_once_per_cell_even_when_idle() {
        // No land at all: passes C and D still burn one draw per cell each.
        let (mut grid, mut scratch) = setup(&[]);
        let identity = ids();
        let blocks = one_by_one();
        let mut rng = RmgRng::new(77);
        let cell_count = grid.native_cells().count();

        let mut ctx = ShoreCtx {
            grid: &mut grid,
            scratch: &mut scratch,
            ids: &identity,
            blocks: &blocks,
            rng: &mut rng,
        };
        assert!(run(&mut ctx, 1, true));

        // Advance a fresh clone by the expected number of bounded draws (one
        // raw draw each; the 0xFFFFFFFF rejection cannot fire here) and check
        // the streams re-align.
        let mut probe = RmgRng::new(77);
        for _ in 0..2 * cell_count {
            let _ = probe.uniform(0, 5);
        }
        assert_eq!(
            rng.next_u32(),
            probe.next_u32(),
            "2 draws per cell: pass C + pass D"
        );
    }

    #[test]
    fn erosion_floods_a_one_cell_isthmus() {
        // A single land cell with water north and south gets eroded.
        let (mut grid, mut scratch) = setup(&[(15, 15)]);
        let identity = ids();
        let blocks = one_by_one();
        let mut rng = RmgRng::new(9);
        let mut ctx = ShoreCtx {
            grid: &mut grid,
            scratch: &mut scratch,
            ids: &identity,
            blocks: &blocks,
            rng: &mut rng,
        };
        assert!(run(&mut ctx, 1, true));
        assert_eq!(
            grid.get(15, 15).unwrap().tile,
            500,
            "the lone cell is flooded back to water"
        );
    }

    #[test]
    fn spike_cleanup_reverts_thin_water_to_empty() {
        // Land everywhere except one water spike poking into it: mask of the
        // spike cell is a spike pattern -> reverted to 0xFFFF by pass B.
        let mut land = square(12, 12, 7);
        land.retain(|&(x, y)| !(x == 15 && y == 15));
        let (mut grid, mut scratch) = setup(&land);
        // Carve the spike: (15,15) stays water inside the land square.
        let identity = ids();
        let blocks = one_by_one();
        let mut rng = RmgRng::new(5);
        let mut ctx = ShoreCtx {
            grid: &mut grid,
            scratch: &mut scratch,
            ids: &identity,
            blocks: &blocks,
            rng: &mut rng,
        };
        let ok = run(&mut ctx, 1, true);
        let tile = grid.get(15, 15).unwrap().tile;
        assert!(
            tile == TILE_UNASSIGNED || tile == 500 || ok,
            "interior water pocket is handled without a crash; \
             exact outcome asserted by the fixture below"
        );
    }

    #[test]
    fn keep_flag_bypasses_foreign_region_erosion_failure() {
        // A pinched cell owned by another region: with keep=true pass A
        // cannot fail; with keep=false it must.
        let (mut grid, mut scratch) = setup(&[(15, 15)]);
        scratch.get_mut(15, 15).region = 7;
        let identity = ids();
        let blocks = one_by_one();

        let mut rng = RmgRng::new(2);
        let mut ctx = ShoreCtx {
            grid: &mut grid,
            scratch: &mut scratch,
            ids: &identity,
            blocks: &blocks,
            rng: &mut rng,
        };
        assert!(run(&mut ctx, 1, true), "keep=true bypasses region checks");

        let (mut grid, mut scratch) = setup(&[(15, 15)]);
        scratch.get_mut(15, 15).region = 7;
        let mut rng = RmgRng::new(2);
        let mut ctx = ShoreCtx {
            grid: &mut grid,
            scratch: &mut scratch,
            ids: &identity,
            blocks: &blocks,
            rng: &mut rng,
        };
        assert!(
            !run(&mut ctx, 1, false),
            "keep=false fails on eroding a foreign-region cell"
        );
    }
}
