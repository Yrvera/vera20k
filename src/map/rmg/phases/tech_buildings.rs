//! Neutral tech-building placement (skipped entirely on map type 0).
//!
//! Two driver paths share one foundation gate:
//! - map type 2: `uniform(0,2)` passes; each pass walks the regions in
//!   creation order and, for every region with a nonzero start quota, makes
//!   one building whose anchor is drawn from that region's cell list.
//! - map types 1/3/4: `uniform(0,4)` buildings, each anchored at a uniform
//!   random clear scratch cell over the whole map (200-try inner reject).
//!
//! Per building exactly one type is drawn from the resolved
//! `NeutralTechBuildings` list, then up to 100 placement attempts run. The
//! foundation walk consumes no RNG; it only decides whether the building
//! commits, so the draw stream depends solely on the pass/count draw, the
//! per-building type draw, and the anchor draws.

use crate::map::rmg::grid::RmgGrid;
use crate::map::rmg::rng::RmgRng;
use crate::map::rmg::scratch::RmgScratch;
use crate::map::rmg::tiles::TileIds;

use super::regions::Regions;
use super::zones::diamond_frame_contains;

/// Placement attempts per building.
const MAX_ATTEMPTS: i32 = 100;
/// Inner clear-anchor draws per attempt on the whole-map path.
const MAX_ANCHOR_DRAWS: i32 = 200;

/// A resolved neutral tech building: its type name plus foundation footprint
/// cell offsets (NW-anchor-relative). The footprint is the base rectangle the
/// original walks from `type+0xDFC`.
#[derive(Debug, Clone)]
pub struct TechType {
    pub name: String,
    pub footprint: Vec<(i16, i16)>,
}

/// Phase inputs.
#[derive(Debug, Clone, Copy)]
pub struct TechArgs {
    pub map_type: i32,
    pub map_w: i32,
    /// Playfield local rect `{x, y, w, h}` — generated maps use `(2, 5, w, h)`.
    pub local_rect: [i32; 4],
    /// Scratch/grid stride: the whole-map anchor draw is `uniform(0, S²−1)`.
    pub stride: i32,
}

/// One recorded placement, emitted later as a neutral `[Structures]` entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TechPlacement {
    pub name: String,
    pub x: i16,
    pub y: i16,
}

/// Everything the phase borrows.
pub struct TechCtx<'a> {
    pub grid: &'a mut RmgGrid,
    pub scratch: &'a mut RmgScratch,
    pub ids: &'a TileIds,
    pub rng: &'a mut RmgRng,
    pub regions: &'a Regions,
    pub types: &'a [TechType],
}

pub fn run(ctx: &mut TechCtx<'_>, args: &TechArgs) -> Vec<TechPlacement> {
    let mut placements = Vec::new();
    // The driver is not called at all for map type 0.
    if args.map_type == 0 || ctx.types.is_empty() {
        return placements;
    }

    if args.map_type == 2 {
        let passes = ctx.rng.uniform(0, 2);
        for _ in 0..passes {
            // Regions in creation order; only quota-bearing regions build.
            for index in 0..ctx.regions.list.len() {
                if ctx.regions.list[index].start_quota <= 0 {
                    continue;
                }
                place_from_region(ctx, args, index, &mut placements);
            }
        }
    } else {
        let count = ctx.rng.uniform(0, 4);
        for _ in 0..count {
            place_whole_map(ctx, args, &mut placements);
        }
    }
    placements
}

/// Draw one building type index into the resolved list.
fn draw_type(ctx: &mut TechCtx<'_>) -> usize {
    ctx.rng.uniform(0, ctx.types.len() as i32 - 1) as usize
}

/// Map-type-2 path: anchor drawn from the region's cell list, ≤100 attempts.
fn place_from_region(
    ctx: &mut TechCtx<'_>,
    args: &TechArgs,
    region_index: usize,
    out: &mut Vec<TechPlacement>,
) {
    let type_index = draw_type(ctx);
    let cell_count = ctx.regions.list[region_index].cells.len() as i32;
    debug_assert!(cell_count > 0, "quota-bearing regions own cells");

    for _ in 0..MAX_ATTEMPTS {
        let pick = ctx.rng.uniform(0, cell_count - 1) as usize;
        let (cx, cy) = ctx.regions.list[region_index].cells[pick];
        let anchor = (i32::from(cx), i32::from(cy));
        if try_place(ctx, args, type_index, anchor, out) {
            return;
        }
    }
}

/// Map-types-1/3/4 path: anchor is a uniform random clear scratch cell over
/// the whole array, with the inner empty-slot / non-clear rejection.
fn place_whole_map(ctx: &mut TechCtx<'_>, args: &TechArgs, out: &mut Vec<TechPlacement>) {
    let type_index = draw_type(ctx);
    for _ in 0..MAX_ATTEMPTS {
        let anchor = draw_clear_anchor(ctx, args.stride);
        if try_place(ctx, args, type_index, anchor, out) {
            return;
        }
    }
}

/// Draw a clear map anchor: reject empty (coord `(0,0)`) scratch slots on
/// every draw, and reject non-clear cells for up to 200 non-empty draws. On
/// exhaustion the original keeps a `(0,0)` anchor, which fails the gate — so
/// the port returns that sentinel and lets `try_place` reject it.
fn draw_clear_anchor(ctx: &mut TechCtx<'_>, stride: i32) -> (i32, i32) {
    let max = stride * stride - 1;
    let mut non_empty_draws = 0;
    loop {
        // Redraw until the drawn slot is a real cell (non-`(0,0)` coord).
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

/// Validate a building's footprint at `anchor` and, on success, mark the
/// cells occupied and record the placement. Returns whether it placed.
fn try_place(
    ctx: &mut TechCtx<'_>,
    args: &TechArgs,
    type_index: usize,
    anchor: (i32, i32),
    out: &mut Vec<TechPlacement>,
) -> bool {
    let anchor_level = match ctx.grid.get(anchor.0, anchor.1) {
        Some(cell) => cell.level,
        // A `(0,0)`/out-of-band anchor fails immediately (border cell).
        None => return false,
    };

    let footprint = ctx.types[type_index].footprint.clone();
    for &(dx, dy) in &footprint {
        let (fx, fy) = (anchor.0 + i32::from(dx), anchor.1 + i32::from(dy));
        if !foundation_cell_ok(ctx, args, fx, fy, anchor_level) {
            return false;
        }
    }

    // All foundation cells passed: occupy them and record the building.
    for &(dx, dy) in &footprint {
        let (fx, fy) = (anchor.0 + i32::from(dx), anchor.1 + i32::from(dy));
        if let Some(cell) = ctx.grid.get_mut(fx, fy) {
            cell.occupied = true;
        }
    }
    out.push(TechPlacement {
        name: ctx.types[type_index].name.clone(),
        x: anchor.0 as i16,
        y: anchor.1 as i16,
    });
    true
}

/// One foundation cell's gate: unoccupied, clear, level-matched, inside the
/// playfield frame, and outside every start's protected clearing.
///
/// The original also rejects land type 3 (Rock), but that is unreachable
/// here: the clear-tile test already restricts the cell to tile 0 / the
/// unassigned sentinel, both of which classify as land type 0, so the
/// land-type branch can never fire on generated terrain. Omitted as a proven
/// no-op.
fn foundation_cell_ok(
    ctx: &TechCtx<'_>,
    args: &TechArgs,
    x: i32,
    y: i32,
    anchor_level: u8,
) -> bool {
    let Some(cell) = ctx.grid.get(x, y) else {
        return false;
    };
    if cell.occupied {
        return false;
    }
    if !ctx.ids.is_clear(cell.tile) {
        return false;
    }
    if cell.level != anchor_level {
        return false;
    }
    if !diamond_frame_contains(
        args.map_w,
        args.local_rect,
        x,
        y,
        cell.level,
        cell.slope != 0,
    ) {
        return false;
    }
    // Protected start-clearing flag (scratch +0x45).
    if ctx.scratch.get(x, y).water_lock {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::rmg::phases::regions::RmgRegion;
    use crate::map::rmg::tiles::SpecialTerrain;

    fn ids() -> TileIds {
        TileIds {
            clear: 0,
            ramp_base: -1,
            ramp_smooth: -1,
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

    fn types() -> Vec<TechType> {
        // A 2x2 and a 1x1, so the type draw picks between distinct footprints.
        vec![
            TechType {
                name: "CATHOSP".into(),
                footprint: vec![(0, 0), (1, 0), (0, 1), (1, 1)],
            },
            TechType {
                name: "CAPOWR".into(),
                footprint: vec![(0, 0)],
            },
        ]
    }

    struct World {
        grid: RmgGrid,
        scratch: RmgScratch,
        args: TechArgs,
    }

    fn world(map_type: i32) -> World {
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
            args: TechArgs {
                map_type,
                map_w,
                local_rect: [2, 5, gen_w, gen_h],
                stride,
            },
        }
    }

    fn regions_with_one(cells: Vec<(i16, i16)>, quota: i32) -> Regions {
        let mut regions = Regions::default();
        regions.list.push(RmgRegion {
            id: 0,
            level: 4,
            active: false,
            seed: cells[0],
            done: false,
            cell_count: cells.len() as i32,
            cells,
            start_quota: quota,
            field_slots: None,
        });
        regions.id_counter = 1;
        regions
    }

    #[test]
    fn map_type_zero_is_a_no_op() {
        let mut w = world(0);
        let identity = ids();
        let type_list = types();
        let regions = Regions::default();
        let mut rng = RmgRng::new(1);
        let mut ctx = TechCtx {
            grid: &mut w.grid,
            scratch: &mut w.scratch,
            ids: &identity,
            rng: &mut rng,
            regions: &regions,
            types: &type_list,
        };
        assert!(run(&mut ctx, &w.args).is_empty());
        // No draws consumed either.
        let mut fresh = RmgRng::new(1);
        assert_eq!(rng.next_u32(), fresh.next_u32());
    }

    #[test]
    fn whole_map_places_neutral_buildings_on_clear_ground() {
        let mut w = world(1);
        let identity = ids();
        let type_list = types();
        let regions = Regions::default();
        let mut rng = RmgRng::new(1234);
        let placements = {
            let mut ctx = TechCtx {
                grid: &mut w.grid,
                scratch: &mut w.scratch,
                ids: &identity,
                rng: &mut rng,
                regions: &regions,
                types: &type_list,
            };
            run(&mut ctx, &w.args)
        };
        // uniform(0,4) buildings; a wide-open map places at least some.
        assert!(placements.len() <= 4);
        for placement in &placements {
            let cell = w
                .grid
                .get(i32::from(placement.x), i32::from(placement.y))
                .expect("anchor in band");
            assert!(cell.occupied, "placed anchor is occupied");
            assert!(cell.tile == 0 || cell.tile == 0xFFFF);
            assert!(["CATHOSP", "CAPOWR"].contains(&placement.name.as_str()));
        }
    }

    #[test]
    fn placed_footprints_do_not_overlap() {
        let mut w = world(1);
        let identity = ids();
        let type_list = types();
        let regions = Regions::default();
        let mut rng = RmgRng::new(555);
        let placements = {
            let mut ctx = TechCtx {
                grid: &mut w.grid,
                scratch: &mut w.scratch,
                ids: &identity,
                rng: &mut rng,
                regions: &regions,
                types: &type_list,
            };
            run(&mut ctx, &w.args)
        };
        // Every footprint cell of every placement is distinct: a later
        // building rejects any cell an earlier one occupied.
        let mut seen = std::collections::HashSet::new();
        for placement in &placements {
            let footprint = type_list
                .iter()
                .find(|t| t.name == placement.name)
                .unwrap()
                .footprint
                .clone();
            for (dx, dy) in footprint {
                let cell = (placement.x + dx, placement.y + dy);
                assert!(seen.insert(cell), "footprint cell {cell:?} reused");
            }
        }
    }

    #[test]
    fn map_type_two_uses_the_region_cell_list() {
        let mut w = world(2);
        let identity = ids();
        let type_list = types();
        // A compact 6x6 block of region cells, all clear at level 4.
        let cells: Vec<(i16, i16)> = (20..26)
            .flat_map(|x| (25..31).map(move |y| (x as i16, y as i16)))
            .filter(|&(x, y)| w.grid.is_valid(i32::from(x), i32::from(y)))
            .collect();
        let regions = regions_with_one(cells.clone(), 2);
        let mut rng = RmgRng::new(99);
        let placements = {
            let mut ctx = TechCtx {
                grid: &mut w.grid,
                scratch: &mut w.scratch,
                ids: &identity,
                rng: &mut rng,
                regions: &regions,
                types: &type_list,
            };
            run(&mut ctx, &w.args)
        };
        // passes = uniform(0,2); every placement anchors inside the region.
        for placement in &placements {
            assert!(
                cells.contains(&(placement.x, placement.y)),
                "anchor {:?} came from the region cell list",
                (placement.x, placement.y)
            );
        }
    }

    #[test]
    fn occupied_cells_block_the_foundation() {
        let mut w = world(2);
        let identity = ids();
        let type_list = types();
        let cells: Vec<(i16, i16)> = (20..26)
            .flat_map(|x| (25..31).map(move |y| (x as i16, y as i16)))
            .filter(|&(x, y)| w.grid.is_valid(i32::from(x), i32::from(y)))
            .collect();
        // Pre-occupy every region cell: no foundation can pass.
        for &(x, y) in &cells {
            w.grid.get_mut(i32::from(x), i32::from(y)).unwrap().occupied = true;
        }
        let regions = regions_with_one(cells, 2);
        let mut rng = RmgRng::new(99);
        let mut ctx = TechCtx {
            grid: &mut w.grid,
            scratch: &mut w.scratch,
            ids: &identity,
            rng: &mut rng,
            regions: &regions,
            types: &type_list,
        };
        assert!(run(&mut ctx, &w.args).is_empty(), "all cells occupied");
    }

    #[test]
    fn protected_clearing_blocks_placement() {
        let mut w = world(1);
        let identity = ids();
        let type_list = types();
        let regions = Regions::default();
        // Mark the whole map as protected start-clearing: nothing places.
        for (x, y) in w.grid.native_cells().collect::<Vec<_>>() {
            w.scratch.get_mut(x, y).water_lock = true;
        }
        let mut rng = RmgRng::new(1234);
        let mut ctx = TechCtx {
            grid: &mut w.grid,
            scratch: &mut w.scratch,
            ids: &identity,
            rng: &mut rng,
            regions: &regions,
            types: &type_list,
        };
        assert!(run(&mut ctx, &w.args).is_empty(), "clearing blocks all");
    }

    #[test]
    fn level_mismatch_rejects_multi_cell_footprints() {
        let mut w = world(2);
        let identity = ids();
        // Only the 2x2 type, so a level mismatch on any of its cells rejects.
        let type_list = vec![TechType {
            name: "CATHOSP".into(),
            footprint: vec![(0, 0), (1, 0), (0, 1), (1, 1)],
        }];
        let cells: Vec<(i16, i16)> = (20..26)
            .flat_map(|x| (25..31).map(move |y| (x as i16, y as i16)))
            .filter(|&(x, y)| w.grid.is_valid(i32::from(x), i32::from(y)))
            .collect();
        // Raise every odd column to a different level: no 2x2 is uniform.
        for &(x, y) in &cells {
            if x % 2 == 1 {
                w.grid.get_mut(i32::from(x), i32::from(y)).unwrap().level = 8;
            }
        }
        let regions = regions_with_one(cells, 2);
        let mut rng = RmgRng::new(7);
        let mut ctx = TechCtx {
            grid: &mut w.grid,
            scratch: &mut w.scratch,
            ids: &identity,
            rng: &mut rng,
            regions: &regions,
            types: &type_list,
        };
        assert!(
            run(&mut ctx, &w.args).is_empty(),
            "no uniform-level 2x2 square exists"
        );
    }

    #[test]
    fn tech_placement_is_deterministic() {
        let snapshot = |seed| {
            let mut w = world(1);
            let identity = ids();
            let type_list = types();
            let regions = Regions::default();
            let mut rng = RmgRng::new(seed);
            let mut ctx = TechCtx {
                grid: &mut w.grid,
                scratch: &mut w.scratch,
                ids: &identity,
                rng: &mut rng,
                regions: &regions,
                types: &type_list,
            };
            let placements = run(&mut ctx, &w.args);
            (placements, rng.next_u32())
        };
        assert_eq!(snapshot(2024), snapshot(2024));
    }

    #[test]
    fn type_draw_advances_the_stream_even_when_nothing_places() {
        // A map with no clear ground: buildings are attempted (count + type
        // draws consumed) but none place.
        let mut w = world(1);
        let identity = ids();
        let type_list = types();
        let regions = Regions::default();
        for (x, y) in w.grid.native_cells().collect::<Vec<_>>() {
            w.grid.get_mut(x, y).unwrap().tile = 500; // all water
        }
        let mut rng = RmgRng::new(3);
        let placements = {
            let mut ctx = TechCtx {
                grid: &mut w.grid,
                scratch: &mut w.scratch,
                ids: &identity,
                rng: &mut rng,
                regions: &regions,
                types: &type_list,
            };
            run(&mut ctx, &w.args)
        };
        assert!(placements.is_empty(), "no clear ground to place on");
        // The count draw plus per-building type + anchor draws were consumed,
        // so the stream advanced past a fresh generator.
        let mut fresh = RmgRng::new(3);
        assert_ne!(rng.next_u32(), fresh.next_u32());
    }
}
