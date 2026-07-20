//! Random Map Generator: reproduces the original's `.SED`-driven map generation.
//!
//! Consumes `map::theater` for tile identities and `util::native_x87` for
//! deterministic float math, and emits an in-memory `map::map_file::MapFile`.
//! Pre-play map construction only — nothing in `sim/` depends on this module.

pub mod emit;
pub mod grid;
pub mod options;
pub mod phases;
pub mod pipeline;
pub mod rng;
pub mod scratch;
pub mod settings;
pub mod tiles;
pub mod x87;

pub use grid::{DIRECTION_OFFSETS, DiamondScan, GridCell, RmgGrid};
pub use options::RmgOptions;
pub use rng::RmgRng;
pub use scratch::RmgScratch;
pub use settings::RmgSettings;
pub use tiles::TileIds;
pub use x87::{Gaussian, TruncF64};

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
    /// The LAT auto-tiling fixup the patch driver runs between painting the
    /// base patches and scattering trees. It creates the sand-LAT transition
    /// tiles the rock pass keys off, so it must precede `Trees`/`Rocks`.
    RecalcAfterPatches,
    Trees,
    /// Rock overlays — temperate theater only (skipped at runtime otherwise).
    Rocks,
    RecalcFinal,
    Emit,
}

/// The pipeline order.
///
/// This is the contract every phase attaches to: each stage reads what the
/// previous ones left behind, so reordering changes generated output even when
/// each individual stage is correct. Details that are easy to get wrong and are
/// therefore spelled out here: the green spread runs *after* region assignment
/// but *before* the first attribute recalculation; hills run *after* tiberium
/// rather than alongside the other terrain shaping; and the tail of the LAT
/// driver runs patches → LAT fixup → trees → rocks in that order (the fixup
/// creates the sand-LAT tiles the rock pass needs), then a final recalc.
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
    Stage::RecalcAfterPatches,
    Stage::Trees,
    Stage::Rocks,
    Stage::RecalcFinal,
    Stage::Emit,
];

/// Interior-dimension lerp endpoints, indexed by `num_players - 2`.
const DIM_MIN: [i32; 7] = [70, 70, 70, 80, 90, 100, 100];
const DIM_MAX: [i32; 7] = [80, 80, 80, 90, 100, 110, 120];
/// Single-precision one-third: the width/height option scale factor.
const THIRD_F32_BITS: u32 = 0x3EAA_AAAB;
/// Scale cap for non-island map types. With options clamped 0..3 the scale
/// tops out near 1.0, so this branch is unreachable on the live path — it is
/// kept because the original computes it, and options are checked pre-clamp
/// nowhere else.
const DIMENSION_SCALE_CAP: f64 = 1.2;

/// The map geometry every phase and the emitter share.
///
/// The generated interior is `gen_w x gen_h`; the full map pads that by
/// (4, 12); the diamond bounds and the linear scratch stride follow from the
/// padded size. The `d` that seeds the native cell scan is the padded width.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapGeometry {
    pub gen_w: i32,
    pub gen_h: i32,
    pub map_w: i32,
    pub map_h: i32,
    pub diamond_min: i32,
    pub diamond_max: i32,
    /// Linear grid stride: `map_w + map_h + 1`.
    pub stride: usize,
}

impl MapGeometry {
    pub fn from_options(options: &RmgOptions) -> Self {
        let (gen_w, gen_h) = generated_dimensions(options);
        let map_w = gen_w + 4;
        let map_h = gen_h + 12;
        Self {
            gen_w,
            gen_h,
            map_w,
            map_h,
            diamond_min: map_w,
            diamond_max: map_w + 2 * map_h,
            stride: (map_w + map_h + 1) as usize,
        }
    }
}

/// Interior dimensions: a lerp between per-player-count endpoints, with the
/// width/height option scaled by a single-precision third.
///
/// The arithmetic is deliberately odd and matches the original instruction
/// stream: the scale is computed at 53-bit truncating precision, narrowed to
/// a 4-byte float slot, and the lerp mixes integer-loaded endpoints with that
/// narrowed scale — `min*(1-s) + max*s`, truncated to int at the end. The cap
/// asymmetry (width re-narrowed, height kept wide) is preserved even though
/// clamped options never reach it.
pub fn generated_dimensions(options: &RmgOptions) -> (i32, i32) {
    let players = options.num_players.clamp(2, 8);
    let index = (players - 2) as usize;

    let third = x87::TruncF64::from_f64(f64::from(f32::from_bits(THIRD_F32_BITS)));
    let scale =
        |option: i32| x87::narrow_to_f32(x87::TruncF64::from_f64(f64::from(option)).mul(third));
    let mut scale_w = scale(options.width);
    let mut scale_h = scale(options.height);

    if !matches!(options.map_type, 3 | 4) {
        let cap = x87::TruncF64::from_f64(DIMENSION_SCALE_CAP);
        if !scale_w.lt(cap) {
            // The width cap round-trips through the 4-byte slot; the height
            // cap stays on the FP stack at full width.
            scale_w = x87::narrow_to_f32(cap);
        }
        if !scale_h.lt(cap) {
            scale_h = cap;
        }
    }

    let one = x87::TruncF64::from_f64(f64::from(1.0f32));
    let lerp = |min: i32, max: i32, s: x87::TruncF64| {
        let low = x87::TruncF64::from_f64(f64::from(min)).mul(one.sub(s));
        let high = x87::TruncF64::from_f64(f64::from(max)).mul(s);
        x87::ftol(low.add(high).to_f64())
    };

    (
        lerp(DIM_MIN[index], DIM_MAX[index], scale_w),
        lerp(DIM_MIN[index], DIM_MAX[index], scale_h),
    )
}

/// Whether a selected map name refers to a random-map seed rather than a map
/// file. Such selections are generated in memory instead of loaded from disk.
pub fn is_seed_selection(map_name: &str) -> bool {
    map_name.to_ascii_lowercase().ends_with(".sed")
}

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

    let geometry = MapGeometry::from_options(&options);
    let mut rng = RmgRng::new(options.seed_u16());
    let mut scratch = RmgScratch::new(geometry.stride, geometry.diamond_min, geometry.diamond_max);
    let mut grid =
        grid::RmgGrid::new(geometry.stride, geometry.diamond_min, geometry.diamond_max);
    let _ = (&settings, &mut rng, &mut scratch, &mut grid);

    let mut stages_run = Vec::with_capacity(STAGE_ORDER.len());
    for stage in STAGE_ORDER {
        // The island passes only exist for the two water-heavy map types.
        if *stage == Stage::IslandPasses && !matches!(options.map_type, 3 | 4) {
            continue;
        }
        // Rock overlays are painted only in the temperate theater; every other
        // theater's LAT driver runs trees but no rocks.
        if *stage == Stage::Rocks && options.theater != 0 {
            continue;
        }
        stages_run.push(*stage);
    }

    Ok(GeneratedMap {
        map_file: emit::empty_map_file(&options, geometry.gen_w as u32, geometry.gen_h as u32),
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
    fn seed_selections_are_recognised_regardless_of_case() {
        assert!(is_seed_selection("RandMap.Sed"));
        assert!(is_seed_selection("randmap.sed"));
        assert!(is_seed_selection("RANDMAP.SED"));
        assert!(!is_seed_selection("bigmap.map"));
        assert!(!is_seed_selection("dustbowl.mmx"));
        assert!(!is_seed_selection("sed"), "the extension must be a suffix");
        assert!(!is_seed_selection(""));
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
    fn lat_driver_tail_is_patches_then_fixup_then_trees_then_rocks() {
        // The native temperate driver order: paint patches, run the LAT fixup
        // that produces the sand-LAT tiles, scatter trees, then rocks.
        assert!(position(Stage::LatPatches) < position(Stage::RecalcAfterPatches));
        assert!(position(Stage::RecalcAfterPatches) < position(Stage::Trees));
        assert!(position(Stage::Trees) < position(Stage::Rocks));
        assert!(position(Stage::Rocks) < position(Stage::RecalcFinal));
        assert!(position(Stage::RecalcFinal) < position(Stage::Emit));
    }

    #[test]
    fn rocks_run_only_in_the_temperate_theater() {
        let settings = RmgSettings::default();
        // Temperate (theater 0) paints rocks; every other theater skips them,
        // but still runs trees.
        let temperate = generate(&RmgOptions::default(), &settings, None).unwrap();
        assert!(temperate.stages_run.contains(&Stage::Rocks));
        assert!(temperate.stages_run.contains(&Stage::Trees));
        for theater in [1, 2, 3, 4] {
            let options = RmgOptions {
                theater,
                ..Default::default()
            };
            let generated = generate(&options, &settings, None).unwrap();
            assert!(
                !generated.stages_run.contains(&Stage::Rocks),
                "theater {theater} must not paint rocks"
            );
            assert!(
                generated.stages_run.contains(&Stage::Trees),
                "theater {theater} still scatters trees"
            );
        }
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

    /// Hand-walked from the dimension formula: `s(0)=0`, `s(1)=0.33333334f`,
    /// `s(2)=0.66666669f`, `s(3)` narrows back to exactly 1.0, so option 0
    /// hits the Min endpoint and option 3 the Max endpoint exactly.
    #[test]
    fn dimensions_lerp_between_the_player_count_endpoints() {
        let dims = |players: i32, size: i32| {
            generated_dimensions(&RmgOptions {
                num_players: players,
                width: size,
                height: size,
                ..Default::default()
            })
        };
        assert_eq!(dims(2, 0), (70, 70));
        assert_eq!(dims(2, 1), (73, 73));
        assert_eq!(dims(2, 2), (76, 76));
        assert_eq!(dims(2, 3), (80, 80));
        assert_eq!(dims(8, 0), (100, 100));
        assert_eq!(dims(8, 1), (106, 106));
        assert_eq!(dims(8, 2), (113, 113));
        assert_eq!(dims(8, 3), (120, 120));
        assert_eq!(dims(5, 0), (80, 80), "endpoint table shifts at 5 players");
    }

    #[test]
    fn width_and_height_options_are_independent() {
        let dims = generated_dimensions(&RmgOptions {
            num_players: 4,
            width: 0,
            height: 3,
            ..Default::default()
        });
        assert_eq!(dims, (70, 80));
    }

    #[test]
    fn geometry_derives_from_the_padded_size() {
        let options = RmgOptions {
            num_players: 4,
            width: 0,
            height: 0,
            ..Default::default()
        };
        let geometry = MapGeometry::from_options(&options);
        assert_eq!((geometry.gen_w, geometry.gen_h), (70, 70));
        assert_eq!((geometry.map_w, geometry.map_h), (74, 82));
        assert_eq!(geometry.diamond_min, 74);
        assert_eq!(geometry.diamond_max, 74 + 2 * 82);
        assert_eq!(geometry.stride, 74 + 82 + 1);
    }

    #[test]
    fn generated_header_uses_the_real_dimensions() {
        let options = RmgOptions {
            num_players: 8,
            width: 3,
            height: 3,
            ..Default::default()
        };
        let generated = generate(&options, &RmgSettings::default(), None).unwrap();
        assert_eq!(generated.map_file.header.width, 124, "120 interior + 4");
        assert_eq!(generated.map_file.header.height, 132, "120 interior + 12");
        assert_eq!(generated.map_file.header.local_width, 120);
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
        assert_eq!(first.map_file.header.width, second.map_file.header.width);
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
