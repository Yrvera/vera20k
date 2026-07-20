//! Random Map Generator: reproduces the original's `.SED`-driven map generation.
//!
//! Consumes `map::theater` for tile identities and `util::native_x87` for
//! deterministic float math, and emits an in-memory `map::map_file::MapFile`.
//! Pre-play map construction only — nothing in `sim/` depends on this module.

pub mod emit;
pub mod options;
pub mod rng;
pub mod scratch;
pub mod settings;

pub use options::RmgOptions;
pub use rng::RmgRng;
pub use scratch::RmgScratch;
pub use settings::RmgSettings;

use anyhow::Result;

use crate::map::map_file::MapFile;
use crate::map::theater::TheaterData;

/// One stage of the generation pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Water,
    WaterFinalize,
    Regions,
    IslandPasses,
    GreenSpread,
    RecalcAfterTerrain,
    Starts,
    TechBuildings,
    Tiberium,
    RegionReset,
    RecalcAfterTiberium,
    Hills,
    LatPatches,
    RecalcFinal,
    Emit,
}

/// The pipeline order.
///
/// This is the contract every phase attaches to: each stage reads what the
/// previous ones left behind, so reordering changes generated output even when
/// each individual stage is correct. Two details are easy to get wrong and are
/// therefore spelled out here: the green spread runs *after* region assignment
/// but *before* the first attribute recalculation, and hills run *after*
/// tiberium rather than alongside the other terrain shaping.
pub const STAGE_ORDER: &[Stage] = &[
    Stage::Water,
    Stage::WaterFinalize,
    Stage::Regions,
    Stage::IslandPasses,
    Stage::GreenSpread,
    Stage::RecalcAfterTerrain,
    Stage::Starts,
    Stage::TechBuildings,
    Stage::Tiberium,
    Stage::RegionReset,
    Stage::RecalcAfterTiberium,
    Stage::Hills,
    Stage::LatPatches,
    Stage::RecalcFinal,
    Stage::Emit,
];

/// Interior dimensions used until the map-prep stage computes real ones.
const PLACEHOLDER_INTERIOR: u32 = 60;

/// A generated map plus the start slots the launch path needs.
///
/// Not `Clone`: `MapFile` isn't, and a generated map is large enough that
/// copying one should be a deliberate act rather than an accident.
#[derive(Debug)]
pub struct GeneratedMap {
    pub map_file: MapFile,
    /// `(slot, x, y)` per generated start position.
    pub start_waypoints: Vec<(u8, u16, u16)>,
    /// Stages actually executed, in order.
    pub stages_run: Vec<Stage>,
}

/// Run the generator.
///
/// Phase bodies land separately; this walks the pipeline order and records it,
/// so the ordering contract is testable before the phases exist.
pub fn generate(
    options: &RmgOptions,
    settings: &RmgSettings,
    // Optional while no phase consumes theater data yet.
    _theater: Option<&TheaterData>,
) -> Result<GeneratedMap> {
    let mut options = options.clone();
    options.normalize();

    let mut rng = RmgRng::new(options.seed_u16());
    let _ = (&settings, &mut rng);

    let mut stages_run = Vec::with_capacity(STAGE_ORDER.len());
    for stage in STAGE_ORDER {
        // The island passes only exist for the two water-heavy map types.
        if *stage == Stage::IslandPasses && !matches!(options.map_type, 3 | 4) {
            continue;
        }
        stages_run.push(*stage);
    }

    Ok(GeneratedMap {
        map_file: emit::empty_map_file(&options, PLACEHOLDER_INTERIOR, PLACEHOLDER_INTERIOR),
        start_waypoints: Vec::new(),
        stages_run,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn position(stage: Stage) -> usize {
        STAGE_ORDER
            .iter()
            .position(|candidate| *candidate == stage)
            .expect("stage missing from the pipeline order")
    }

    #[test]
    fn green_spread_runs_after_regions_and_before_the_first_recalc() {
        assert!(position(Stage::Regions) < position(Stage::GreenSpread));
        assert!(position(Stage::GreenSpread) < position(Stage::RecalcAfterTerrain));
    }

    #[test]
    fn hills_run_after_tiberium_and_before_lat_patches() {
        assert!(position(Stage::Tiberium) < position(Stage::Hills));
        assert!(position(Stage::Hills) < position(Stage::LatPatches));
    }

    #[test]
    fn starts_precede_everything_that_places_against_them() {
        assert!(position(Stage::Starts) < position(Stage::TechBuildings));
        assert!(position(Stage::Starts) < position(Stage::Tiberium));
    }

    #[test]
    fn island_passes_are_skipped_for_ordinary_map_types() {
        let settings = RmgSettings::default();
        for map_type in [0, 1, 2] {
            let options = RmgOptions {
                map_type,
                ..Default::default()
            };
            let generated = generate(&options, &settings, None).unwrap();
            assert!(
                !generated.stages_run.contains(&Stage::IslandPasses),
                "map type {map_type} must not run the island passes"
            );
        }
        for map_type in [3, 4] {
            let options = RmgOptions {
                map_type,
                ..Default::default()
            };
            let generated = generate(&options, &settings, None).unwrap();
            assert!(
                generated.stages_run.contains(&Stage::IslandPasses),
                "map type {map_type} must run the island passes"
            );
        }
    }

    #[test]
    fn generate_normalizes_before_use() {
        // Out-of-range inputs must be clamped before the RNG conversion.
        let options = RmgOptions {
            seed: 0x9999_9999u32 as i32,
            num_players: 99,
            ..Default::default()
        };
        let generated = generate(&options, &RmgSettings::default(), None).unwrap();
        assert!(!generated.stages_run.is_empty());
    }

    #[test]
    fn theater_option_reaches_the_generated_header() {
        let options = RmgOptions {
            theater: 1,
            ..Default::default()
        };
        let generated = generate(&options, &RmgSettings::default(), None).unwrap();
        assert_eq!(generated.map_file.header.theater, "SNOW");
    }

    #[test]
    fn generation_is_reproducible_from_the_same_options() {
        let options = RmgOptions {
            seed: 4321,
            num_players: 6,
            ..Default::default()
        };
        let settings = RmgSettings::default();
        let first = generate(&options, &settings, None).unwrap();
        let second = generate(&options, &settings, None).unwrap();

        assert_eq!(first.stages_run, second.stages_run);
        assert_eq!(first.start_waypoints, second.start_waypoints);
        assert_eq!(
            first.map_file.header.width,
            second.map_file.header.width
        );
        assert_eq!(
            first.map_file.header.theater,
            second.map_file.header.theater
        );
    }

    #[test]
    fn the_generator_rng_is_independent_of_the_match_rng() {
        let mut first = RmgRng::new(77);
        let mut unrelated = crate::sim::rng::SimRng::new(999);
        for _ in 0..10 {
            let _ = unrelated.next_u32();
        }
        let mut second = RmgRng::new(77);
        assert_eq!(
            first.next_u32(),
            second.next_u32(),
            "generator draws must not depend on any other stream"
        );
    }
}
