//! Rock overlay scattering (temperate theater only).
//!
//! After trees, the temperate painter sprinkles SROCK / TROCK overlays. A
//! target count is drawn from the map area, then up to `target·5` attempts each
//! pick a random non-border cell and, if it carries no overlay, stamp a rock:
//! SROCK on the sand-LAT transition tiles, TROCK on clear or green terrain. Any
//! overlay-free cell counts toward the target even when it matches neither rock
//! terrain — a native quirk this reproduces.
//!
//! Non-temperate theaters skip this phase entirely (the orchestrator gates it).

use crate::map::rmg::rng::RmgRng;
use crate::map::rmg::scratch::RmgScratch;
use crate::map::rmg::tiles::TileIds;

use crate::map::rmg::grid::RmgGrid;

/// Sand-rock overlay base (`0xA8`); five variants SROCK01..SROCK05.
const SROCK_BASE: i32 = 168;
/// Terrain-rock overlay base (`0xAD`); five variants TROCK01..TROCK05.
const TROCK_BASE: i32 = 173;
/// Rock-variant draw is `uniform(0, 4)`.
const ROCK_VARIANT_MAX: i32 = 4;
/// Target count is `uniform(0, area2 / 200)`.
const AREA_DIVISOR: i32 = 200;
/// Attempts cap is `target · 5`.
const ATTEMPTS_MULT: i32 = 5;
/// Sand-LAT transition span (`sand_lat .. sand_lat + 0x10`).
const LAT_SPAN: i32 = 0x10;

/// Phase inputs.
#[derive(Debug, Clone, Copy)]
pub struct RockArgs {
    /// Full map width (`DAT_0087f8dc`).
    pub map_w: i32,
    /// Full map height (`DAT_0087f8e0`).
    pub map_h: i32,
    /// Grid/scratch stride: the slot draw is `uniform(0, S²−1)`.
    pub stride: i32,
}

/// Everything the phase borrows.
pub struct RockCtx<'a> {
    pub grid: &'a mut RmgGrid,
    pub scratch: &'a RmgScratch,
    pub ids: &'a TileIds,
    pub rng: &'a mut RmgRng,
}

/// Scatter rock overlays onto the grid. Returns the number of cells the pass
/// counted (including the overlay-free-but-no-terrain-match quirk cells).
pub fn run(ctx: &mut RockCtx<'_>, args: &RockArgs) -> i32 {
    // area2 = (H + 4) · W · 2; target = uniform(0, area2 / 200).
    let area2 = (args.map_h + 4) * args.map_w * 2;
    let target = ctx.rng.uniform(0, area2 / AREA_DIVISOR);
    let attempts_cap = target * ATTEMPTS_MULT;
    if attempts_cap <= 0 {
        return 0;
    }

    let max_slot = args.stride * args.stride - 1;
    let mut placed = 0i32;
    let mut attempts = 0i32;
    loop {
        if placed >= target {
            return placed;
        }
        let (x, y) = draw_nonborder_slot(ctx.rng, ctx.scratch, args.stride, max_slot);

        let (overlay, tile) = {
            let cell = ctx.grid.get(x, y).expect("picked slot is an in-band cell");
            (cell.overlay, cell.tile)
        };
        if overlay == -1 {
            if is_sand_lat_transition(ctx.ids, tile) {
                let variant = ctx.rng.uniform(0, ROCK_VARIANT_MAX);
                ctx.grid.get_mut(x, y).unwrap().overlay = variant + SROCK_BASE;
            } else if ctx.ids.is_clear(tile) || ctx.ids.is_green_lat(tile) {
                let variant = ctx.rng.uniform(0, ROCK_VARIANT_MAX);
                ctx.grid.get_mut(x, y).unwrap().overlay = variant + TROCK_BASE;
            }
            // else: overlay-free but no rock terrain — still counts.
            placed += 1;
        }

        attempts += 1;
        if attempts >= attempts_cap {
            return placed;
        }
    }
}

/// Draw one slot uniform over `[0, S²−1]`, rejecting the shared `(0,0)` border
/// cell (every out-of-band scratch record reads coord `(0,0)`).
fn draw_nonborder_slot(
    rng: &mut RmgRng,
    scratch: &RmgScratch,
    stride: i32,
    max_slot: i32,
) -> (i32, i32) {
    loop {
        let idx = rng.uniform(0, max_slot);
        let (cx, cy) = (idx % stride, idx / stride);
        let record = scratch.get(cx, cy);
        if (record.x, record.y) != (0, 0) {
            return (cx, cy);
        }
    }
}

/// The sand-rock terrain test: the clear-to-sand LAT transition range only.
///
/// This deliberately *excludes* the sand base tile, unlike `TileIds::is_sand_lat`
/// — the original's sand-rock predicate checks only `[sand_lat, sand_lat+0x10)`,
/// while its green predicate (used below via `is_green_lat`) does include the
/// green base tile. The asymmetry is real and reproduced.
fn is_sand_lat_transition(ids: &TileIds, tile: i32) -> bool {
    ids.sand_lat != -1 && tile >= ids.sand_lat && tile < ids.sand_lat + LAT_SPAN
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::rmg::tiles::SpecialTerrain;

    fn ids() -> TileIds {
        TileIds {
            clear: 0,
            ramp_base: -1,
            rough: -1,
            sand: 800,
            green: 100,
            rough_lat: -1,
            sand_lat: 810,
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

    struct World {
        grid: RmgGrid,
        scratch: RmgScratch,
        args: RockArgs,
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
            args: RockArgs {
                map_w,
                map_h,
                stride,
            },
        }
    }

    fn run_once(seed: u16) -> (i32, World) {
        let mut w = world();
        let identity = ids();
        let mut rng = RmgRng::new(seed);
        let placed = {
            let mut ctx = RockCtx {
                grid: &mut w.grid,
                scratch: &w.scratch,
                ids: &identity,
                rng: &mut rng,
            };
            run(&mut ctx, &w.args)
        };
        (placed, w)
    }

    #[test]
    fn rocks_are_only_ever_srock_or_trock() {
        let (_placed, world) = run_once(1234);
        let mut any = false;
        for (x, y) in world.grid.native_cells().collect::<Vec<_>>() {
            let overlay = world.grid.get(x, y).unwrap().overlay;
            if overlay != -1 {
                any = true;
                assert!(
                    (168..=172).contains(&overlay) || (173..=177).contains(&overlay),
                    "({x},{y}) overlay {overlay} is a SROCK/TROCK index"
                );
            }
        }
        assert!(any, "a clear temperate map gets some rocks");
    }

    #[test]
    fn clear_ground_gets_trock_not_srock() {
        // The whole map is clear (tile 0) → only TROCK (173..177) appears.
        let (_placed, world) = run_once(55);
        for (x, y) in world.grid.native_cells().collect::<Vec<_>>() {
            let overlay = world.grid.get(x, y).unwrap().overlay;
            if overlay != -1 {
                assert!(
                    (173..=177).contains(&overlay),
                    "clear cell ({x},{y}) took TROCK, got {overlay}"
                );
            }
        }
    }

    #[test]
    fn sand_lat_transition_gets_srock() {
        // Paint a band of sand-LAT transition tiles; those cells take SROCK.
        let mut w = world();
        let identity = ids();
        let sand_lat_tile = identity.sand_lat + 3; // inside [810, 826)
        let band: Vec<(i32, i32)> = w
            .grid
            .native_cells()
            .filter(|&(x, y)| (30..40).contains(&x) && (30..40).contains(&y))
            .collect();
        for &(x, y) in &band {
            w.grid.get_mut(x, y).unwrap().tile = sand_lat_tile;
        }
        let mut rng = RmgRng::new(88);
        {
            let mut ctx = RockCtx {
                grid: &mut w.grid,
                scratch: &w.scratch,
                ids: &identity,
                rng: &mut rng,
            };
            run(&mut ctx, &w.args);
        }
        for &(x, y) in &band {
            let overlay = w.grid.get(x, y).unwrap().overlay;
            if overlay != -1 {
                assert!(
                    (168..=172).contains(&overlay),
                    "sand-LAT cell ({x},{y}) took SROCK, got {overlay}"
                );
            }
        }
    }

    #[test]
    fn sand_base_tile_is_not_sand_lat_transition() {
        // The predicate excludes the sand base tile (the native asymmetry).
        let identity = ids();
        assert!(!is_sand_lat_transition(&identity, identity.sand));
        assert!(is_sand_lat_transition(&identity, identity.sand_lat));
        assert!(is_sand_lat_transition(&identity, identity.sand_lat + 0xF));
        assert!(!is_sand_lat_transition(&identity, identity.sand_lat + 0x10));
    }

    #[test]
    fn existing_overlays_are_never_overwritten() {
        let mut w = world();
        let identity = ids();
        // Pre-place an ore overlay; the rock pass must leave it untouched.
        w.grid.get_mut(34, 34).unwrap().overlay = 102;
        let mut rng = RmgRng::new(3);
        {
            let mut ctx = RockCtx {
                grid: &mut w.grid,
                scratch: &w.scratch,
                ids: &identity,
                rng: &mut rng,
            };
            run(&mut ctx, &w.args);
        }
        assert_eq!(
            w.grid.get(34, 34).unwrap().overlay,
            102,
            "the pre-existing overlay survives"
        );
    }

    #[test]
    fn rock_placement_is_deterministic() {
        let snapshot = |seed| {
            let (_placed, world) = run_once(seed);
            world
                .grid
                .native_cells()
                .map(|(x, y)| world.grid.get(x, y).unwrap().overlay)
                .collect::<Vec<_>>()
        };
        assert_eq!(snapshot(2024), snapshot(2024));
    }
}
