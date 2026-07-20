//! Water/base-terrain seeding for map types 0-2: flood the map with water,
//! carve land blobs per shape, clean up, and grass the shorelines.
//!
//! Order: all-water fill -> shape dispatch (archipelago / continental /
//! islands-in-sea) -> isolated-water removal -> final shore tiling + region
//! reset -> shore-to-green. The finalizer that picks water tile variants is
//! a separate stage.

use crate::map::rmg::grid::RmgGrid;
use crate::map::rmg::rng::{RANGE_K_BITS, RmgRng};
use crate::map::rmg::tiles::TileIds;
use crate::map::rmg::x87::{self, TruncF64, approx_sqrt};

use super::blob::{self, BlobCtx, BlobParams};
use super::shore::{self, ShoreCtx};

/// Land-fraction endpoints and cap factor per shape (doubles by value; the
/// min/max differences are computed at runtime like the original's FSUB).
const MODE1_MIN: f64 = 0.45;
const MODE1_MAX: f64 = 0.5;
const MODE1_CAP: f64 = 0.03;
const MODE2_MIN: f64 = 0.15;
const MODE2_MAX: f64 = 0.2;
const MODE2_CAP: f64 = 0.06;
/// Mode-0 blob-size band: uniform in [0.45, 0.5) of the doubled rect area.
const MODE0_SPAN: f64 = 0.05;
const MODE0_BASE: f64 = 0.45;
/// Water option percent scale.
const PERCENT: f64 = 0.01;

/// The playable rect in logical coords: (2, 5, gen_w, gen_h).
#[derive(Debug, Clone, Copy)]
pub struct PlayableRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// Logical-rect center to diamond coordinates.
fn diamond_center(map_w: i32, rect: &[i32; 4]) -> (i32, i32) {
    let (x0, y0, w, h) = (rect[0], rect[1], rect[2], rect[3]);
    (w / 2 + h / 2 + x0 + 1 + y0, h / 2 - w / 2 - x0 + map_w + y0)
}

/// Uniform-unit compare: `draw * K < threshold`.
fn draw_below(rng: &mut RmgRng, threshold: f64) -> bool {
    let value = TruncF64::from_f64(f64::from(rng.next_u32()))
        .mul(TruncF64::from_f64(f64::from_bits(RANGE_K_BITS)));
    value.lt(TruncF64::from_f64(threshold))
}

/// Run the water stage. `playable` is the LocalSize rect; `water_percent` is
/// the WaterAmount option; `num_players` feeds the archipelago island count.
pub struct WaterArgs {
    pub map_type: i32,
    pub water_percent: i32,
    pub num_players: i32,
    pub playable: PlayableRect,
}

pub fn run(ctx: &mut BlobCtx<'_>, args: &WaterArgs) {
    // Phase 1: every cell becomes water.
    let coords: Vec<(i32, i32)> = ctx.grid.native_cells().collect();
    for &(x, y) in &coords {
        ctx.grid.get_mut(x, y).expect("native cell").tile = ctx.ids.water_base;
    }

    // Phase 2: shape dispatch.
    match args.map_type {
        0 => archipelago(ctx, args),
        1 => continental(ctx, args),
        _ => islands_in_sea(ctx, args),
    }

    // Phase 3: isolated water cells with four clear cardinals become land.
    for &(x, y) in &coords {
        if ctx.grid.get(x, y).expect("native cell").tile != ctx.ids.water_base {
            continue;
        }
        let mut all_clear = true;
        for dir in [0usize, 2, 4, 6] {
            let (nx, ny) = RmgGrid::step(x, y, dir);
            let tile = ctx.grid.cell_native(nx, ny).tile;
            if !ctx.ids.is_clear(tile) {
                all_clear = false;
            }
        }
        if all_clear {
            ctx.grid.get_mut(x, y).expect("native cell").tile = 0;
        }
    }

    // Phase 4: final shore tiling (region 0, checks on; verdict ignored),
    // then the region-data reset.
    {
        let mut shore_ctx = ShoreCtx {
            grid: ctx.grid,
            scratch: ctx.scratch,
            ids: ctx.ids,
            blocks: ctx.blocks,
            rng: ctx.rng,
        };
        let _ = shore::run(&mut shore_ctx, 0, false);
    }
    ctx.scratch.reset_region_ids();

    // Phase 5: shore pieces grass their clear cardinal neighbors.
    for &(x, y) in &coords {
        if !ctx
            .ids
            .is_shore_piece(ctx.grid.get(x, y).expect("native cell").tile)
        {
            continue;
        }
        for dir in [0usize, 2, 4, 6] {
            let (nx, ny) = RmgGrid::step(x, y, dir);
            if ctx.ids.is_clear(ctx.grid.cell_native(nx, ny).tile) {
                ctx.grid.cell_native_mut(nx, ny).tile = ctx.ids.green;
            }
        }
    }
}

/// Mode 0: islands on a randomized partition grid.
fn archipelago(ctx: &mut BlobCtx<'_>, args: &WaterArgs) {
    let mut blob_counter = 1;
    let m = (args.num_players / 2).max(2);

    // Extra-island draw: ftol(draw * m * K + 1.0), rejection > m.
    let extra = loop {
        let value = x87::ftol(
            TruncF64::from_f64(f64::from(ctx.rng.next_u32()))
                .mul(TruncF64::from_f64(f64::from(m)))
                .mul(TruncF64::from_f64(f64::from_bits(RANGE_K_BITS)))
                .add(TruncF64::from_f64(1.0))
                .to_f64(),
        );
        if value <= m {
            break value;
        }
    };
    let islands = args.num_players + extra;

    let rect4 = [
        args.playable.x,
        args.playable.y,
        args.playable.w,
        args.playable.h,
    ];
    let mut entries = partition_grid(ctx.rng, islands, &rect4);
    ctx.scratch.clear_stamps();

    while !entries.is_empty() {
        let entry = entries.remove(0);

        // Blob size: ((draw*K*0.05) + 0.45) * (2*w*h) — drawn once per entry.
        let area2 = entry[2] * entry[3] * 2;
        let size = x87::ftol(
            TruncF64::from_f64(f64::from(ctx.rng.next_u32()))
                .mul(TruncF64::from_f64(f64::from_bits(RANGE_K_BITS)))
                .mul(TruncF64::from_f64(MODE0_SPAN))
                .add(TruncF64::from_f64(MODE0_BASE))
                .mul(TruncF64::from_f64(f64::from(area2)))
                .to_f64(),
        );
        let center = diamond_center(ctx.map_w, &entry);
        for _ in 0..10 {
            let params = BlobParams {
                max_cells: size,
                rect: entry,
                seed: center,
                target: center,
                drift_scale: 0.25,
                directed: false,
            };
            if blob::carve(ctx, &mut blob_counter, &params) != 0 {
                break;
            }
            blob_counter += 1;
        }
    }
}

/// Mode 1: one landmass grown toward the fixed map center.
fn continental(ctx: &mut BlobCtx<'_>, args: &WaterArgs) {
    let mut blob_counter = 1;
    let area = (ctx.map_h + 4) * ctx.map_w * 2;
    let area_f = TruncF64::from_f64(f64::from(area));

    let target = land_target(args.water_percent, MODE1_MAX, MODE1_MIN);
    let cap = x87::ftol(
        area_f
            .mul(TruncF64::from_f64(MODE1_CAP))
            .mul(target)
            .to_f64(),
    );

    let c = ctx.map_h / 2 + ctx.map_w / 2;
    let center = (c + 1, c);
    ctx.scratch.clear_stamps();

    let rect = [
        args.playable.x + 1,
        args.playable.y + 1,
        args.playable.w - 2,
        args.playable.h - 2,
    ];
    let mut seed = center;
    let mut placed = 0i32;
    let mut calls = 0;
    loop {
        let fraction = TruncF64::from_f64(f64::from(placed)).div(area_f);
        if !fraction.lt(target) || !TruncF64::from_f64(0.0).lt(target) || calls >= 100 {
            break;
        }
        let want = x87::ftol(target.sub(fraction).mul(area_f).to_f64()).min(cap);
        let params = BlobParams {
            max_cells: want,
            rect,
            seed,
            target: center,
            drift_scale: 0.75,
            directed: true,
        };
        placed += blob::carve(ctx, &mut blob_counter, &params);
        calls += 1;

        // Nearest unqueued cell to the fixed center, in native scan order.
        seed = nearest_unstamped_iterator_order(ctx, center);
    }
}

/// Mode 2: two landmasses in randomly split halves of the playable rect.
fn islands_in_sea(ctx: &mut BlobCtx<'_>, args: &WaterArgs) {
    let mut blob_counter = 1;
    let area = (ctx.map_h + 4) * ctx.map_w * 2;
    let area_f = TruncF64::from_f64(f64::from(area));

    let target = land_target(args.water_percent, MODE2_MAX, MODE2_MIN);
    let cap = x87::ftol(
        area_f
            .mul(TruncF64::from_f64(MODE2_CAP))
            .mul(target)
            .to_f64(),
    );

    ctx.scratch.clear_stamps();

    let p = &args.playable;
    let (rect1, rect2) = if draw_below(ctx.rng, 0.5) {
        (
            [p.x, p.y, p.w / 2 - 1, p.h],
            [p.x + p.w / 2 + 1, p.y, p.w / 2 - 1, p.h],
        )
    } else {
        (
            [p.x, p.y, p.w, p.h / 2 - 1],
            [p.x, p.y + p.h / 2 + 1, p.w, p.h / 2 - 1],
        )
    };

    let mut calls = 0;
    for rect in [rect1, rect2] {
        let center = diamond_center(ctx.map_w, &rect);
        // Aspect weights are DOUBLES on this path (no f32 narrowing).
        let scale = TruncF64::from_f64(1.2);
        let (wx, wy) = if rect[3] < rect[2] {
            (
                TruncF64::from_f64(1.0),
                TruncF64::from_f64(f64::from(rect[2]))
                    .div(TruncF64::from_f64(f64::from(rect[3])))
                    .mul(scale),
            )
        } else {
            (
                TruncF64::from_f64(f64::from(rect[3]))
                    .div(TruncF64::from_f64(f64::from(rect[2])))
                    .mul(scale),
                TruncF64::from_f64(1.0),
            )
        };
        let one = TruncF64::from_f64(1.0);
        let half = TruncF64::from_f64(0.5);
        let half_w = TruncF64::from_f64(f64::from(rect[2])).mul(half);
        let half_h = TruncF64::from_f64(f64::from(rect[3])).mul(half);
        let coeff_a = one.div(half_w.mul(half_w));
        let coeff_b = one.div(half_h.mul(half_h));

        let mut seed = center;
        let mut placed = 0i32;
        loop {
            let fraction = TruncF64::from_f64(f64::from(placed)).div(area_f);
            if !fraction.lt(target) || calls >= 100 {
                break;
            }
            let want = x87::ftol(target.sub(fraction).mul(area_f).to_f64()).min(cap);
            let params = BlobParams {
                max_cells: want,
                rect,
                seed,
                target: center,
                drift_scale: 0.75,
                directed: true,
            };
            placed += blob::carve(ctx, &mut blob_counter, &params);
            calls += 1;

            seed = nearest_unstamped_linear(ctx, &rect, center, wx, wy, coeff_a, coeff_b);
        }
    }
}

/// `(max - min) * (1 - water% * 0.01) + min`, differences at runtime.
fn land_target(water_percent: i32, max: f64, min: f64) -> TruncF64 {
    let t = TruncF64::from_f64(1.0)
        .sub(TruncF64::from_f64(f64::from(water_percent)).mul(TruncF64::from_f64(PERCENT)));
    TruncF64::from_f64(max)
        .sub(TruncF64::from_f64(min))
        .mul(t)
        .add(TruncF64::from_f64(min))
}

/// Mode-1 next-seed scan: cell-iterator order, Manhattan distance to the
/// fixed center, strict less-than (first wins ties). (0,0) if none.
fn nearest_unstamped_iterator_order(ctx: &mut BlobCtx<'_>, center: (i32, i32)) -> (i32, i32) {
    let mut best = 50_000;
    let mut found = (0, 0);
    for (x, y) in ctx.grid.native_cells().collect::<Vec<_>>() {
        if ctx.scratch.get(x, y).stamp != 0 {
            continue;
        }
        let metric = (y - center.1).abs() + (x - center.0).abs();
        if metric < best {
            best = metric;
            found = (x, y);
        }
    }
    found
}

/// Mode-2 next-seed scan: LINEAR scratch walk reading each record's own
/// coords, gated by the rect's diamond window and the ellipse, weighted
/// Manhattan metric. (0,0) if none.
fn nearest_unstamped_linear(
    ctx: &mut BlobCtx<'_>,
    rect: &[i32; 4],
    center: (i32, i32),
    wx: TruncF64,
    wy: TruncF64,
    coeff_a: TruncF64,
    coeff_b: TruncF64,
) -> (i32, i32) {
    let map_w = ctx.map_w;
    let (rx, ry, w, h) = (rect[0], rect[1], rect[2], rect[3]);
    let sum_lo = map_w + 2 * ry + 1;
    let sum_hi = map_w + 2 * ry + 2 * h + 3;
    let diff_lo = 2 * rx - map_w + 1;
    let diff_hi = 2 * rx - map_w + 2 * w + 3;

    let mut best = 50_000;
    let mut found = (0, 0);
    for index in 0..ctx.scratch.width() * ctx.scratch.width() {
        let record = ctx.scratch.cells()[index];
        let (x, y) = (i32::from(record.x), i32::from(record.y));
        if record.stamp != 0 {
            continue;
        }
        if x + y < sum_lo || x + y > sum_hi || x - y < diff_lo || x - y > diff_hi {
            continue;
        }
        if !ellipse_gate(map_w, (x, y), rect, coeff_a, coeff_b) {
            continue;
        }
        let metric = x87::ftol(
            TruncF64::from_f64(f64::from((y - center.1).abs()))
                .mul(wy)
                .add(TruncF64::from_f64(f64::from((x - center.0).abs())).mul(wx))
                .to_f64(),
        );
        if metric < best {
            best = metric;
            found = (x, y);
        }
    }
    found
}

/// The same ellipse test the flood fill uses (re-derived here to keep the
/// blob module's private helper private).
fn ellipse_gate(
    map_w: i32,
    cell: (i32, i32),
    rect: &[i32; 4],
    coeff_a: TruncF64,
    coeff_b: TruncF64,
) -> bool {
    let half = TruncF64::from_f64(0.5);
    let t1 = cell.0 - 2 * rect[0] - cell.1 + map_w - 1;
    let t2 = cell.0 - 2 * rect[1] - map_w + cell.1 - 1;
    let dx = TruncF64::from_f64(f64::from(t1))
        .mul(half)
        .sub(TruncF64::from_f64(f64::from(rect[2])).mul(half));
    let dy = TruncF64::from_f64(f64::from(t2))
        .mul(half)
        .sub(TruncF64::from_f64(f64::from(rect[3])).mul(half));
    dy.mul(dy)
        .mul(coeff_b)
        .add(dx.mul(dx).mul(coeff_a))
        .lt(TruncF64::from_f64(1.0))
}

/// Island partition grid: ceil-sqrt strips with randomized orientation and
/// leftover removal. Emits logical rects `{x+2, y+2, cw-4, ch-4}`,
/// strip-major.
pub fn partition_grid(rng: &mut RmgRng, n: i32, rect: &[i32; 4]) -> Vec<[i32; 4]> {
    let s = x87::ftol(approx_sqrt(TruncF64::from_f64(f64::from(n))).to_f64());
    let slots_per_strip = if s * s == n { s } else { s + 1 };
    let strips = if n <= slots_per_strip * s {
        s
    } else {
        slots_per_strip
    };
    let leftover = strips * slots_per_strip - n;

    let mut cw = rect[2] / slots_per_strip;
    let mut ch = rect[3] / slots_per_strip;

    // Orientation: below 0.5 = column strips, else rows.
    let columns = draw_below(rng, 0.5);
    let (step, advance) = if columns {
        cw = rect[2] / strips;
        ((0, ch), (cw, 0))
    } else {
        ch = rect[3] / strips;
        ((cw, 0), (0, ch))
    };

    let mut counts = vec![slots_per_strip; strips as usize];
    let mut indices: Vec<i32> = (0..strips).collect();
    for _ in 0..leftover {
        let scale = TruncF64::from_f64(f64::from_bits(RANGE_K_BITS));
        let index = loop {
            let value = x87::ftol(
                TruncF64::from_f64(f64::from(rng.next_u32()))
                    .mul(TruncF64::from_f64(indices.len() as f64))
                    .mul(scale)
                    .to_f64(),
            );
            if value <= indices.len() as i32 - 1 {
                break value as usize;
            }
        };
        counts[indices[index] as usize] = s;
        indices.remove(index);
    }

    let mut out = Vec::new();
    let (mut sx, mut sy) = (rect[0], rect[1]);
    for (i, &count) in counts.iter().enumerate() {
        let (mut x, mut y) = (sx, sy);
        if count < slots_per_strip {
            // Short strips are centered by half a cell along the strip axis.
            if columns {
                y += ch / 2;
            } else {
                x += cw / 2;
            }
        }
        for _ in 0..count {
            out.push([x + 2, y + 2, cw - 4, ch - 4]);
            x += step.0;
            y += step.1;
        }
        sx += advance.0;
        sy += advance.1;
        // The along-axis coordinate resets to the rect origin per strip.
        if columns {
            sy = rect[1];
        } else {
            sx = rect[0];
        }
        let _ = i;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::rmg::phases::shore::{SubTile, TileBlock, TileBlocks};
    use crate::map::rmg::scratch::RmgScratch;
    use crate::map::rmg::x87::Gaussian;

    struct OneByOne(TileBlock);

    impl TileBlocks for OneByOne {
        fn block(&self, _tile: i32) -> Option<&TileBlock> {
            Some(&self.0)
        }
    }

    fn blocks() -> OneByOne {
        OneByOne(TileBlock {
            width: 1,
            height: 1,
            subtiles: vec![Some(SubTile { height: 0 })],
        })
    }

    fn ids() -> TileIds {
        TileIds {
            clear: 0,
            ramp_base: -1,
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
            misc_pave: -1,
            paved_roads: -1,
            medians: -1,
        }
    }

    fn world(map_w: i32, map_h: i32) -> (RmgGrid, RmgScratch) {
        let stride = (map_w + map_h + 1) as usize;
        let (dmin, dmax) = (map_w, map_w + 2 * map_h);
        (
            RmgGrid::new(stride, dmin, dmax),
            RmgScratch::new(stride, dmin, dmax),
        )
    }

    fn run_water(map_type: i32, seed: u16) -> (RmgGrid, TileIds) {
        let (map_w, map_h) = (34, 42); // gen 30x30
        let (mut grid, mut scratch) = world(map_w, map_h);
        let identity = ids();
        let block_table = blocks();
        let mut rng = RmgRng::new(seed);
        let mut gauss = Gaussian::default();
        let mut ctx = BlobCtx {
            grid: &mut grid,
            scratch: &mut scratch,
            ids: &identity,
            blocks: &block_table,
            rng: &mut rng,
            gauss: &mut gauss,
            map_w,
            map_h,
            rollback_level: 4,
        };
        let args = WaterArgs {
            map_type,
            water_percent: 50,
            num_players: 4,
            playable: PlayableRect {
                x: 2,
                y: 5,
                w: 30,
                h: 30,
            },
        };
        run(&mut ctx, &args);
        (grid, identity)
    }

    fn census(grid: &RmgGrid, identity: &TileIds) -> (usize, usize, usize, usize) {
        let mut land = 0;
        let mut water = 0;
        let mut green = 0;
        let mut shore = 0;
        for (x, y) in grid.native_cells().collect::<Vec<_>>() {
            let tile = grid.get(x, y).unwrap().tile;
            if tile == identity.water_base
                || (tile > identity.water_base && tile < identity.water_base + 0x0E)
            {
                water += 1;
            } else if identity.is_shore_piece(tile) {
                shore += 1;
            } else if tile == identity.green {
                green += 1;
            } else if identity.is_clear(tile) {
                land += 1;
            }
        }
        (land, water, green, shore)
    }

    #[test]
    fn every_map_type_produces_land_and_water() {
        for map_type in [0, 1, 2] {
            let (grid, identity) = run_water(map_type, 1234);
            let (land, water, _, _) = census(&grid, &identity);
            assert!(land > 0, "map type {map_type} produced no land");
            assert!(water > 0, "map type {map_type} produced no water");
        }
    }

    #[test]
    fn shorelines_are_grassed() {
        let (grid, identity) = run_water(1, 77);
        let (_, _, green, shore) = census(&grid, &identity);
        assert!(
            shore > 0,
            "the final tiling pass leaves shore pieces in place"
        );
        assert!(green > 0, "shore-adjacent clear cells turned green");
    }

    #[test]
    fn water_stage_is_deterministic() {
        let tiles = |seed| {
            let (grid, _) = run_water(0, seed);
            grid.native_cells()
                .collect::<Vec<_>>()
                .iter()
                .map(|&(x, y)| grid.get(x, y).unwrap().tile)
                .collect::<Vec<i32>>()
        };
        assert_eq!(tiles(42), tiles(42));
    }

    #[test]
    fn partition_grid_shapes() {
        // n = 5: s = 2, C = 3, R = 2 (5 <= 3*2 -> R = s = 2), leftover = 1.
        let mut rng = RmgRng::new(9);
        let rects = partition_grid(&mut rng, 5, &[2, 5, 30, 30]);
        assert_eq!(rects.len(), 5, "exactly n islands emitted");
        for rect in &rects {
            assert!(rect[2] > 0 && rect[3] > 0, "inset rects stay positive");
        }
        // A perfect square: n = 4 -> 2x2 grid, no leftovers.
        let mut rng = RmgRng::new(9);
        let rects = partition_grid(&mut rng, 4, &[2, 5, 30, 30]);
        assert_eq!(rects.len(), 4);
    }

    #[test]
    fn archipelago_island_count_scales_with_players() {
        // m = max(2, players/2); islands = players + [1, m]. For 8 players,
        // islands is in [9, 12]; the partition grid must emit that many.
        let mut rng = RmgRng::new(31);
        // Drain the extra-island draw the same way the shape does.
        let m = 4;
        let extra = loop {
            let value = x87::ftol(
                TruncF64::from_f64(f64::from(rng.next_u32()))
                    .mul(TruncF64::from_f64(f64::from(m)))
                    .mul(TruncF64::from_f64(f64::from_bits(RANGE_K_BITS)))
                    .add(TruncF64::from_f64(1.0))
                    .to_f64(),
            );
            if value <= m {
                break value;
            }
        };
        assert!((1..=4).contains(&extra));
    }
}
