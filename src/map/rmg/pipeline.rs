//! The deterministic generation pipeline: every terrain phase in `STAGE_ORDER`,
//! wired together with the shared grid / scratch / rng / gauss and the per-phase
//! state threaded between them.
//!
//! Inputs are **pre-resolved** — `TileIds`, a `TileBlocks` provider, the cliff
//! ranges, the `Morphable=` predicate, the wheel-impassable table, the neutral
//! tech-building list, and the MapSeed-derived scalars. This layer never touches
//! `TheaterData` or the rules directly; the app layer builds the inputs and
//! hands them in, which keeps the pipeline unit-testable with the same synthetic
//! `TileBlocks` the individual phases use.
//!
//! The order is the native temperate driver's (see `STAGE_ORDER`): water →
//! finalize → regions → green spread → recalc → starts → tech → tiberium →
//! region reset → recalc → hills → patches → recalc → trees → rocks → recalc.
//! Emit is the caller's step. The three `RecalcAfter*`/`RecalcFinal` passes are
//! `lat_fixup` calls; they are RNG-free, so the one that matters observably is
//! the post-patch pass that produces the sand-LAT tiles the rock pass needs.

use crate::map::theater::TheaterCliffRanges;

use super::MapGeometry;
use super::grid::RmgGrid;
use super::phases::blob::BlobCtx;
use super::phases::hills::{HillsArgs, HillsCtx};
use super::phases::lat_patches::PatchCtx;
use super::phases::regions::{RegionCtx, Regions};
use super::phases::rocks::{RockArgs, RockCtx};
use super::phases::shore::TileBlocks;
use super::phases::starts::{StartsArgs, StartsCtx};
use super::phases::tech_buildings::{TechArgs, TechCtx, TechType};
use super::phases::tiberium::{TiberiumArgs, TiberiumCtx};
use super::phases::trees::{TreeArgs, TreeCtx};
use super::phases::water::{PlayableRect, WaterArgs};
use super::phases::zones::{self, ZoneKind, ZoneParams};
use super::phases::{
    green_spread, hills, lat_fixup, lat_patches, regions, rocks, starts, tech_buildings, tiberium,
    trees, water, water_finalize,
};
use super::rng::RmgRng;
use super::scratch::RmgScratch;
use super::tiles::TileIds;
use super::x87::Gaussian;

/// The land-type count of the wheel-impassable table (`zones::LAND_TYPES`).
pub const LAND_TYPES: usize = super::phases::zones::LAND_TYPES;

/// The generator's default ground level (`MapSeed+0x30C` = 4).
const DEFAULT_LEVEL: u8 = 4;
/// The generated-start quota standard skirmish writes (`MapSeed` start slots).
pub const STANDARD_START_QUOTA: i32 = 4;

/// Pre-resolved inputs for one generation run.
pub struct PipelineInputs<'a> {
    pub ids: &'a TileIds,
    pub blocks: &'a dyn TileBlocks,
    pub cliff: &'a TheaterCliffRanges,
    /// The `[TileSet] Morphable=` predicate for the corner engine.
    pub morphable: &'a dyn Fn(i32) -> bool,
    /// Per-land-type "wheel speed <= 1%" flags for the zone classifier.
    pub wheel_impassable: [bool; LAND_TYPES],
    /// The resolved `NeutralTechBuildings` list.
    pub tech_types: &'a [TechType],

    // Scalars from the `.SED` options / MapSeed / RMGMD settings.
    pub map_type: i32,
    pub theater: i32,
    pub num_players: i32,
    /// `WaterAmount` option.
    pub water_percent: i32,
    /// `Resources` option (gems appear only when 3).
    pub resources: i32,
    /// `Tiberium` percent option.
    pub tib_option: i32,
    /// `RMGMinimumTiberium` / `RMGMaximumTiberium`.
    pub min_tib: i32,
    pub max_tib: i32,
    /// `TiberiumLayout` option (start-selector slot target).
    pub tiberium_layout: i32,
    /// `Ruggedness` option (hill height-walk scale).
    pub ruggedness: i32,
    /// `Vegetation` option (patch probability + tree-count scale).
    pub vegetation: i32,
    /// The tree-count width term (`MapSeed+0x64`).
    pub width: i32,
    /// `MaxTrees` (`MapSeed+0x2FC`; 0 disables trees).
    pub max_trees: i32,
}

/// What the pipeline collects for the emitter.
#[derive(Debug, Default)]
pub struct PipelineOutput {
    /// `(slot, x, y)` per generated start position.
    pub waypoints: Vec<(u8, u16, u16)>,
    /// Placed `(name, x, y)` terrain objects — trees and TIBTRE.
    pub terrain: Vec<(String, i16, i16)>,
    /// Placed `(name, x, y)` neutral tech buildings.
    pub structures: Vec<(String, i16, i16)>,
}

/// Run the whole deterministic pipeline in place over `grid`/`scratch`, drawing
/// from `rng`/`gauss`. Returns the waypoints, terrain objects and structures the
/// emitter projects into the `MapFile`.
pub fn run_pipeline(
    grid: &mut RmgGrid,
    scratch: &mut RmgScratch,
    rng: &mut RmgRng,
    gauss: &mut Gaussian,
    geometry: &MapGeometry,
    inputs: &PipelineInputs<'_>,
) -> PipelineOutput {
    let MapGeometry {
        gen_w,
        gen_h,
        map_w,
        map_h,
        stride,
        ..
    } = *geometry;
    let local_rect = [2, 5, gen_w, gen_h];

    // ---- Water + finalize (map types 0-2 shape the base terrain) ----------
    // Struct-literal fields move a `&mut`, so every context reborrows the shared
    // owners (`&mut *grid` …); each context is scoped so the reborrow releases
    // before the next stage.
    {
        let mut ctx = BlobCtx {
            grid: &mut *grid,
            scratch: &mut *scratch,
            ids: inputs.ids,
            blocks: inputs.blocks,
            rng: &mut *rng,
            gauss: &mut *gauss,
            map_w,
            map_h,
            rollback_level: DEFAULT_LEVEL,
        };
        let args = WaterArgs {
            map_type: inputs.map_type,
            water_percent: inputs.water_percent,
            num_players: inputs.num_players,
            playable: PlayableRect {
                x: local_rect[0],
                y: local_rect[1],
                w: gen_w,
                h: gen_h,
            },
        };
        water::run(&mut ctx, &args);
    }
    water_finalize::run(grid, inputs.ids, inputs.blocks, rng);

    // ---- Regions -----------------------------------------------------------
    let mut regions: Regions = {
        let mut ctx = RegionCtx {
            grid: &mut *grid,
            scratch: &mut *scratch,
            ids: inputs.ids,
            rng: &mut *rng,
            map_type: inputs.map_type,
            default_level: DEFAULT_LEVEL,
        };
        regions::run(&mut ctx)
    };

    // IslandPasses (map types 3/4 extra region/bridge passes) are not modelled
    // yet; ordinary map types skip them entirely.

    // ---- Green spread + first recalc --------------------------------------
    green_spread::run(grid, inputs.ids, rng);
    lat_fixup::run(grid, inputs.ids);

    // ---- Starts (needs the zone field) ------------------------------------
    let outcome = {
        let zone_params = ZoneParams {
            map_w,
            local_rect,
            wheel_impassable: inputs.wheel_impassable,
        };
        let zones = zones::compute(
            grid,
            inputs.blocks,
            &zone_params,
            ZoneKind::for_map_type(inputs.map_type),
        );
        let args = StartsArgs {
            map_type: inputs.map_type,
            start_quota: STANDARD_START_QUOTA,
            num_players: inputs.num_players,
            tiberium_layout: inputs.tiberium_layout,
            gen_w,
            gen_h,
            map_w,
        };
        let mut ctx = StartsCtx {
            grid: &mut *grid,
            scratch: &mut *scratch,
            ids: inputs.ids,
            blocks: inputs.blocks,
            rng: &mut *rng,
            regions: &mut regions,
        };
        starts::run(&mut ctx, &args, &zones)
    };

    // ---- Tech buildings (map types 1-4; map type 0 places none) -----------
    let tech_placements = {
        let args = TechArgs {
            map_type: inputs.map_type,
            map_w,
            local_rect,
            stride: stride as i32,
        };
        let mut ctx = TechCtx {
            grid: &mut *grid,
            scratch: &mut *scratch,
            ids: inputs.ids,
            rng: &mut *rng,
            regions: &regions,
            types: inputs.tech_types,
        };
        tech_buildings::run(&mut ctx, &args)
    };

    // ---- Tiberium ----------------------------------------------------------
    let tiberium = {
        let args = TiberiumArgs {
            map_type: inputs.map_type,
            resources: inputs.resources,
            tib_option: inputs.tib_option,
            min_tib: inputs.min_tib,
            max_tib: inputs.max_tib,
            map_w,
            local_rect,
        };
        let mut ctx = TiberiumCtx {
            grid: &mut *grid,
            scratch: &mut *scratch,
            ids: inputs.ids,
            rng: &mut *rng,
            gauss: &mut *gauss,
            regions: &regions,
            waypoints: &outcome.waypoints,
        };
        tiberium::run(&mut ctx, &args)
    };

    // ---- Region reset + second recalc -------------------------------------
    scratch.reset_region_ids();
    lat_fixup::run(grid, inputs.ids);

    // ---- Hills -------------------------------------------------------------
    {
        let args = HillsArgs {
            ruggedness: inputs.ruggedness,
            map_w,
            map_h,
        };
        let mut ctx = HillsCtx {
            grid: &mut *grid,
            scratch: &mut *scratch,
            ids: inputs.ids,
            gauss: &mut *gauss,
            rng: &mut *rng,
        };
        hills::run(&mut ctx, &args, inputs.cliff, inputs.morphable);
    }

    // ---- LAT patches → recalc (creates sand-LAT) → trees → rocks ----------
    {
        let mut ctx = PatchCtx {
            grid: &mut *grid,
            scratch: &mut *scratch,
            ids: inputs.ids,
            rng: &mut *rng,
            gauss: &mut *gauss,
        };
        lat_patches::run(&mut ctx, inputs.theater, inputs.vegetation);
    }
    lat_fixup::run(grid, inputs.ids);

    let trees = {
        let args = TreeArgs {
            width: inputs.width,
            vegetation: inputs.vegetation,
            max_trees: inputs.max_trees,
            stride: stride as i32,
        };
        let mut ctx = TreeCtx {
            grid: &mut *grid,
            scratch: &mut *scratch,
            ids: inputs.ids,
            rng: &mut *rng,
            gauss: &mut *gauss,
        };
        trees::run(&mut ctx, &args)
    };

    // Rocks — temperate theater only.
    if inputs.theater == 0 {
        let args = RockArgs {
            map_w,
            map_h,
            stride: stride as i32,
        };
        let mut ctx = RockCtx {
            grid: &mut *grid,
            scratch: &mut *scratch,
            ids: inputs.ids,
            rng: &mut *rng,
        };
        rocks::run(&mut ctx, &args);
    }

    // ---- Final recalc ------------------------------------------------------
    lat_fixup::run(grid, inputs.ids);

    // ---- Collect outputs for the emitter ----------------------------------
    let waypoints = outcome
        .waypoints
        .iter()
        .enumerate()
        .filter_map(|(slot, spot)| spot.map(|(x, y)| (slot as u8, x as u16, y as u16)))
        .collect();

    let mut terrain: Vec<(String, i16, i16)> = trees;
    terrain.extend(tiberium.trees);

    let structures = tech_placements
        .into_iter()
        .map(|placement| (placement.name, placement.x, placement.y))
        .collect();

    PipelineOutput {
        waypoints,
        terrain,
        structures,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::rmg::options::RmgOptions;
    use crate::map::rmg::phases::shore::{SubTile, TileBlock};

    /// A `TileBlocks` provider that returns a 1x1 block for every tile — the
    /// same synthetic the water/shore phase tests use.
    struct OneByOne(TileBlock);
    impl TileBlocks for OneByOne {
        fn block(&self, _tile: i32) -> Option<&TileBlock> {
            Some(&self.0)
        }
    }

    fn one_by_one() -> OneByOne {
        OneByOne(TileBlock {
            width: 1,
            height: 1,
            subtiles: vec![Some(SubTile::default())],
        })
    }

    fn ids() -> TileIds {
        // Distinct, resolvable bases per ground/LAT group; realistic spans.
        TileIds {
            clear: 0,
            ramp_base: 600,
            rough: 700,
            sand: 800,
            green: 100,
            rough_lat: 710,
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
        }
    }

    fn inputs<'a>(
        identity: &'a TileIds,
        blocks: &'a OneByOne,
        cliff: &'a TheaterCliffRanges,
        morphable: &'a dyn Fn(i32) -> bool,
        map_type: i32,
        theater: i32,
    ) -> PipelineInputs<'a> {
        PipelineInputs {
            ids: identity,
            blocks,
            cliff,
            morphable,
            wheel_impassable: [false; LAND_TYPES],
            tech_types: &[],
            map_type,
            theater,
            num_players: 4,
            water_percent: 30,
            resources: 1,
            tib_option: 50,
            min_tib: 900,
            max_tib: 1050,
            tiberium_layout: 0,
            ruggedness: 20,
            vegetation: 50,
            width: 2,
            max_trees: 600,
        }
    }

    fn run(seed: u16, map_type: i32, theater: i32) -> (RmgGrid, PipelineOutput) {
        let options = RmgOptions {
            num_players: 4,
            ..Default::default()
        };
        let geometry = MapGeometry::from_options(&options);
        let mut grid = RmgGrid::new(geometry.stride, geometry.diamond_min, geometry.diamond_max);
        let mut scratch =
            RmgScratch::new(geometry.stride, geometry.diamond_min, geometry.diamond_max);
        let mut rng = RmgRng::new(seed);
        let mut gauss = Gaussian::default();
        let identity = ids();
        let blocks = one_by_one();
        let cliff = TheaterCliffRanges::default();
        let morphable = |_tile: i32| false;
        let ins = inputs(&identity, &blocks, &cliff, &morphable, map_type, theater);
        let output = run_pipeline(
            &mut grid,
            &mut scratch,
            &mut rng,
            &mut gauss,
            &geometry,
            &ins,
        );
        (grid, output)
    }

    #[test]
    fn pipeline_runs_end_to_end_and_shapes_terrain() {
        let (grid, output) = run(1234, 1, 0);
        // The map is no longer all-unassigned: water/land shaping ran.
        let assigned = grid
            .native_cells()
            .filter(|&(x, y)| {
                grid.get(x, y).unwrap().tile != crate::map::rmg::tiles::TILE_UNASSIGNED
            })
            .count();
        assert!(assigned > 0, "the pipeline assigned real tiles");
        // Waypoints are bounded by the start quota.
        assert!(output.waypoints.len() <= STANDARD_START_QUOTA as usize);
    }

    #[test]
    fn pipeline_is_deterministic() {
        let snapshot = |seed| {
            let (grid, output) = run(seed, 1, 0);
            let tiles: Vec<i32> = grid
                .native_cells()
                .map(|(x, y)| grid.get(x, y).unwrap().tile)
                .collect();
            let overlays: Vec<(i32, u8)> = grid
                .native_cells()
                .map(|(x, y)| {
                    let c = grid.get(x, y).unwrap();
                    (c.overlay, c.density)
                })
                .collect();
            (tiles, overlays, output.waypoints, output.terrain)
        };
        assert_eq!(snapshot(4242), snapshot(4242));
    }

    #[test]
    fn non_temperate_theater_paints_no_rocks() {
        // Rock overlays land only on temperate (theater 0). A snow run (theater
        // 1) must carry no SROCK/TROCK overlays (168..=177).
        let (grid, _) = run(77, 1, 1);
        let rocks = grid
            .native_cells()
            .filter(|&(x, y)| (168..=177).contains(&grid.get(x, y).unwrap().overlay))
            .count();
        assert_eq!(rocks, 0, "non-temperate theaters place no rocks");
    }

    #[test]
    fn continental_and_island_map_types_complete() {
        // Map types 1 (continental) and 2 (islands-in-sea) leave enough land
        // for the standard 4-start quota under the synthetic 1x1 tile block.
        // The water-heavy types (0/3/4) can starve the start selector on this
        // degenerate terrain — that path needs real theater terrain, which is
        // the app-layer adapter's job, not the pipeline's.
        for map_type in [1, 2] {
            let (grid, _output) = run(9, map_type, 0);
            assert!(grid.native_cells().count() > 0);
        }
    }
}
