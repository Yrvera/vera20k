//! Assembles the pipeline's pre-resolved inputs from a real theater + rules and
//! runs the generator end-to-end into a populated `MapFile`.
//!
//! This is the layer between `run_pipeline` (which takes already-resolved
//! inputs) and the app: it resolves `TileIds`, the cliff ranges, the wheel-
//! impassable land table (from rules), and a per-tile `Morphable=` table, then
//! drives the pipeline and projects the grid through `emit::populate`. Theater
//! resolution is snapshotted into `ResolvedTheaterInputs` so the run itself
//! never borrows `TheaterData` — which keeps `generate_map` unit-testable with a
//! hand-built resolution and a synthetic `TileBlocks`.

use crate::map::theater::{TheaterCliffRanges, TheaterData};
use crate::rules::locomotor_type::SpeedType;
use crate::rules::terrain_rules::TerrainRules;

use super::phases::shore::TileBlocks;
use super::phases::tech_buildings::TechType;
use super::pipeline::{self, LAND_TYPES, PipelineInputs};
use super::rng::RmgRng;
use super::scratch::RmgScratch;
use super::tiles::TileIds;
use super::x87::Gaussian;
use super::{GeneratedMap, MapGeometry, RmgOptions, RmgSettings, STAGE_ORDER, Stage, emit, grid};

/// Land type → rules section, the RA2 `LandType` enum order (verified against
/// the zone classifier's `LAND_WATER == 2` / `LAND_BEACH == 6`). Indices past
/// Cliff never occur on generated terrain, so they resolve to no section.
const LAND_TYPE_SECTIONS: [Option<&str>; LAND_TYPES] = [
    Some("Clear"),    // 0
    Some("Road"),     // 1
    Some("Water"),    // 2
    Some("Rock"),     // 3
    Some("Wall"),     // 4
    Some("Tiberium"), // 5
    Some("Beach"),    // 6
    Some("Rough"),    // 7
    Some("Cliff"),    // 8
    None,             // 9
    None,             // 10
    None,             // 11
    None,             // 12
    None,             // 13
    None,             // 14
    None,             // 15
];

/// Wheel speed at or below this percent classifies the land as wheel-impassable.
const WHEEL_IMPASSABLE_MAX: u8 = 1;

/// Per-land-type "Wheel speed <= 1%" flags for the zone classifier, read from
/// the terrain rules. Stock rules make only Rock wheel-impassable.
pub fn wheel_impassable_from_rules(rules: &TerrainRules) -> [bool; LAND_TYPES] {
    std::array::from_fn(|land| {
        LAND_TYPE_SECTIONS[land]
            .and_then(|name| rules.semantics_by_name(name))
            .and_then(|sem| sem.cost_for_speed_type(SpeedType::Wheel))
            .is_some_and(|wheel| wheel <= WHEEL_IMPASSABLE_MAX)
    })
}

/// The theater/rules facts the generator needs, snapshotted so the run does not
/// borrow `TheaterData`.
#[derive(Debug, Clone)]
pub struct ResolvedTheaterInputs {
    pub ids: TileIds,
    pub cliff: TheaterCliffRanges,
    pub wheel_impassable: [bool; LAND_TYPES],
    /// Per-tile `Morphable=` flag, indexed by flat tile id.
    pub morphable: Vec<bool>,
}

impl ResolvedTheaterInputs {
    /// Snapshot a theater + rules into the resolved input set.
    pub fn from_theater(theater: &TheaterData, rules: &TerrainRules) -> Self {
        let morphable = (0..theater.lookup.len())
            .map(|tile| theater.lookup.is_morphable(tile as u16))
            .collect();
        Self {
            ids: TileIds::resolve(theater),
            cliff: theater.cliff_ranges,
            wheel_impassable: wheel_impassable_from_rules(rules),
            morphable,
        }
    }

    fn is_morphable(&self, tile: i32) -> bool {
        tile >= 0 && (tile as usize) < self.morphable.len() && self.morphable[tile as usize]
    }
}

/// Run the full generator, returning a populated `MapFile` plus the generated
/// start waypoints and the executed stage list.
///
/// `resolved` is a theater/rules snapshot; `blocks` provides tile-block layouts
/// (from `theater_blocks::TheaterTileBlocks`); `tech_types` is the resolved
/// neutral tech-building list (empty places no tech buildings).
pub fn generate_map(
    options: &RmgOptions,
    settings: &RmgSettings,
    resolved: &ResolvedTheaterInputs,
    blocks: &dyn TileBlocks,
    tech_types: &[TechType],
) -> GeneratedMap {
    let mut options = options.clone();
    options.normalize();

    let geometry = MapGeometry::from_options(&options);
    let mut rng = RmgRng::new(options.seed_u16());
    let mut scratch = RmgScratch::new(geometry.stride, geometry.diamond_min, geometry.diamond_max);
    let mut grid = grid::RmgGrid::new(geometry.stride, geometry.diamond_min, geometry.diamond_max);
    let mut gauss = Gaussian::default();

    let morphable = |tile: i32| resolved.is_morphable(tile);
    let inputs = PipelineInputs {
        ids: &resolved.ids,
        blocks,
        cliff: &resolved.cliff,
        morphable: &morphable,
        wheel_impassable: resolved.wheel_impassable,
        tech_types,
        map_type: options.map_type,
        theater: options.theater,
        num_players: options.num_players,
        water_percent: options.water_amount,
        resources: options.resources,
        tib_option: options.tiberium,
        min_tib: settings.min_tiberium,
        max_tib: settings.max_tiberium,
        tiberium_layout: options.tiberium_layout,
        ruggedness: options.ruggedness,
        vegetation: options.vegetation,
        // MapSeed+0x64 tree-count width term — the dialog Width option
        // (0..3); a final binary confirmation is still open.
        width: options.width,
        max_trees: settings.max_trees,
    };

    let output = pipeline::run_pipeline(
        &mut grid,
        &mut scratch,
        &mut rng,
        &mut gauss,
        &geometry,
        &inputs,
    );

    let mut map_file = emit::empty_map_file(&options, geometry.gen_w as u32, geometry.gen_h as u32);
    emit::populate(
        &mut map_file,
        &grid,
        &output.terrain,
        &output.structures,
        &output.waypoints,
    );

    GeneratedMap {
        map_file,
        start_waypoints: output.waypoints,
        stages_run: executed_stages(&options),
    }
}

/// Walk `STAGE_ORDER`, dropping the stages this configuration skips (the island
/// passes off the water-heavy map types; rocks off the non-temperate theaters).
fn executed_stages(options: &RmgOptions) -> Vec<Stage> {
    STAGE_ORDER
        .iter()
        .copied()
        .filter(|stage| {
            if *stage == Stage::IslandPasses && !matches!(options.map_type, 3 | 4) {
                return false;
            }
            if *stage == Stage::Rocks && options.theater != 0 {
                return false;
            }
            true
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::rmg::phases::shore::{SubTile, TileBlock};
    use crate::rules::ini_parser::IniFile;

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

    fn resolved() -> ResolvedTheaterInputs {
        ResolvedTheaterInputs {
            ids: ids(),
            cliff: TheaterCliffRanges::default(),
            wheel_impassable: {
                let mut w = [false; LAND_TYPES];
                w[3] = true; // Rock, as in stock rules
                w
            },
            morphable: Vec::new(),
        }
    }

    #[test]
    fn wheel_table_marks_only_rock_for_stock_rules() {
        // Stock rules give Rock Wheel=0% and everything else a passable value.
        let rules = TerrainRules::from_ini(&IniFile::from_str(
            "[Clear]\nWheel=100%\n\
             [Road]\nWheel=100%\n\
             [Rough]\nWheel=60%\n\
             [Rock]\nWheel=0%\n\
             [Water]\nWheel=0%\n\
             [Cliff]\nWheel=0%\n",
        ));
        let wheel = wheel_impassable_from_rules(&rules);
        assert!(wheel[3], "Rock (land 3) is wheel-impassable");
        assert!(!wheel[0], "Clear passable");
        assert!(!wheel[1], "Road passable");
        assert!(!wheel[7], "Rough passable at 60%");
        // Water and Cliff are also 0% here, but the classifier returns before
        // (water) or never reaches (cliff) their wheel check on generated maps.
        assert!(wheel[2] && wheel[8]);
    }

    #[test]
    fn missing_wheel_key_is_passable() {
        let rules = TerrainRules::from_ini(&IniFile::from_str(""));
        let wheel = wheel_impassable_from_rules(&rules);
        // No section defines Wheel → nothing is wheel-impassable.
        assert!(wheel.iter().all(|&blocked| !blocked));
    }

    fn options(map_type: i32, theater: i32) -> RmgOptions {
        RmgOptions {
            num_players: 4,
            map_type,
            theater,
            water_amount: 30,
            tiberium: 50,
            vegetation: 50,
            ruggedness: 20,
            ..Default::default()
        }
    }

    #[test]
    fn generate_map_produces_a_populated_map() {
        let resolved = resolved();
        let blocks = one_by_one();
        let settings = RmgSettings::default();
        let generated = generate_map(&options(1, 0), &settings, &resolved, &blocks, &[]);
        // Every in-band cell is projected, and the terrain is shaped (not all
        // no-tile). The header carries the generated dimensions.
        assert!(!generated.map_file.cells.is_empty(), "cells were emitted");
        let real = generated
            .map_file
            .cells
            .iter()
            .filter(|c| c.tile_index != -1)
            .count();
        assert!(real > 0, "some cells carry real tiles");
        assert_eq!(generated.map_file.header.theater, "TEMPERATE");
        // Rocks run on temperate.
        assert!(generated.stages_run.contains(&Stage::Rocks));
    }

    #[test]
    fn generate_map_is_deterministic() {
        let resolved = resolved();
        let blocks = one_by_one();
        let settings = RmgSettings::default();
        let snapshot = || {
            let g = generate_map(&options(1, 0), &settings, &resolved, &blocks, &[]);
            let tiles: Vec<i32> = g.map_file.cells.iter().map(|c| c.tile_index).collect();
            let overlays: Vec<(u16, u16, u8)> = g
                .map_file
                .overlays
                .iter()
                .map(|o| (o.rx, o.ry, o.overlay_id))
                .collect();
            (tiles, overlays, g.start_waypoints)
        };
        assert_eq!(snapshot(), snapshot());
    }

    #[test]
    fn non_temperate_theater_skips_rocks_stage() {
        let resolved = resolved();
        let blocks = one_by_one();
        let settings = RmgSettings::default();
        let generated = generate_map(&options(1, 1), &settings, &resolved, &blocks, &[]);
        assert_eq!(generated.map_file.header.theater, "SNOW");
        assert!(!generated.stages_run.contains(&Stage::Rocks));
        assert!(generated.stages_run.contains(&Stage::Trees));
    }
}

#[cfg(test)]
mod send_check {
    /// The worker that generation is about to move onto needs to own its
    /// inputs, so every one of them has to cross a thread boundary.
    #[test]
    fn generator_inputs_and_output_are_send() {
        const fn assert_send<T: Send>() {}
        assert_send::<crate::map::rmg::RmgOptions>();
        assert_send::<crate::map::rmg::RmgSettings>();
        assert_send::<super::ResolvedTheaterInputs>();
        assert_send::<crate::map::rmg::theater_blocks::TheaterTileBlocks>();
        assert_send::<crate::map::rmg::GeneratedMap>();
        assert_send::<crate::map::rmg::preview::PreviewPalette>();
    }
}
