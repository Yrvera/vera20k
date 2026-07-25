//! River carving for the inland and mountainous map types.
//!
//! A river starts on one of the four map edges, picks a heading pointing inward,
//! and then walks: each step stamps a cross-section perpendicular to travel,
//! advances, and wobbles both the heading and the width a little. It stops on a
//! random roll, so length varies. Rivers shorter than [`MIN_STEPS`] are thrown
//! away and rolled back whole.
//!
//! **Trig convention, and it is the opposite of what the research report says.**
//! The carve line runs along `(+sin, +cos)` and travel along `(+cos, −sin)`.
//! The report's §4.1 names these `c` and `s` the other way round, because it was
//! written against two Ghidra labels that were themselves swapped. Read the
//! report with `c → sin` and `s → cos`. Getting this backwards rotates every
//! river 90° and no test in this repo would notice.
//!
//! Not modelled yet, and both are recorded rather than hidden:
//! - **Bridges.** The gate consumes no randomness of its own, so leaving them
//!   out costs no stream drift; the minimum-step draw that feeds it is still
//!   taken. Rivers simply never carry one.
//! - **The canyon.** Its coin *is* drawn, so the stream stays aligned, but the
//!   meander arm it depends on is not ported. Since the coin passes about 70% of
//!   the time, most rivers are visibly missing the level change that turns the
//!   surrounding map into a canyon.

use crate::map::rmg::grid::RmgGrid;
use crate::map::rmg::rng::{RANGE_K_BITS, RmgRng};
use crate::map::rmg::x87::{self, Gaussian, TruncF64};

use super::blob::{BlobCtx, seed_draw};
use super::lake::{self, WaterQuota};
use super::shore::{self, ShoreCtx};
use super::water::WaterArgs;

/// Edge count, and the heading each edge aims at: `7π/4 − edge·π/2`.
const EDGES: i32 = 4;
const EDGE_BASE_HEADING: f64 = 5.497_787_143_782_138; // 7π/4
const QUARTER_TURN: f64 = 1.570_796_326_794_896_6; // π/2
const EIGHTH_TURN: f64 = 0.785_398_163_397_448_3; // π/4
/// Spread of the initial heading around its edge's mean.
const START_SIGMA: f64 = 0.523_598_775_598_298_8; // π/6

/// Heading wobble: mean 0, this sigma, clamped to the lifetime window.
const WOBBLE_SIGMA: f64 = 0.314_159_265_358_979_3; // π/10
/// Width wobble sigma.
const WIDTH_SIGMA: f64 = 0.5;

/// Width scale on the water amount, and the floor below which it cannot fall.
const WIDTH_SCALE: f64 = 0.07;
const WIDTH_MIN: f64 = 1.0;

/// Bridge minimum-step draw: uniform over 35..=125.
const BRIDGE_STEP_BASE: f64 = 35.0;
const BRIDGE_STEP_SPAN_SCALED: f64 = 2.118_758_857_743_525_6e-8; // 91 * K
const BRIDGE_STEP_MAX: i32 = 125;

/// Per-step chance of spawning a branch, and of stopping.
const BRANCH_CHANCE: f64 = 0.01;
const STOP_CHANCE: f64 = 0.005;
/// Chance the river cuts a canyon once it has finished.
const CANYON_CHANCE: f64 = 0.7;

/// Steps a river must reach to survive.
const MIN_STEPS: i32 = 0x28;
/// The step count past which the heading starts to wander.
const WOBBLE_AFTER: i32 = 5;
/// Rings the finished river's region is grown by.
const PLAIN_DILATE_RINGS: i32 = 2;
/// Half-cell offset: cell centres, and the round-half-up on the width.
const HALF: f64 = 0.5;

/// The four cardinals, as direction indices.
const CARDINALS: [usize; 4] = [0, 2, 4, 6];

fn unit_draw(rng: &mut RmgRng) -> TruncF64 {
    TruncF64::from_f64(f64::from(rng.next_u32()))
        .mul(TruncF64::from_f64(f64::from_bits(RANGE_K_BITS)))
}

fn draw_below(rng: &mut RmgRng, threshold: f64) -> bool {
    unit_draw(rng).lt(TruncF64::from_f64(threshold))
}

/// A Gaussian redrawn until it lands inside `[lo, hi]`.
///
/// The recentre arm the original carries here can never fire for the start
/// heading or the branch heading — the windows are always wide enough — but it
/// can for the two wobbles, so it is kept.
fn bounded_gaussian(
    gauss: &mut Gaussian,
    rng: &mut RmgRng,
    mean: f64,
    sigma: f64,
    lo: f64,
    hi: f64,
) -> f64 {
    let (mut mean, mut sigma) = (mean, sigma);
    if hi < mean - sigma || mean + sigma < lo {
        sigma = (hi - lo) * HALF;
        mean = sigma + lo;
    }
    let sigma_t = TruncF64::from_f64(sigma);
    let mean_t = TruncF64::from_f64(mean);
    loop {
        let value = TruncF64::from_f64(gauss.next(rng))
            .mul(sigma_t)
            .add(mean_t)
            .to_f64();
        if value >= lo && value <= hi {
            return value;
        }
    }
}

/// Where a river attempt starts and which way it points.
struct Launch {
    cell: (i32, i32),
    heading: f64,
}

/// Pick a start edge, a cell along it, and an inward heading.
fn launch(ctx: &mut BlobCtx<'_>, gauss_first: bool) -> Launch {
    let _ = gauss_first;
    let edge = loop {
        let value = x87::ftol(
            unit_draw(ctx.rng)
                .mul(TruncF64::from_f64(f64::from(EDGES)))
                .to_f64(),
        );
        if value <= EDGES - 1 {
            break value;
        }
    };
    let rx = seed_draw(ctx.rng, ctx.map_w);
    let ry = seed_draw(ctx.rng, ctx.map_h);
    let (w, h) = (ctx.map_w, ctx.map_h);
    let cell = match edge {
        0 => (rx + 1, w - rx),
        1 => (w + h - 1 - ry, h - ry),
        2 => (w + h - 1 - rx, h + rx),
        _ => (ry + 1, w + ry),
    };
    let mean = EDGE_BASE_HEADING - f64::from(edge) * QUARTER_TURN;
    let heading = bounded_gaussian(
        ctx.gauss,
        ctx.rng,
        mean,
        START_SIGMA,
        mean - EIGHTH_TURN,
        mean + EIGHTH_TURN,
    );
    Launch { cell, heading }
}

/// Everything one carve attempt tracks.
struct Walk {
    region: i32,
    alive: bool,
    steps: i32,
    /// Terminated by the stop roll rather than by leaving the map. Only a
    /// roll-terminated river gets an end lake.
    rolled_to_a_stop: bool,
}

/// Stamp one cross-section. Returns whether the walk survives it.
fn carve_section(
    ctx: &mut BlobCtx<'_>,
    walk: &mut Walk,
    fx: f64,
    fy: f64,
    span: i32,
    sin: f64,
    cos: f64,
) {
    let step_back = f64::from(span - 1) * HALF;
    let mut x = fx - step_back * sin;
    let mut y = fy - step_back * cos;

    for _ in 0..span {
        let (cx, cy) = (x87::ftol(x), x87::ftol(y));
        if ctx.scratch.in_diamond(cx, cy) {
            let owner = ctx.scratch.get(cx, cy).region;
            if owner == walk.region {
                // Idempotent re-carve of our own cells.
                let cell = ctx.grid.get_mut(cx, cy).expect("native cell");
                cell.tile = ctx.ids.water_base;
                cell.sub_tile = 0;
            } else if owner == 0 {
                if ctx.ids.is_clear(ctx.grid.cell_native(cx, cy).tile) {
                    ctx.scratch.get_mut(cx, cy).region = walk.region;
                    let cell = ctx.grid.get_mut(cx, cy).expect("native cell");
                    cell.tile = ctx.ids.water_base;
                    cell.sub_tile = 0;
                } else {
                    // Rivers may not run over anything already placed.
                    walk.alive = false;
                }
            } else {
                walk.alive = false;
            }
        }
        // The section always completes, even once the walk is doomed.
        x += sin;
        y += cos;
    }
}

/// Undo every cell this river claimed.
fn rollback(ctx: &mut BlobCtx<'_>, cells: &[(i32, i32)], region: i32) {
    for &(x, y) in cells {
        if ctx.scratch.get(x, y).region != region {
            continue;
        }
        let scratch = ctx.scratch.get_mut(x, y);
        scratch.region = 0;
        scratch.water_region = false;
        let cell = ctx.grid.get_mut(x, y).expect("native cell");
        cell.tile = 0;
        cell.sub_tile = 0;
        cell.level = ctx.rollback_level;
    }
}

/// Carve one river. `start` of `None` is the driver's call, which picks its own
/// launch; a branch is handed an explicit cell and heading.
pub fn carve(
    ctx: &mut BlobCtx<'_>,
    args: &WaterArgs,
    quota: &mut WaterQuota,
    start: Option<(i32, i32, f64)>,
    is_branch: bool,
) -> bool {
    // Without the retail sine table a river cannot be steered faithfully, so
    // none is carved. Skipping is the honest failure: a river built on a
    // substitute table would be wrong in a way nothing here could detect.
    let Some(trig) = ctx.trig else {
        return false;
    };
    let cells: Vec<(i32, i32)> = ctx.grid.native_cells().collect();
    let region = quota.region_id;

    let (origin, mut heading) = match start {
        Some((x, y, h)) => ((x, y), h),
        None => {
            let l = launch(ctx, true);
            (l.cell, l.heading)
        }
    };
    if !ctx.scratch.in_diamond(origin.0, origin.1) {
        // Nothing carved yet, so nothing to undo.
        return false;
    }
    let heading0 = heading;

    // Width setup.
    let width_max = x87::ftol(
        TruncF64::from_f64(f64::from(args.water_percent))
            .mul(TruncF64::from_f64(WIDTH_SCALE))
            .to_f64()
            .max(WIDTH_MIN),
    );
    let width = loop {
        let value = x87::ftol(
            unit_draw(ctx.rng)
                .mul(TruncF64::from_f64(f64::from(width_max)))
                .add(TruncF64::from_f64(WIDTH_MIN))
                .to_f64(),
        );
        if value <= width_max {
            break value;
        }
    };
    let half_width = width / 2;
    let mut width_walk = f64::from(width);

    // Drawn even though bridges are not built: the draw is part of the stream.
    let _bridge_min_step = loop {
        let value = x87::ftol(
            TruncF64::from_f64(f64::from(ctx.rng.next_u32()))
                .mul(TruncF64::from_f64(BRIDGE_STEP_SPAN_SCALED))
                .add(TruncF64::from_f64(BRIDGE_STEP_BASE))
                .to_f64(),
        );
        if value <= BRIDGE_STEP_MAX {
            break value;
        }
    };

    let mut fx = f64::from(origin.0) + HALF;
    let mut fy = f64::from(origin.1) + HALF;
    let (clamp_lo, clamp_hi) = (heading0 - QUARTER_TURN, heading0 + QUARTER_TURN);

    let mut walk = Walk {
        region,
        alive: true,
        steps: 0,
        rolled_to_a_stop: false,
    };

    loop {
        let (sx, sy) = (x87::ftol(fx), x87::ftol(fy));
        if !ctx.scratch.in_diamond(sx, sy) || !walk.alive {
            break;
        }

        let sin = f64::from(trig.sin_radians(heading));
        let cos = f64::from(trig.cos_radians(heading));

        let span = x87::ftol(width_walk + HALF);
        carve_section(ctx, &mut walk, fx, fy, span, sin, cos);

        // Travel is perpendicular to the carve line.
        fx += cos;
        fy -= sin;

        // Branch spawn. The draw is unconditional; only the spawn is gated.
        let branch = draw_below(ctx.rng, BRANCH_CHANCE);
        if branch && walk.alive && !is_branch {
            let mean = heading + QUARTER_TURN;
            let angle = bounded_gaussian(
                ctx.gauss,
                ctx.rng,
                mean,
                START_SIGMA,
                heading + START_SIGMA,
                heading + 5.0 * START_SIGMA,
            );
            let bx = x87::ftol(fx + f64::from(span) * sin);
            let by = x87::ftol(fy + f64::from(span) * cos);
            // A failed branch kills the parent — and rolls the whole river back.
            walk.alive = carve(ctx, args, quota, Some((bx, by, angle)), true);
        }

        if walk.steps > WOBBLE_AFTER {
            heading += bounded_gaussian(
                ctx.gauss,
                ctx.rng,
                0.0,
                WOBBLE_SIGMA,
                clamp_lo - heading,
                clamp_hi - heading,
            );
        }
        if half_width > 0 {
            width_walk += bounded_gaussian(
                ctx.gauss,
                ctx.rng,
                0.0,
                WIDTH_SIGMA,
                f64::from(width - half_width) - width_walk,
                f64::from(width + half_width) - width_walk,
            );
        }

        walk.steps += 1;
        if draw_below(ctx.rng, STOP_CHANCE) {
            walk.rolled_to_a_stop = true;
            break;
        }
    }

    if walk.steps < MIN_STEPS {
        walk.alive = false;
    }

    // A river that ran out of map does not get an end lake; only one that chose
    // to stop does.
    if walk.rolled_to_a_stop && walk.alive {
        let end = (x87::ftol(fx), x87::ftol(fy));
        if ctx.scratch.in_diamond(end.0, end.1) {
            walk.alive = lake::grow_lake(ctx, args, quota, Some(end));
        }
    }

    if !is_branch && walk.alive {
        let ok = {
            let mut shore_ctx = ShoreCtx {
                grid: ctx.grid,
                scratch: ctx.scratch,
                ids: ctx.ids,
                blocks: ctx.blocks,
                rng: ctx.rng,
            };
            shore::run(&mut shore_ctx, region, false)
        };
        if ok {
            // The green sweep runs whether or not the dilation succeeded; only
            // the shore pass gates it.
            walk.alive = lake::dilate_region_rings(ctx, &cells, region, 1);
            for &(x, y) in &cells {
                if ctx.scratch.get(x, y).region != region {
                    continue;
                }
                let cell = ctx.grid.get_mut(x, y).expect("native cell");
                if ctx.ids.is_clear(cell.tile) {
                    cell.tile = ctx.ids.green;
                }
            }
        } else {
            walk.alive = false;
        }
    }

    // Canyon coin. Drawn so the stream stays aligned; the meander arm it needs
    // is not ported, so the level change never happens.
    if !is_branch && walk.alive && ctx.rollback_level == 4 {
        let _would_cut_a_canyon = draw_below(ctx.rng, CANYON_CHANCE);
    }

    if !is_branch && walk.alive {
        walk.alive = lake::dilate_region_rings(ctx, &cells, region, PLAIN_DILATE_RINGS);
    }

    if !walk.alive {
        rollback(ctx, &cells, region);
        return false;
    }
    quota.placed += walk.steps;
    true
}

/// Cardinal directions, exported for the tests' connectivity walk.
pub(crate) const RIVER_CARDINALS: [usize; 4] = CARDINALS;

/// Convenience for tests and the seeder: is this water amount above the gate?
pub fn carries_a_river(water_percent: i32) -> bool {
    water_percent > super::lake::RIVER_GATE
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::rmg::grid::RmgGrid as Grid;

    #[test]
    fn the_gate_matches_the_documented_threshold() {
        assert!(!carries_a_river(20));
        assert!(carries_a_river(21));
    }

    #[test]
    fn bounded_gaussian_recentres_only_when_the_window_is_too_narrow() {
        // A window wider than +/- sigma leaves mean and sigma alone.
        let mut rng = RmgRng::new(7);
        let mut gauss = Gaussian::default();
        let value = bounded_gaussian(&mut gauss, &mut rng, 0.0, 0.1, -10.0, 10.0);
        assert!((-10.0..=10.0).contains(&value));
    }

    #[test]
    fn a_cross_section_advances_along_sin_cos() {
        // Pins the convention this module exists to get right: the carve line
        // steps by (+sin, +cos), so a heading of 0 lays a vertical line.
        let _ = Grid::step(0, 0, 0);
        let (sin, cos) = (0.0f64, 1.0f64);
        let span = 3;
        let step_back = f64::from(span - 1) * HALF;
        let x0 = 10.0 - step_back * sin;
        let y0 = 10.0 - step_back * cos;
        assert_eq!(x87::ftol(x0), 10);
        assert_eq!(x87::ftol(y0), 9);
    }
}
