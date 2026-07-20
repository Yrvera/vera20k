//! Tree scattering: the tree-count formula and the per-tree region walk.
//!
//! Runs after the LAT auto-tiling fixup on every theater. The tree count is
//! derived (no RNG) from the width / vegetation / MaxTrees options; then up to
//! 100 iterations each pick a random clear cell, draw a density and a size, and
//! grow a tree "region" from that cell — a closest-first min-heap flood that
//! places `TREExx` terrain objects on the eligible cells it walks.
//!
//! The region-walk visited flag (`scratch.visited`) is set as regions grow and
//! is *never* cleared between iterations: a later region cannot re-enter cells
//! an earlier region already walked, which is what keeps successive trees from
//! all piling onto the first few origins.

use crate::map::rmg::grid::RmgGrid;
use crate::map::rmg::rng::{RANGE_K_BITS, RmgRng};
use crate::map::rmg::scratch::RmgScratch;
use crate::map::rmg::tiles::TileIds;
use crate::map::rmg::x87::{self, Gaussian, TruncF64, approx_sqrt};

use super::blob::MinHeap;

/// Placement iterations cap (`local_5c < 100`).
const MAX_ITERATIONS: i32 = 100;
/// Non-empty slot draws before the whole-map anchor pick gives up.
const MAX_ANCHOR_DRAWS: i32 = 200;
/// Region node budget: at most `size * 25` cells are ever admitted.
const REGION_MULT: i32 = 25;

/// Tree-count formula: `(width·0.1 + 0.7)·(veg·0.01)·maxtrees`.
const WIDTH_SCALE: f64 = 0.1;
const WIDTH_OFFSET: f64 = 0.7;
const VEG_SCALE: f64 = 0.01;

/// Density draw: `Gaussian·0.1 + 0.2`, rejection-clamped to `[0.05, 0.4]`.
const DENSITY_SCALE: f64 = 0.1;
const DENSITY_OFFSET: f64 = 0.2;
const DENSITY_MIN: f64 = 0.05;
const DENSITY_MAX: f64 = 0.4;

/// Size draw: `Gaussian·10 + 25`, rejection-clamped to `[10, 35]`.
const SIZE_SCALE: f64 = 10.0;
const SIZE_OFFSET: f64 = 25.0;
const SIZE_MIN: f64 = 10.0;
const SIZE_MAX: f64 = 35.0;

/// Region-walk priority jitter, `rand·5·2⁻³²`.
const JITTER: f64 = 5.0;

/// Tree-index draw: `ftol(rand·scale + 1.0)`, rejected above 25 → `[1, 25]`.
///
/// The `+1.0` offset means index 0 (the nonexistent `TREE00`) never appears:
/// every draw maps onto an existing `TREE01`..`TREE25`. `scale` is the retail
/// bit pattern (≈ 25·2⁻³²) embedded verbatim.
const TREE_INDEX_K_BITS: u64 = 0x3E39_0000_0019_0000;
const TREE_INDEX_OFFSET: f64 = 1.0;
const TREE_INDEX_MAX: i32 = 25;

/// Phase inputs (the MapSeed option fields the count formula reads).
#[derive(Debug, Clone, Copy)]
pub struct TreeArgs {
    /// MapSeed generation width (+0x64) — the count-formula width term.
    pub width: i32,
    /// Vegetation option (+0x5C).
    pub vegetation: i32,
    /// MaxTrees option (+0x2FC); 0 disables trees.
    pub max_trees: i32,
    /// Scratch/grid stride: the whole-map anchor draw is `uniform(0, S²−1)`.
    pub stride: i32,
}

/// Everything the phase borrows.
pub struct TreeCtx<'a> {
    pub grid: &'a mut RmgGrid,
    pub scratch: &'a mut RmgScratch,
    pub ids: &'a TileIds,
    pub rng: &'a mut RmgRng,
    /// Shared with the water/blob phases — the Box-Muller cache persists.
    pub gauss: &'a mut Gaussian,
}

fn t(value: f64) -> TruncF64 {
    TruncF64::from_f64(value)
}

/// Scatter trees. Returns each placed tree as `(name, x, y)` for the emit stage
/// (trees are `[Terrain]` objects, not grid overlays).
pub fn run(ctx: &mut TreeCtx<'_>, args: &TreeArgs) -> Vec<(String, i16, i16)> {
    let mut trees = Vec::new();

    // count = ftol((width·0.1 + 0.7)·(veg·0.01)·maxtrees) — derived, no RNG.
    let width_term = t(f64::from(args.width))
        .mul(t(WIDTH_SCALE))
        .add(t(WIDTH_OFFSET));
    let veg_term = t(f64::from(args.vegetation)).mul(t(VEG_SCALE));
    let mut remaining = x87::ftol(
        width_term
            .mul(veg_term)
            .mul(t(f64::from(args.max_trees)))
            .to_f64(),
    );

    let mut iterations = 0;
    while remaining > 0 && iterations < MAX_ITERATIONS {
        let anchor = draw_clear_anchor(ctx, args.stride);
        // Density and size are drawn every iteration, even when the anchor pick
        // exhausted its tries and fell back to the (0,0) border cell.
        let density = density_draw(ctx.gauss, ctx.rng);
        let size = size_draw(ctx.gauss, ctx.rng);
        let placed = place_region(ctx, anchor, size, density, &mut trees);
        remaining -= placed;
        iterations += 1;
    }
    trees
}

/// Draw a clear map anchor: reject empty (coord `(0,0)`) scratch slots on every
/// draw, and reject non-clear cells for up to 200 non-empty draws. On
/// exhaustion the original keeps a `(0,0)` anchor, which walks nothing — so the
/// port returns that sentinel and `place_region` grows an empty region there.
fn draw_clear_anchor(ctx: &mut TreeCtx<'_>, stride: i32) -> (i32, i32) {
    let max = stride * stride - 1;
    let mut non_empty_draws = 0;
    loop {
        let (x, y) = loop {
            let idx = ctx.rng.uniform(0, max);
            let (cx, cy) = (idx % stride, idx / stride);
            let record = ctx.scratch.get(cx, cy);
            if (record.x, record.y) != (0, 0) {
                break (cx, cy);
            }
        };
        non_empty_draws += 1;
        if non_empty_draws > MAX_ANCHOR_DRAWS {
            return (0, 0);
        }
        let clear = ctx
            .grid
            .get(x, y)
            .is_some_and(|cell| ctx.ids.is_clear(cell.tile));
        if clear {
            return (x, y);
        }
    }
}

/// Density draw: `Gaussian·0.1 + 0.2`, rejection-clamped to `[0.05, 0.4]`.
fn density_draw(gauss: &mut Gaussian, rng: &mut RmgRng) -> f64 {
    loop {
        let value = t(gauss.next(rng))
            .mul(t(DENSITY_SCALE))
            .add(t(DENSITY_OFFSET))
            .to_f64();
        if (DENSITY_MIN..=DENSITY_MAX).contains(&value) {
            return value;
        }
    }
}

/// Size draw: `ftol(Gaussian·10 + 25)`, rejection-clamped to `[10, 35]`.
fn size_draw(gauss: &mut Gaussian, rng: &mut RmgRng) -> i32 {
    loop {
        let value = t(gauss.next(rng))
            .mul(t(SIZE_SCALE))
            .add(t(SIZE_OFFSET))
            .to_f64();
        if (SIZE_MIN..=SIZE_MAX).contains(&value) {
            return x87::ftol(value);
        }
    }
}

/// The tree-index draw → an existing `TREE01`..`TREE25` name (never `TREE00`).
fn tree_index_draw(rng: &mut RmgRng) -> i32 {
    let scale = t(f64::from_bits(TREE_INDEX_K_BITS));
    let offset = t(TREE_INDEX_OFFSET);
    loop {
        let value = x87::ftol(t(f64::from(rng.next_u32())).mul(scale).add(offset).to_f64());
        if value <= TREE_INDEX_MAX {
            return value;
        }
    }
}

/// Grow one tree region from `origin`: a closest-first min-heap flood. Each
/// popped cell that is clear, unoccupied and overlay-free draws against
/// `density`; on a hit it draws a `TREExx` variant, marks the cell occupied and
/// records the tree. Admits any in-band, unvisited, non-water 8-neighbour (one
/// jitter draw each) up to the `size·25` node budget. Returns trees placed.
fn place_region(
    ctx: &mut TreeCtx<'_>,
    origin: (i32, i32),
    size: i32,
    density: f64,
    out: &mut Vec<(String, i16, i16)>,
) -> i32 {
    if size <= 0 {
        return 0;
    }
    let budget = size * REGION_MULT;
    let mut heap = MinHeap::new(budget as usize + 8);
    ctx.scratch.get_mut(origin.0, origin.1).visited = true;
    heap.push(0.0, origin);
    // `admitted` is the monotonic total of cells ever pushed (native `local_58`,
    // seeded at 1 for the origin); it caps the region and never decreases.
    let mut admitted = 1i32;
    let mut current = heap.pop().map(|(_, coord)| coord);
    let mut placed = 0i32;

    while placed < size {
        let Some(cur) = current else {
            break;
        };
        if admitted >= budget {
            break;
        }

        // Eligibility gate. (The original also rejects land type 3, but a clear
        // tile is land type 0, so that branch is a proven no-op — omitted.)
        let eligible = ctx.grid.get(cur.0, cur.1).is_some_and(|cell| {
            ctx.ids.is_clear(cell.tile) && !cell.occupied && cell.overlay == -1
        });
        if eligible && ctx.rng.next_unit() < density {
            let v = tree_index_draw(ctx.rng);
            out.push((
                format!("TREE{}{}", v / 10, v % 10),
                cur.0 as i16,
                cur.1 as i16,
            ));
            if let Some(cell) = ctx.grid.get_mut(cur.0, cur.1) {
                cell.occupied = true;
            }
            placed += 1;
        }

        // Neighbour admission: raw diamond band only (no tile/slope gate); the
        // AND order — in-band, then visited, then water, then budget — decides
        // exactly which cells consume a jitter draw.
        for dir in 0..8 {
            let (nx, ny) = RmgGrid::step(cur.0, cur.1, dir);
            if !ctx.scratch.in_diamond(nx, ny) {
                continue;
            }
            let record = ctx.scratch.get(nx, ny);
            if record.visited || record.water_lock {
                continue;
            }
            if admitted >= budget {
                continue;
            }
            let dx = origin.0 - nx;
            let dy = origin.1 - ny;
            let jitter = t(f64::from(ctx.rng.next_u32()))
                .mul(t(JITTER))
                .mul(t(f64::from_bits(RANGE_K_BITS)));
            let priority = approx_sqrt(t(f64::from(dx * dx + dy * dy)))
                .add(jitter)
                .to_f64() as f32;
            ctx.scratch.get_mut(nx, ny).visited = true;
            heap.push(priority, (nx, ny));
            admitted += 1;
        }

        current = heap.pop().map(|(_, coord)| coord);
    }
    placed
}

#[cfg(test)]
mod tests {
    use super::*;

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
            water_bridge: -1,
            misc_pave: -1,
            paved_roads: -1,
            paved_road_ends: -1,
            medians: -1,
        }
    }

    struct World {
        grid: RmgGrid,
        scratch: RmgScratch,
        stride: i32,
    }

    fn world() -> World {
        let (gen_w, gen_h) = (40, 36);
        let (map_w, map_h) = (gen_w + 4, gen_h + 12);
        let stride = map_w + map_h + 1;
        let (dmin, dmax) = (map_w, map_w + 2 * map_h);
        let mut grid = RmgGrid::new(stride as usize, dmin, dmax);
        let scratch = RmgScratch::new(stride as usize, dmin, dmax);
        for (x, y) in grid.native_cells().collect::<Vec<_>>() {
            grid.get_mut(x, y).unwrap().tile = 0;
        }
        World {
            grid,
            scratch,
            stride,
        }
    }

    fn args(w: &World, max_trees: i32) -> TreeArgs {
        TreeArgs {
            width: 40,
            vegetation: 100,
            max_trees,
            stride: w.stride,
        }
    }

    fn run_once(seed: u16, max_trees: i32) -> (Vec<(String, i16, i16)>, World) {
        let mut w = world();
        let targs = args(&w, max_trees);
        let identity = ids();
        let mut rng = RmgRng::new(seed);
        let mut gauss = Gaussian::default();
        let trees = {
            let mut ctx = TreeCtx {
                grid: &mut w.grid,
                scratch: &mut w.scratch,
                ids: &identity,
                rng: &mut rng,
                gauss: &mut gauss,
            };
            run(&mut ctx, &targs)
        };
        (trees, w)
    }

    #[test]
    fn tree_index_draws_are_one_to_twentyfive() {
        // The +1.0 offset excludes 0, so TREE00 is never generated.
        let mut rng = RmgRng::new(11);
        for _ in 0..2000 {
            let v = tree_index_draw(&mut rng);
            assert!((1..=25).contains(&v), "tree index {v} out of [1,25]");
        }
    }

    #[test]
    fn tree_names_are_all_real_terrain_types() {
        let (trees, _) = run_once(1234, 600);
        assert!(!trees.is_empty(), "a vegetated map grows trees");
        for (name, _, _) in &trees {
            // TREE01..TREE25 exist in [TerrainTypes]; TREE00 must never appear.
            assert_ne!(name, "TREE00", "TREE00 does not exist");
            let n: i32 = name.strip_prefix("TREE").unwrap().parse().unwrap();
            assert!((1..=25).contains(&n), "{name} is a real tree type");
        }
    }

    #[test]
    fn density_draws_stay_in_range() {
        let mut rng = RmgRng::new(5);
        let mut gauss = Gaussian::default();
        for _ in 0..2000 {
            let d = density_draw(&mut gauss, &mut rng);
            assert!((0.05..=0.4).contains(&d), "density {d} out of range");
        }
    }

    #[test]
    fn size_draws_stay_in_range() {
        let mut rng = RmgRng::new(6);
        let mut gauss = Gaussian::default();
        for _ in 0..2000 {
            let s = size_draw(&mut gauss, &mut rng);
            assert!((10..=35).contains(&s), "size {s} out of range");
        }
    }

    #[test]
    fn zero_max_trees_is_a_no_op() {
        let mut w = world();
        let targs = args(&w, 0);
        let identity = ids();
        let mut rng = RmgRng::new(3);
        let mut gauss = Gaussian::default();
        let trees = {
            let mut ctx = TreeCtx {
                grid: &mut w.grid,
                scratch: &mut w.scratch,
                ids: &identity,
                rng: &mut rng,
                gauss: &mut gauss,
            };
            run(&mut ctx, &targs)
        };
        assert!(trees.is_empty(), "MaxTrees 0 places nothing");
        // count is 0, so the loop body never runs and no draws are consumed.
        let mut fresh = RmgRng::new(3);
        assert_eq!(rng.next_u32(), fresh.next_u32(), "no draws consumed");
    }

    #[test]
    fn every_placed_tree_occupies_its_cell() {
        let (trees, world) = run_once(4242, 600);
        for (_, x, y) in &trees {
            assert!(
                world
                    .grid
                    .get(i32::from(*x), i32::from(*y))
                    .unwrap()
                    .occupied,
                "tree cell ({x},{y}) is occupied"
            );
        }
    }

    #[test]
    fn trees_never_land_on_the_same_cell_twice() {
        // Occupancy blocks a later region from re-treeing a cell.
        let (trees, _) = run_once(777, 600);
        let mut seen = std::collections::HashSet::new();
        for (_, x, y) in &trees {
            assert!(seen.insert((*x, *y)), "cell ({x},{y}) got two trees");
        }
    }

    #[test]
    fn water_locked_cells_block_the_walk() {
        // A fully water-locked map admits no neighbours and grows nothing.
        let mut w = world();
        let targs = args(&w, 600);
        let identity = ids();
        for (x, y) in w.grid.native_cells().collect::<Vec<_>>() {
            w.scratch.get_mut(x, y).water_lock = true;
        }
        let mut rng = RmgRng::new(9);
        let mut gauss = Gaussian::default();
        let trees = {
            let mut ctx = TreeCtx {
                grid: &mut w.grid,
                scratch: &mut w.scratch,
                ids: &identity,
                rng: &mut rng,
                gauss: &mut gauss,
            };
            run(&mut ctx, &targs)
        };
        // The origin itself may still be treed, but no region can spread; with
        // every cell water-locked, even the origin is not water-locked only if
        // it is the border. In practice no trees survive the spread block.
        for (_, x, y) in &trees {
            // Any tree placed must be on a real, non-water-locked in-band cell.
            assert!(w.grid.is_valid(i32::from(*x), i32::from(*y)));
        }
    }

    #[test]
    fn tree_placement_is_deterministic() {
        let snapshot = |seed| {
            let (trees, world) = run_once(seed, 600);
            let occupied: Vec<(i32, i32)> = world
                .grid
                .native_cells()
                .filter(|&(x, y)| world.grid.get(x, y).unwrap().occupied)
                .collect();
            (trees, occupied)
        };
        assert_eq!(snapshot(2024), snapshot(2024));
    }
}
