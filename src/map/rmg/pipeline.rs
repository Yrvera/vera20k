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
use super::{MapGeometry, Stage};

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
    /// Whether a river on this map may allow a bridge, decided by the
    /// generation's very first draw. Only the river carver reads the value; the
    /// draw that produces it is stream-relevant to every map type.
    pub bridge_enabled: bool,
}

/// Watches the grid between stages, for callers that draw the map as it builds.
///
/// Observation only: the grid arrives shared and the RNG, scratch and per-phase
/// state are not passed at all, so an observer cannot change what a run
/// generates. It fires at the boundary *named* by the stage whether or not that
/// stage did any work — a stage this configuration skips leaves the grid
/// untouched, so its boundary looks exactly like the one before it.
///
/// The waypoint slice is empty until `Starts` has run.
pub type StageObserver<'a> = &'a mut dyn FnMut(Stage, &RmgGrid, &[(u8, u16, u16)]);

/// What the pipeline collects for the emitter.
#[derive(Debug, Default)]
pub struct PipelineOutput {
    /// `(slot, x, y)` per generated start position.
    pub waypoints: Vec<(u8, u16, u16)>,
    /// Placed `(name, x, y)` terrain objects — trees and TIBTRE.
    pub terrain: Vec<(String, i16, i16)>,
    /// Placed `(name, x, y)` neutral tech buildings.
    pub structures: Vec<(String, i16, i16)>,
    /// Start slots no region could fill. See `StartsOutcome`.
    pub unfilled_start_slots: usize,
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
    observe: StageObserver<'_>,
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
    // Emit-form start positions, filled in once `Starts` has produced them so
    // every later boundary can hand them to the observer.
    let mut waypoints: Vec<(u8, u16, u16)> = Vec::new();

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
            trig: ctx_trig_placeholder,
            map_w,
            map_h,
            rollback_level: DEFAULT_LEVEL,
        };
        let args = WaterArgs {
            map_type: inputs.map_type,
            water_percent: inputs.water_percent,
            num_players: inputs.num_players,
            bridge_enabled: inputs.bridge_enabled,
            playable: PlayableRect {
                x: local_rect[0],
                y: local_rect[1],
                w: gen_w,
                h: gen_h,
            },
        };
        water::run(&mut ctx, &args);
    }
    observe(Stage::Water, grid, &waypoints);
    water_finalize::run(grid, inputs.ids, inputs.blocks, rng);
    observe(Stage::WaterFinalize, grid, &waypoints);

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
    observe(Stage::Regions, grid, &waypoints);

    // IslandPasses (map types 3/4 extra region/bridge passes) are not modelled
    // yet; ordinary map types skip them entirely.
    observe(Stage::IslandPasses, grid, &waypoints);

    // ---- Green spread + first recalc --------------------------------------
    green_spread::run(grid, inputs.ids, rng);
    observe(Stage::GreenSpread, grid, &waypoints);
    lat_fixup::run(grid, inputs.ids);
    observe(Stage::RecalcAfterTerrain, grid, &waypoints);

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
    // Captured before `outcome` moves into the tiberium phase, which is also
    // the phase that turns each unfilled slot into a field seeded at the map
    // origin.
    let unfilled_start_slots = outcome.unfilled_start_slots;
    waypoints = outcome
        .waypoints
        .iter()
        .enumerate()
        .filter_map(|(slot, spot)| spot.map(|(x, y)| (slot as u8, x as u16, y as u16)))
        .collect();
    observe(Stage::Starts, grid, &waypoints);

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
    observe(Stage::TechBuildings, grid, &waypoints);

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
            trig: ctx_trig_placeholder,
            regions: &regions,
            waypoints: &outcome.waypoints,
        };
        tiberium::run(&mut ctx, &args)
    };
    observe(Stage::Tiberium, grid, &waypoints);

    // ---- Region reset + second recalc -------------------------------------
    scratch.reset_region_ids();
    observe(Stage::RegionReset, grid, &waypoints);
    lat_fixup::run(grid, inputs.ids);
    observe(Stage::RecalcAfterTiberium, grid, &waypoints);

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
            trig: ctx_trig_placeholder,
            rng: &mut *rng,
        };
        hills::run(&mut ctx, &args, inputs.cliff, inputs.morphable);
    }
    observe(Stage::Hills, grid, &waypoints);

    // ---- LAT patches → recalc (creates sand-LAT) → trees → rocks ----------
    {
        let mut ctx = PatchCtx {
            grid: &mut *grid,
            scratch: &mut *scratch,
            ids: inputs.ids,
            rng: &mut *rng,
            gauss: &mut *gauss,
            trig: ctx_trig_placeholder,
        };
        lat_patches::run(&mut ctx, inputs.theater, inputs.vegetation);
    }
    observe(Stage::LatPatches, grid, &waypoints);
    lat_fixup::run(grid, inputs.ids);
    observe(Stage::RecalcAfterPatches, grid, &waypoints);

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
            trig: ctx_trig_placeholder,
        };
        trees::run(&mut ctx, &args)
    };
    observe(Stage::Trees, grid, &waypoints);

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
    observe(Stage::Rocks, grid, &waypoints);

    // ---- Final recalc ------------------------------------------------------
    lat_fixup::run(grid, inputs.ids);
    observe(Stage::RecalcFinal, grid, &waypoints);

    // ---- Collect outputs for the emitter ----------------------------------
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
        unfilled_start_slots,
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
        tech_types: &'a [TechType],
    ) -> PipelineInputs<'a> {
        PipelineInputs {
            ids: identity,
            blocks,
            cliff,
            morphable,
            wheel_impassable: [false; LAND_TYPES],
            tech_types,
            map_type,
            theater,
            num_players: 4,
            water_percent: 30,
            bridge_enabled: false,
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
        run_with_types(seed, map_type, theater, &[])
    }

    /// The stock YR neutral catalog, resolved from the real INIs so the gate
    /// tests exercise the same list the app wires in.
    fn stock_tech_types() -> Vec<TechType> {
        crate::map::rmg::tech_catalog::resolve(
            &crate::rules::ini_parser::IniFile::from_str(include_str!("../../../ini/rulesmd.ini")),
            &crate::rules::ini_parser::IniFile::from_str(include_str!("../../../ini/artmd.ini")),
        )
    }

    fn run_with_types(
        seed: u16,
        map_type: i32,
        theater: i32,
        tech_types: &[TechType],
    ) -> (RmgGrid, PipelineOutput) {
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
        let ins = inputs(
            &identity, &blocks, &cliff, &morphable, map_type, theater, tech_types,
        );
        let output = run_pipeline(
            &mut grid,
            &mut scratch,
            &mut rng,
            &mut gauss,
            &geometry,
            &ins,
            &mut |_, _, _| {},
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

    /// Rock gate, on-side. The original runs the rock overlay pass only on the
    /// `theater == 0` branch of "RMG: Creating LATs, rocks etc"; that branch
    /// must actually produce overlays, or the off-side test below proves
    /// nothing. (Gate verified 2026-07-25: `decompile_function 0x00598960`,
    /// `0x005a3ae0` has the overlay loop, `0x005a4280` does not.)
    #[test]
    fn temperate_theater_paints_rocks() {
        let (grid, _) = run(77, 1, 0);
        let rocks = grid
            .native_cells()
            .filter(|&(x, y)| (168..=177).contains(&grid.get(x, y).unwrap().overlay))
            .count();
        assert!(rocks > 0, "temperate places rocks");
    }

    /// Tech gate, on-side. The original calls the placement driver whenever
    /// `map_type != 0` (verified 2026-07-25: `decompile_function 0x00598960`,
    /// the `RMG: Adding tech buildings` block). The off-side is pinned at phase
    /// level by `tech_buildings::tests::map_type_zero_is_a_no_op`, which also
    /// uses a non-empty catalog; map type 0 cannot be driven through the whole
    /// pipeline on this synthetic terrain until the start-starvation assert at
    /// `starts.rs:332` is resolved.
    ///
    /// Seed 9 is the combination the neighbouring map-type test already relies
    /// on: the 1x1 synthetic tile block starves the start selector on many
    /// seeds, which is a fixture limitation, not a generator property.
    #[test]
    fn tech_buildings_are_placed_for_non_zero_map_types() {
        let types = stock_tech_types();
        assert!(!types.is_empty(), "stock catalog resolves");
        let placed: usize = [1, 2]
            .into_iter()
            .map(|map_type| run_with_types(9, map_type, 0, &types).1.structures.len())
            .sum();
        assert!(placed > 0, "map types 1/2 place tech buildings");
    }

    /// The catalog is what stops the phase being a no-op: with an empty list
    /// the same run places nothing, which is the regression this pins.
    #[test]
    fn an_empty_catalog_places_no_tech_buildings() {
        let (_, output) = run_with_types(9, 1, 0, &[]);
        assert!(output.structures.is_empty());
    }

    /// Every placed structure names a type from the resolved catalog.
    #[test]
    fn placed_tech_buildings_come_from_the_resolved_catalog() {
        let types = stock_tech_types();
        let names: Vec<&str> = types.iter().map(|t| t.name.as_str()).collect();
        for map_type in [1, 2] {
            let (_, output) = run_with_types(9, map_type, 0, &types);
            for (name, _, _) in &output.structures {
                assert!(
                    names.contains(&name.as_str()),
                    "{name} is in NeutralTechBuildings"
                );
            }
        }
    }

    /// Start starvation is reachable, and the port now stops short and counts
    /// instead of aborting the run.
    ///
    /// This pins a DELIBERATE DIVERGENCE, not parity. The original cannot fail
    /// here — its two start routines hardcode success — so on starvation it
    /// either walks a null array pointer and faults, or reads uninitialised
    /// heap whose contents follow the process's allocation history rather than
    /// the seed. There is nothing reproducible to copy.
    #[test]
    fn a_starved_map_counts_its_unfilled_start_slots() {
        let (_, output) = run(9, 0, 0);
        assert!(
            output.unfilled_start_slots > 0,
            "the 1x1 synthetic tile block starves the selector on this fixture"
        );
        // Conservation: every quota slot is either written or counted. This is
        // what pins the counter to the whole remaining tail rather than to 1.
        assert_eq!(
            output.waypoints.len() + output.unfilled_start_slots,
            STANDARD_START_QUOTA as usize,
            "written + unfilled must account for every start slot"
        );
    }

    /// A starved region cannot leak a tiberium field onto the map origin, and
    /// the reason is structural rather than lucky.
    ///
    /// The tiberium phase substitutes `(0, 0)` for an unwritten waypoint, so
    /// starvation looks like it should stack fields in the map corner. It
    /// cannot: starvation means the selection was shorter than the quota, so
    /// the drain's `min(quota, selection.len())` consumes the whole selection
    /// and leaves the region's field-slot list empty; the tiberium phase then
    /// takes its `field_count == 0` early-out and skips the region entirely,
    /// before pass 2 — the only place the origin substitution happens — is ever
    /// reached.
    ///
    /// Pinned because it is a coupling between two phases that no single file
    /// states: widening the drain, or moving the early-out, would silently put
    /// tiberium in the corner of every starved map.
    #[test]
    fn a_starved_slot_never_seeds_tiberium_at_the_origin() {
        let (_, output) = run(9, 0, 0);
        assert!(output.unfilled_start_slots > 0, "fixture starves");
        assert!(
            !output.terrain.iter().any(|(_, x, y)| *x == 0 && *y == 0),
            "no terrain object may land on the map origin; terrain was {:?}",
            output.terrain
        );
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
