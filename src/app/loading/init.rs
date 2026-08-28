//! App initialization helpers — map loading, entity spawning, asset loading.
//!
//! Extracted from app.rs to keep the main orchestrator under the 400-line limit.
//! These functions run once at startup (not per-frame).
//!
//! ## Dependency rules
//! - Part of the app layer — may depend on everything.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::app::frontend::list_maps::{
    LoadedMap, LoadedMapSource, load_map_by_name_or_path_with_assets, try_load_mmx,
};
use crate::app::frontend::skirmish::{
    build_overlay_atlas_from_map, house_color_map_for_launch_session,
    seed_skirmish_opening_if_needed,
};
use crate::app::loading::init_helpers::{
    build_entity_atlases, build_sidebar_cameo_atlas, build_tile_atlas, load_art_ini,
    load_rules_with_merged_ini, log_trigger_graph_diagnostics, parse_debug_spawn_units_env,
    scheduler_anim_roots, theater_ext_for,
};
use crate::match_bootstrap::LoadingStartup;
use crate::sim::scenario_bootstrap::{
    PreloadedBattleStartPlan, ScenarioBootstrapRng,
    apply_explicit_skirmish_launch_session_with_overlay_registry,
    apply_preloaded_battle_launch_session_with_overlay_registry, initialize_map_roster_houses,
    initialize_skirmish_launch_houses,
};

use crate::assets::asset_manager::AssetManager;
use crate::assets::shp_file::ShpFile;
use crate::map::actions::ActionMap;
use crate::map::basic::{BasicSection, BridgeDestroyabilityMode};
use crate::map::cell_tags::CellTagMap;
use crate::map::events::EventMap;
use crate::map::houses::{self, HouseColorMap, HouseRoster};
use crate::map::lighting::{self, CellLightGrid, LightingConfig, LightingProfileUnits, PointLight};
use crate::map::map_file::MapFile;
use crate::map::overlay::{OverlayEntry, TerrainObject};
use crate::map::overlay_types::OverlayTypeRegistry;
use crate::map::resolved_terrain::ResolvedTerrainGrid;
use crate::map::tags::TagMap;
use crate::map::terrain::{self, LocalBounds, TerrainGrid};
use crate::map::theater;
use crate::map::trigger_graph::TriggerGraph;
use crate::map::triggers::TriggerMap;
use crate::map::waypoints::{self, Waypoint};
use crate::render::batch::BatchRenderer;
use crate::render::bridge_atlas::BridgeAtlas;
use crate::render::bridge_railing_atlas::{BridgeRailingAtlas, BridgeRailingTileBases};
use crate::render::cursor_atlas;
use crate::render::gpu::GpuContext;
use crate::render::overlay_atlas::OverlayAtlas;
use crate::render::sidebar_cameo_atlas::SidebarCameoAtlas;
use crate::render::sidebar_chrome::SidebarChromeSet;
use crate::render::sprite_atlas::SpriteAtlas;
use crate::render::tile_atlas::TileAtlas;
use crate::render::unit_atlas::UnitAtlas;
use crate::rules::art_data::ArtRegistry;
use crate::rules::ini_parser::IniFile;
use crate::rules::ruleset::RuleSet;
use crate::sim::trigger_runtime::TriggerRuntime;
use crate::sim::world::Simulation;

fn resolved_overlay_shp_ids(
    registry: &OverlayTypeRegistry,
    rules_ini: &IniFile,
    art: &ArtRegistry,
    asset_manager: &AssetManager,
    theater_ext: &str,
    theater_name: &str,
) -> BTreeSet<u8> {
    let mut available = BTreeSet::new();
    for raw_id in 0..registry.len().min(usize::from(u8::MAX) + 1) {
        let overlay_id = raw_id as u8;
        let Some(name) = registry.name(overlay_id) else {
            continue;
        };
        let image_id = art.resolve_overlay_image_id(name, rules_ini);
        let native_names = crate::rules::art_data::native_overlay_shp_names(
            art,
            &image_id,
            theater_ext,
            theater_name,
        );
        if native_names
            .iter()
            .filter_map(|candidate| asset_manager.load_file_from_mix(candidate))
            .any(|loaded| ShpFile::from_bytes(&loaded.bytes).is_ok())
        {
            available.insert(overlay_id);
        }
    }
    available
}

#[cfg(test)]
mod native_overlay_shp_tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(1);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn with_asset(label: &str, filename: &str) -> Self {
            let sequence = TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "vera20k-overlay-get-shp-{label}-{}-{sequence}",
                std::process::id()
            ));
            std::fs::create_dir(&path).expect("create overlay SHP test directory");
            std::fs::write(path.join(filename), valid_test_shp())
                .expect("write overlay SHP test asset");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn valid_test_shp() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&2u16.to_le_bytes());
        data.extend_from_slice(&2u16.to_le_bytes());
        data.extend_from_slice(&1u16.to_le_bytes());

        let data_offset = 32u32;
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&2u16.to_le_bytes());
        data.extend_from_slice(&2u16.to_le_bytes());
        data.push(0);
        data.extend_from_slice(&[0u8; 11]);
        data.extend_from_slice(&data_offset.to_le_bytes());
        data.extend_from_slice(&[1, 2, 3, 0]);
        data
    }

    fn theater_overlay_fixture() -> (OverlayTypeRegistry, IniFile, ArtRegistry) {
        let rules_ini = IniFile::from_str("[OverlayTypes]\n0=NAME\n");
        let art_ini = IniFile::from_str("[NAME]\nTheater=yes\n");
        let registry = OverlayTypeRegistry::from_ini(&rules_ini, Some(&art_ini));
        let art = ArtRegistry::from_ini(&art_ini);
        (registry, rules_ini, art)
    }

    #[test]
    fn gsi_04_07_placement_map_pack_get_shp_uses_native_primary_and_generic_retry() {
        let (registry, rules_ini, art) = theater_overlay_fixture();
        assert_eq!(
            crate::rules::art_data::native_overlay_shp_names(&art, "NAME", "TEM", "TEMPERATE"),
            ["NAME.TEM".to_string(), "NGME.TEM".to_string()]
        );

        for (label, filename, expected_available) in [
            ("native-extension", "NAME.TEM", true),
            ("ineligible-shp-alternative", "NAME.SHP", false),
            ("generic-letter-retry", "NGME.TEM", true),
        ] {
            let directory = TestDirectory::with_asset(label, filename);
            let asset_manager = AssetManager::from_loose_root_for_test(directory.path());
            let available = resolved_overlay_shp_ids(
                &registry,
                &rules_ini,
                &art,
                &asset_manager,
                "TEM",
                "TEMPERATE",
            );

            assert_eq!(
                available.contains(&0),
                expected_available,
                "only {filename} is present"
            );
        }
    }
}

#[cfg(test)]
mod map_wall_owner_candidate_tests {
    use super::*;
    use crate::map::entities::EntityCategory;
    use crate::map::resolved_terrain::{ResolvedTerrainCell, zone_class};
    use crate::rules::terrain_rules::{LandType, SpeedCostProfile, TerrainClass};
    use crate::sim::components::{BuildingUp, Health};
    use crate::sim::game_entity::GameEntity;
    use crate::sim::overlay_grid::{MapWallOwnerCandidate, OverlayGrid};
    use crate::sim::power_system::PowerState;
    use crate::sim::radiation::RadDetonation;
    use crate::sim::runtime::map_wall_owner_candidate_from_building;

    fn flat_cell(rx: u16, ry: u16) -> ResolvedTerrainCell {
        let land = LandType::Clear.as_index();
        let speed_costs = SpeedCostProfile::default();
        ResolvedTerrainCell {
            rx,
            ry,
            source_tile_index: 0,
            source_sub_tile: 0,
            final_tile_index: 0,
            final_sub_tile: 0,
            is_wood_bridge_repair_tile: false,
            level: 0,
            filled_clear: true,
            tileset_index: None,
            land_type: land,
            yr_cell_land_type: land,
            slope_type: 0,
            template_height: 0,
            height_in_pixels: 0,
            render_offset_x: 0,
            render_offset_y: 0,
            terrain_class: TerrainClass::Clear,
            speed_costs,
            is_water: false,
            is_cliff_like: false,
            is_rough: false,
            is_road: false,
            accepts_smudge: true,
            allows_tiberium: false,
            variant: 0,
            has_ramp: false,
            canonical_ramp: None,
            ground_walk_blocked: false,
            terrain_object_blocks: false,
            terrain_object_occupation: None,
            overlay_blocks: false,
            overlay_zone_type: None,
            outside_playfield: false,
            zone_type: zone_class::GROUND,
            base_ground_walk_blocked: false,
            base_build_blocked: false,
            base_land_type: land,
            base_yr_cell_land_type: land,
            base_terrain_class: TerrainClass::Clear,
            base_speed_costs: speed_costs,
            build_blocked: false,
            has_bridge_deck: false,
            bridge_walkable: false,
            bridge_transition: false,
            bridge_deck_level: 0,
            bridge_layer: None,
            bridge_facts: crate::map::bridge_facts::BridgeCellFacts::default(),
            tube_index: None,
            radar_left: [0; 3],
            radar_right: [0; 3],
            has_damaged_data: false,
            bridgehead_anchor_class_at_load: None,
        }
    }

    fn flat_terrain(width: u16, height: u16) -> ResolvedTerrainGrid {
        let cells = (0..height)
            .flat_map(|ry| (0..width).map(move |rx| flat_cell(rx, ry)))
            .collect();
        ResolvedTerrainGrid::from_cells(width, height, cells)
    }

    fn building(
        stable_id: u64,
        type_id: &str,
        owner: &str,
        rx: u16,
        ry: u16,
        foundation: &str,
    ) -> GameEntity {
        let mut entity = GameEntity::test_default(stable_id, type_id, owner, rx, ry);
        entity.category = EntityCategory::Structure;
        entity.foundation = foundation.to_string();
        entity.lifecycle.object_alive = true;
        entity.lifecycle.cell_marked = true;
        entity
    }

    #[test]
    fn gsi_04_07_placement_production_wall_owner_uses_building_get_coords_center() {
        let terrain = flat_terrain(15, 10);
        let gacnst = building(1, "GACNST", "GDI", 8, 8, "4x4");
        let gaspot = building(2, "GASPOT", "Neutral", 14, 9, "1x1");
        let gacnst_candidate = map_wall_owner_candidate_from_building(&gacnst, &terrain, true);
        let gaspot_candidate = map_wall_owner_candidate_from_building(&gaspot, &terrain, true);

        assert_eq!(
            (gacnst_candidate.world_x, gacnst_candidate.world_y),
            (8 * 256 + 128 + 3 * 128, 8 * 256 + 128 + 3 * 128)
        );
        assert_eq!(
            (gaspot_candidate.world_x, gaspot_candidate.world_y),
            (14 * 256 + 128, 9 * 256 + 128),
            "a 1x1 foundation has zero GetCoords projection"
        );
        assert_eq!(
            (
                gacnst_candidate.foundation_width,
                gacnst_candidate.foundation_height
            ),
            (4, 4),
            "candidate uses the dimensions stamped on the entity"
        );

        let registry = OverlayTypeRegistry::from_ini(
            &IniFile::from_str("[OverlayTypes]\n0=GAWALL\n[GAWALL]\nWall=yes\n"),
            None,
        );
        let mut projected_grid = OverlayGrid::new(15, 10);
        projected_grid.place_overlay(12, 9, 0, 0);
        projected_grid.reconstruct_map_wall_owners(
            &terrain,
            &registry,
            &[gacnst_candidate, gaspot_candidate],
        );
        assert_eq!(projected_grid.cell(12, 9).wall_owner, Some(gacnst.owner));

        let mut northwest_grid = OverlayGrid::new(15, 10);
        northwest_grid.place_overlay(12, 9, 0, 0);
        northwest_grid.reconstruct_map_wall_owners(
            &terrain,
            &registry,
            &[
                MapWallOwnerCandidate {
                    world_x: 8 * 256 + 128,
                    world_y: 8 * 256 + 128,
                    ..gacnst_candidate
                },
                MapWallOwnerCandidate {
                    world_x: 14 * 256 + 128,
                    world_y: 9 * 256 + 128,
                    ..gaspot_candidate
                },
            ],
        );
        assert_eq!(
            northwest_grid.cell(12, 9).wall_owner,
            Some(gaspot.owner),
            "the former north-west candidate coordinates select the wrong owner"
        );
    }

    #[test]
    fn gsi_04_10_all_terrain_clears_same_cell_tiberium_before_entity_admission() {
        let ini = IniFile::from_str(
            "[General]\nTreeStrength=200\n\
             [TerrainTypes]\n0=TREE01\n1=TIBTRE01\n\
             [TREE01]\nSpawnsTiberium=no\n\
             [TIBTRE01]\nSpawnsTiberium=yes\n\
             [OverlayTypes]\n0=ORE\n1=ROCK\n\
             [ORE]\nTiberium=yes\n\
             [ROCK]\nIsARock=yes\n",
        );
        let rules = RuleSet::from_ini(&ini).expect("rules");
        let registry = OverlayTypeRegistry::from_ini(&ini, None);
        assert!(
            !rules
                .terrain_object_type_case_insensitive("TREE01")
                .expect("ordinary tree type")
                .spawns_tiberium
        );
        let mut sim = Simulation::with_seed(0x0410);
        let rng_before = sim.scenario_rng.state();

        let mut terrain = flat_terrain(6, 3);
        let mut overlays = OverlayGrid::new(6, 3);
        overlays.place_overlay(1, 1, 0, 7);
        overlays.place_overlay(2, 1, 0, 8);
        overlays.place_overlay(3, 1, 1, 9);
        overlays.place_overlay(4, 1, 0, 10);
        for rx in 1..=4 {
            crate::sim::overlay_grid::recalc_overlay_passability(
                &mut overlays,
                &mut terrain,
                &registry,
                rx,
                1,
            );
        }
        assert_eq!(
            terrain.cell(1, 1).unwrap().terrain_class,
            TerrainClass::Tiberium
        );
        assert_eq!(
            terrain.cell(2, 1).unwrap().terrain_class,
            TerrainClass::Tiberium
        );
        assert_eq!(
            terrain.cell(4, 1).unwrap().terrain_class,
            TerrainClass::Tiberium,
            "the unknown Terrain fixture starts with the same projected ore state"
        );
        overlays.take_dirty_cells();
        let terrain_objects = [
            TerrainObject {
                rx: 1,
                ry: 1,
                name: "TREE01".to_string(),
            },
            TerrainObject {
                rx: 2,
                ry: 1,
                name: "TIBTRE01".to_string(),
            },
            TerrainObject {
                rx: 3,
                ry: 1,
                name: "TREE01".to_string(),
            },
            TerrainObject {
                rx: 4,
                ry: 1,
                name: "UNKNOWN".to_string(),
            },
        ];

        let cleared = crate::sim::terrain_spawn::clear_tiberium_source_cells_for_terrain(
            &mut overlays,
            &mut terrain,
            &terrain_objects,
            &rules,
            &registry,
        );

        assert_eq!(cleared, BTreeSet::from([(1, 1), (2, 1)]));
        assert_eq!(overlays.cell(1, 1).overlay_id, None);
        assert_eq!(overlays.cell(1, 1).overlay_data, 0);
        assert_eq!(overlays.cell(2, 1).overlay_id, None);
        assert_eq!(overlays.cell(2, 1).overlay_data, 0);
        assert_eq!(overlays.cell(3, 1).overlay_id, Some(1));
        assert_eq!(overlays.cell(3, 1).overlay_data, 9);
        assert_eq!(overlays.cell(4, 1).overlay_id, Some(0));
        assert_eq!(overlays.cell(4, 1).overlay_data, 10);
        assert_eq!(
            terrain.cell(1, 1).unwrap().terrain_class,
            TerrainClass::Clear
        );
        assert_eq!(
            terrain.cell(2, 1).unwrap().terrain_class,
            TerrainClass::Clear
        );
        assert_eq!(
            terrain.cell(4, 1).unwrap().terrain_class,
            TerrainClass::Tiberium,
            "an unknown TerrainType leaves the resolved ore projection intact"
        );
        assert!(
            overlays.take_dirty_cells().is_empty(),
            "map-load TerrainClass::Unlimbo clearing must not emit runtime dirtiness"
        );

        let constructed = crate::sim::terrain_spawn::construct_terrain_objects(
            &mut sim,
            &terrain_objects,
            &rules,
            false,
        );
        assert_eq!(constructed, 3);
        assert_eq!(
            sim.production.terrain_object_cells,
            BTreeMap::from([((1, 1), 1), ((2, 1), 2), ((3, 1), 3)]),
            "recognized Terrain entries construct in source order while UNKNOWN is skipped"
        );
        assert_eq!(sim.scenario_rng.state(), rng_before);
    }

    fn lighting_rules() -> RuleSet {
        let mut rules = RuleSet::from_ini(&IniFile::from_str(
            "[BuildingTypes]\n0=LAMP\n\
             [LAMP]\nStrength=100\nPowered=yes\nPower=-100\n\
             LightVisibility=2048\nLightIntensity=.2\n\
             LightRedTint=.1\nLightGreenTint=.2\nLightBlueTint=.3\n",
        ))
        .expect("lighting rules");
        rules.radiation = crate::rules::ruleset::RadiationRules {
            duration_multiple: 1,
            application_delay: 16,
            level_max: 500,
            level_delay: 90,
            light_delay: 90,
            level_factor: 0.2,
            light_factor: crate::util::fixed_math::sim_from_f32(0.1),
            tint_factor: crate::util::fixed_math::sim_from_f32(1.0),
            color: (0, 255, 0),
            site_warhead: "RadSite".to_string(),
        };
        rules
    }

    fn seed_live_lamp(sim: &mut Simulation) -> crate::sim::intern::InternedId {
        let owner = sim.interner.intern("House");
        let type_ref = sim.interner.intern("LAMP");
        let mut lamp = GameEntity::new_at_frame_zero_for_test(
            41,
            4,
            5,
            0,
            0,
            owner,
            Health {
                current: 100,
                max: 100,
            },
            type_ref,
            EntityCategory::Structure,
            0,
            0,
            false,
        );
        lamp.lifecycle.object_alive = true;
        lamp.lifecycle.in_limbo = false;
        lamp.lifecycle.cell_marked = true;
        lamp.building_up = Some(BuildingUp {
            elapsed_ticks: 1,
            total_ticks: 10,
        });
        sim.entities_mut().insert(lamp);
        owner
    }

    #[test]
    fn gsi_04_20_building_lamp_fingerprint_tracks_lifecycle_power_and_detail() {
        let rules = lighting_rules();
        let mut sim = Simulation::with_seed(0x420);
        let owner = seed_live_lamp(&mut sim);
        let config = LightingConfig::default();

        let lit = derive_lighting_view(&config, Some(&sim), Some(&rules), 2);
        assert_eq!(
            lit.point_lights.len(),
            1,
            "BuildingUp does not suppress LightSource"
        );
        let low_detail = derive_lighting_view(&config, Some(&sim), Some(&rules), 1);
        assert!(low_detail.point_lights.is_empty());
        assert_ne!(lit.fingerprint, low_detail.fingerprint);

        sim.power_states.insert(
            owner,
            PowerState {
                is_low_power: true,
                ..PowerState::default()
            },
        );
        let offline = derive_lighting_view(&config, Some(&sim), Some(&rules), 2);
        assert!(offline.point_lights.is_empty());
        assert_ne!(lit.fingerprint, offline.fingerprint);

        sim.power_states
            .get_mut(&owner)
            .expect("power state")
            .is_low_power = false;
        let restored = derive_lighting_view(&config, Some(&sim), Some(&rules), 2);
        assert_eq!(restored.point_lights.len(), 1);
        assert_eq!(lit.fingerprint, restored.fingerprint);

        sim.entities_mut()
            .get_mut(41)
            .expect("lamp")
            .lifecycle
            .in_limbo = true;
        let limbo = derive_lighting_view(&config, Some(&sim), Some(&rules), 2);
        assert!(limbo.point_lights.is_empty());
        sim.entities_mut()
            .get_mut(41)
            .expect("lamp")
            .lifecycle
            .in_limbo = false;
        sim.entities_mut().get_mut(41).expect("lamp").dying = true;
        let dying = derive_lighting_view(&config, Some(&sim), Some(&rules), 2);
        assert!(dying.point_lights.is_empty());
    }

    #[test]
    fn gsi_04_20_building_lamp_tracks_capture_power_and_sale_lifecycle() {
        let rules = lighting_rules();
        let mut sim = Simulation::with_seed(0x4201);
        let _original_owner = seed_live_lamp(&mut sim);
        let captured_owner = sim.interner.intern("Captured");
        let config = LightingConfig::default();

        let before_capture = derive_lighting_view(&config, Some(&sim), Some(&rules), 2);
        assert_eq!(before_capture.point_lights.len(), 1);

        sim.power_states.insert(
            captured_owner,
            PowerState {
                is_low_power: true,
                ..PowerState::default()
            },
        );
        sim.change_owner(41, captured_owner);
        let captured_offline = derive_lighting_view(&config, Some(&sim), Some(&rules), 2);
        assert!(captured_offline.point_lights.is_empty());
        assert_ne!(before_capture.fingerprint, captured_offline.fingerprint);

        sim.power_states
            .get_mut(&captured_owner)
            .expect("captured owner power state")
            .is_low_power = false;
        let captured_online = derive_lighting_view(&config, Some(&sim), Some(&rules), 2);
        assert_eq!(captured_online.point_lights.len(), 1);
        assert_eq!(before_capture.fingerprint, captured_online.fingerprint);

        assert!(crate::sim::production::sell_building(&mut sim, &rules, 41));
        let sold = derive_lighting_view(&config, Some(&sim), Some(&rules), 2);
        assert!(sold.point_lights.is_empty());
        assert_ne!(captured_online.fingerprint, sold.fingerprint);
    }

    #[test]
    fn gsi_04_20_composed_scenario_lamp_and_radiation_reach_world_tint_consumer() {
        let rules = lighting_rules();
        let mut sim = Simulation::with_seed(0x4202);
        seed_live_lamp(&mut sim);
        sim.session.lighting.current_ambient = 80;
        sim.session.lighting.target_ambient = sim.session.lighting.ion.ambient_percent;
        sim.session.lighting.selected_profile =
            crate::sim::scenario_session::ScenarioLightingProfile::Ion;
        sim.radiation.apply_detonation(
            RadDetonation {
                rx: 4,
                ry: 5,
                rad_level: 500,
                spread: 10,
            },
            0,
            &rules.radiation,
            None,
        );

        let terrain = flat_terrain(10, 10);
        let view = derive_lighting_view(&LightingConfig::default(), Some(&sim), Some(&rules), 2);
        assert_eq!(view.profile.ambient_percent, 80);
        assert_eq!(
            view.point_lights.len(),
            2,
            "lamp and RadSite are both present"
        );

        let base = lighting::build_cell_light_grid_from_heights_and_units_with_detail(
            terrain.iter().map(|cell| ((cell.rx, cell.ry), cell.level)),
            view.profile,
            view.detail_level,
        );
        let composed = build_lighting_grid_from_view(&terrain, &view);
        let center = composed.cell_light_at((4, 5)).expect("composed cell light");
        assert_eq!(
            center.raw_additive_intensity,
            view.point_lights
                .iter()
                .map(|light| light.intensity)
                .sum::<i32>(),
            "both centered sources accumulate over the selected scenario profile"
        );
        assert_ne!(
            composed.unit_tint_at((4, 5), 0),
            base.unit_tint_at((4, 5), 0),
            "the world-instance unit tint consumer observes the composed grid"
        );
    }

    #[test]
    fn gsi_04_20_same_cell_radiation_merge_changes_complete_light_fingerprint() {
        let rules = lighting_rules();
        let mut sim = Simulation::with_seed(0x421);
        let detonation = RadDetonation {
            rx: 10,
            ry: 11,
            rad_level: 500,
            spread: 10,
        };
        sim.radiation
            .apply_detonation(detonation, 0, &rules.radiation, None);
        let old_epoch = crate::app::presentation::radiation_light::radiation_light_epoch(
            &sim.radiation,
            &rules.radiation,
        );
        let first = derive_lighting_view(&LightingConfig::default(), Some(&sim), Some(&rules), 2);
        assert_eq!(first.point_lights.len(), 1);

        sim.radiation
            .apply_detonation(detonation, 0, &rules.radiation, None);
        assert_eq!(
            crate::app::presentation::radiation_light::radiation_light_epoch(
                &sim.radiation,
                &rules.radiation,
            ),
            old_epoch,
            "the former center-plus-step epoch cannot see a same-cell rearm"
        );
        let merged = derive_lighting_view(&LightingConfig::default(), Some(&sim), Some(&rules), 2);
        assert_eq!(merged.point_lights.len(), 1);
        assert_ne!(first.fingerprint, merged.fingerprint);
        assert_ne!(
            first.point_lights[0].intensity,
            merged.point_lights[0].intensity
        );
    }
}

/// Scenario/runtime inputs produced by loading a map (F11): everything the
/// sim runtime, trigger machinery, and match view facts consume.
pub struct ScenarioLoadInputs {
    pub(crate) startup: LoadingStartup,
    pub(crate) map_source: LoadedMapSource,
    /// Digest of the parsed source map INI used for strict save compatibility.
    pub(crate) map_hash: Option<u64>,
    pub basic: BasicSection,
    pub terrain_grid: Option<TerrainGrid>,
    pub resolved_terrain: Option<ResolvedTerrainGrid>,
    pub simulation: Option<Simulation>,
    /// Overlay entries for per-frame instance generation.
    pub overlays: Vec<OverlayEntry>,
    /// Terrain objects for per-frame instance generation.
    pub terrain_objects: Vec<TerrainObject>,
    pub waypoints: HashMap<u32, Waypoint>,
    pub cell_tags: CellTagMap,
    pub tags: TagMap,
    pub triggers: TriggerMap,
    pub events: EventMap,
    pub actions: ActionMap,
    pub trigger_graph: TriggerGraph,
    pub trigger_runtime: TriggerRuntime,
    /// Overlay type registry — kept so wall placement can look up overlay_id by name.
    pub overlay_registry: OverlayTypeRegistry,
    pub house_roster: HouseRoster,
    /// Cell (rx, ry) → terrain elevation z for overlay/entity height lookup.
    pub height_map: BTreeMap<(u16, u16), u8>,
    /// Cell (rx, ry) → bridge deck elevation z. Only bridge cells present.
    pub bridge_height_map: BTreeMap<(u16, u16), u8>,
    pub tactical_bridge_inverse_map: BTreeMap<(u16, u16), crate::map::terrain::TacticalBridgeCell>,
    /// Parsed rules.ini data — kept for combat system weapon/warhead lookups.
    pub rules: Option<RuleSet>,
    /// Parsed [Lighting] config used for transient lighting rebuilds.
    pub map_lighting_config: LightingConfig,
    /// Current map theater name (e.g., "DESERT", "TEMPERATE").
    pub theater_name: String,
    /// Current theater extension (e.g., "des", "tem").
    pub theater_ext: String,
    /// Preferred initial local owner when the loader seeded a sandbox opening.
    pub initial_local_owner: Option<String>,
    /// Keep full map visibility for the empty-map sandbox opening.
    pub sandbox_full_visibility: bool,
    /// True when MCV seeding was deferred for spawn-pick phase.
    /// The map has 2+ multiplayer start waypoints and the player should pick one.
    pub spawn_pick_pending: bool,
    /// World point the camera should be centred on, in the frame
    /// `terrain::iso_to_screen` produces. Converted to a camera top-left by the
    /// transition, which knows the scaled sidebar width and the live zoom.
    pub camera_anchor_x: f32,
    pub camera_anchor_y: f32,
}

/// Presentation assets produced by loading a map (F11): atlases, chrome,
/// fonts, and derived display tables. Built FROM the constructed scenario;
/// nothing here feeds back into it.
pub struct PresentationLoadAssets {
    pub tile_atlas: Option<TileAtlas>,
    pub unit_atlas: Option<UnitAtlas>,
    /// Palette + per-house RGB ramp GPU resources for the voxel sprite shader.
    /// None when no theater palette is available (rare).
    pub palette_set: Option<crate::render::palette_textures::PaletteSet>,
    pub sprite_atlas: Option<SpriteAtlas>,
    pub overlay_atlas: Option<OverlayAtlas>,
    pub bridge_atlas: Option<BridgeAtlas>,
    pub bridge_railing_atlas: Option<BridgeRailingAtlas>,
    pub sidebar_cameo_atlas: Option<SidebarCameoAtlas>,
    pub sidebar_chrome: Option<SidebarChromeSet>,
    pub(crate) software_cursor: Option<crate::app::presentation::render::SoftwareCursor>,
    /// Overlay ID → type name mapping (from rules.ini [OverlayTypes]).
    pub overlay_names: BTreeMap<u8, String>,
    /// Exact SHP frame-header radar RGB for each native-selected overlay frame.
    pub overlay_radar_colors: HashMap<(u8, u8), [u8; 3]>,
    /// Owner name → house color index mapping (from map [Houses] sections).
    pub house_color_map: HouseColorMap,
    /// Per-cell RGB tint from map [Lighting] section.
    pub lighting_grid: CellLightGrid,
    /// CSF string table — localized display names loaded from language MIX.
    pub csf: Option<crate::assets::csf_file::CsfFile>,
    /// Parsed GAME.FNT bitmap font for authentic sidebar text rendering.
    pub fnt_file: Option<crate::assets::fnt_file::FntFile>,
}

/// All data produced by loading a map (F11): concrete scenario/runtime inputs
/// and presentation assets, no longer one cross-domain bag, plus the leased
/// process asset manager on its way home.
pub struct MapLoadResult {
    pub(crate) scenario: ScenarioLoadInputs,
    pub(crate) presentation: PresentationLoadAssets,
    /// The leased process manager returning to `ProcessAssets` (F11 slot).
    pub asset_manager: Option<AssetManager>,
}

pub(crate) struct MapLoadInitial {
    map_data: MapFile,
    map_source: LoadedMapSource,
    /// Move-only generated-map authority; fixed maps never synthesize one.
    mapgen_rng_continuation: Option<crate::rng_continuation::MapGenRngContinuation>,
}

impl MapLoadInitial {
    pub(crate) fn theater_name(&self) -> &str {
        &self.map_data.header.theater
    }

    pub(crate) fn map_data(&self) -> &MapFile {
        &self.map_data
    }

    #[cfg(test)]
    pub(crate) fn has_mapgen_rng_continuation(&self) -> bool {
        self.mapgen_rng_continuation.is_some()
    }
}

/// Continue loading from the exact map produced by the accepted setup run.
///
/// gamemd provenance: FUN_00595BC0's accepted generated scenario reaches
/// Scenario initialization 0x00684620 and Full_Init 0x00686B20; loading does
/// not invoke a second generator solely to recover that scenario.
pub(crate) fn retained_random_map_initial(
    seed_name: String,
    generated: crate::map::rmg::GeneratedMap,
    progress: &mut dyn crate::app::loading::pump::LoadingProgressSink,
) -> MapLoadInitial {
    progress.milestone(8);
    let mapgen_rng_continuation = generated.mapgen_continuation;
    MapLoadInitial {
        map_data: generated.map_file,
        map_source: LoadedMapSource::Generated { seed_name },
        mapgen_rng_continuation: Some(mapgen_rng_continuation),
    }
}

pub(crate) fn load_csf(
    asset_manager: &AssetManager,
) -> anyhow::Result<crate::assets::csf_file::CsfFile> {
    let bytes = asset_manager
        .get_ref("ra2md.csf")
        .ok_or_else(|| anyhow::anyhow!("required retail string table ra2md.csf is missing"))?;
    let csf = crate::assets::csf_file::CsfFile::from_bytes(bytes)
        .map_err(|err| anyhow::anyhow!("could not parse required ra2md.csf: {err:#}"))?;
    log::info!("Loaded CSF string table: ra2md.csf");
    Ok(csf)
}

/// Fully-derived render-facing lighting view. The simulation owns only the
/// scenario controller and source inputs; the per-cell grid remains app state.
pub(crate) struct DerivedLightingView {
    pub(crate) profile: LightingProfileUnits,
    pub(crate) point_lights: Vec<PointLight>,
    pub(crate) detail_level: u32,
    pub(crate) fingerprint: u64,
}

/// Derive the complete visible lighting input from one committed world view.
pub(crate) fn derive_lighting_view(
    lighting_config: &LightingConfig,
    simulation: Option<&Simulation>,
    rules: Option<&RuleSet>,
    detail_level: u32,
) -> DerivedLightingView {
    let mut fingerprint = LightingFingerprint::new();
    let profile = simulation.map_or_else(
        || lighting::normal_profile_units(lighting_config),
        |sim| {
            let state = &sim.session.lighting;
            let selected = match state.selected_profile {
                crate::sim::scenario_session::ScenarioLightingProfile::Normal => state.normal,
                crate::sim::scenario_session::ScenarioLightingProfile::Ion => state.ion,
            };
            fingerprint.mix_i32(state.target_ambient);
            fingerprint.mix_u64(match state.selected_profile {
                crate::sim::scenario_session::ScenarioLightingProfile::Normal => 0,
                crate::sim::scenario_session::ScenarioLightingProfile::Ion => 1,
            });
            fingerprint.mix_i32(state.transition_timer.start_frame());
            fingerprint.mix_i32(state.transition_timer.duration());
            LightingProfileUnits {
                ambient_percent: state.current_ambient,
                red_percent: selected.red_percent,
                green_percent: selected.green_percent,
                blue_percent: selected.blue_percent,
                ground_units: selected.ground_units,
                level_units: selected.level_units,
            }
        },
    );
    fingerprint.mix_profile(profile);
    fingerprint.mix_u64(u64::from(detail_level));

    let building_lights = collect_live_building_lights(simulation, rules, detail_level);
    let radiation_lights = match (simulation, rules) {
        (Some(sim), Some(rules)) => {
            crate::app::presentation::radiation_light::collect_radiation_lights(sim, rules)
        }
        _ => Vec::new(),
    };

    let mut point_lights = Vec::with_capacity(building_lights.len() + radiation_lights.len());
    for (stable_id, light) in building_lights {
        fingerprint.mix_u64(0x42);
        fingerprint.mix_u64(stable_id);
        fingerprint.mix_point_light(&light);
        point_lights.push(light);
    }
    for light in radiation_lights {
        fingerprint.mix_u64(0x52);
        fingerprint.mix_point_light(&light);
        point_lights.push(light);
    }

    DerivedLightingView {
        profile,
        point_lights,
        detail_level: detail_level.min(2),
        fingerprint: fingerprint.finish(),
    }
}

/// Build the cell grid for an already-derived complete view.
pub(crate) fn build_lighting_grid_from_view(
    resolved_terrain: &ResolvedTerrainGrid,
    view: &DerivedLightingView,
) -> CellLightGrid {
    let mut grid = lighting::build_cell_light_grid_from_heights_and_units_with_detail(
        resolved_terrain
            .iter()
            .map(|cell| ((cell.rx, cell.ry), cell.level)),
        view.profile,
        view.detail_level,
    );
    lighting::accumulate_point_lights(&mut grid, &view.point_lights);
    grid
}

/// Rebuild transient app lighting from the selected scenario profile plus the
/// current live building and radiation sources.
pub(crate) fn rebuild_lighting_grid_from_sim(
    resolved_terrain: &ResolvedTerrainGrid,
    lighting_config: &LightingConfig,
    simulation: Option<&Simulation>,
    rules: Option<&RuleSet>,
    detail_level: u32,
) -> CellLightGrid {
    let view = derive_lighting_view(lighting_config, simulation, rules, detail_level);
    build_lighting_grid_from_view(resolved_terrain, &view)
}

fn collect_live_building_lights(
    simulation: Option<&Simulation>,
    rules: Option<&RuleSet>,
    detail_level: u32,
) -> Vec<(u64, PointLight)> {
    let (Some(sim), Some(rules)) = (simulation, rules) else {
        return Vec::new();
    };
    if detail_level < 2 {
        return Vec::new();
    }
    sim.entities()
        .values()
        .filter(|entity| {
            entity.category == crate::map::entities::EntityCategory::Structure
                && entity.lifecycle.object_alive
                && !entity.lifecycle.in_limbo
                && entity.lifecycle.cell_marked
                && !entity.dying
                && entity.health.current > 0
                && crate::sim::power_system::is_building_powered(
                    &sim.power_states,
                    rules,
                    entity,
                    &sim.interner,
                )
        })
        .filter_map(|entity| {
            let type_id = sim.interner.resolve(entity.type_ref);
            let obj = rules.object(type_id)?;
            let light = lighting::point_light_from_object(
                entity.position.rx,
                entity.position.ry,
                obj.light_visibility,
                obj.light_intensity,
                [
                    obj.light_red_tint,
                    obj.light_green_tint,
                    obj.light_blue_tint,
                ],
            )?;
            Some((entity.stable_id, light))
        })
        .collect()
}

struct LightingFingerprint(u64);

impl LightingFingerprint {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;

    fn new() -> Self {
        Self(Self::OFFSET)
    }

    fn mix_u64(&mut self, value: u64) {
        for byte in value.to_le_bytes() {
            self.0 ^= u64::from(byte);
            self.0 = self.0.wrapping_mul(Self::PRIME);
        }
    }

    fn mix_i32(&mut self, value: i32) {
        self.mix_u64(u64::from(value as u32));
    }

    fn mix_profile(&mut self, profile: LightingProfileUnits) {
        self.mix_i32(profile.ambient_percent);
        self.mix_i32(profile.red_percent);
        self.mix_i32(profile.green_percent);
        self.mix_i32(profile.blue_percent);
        self.mix_i32(profile.ground_units);
        self.mix_i32(profile.level_units);
    }

    fn mix_point_light(&mut self, light: &PointLight) {
        self.mix_u64(u64::from(light.rx));
        self.mix_u64(u64::from(light.ry));
        self.mix_i32(light.center_x);
        self.mix_i32(light.center_y);
        self.mix_i32(light.radius_leptons);
        self.mix_i32(light.intensity);
        for tint in light.tint {
            self.mix_i32(tint);
        }
        self.mix_u64(u64::from(u8::from(light.active)));
        self.mix_u64(u64::from(u8::from(light.detail)));
    }

    fn finish(self) -> u64 {
        self.0
    }
}

// Scenario-menu metadata is map-owned (F06); re-exported for app callers.
pub use crate::map::scenario_menu::MapMenuEntry;

pub(crate) fn load_map_initial_with_assets(
    ra2_dir: PathBuf,
    asset_manager: &mut AssetManager,
    requested_map: Option<&str>,
    progress: &mut dyn crate::app::loading::pump::LoadingProgressSink,
) -> Result<MapLoadInitial> {
    // TubeMovement and random-map geometry share the retail executable's
    // sine/cosine table.  Install it before selecting the map source: explicit
    // `[Tubes]` are valid in ordinary .map/.mpr/.mmx content, not just .SED
    // random maps.  OnceLock keeps repeated map loads read-only and cheap.
    crate::map::rmg::trig::install_from_dir(&ra2_dir);
    if !crate::map::retail_trig::wave_tables_available() {
        anyhow::bail!(
            "{} does not provide the verified gamemd sine/Acos tables required by stock Sonic Wave simulation",
            ra2_dir.join("gamemd.exe").display()
        );
    }

    // Check RA2_QUICKPLAY env var: if it names a .map/.mpr file, load that directly.
    // UI-selected map name/path (requested_map) takes precedence.
    // Default: try testmap1.map in the project directory first, then fall back to .mmx files.
    let quickplay_map: Option<String> = std::env::var("RA2_QUICKPLAY")
        .ok()
        .filter(|v| v.ends_with(".map") || v.ends_with(".mpr") || v.ends_with(".mmx"));
    // `RA2_QUICKPLAY=<name>.sed` forces a random-map generation without going
    // through the skirmish UI — a dev shortcut for exercising the generator.
    let quickplay_seed: Option<String> = std::env::var("RA2_QUICKPLAY")
        .ok()
        .filter(|v| crate::map::rmg::is_seed_selection(v));

    // A `.SED` selection names a random-map seed, not a map file: the map is
    // generated in memory from its options. Handled before the file-loading
    // branches below, which would look for a map file that does not exist.
    let seed_selection: Option<String> = requested_map
        .filter(|name| crate::map::rmg::is_seed_selection(name))
        .map(str::to_string)
        .or(quickplay_seed);
    if let Some(seed_name) = seed_selection.as_deref() {
        let mut options = crate::map::rmg::RmgOptions::default();
        match std::fs::read(ra2_dir.join(seed_name)) {
            Ok(bytes) => match crate::rules::ini_parser::IniFile::from_bytes(&bytes) {
                Ok(ini) => options.apply_sed(&ini),
                Err(err) => {
                    log::warn!("random map: {seed_name} is not valid INI ({err}); using defaults")
                }
            },
            // Missing seed file is not fatal: the original's options object
            // keeps its constructor defaults when a key (or the file) is absent.
            Err(err) => log::warn!("random map: cannot read {seed_name} ({err}); using defaults"),
        }
        options.normalize();

        let settings = crate::map::rmg::RmgSettings::load(asset_manager);
        let theater_name = crate::map::rmg::emit::theater_name(options.theater);
        let theater = crate::map::theater::load_theater(asset_manager, theater_name)
            .ok_or_else(|| anyhow::anyhow!("random map: theater {theater_name} unavailable"))?;

        // Terrain rules feed the zone classifier's wheel-impassable table. The
        // table is observably inert during generation (the classifier runs at
        // start-placement time, before any rock/rough/cliff terrain exists), but
        // it is resolved faithfully from `rulesmd.ini` when present; a missing
        // file falls back to the passable defaults.
        let terrain_rules = asset_manager
            .get_ref("rulesmd.ini")
            .and_then(|bytes| crate::rules::ini_parser::IniFile::from_bytes(bytes).ok())
            .map(|ini| crate::rules::terrain_rules::TerrainRules::from_ini(&ini))
            .unwrap_or_default();
        let resolved = crate::map::rmg::build::ResolvedTheaterInputs::from_theater(
            &theater,
            &terrain_rules,
            crate::map::rmg::trig::global().cloned(),
        );

        // Tile-block layouts (sub-cell height/terrain grids) from the theater's
        // real TMP data — the shore tiler and zone classifier read these.
        let blocks =
            crate::map::rmg::theater_blocks::TheaterTileBlocks::build(&theater.lookup, |name| {
                asset_manager.get(name)
            });

        // `[AI] NeutralTechBuildings` plus each type's `Foundation=`. The phase
        // runs for every map type except 0, so an empty list here would both
        // strip the buildings and skip the draws the original consumes.
        let tech_types = crate::app::loading::init_helpers::load_neutral_tech_types(asset_manager);
        let generated = crate::map::rmg::build::generate_map(
            &options,
            &settings,
            &resolved,
            &blocks,
            &tech_types,
        );
        log::info!(
            "Random map generated: theater={}, {}x{}, seed={}, players={}",
            generated.map_file.header.theater,
            generated.map_file.header.width,
            generated.map_file.header.height,
            options.seed,
            options.num_players
        );
        if generated.unfilled_start_slots > 0 {
            log::warn!(
                "Random map is short of spawns: {} start slot(s) could not be \
                 filled (seed={}, players={}); those players have no start position",
                generated.unfilled_start_slots,
                options.seed,
                options.num_players
            );
        }

        progress.milestone(8);
        let mapgen_rng_continuation = generated.mapgen_continuation;
        return Ok(MapLoadInitial {
            map_data: generated.map_file,
            map_source: LoadedMapSource::Generated {
                seed_name: seed_name.to_string(),
            },
            mapgen_rng_continuation: Some(mapgen_rng_continuation),
        });
    }

    let loaded_map: LoadedMap =
        if let Some(map_name) = requested_map.filter(|m| !m.eq_ignore_ascii_case("auto")) {
            load_map_by_name_or_path_with_assets(&ra2_dir, map_name, &asset_manager)?
        } else if let Some(ref map_name) = quickplay_map {
            load_map_by_name_or_path_with_assets(&ra2_dir, map_name, &asset_manager)?
        } else if Path::new("testmap1.map").exists() {
            let bytes: Vec<u8> = std::fs::read("testmap1.map")?;
            log::info!("Loading default map: testmap1.map");
            LoadedMap {
                map: MapFile::from_bytes(&bytes)?,
                source: LoadedMapSource::Loose {
                    path: PathBuf::from("testmap1.map"),
                    payload_len: bytes.len(),
                },
            }
        } else {
            let mmx_names: &[&str] = &[
                "Dustbowl.mmx",
                "Barrel.mmx",
                "GoldSt.mmx",
                "Kaliforn.mmx",
                "Hills.mmx",
                "Grinder.mmx",
                "Break.mmx",
                "Potomac.mmx",
                "Arena.mmx",
                "Lostlake.mmx",
                "Oceansid.mmx",
                "Pacific.mmx",
            ];
            try_load_mmx(&ra2_dir, mmx_names)?
        };
    let LoadedMap {
        map: map_data,
        source: map_source,
    } = loaded_map;
    log::info!(
        "Map loaded: title={:?}, theater={}, {}x{}, {} cells, {} entities",
        map_data.basic.name,
        map_data.header.theater,
        map_data.header.width,
        map_data.header.height,
        map_data.cells.len(),
        map_data.entities.len()
    );
    log_trigger_graph_diagnostics(&map_data);

    // First post-render-handoff milestone: the selected map has been opened and
    // parsed. (gamemd emits 8 at theater-init entry; in our pipeline the map is
    // parsed first, so 8 marks the end of this initial phase.)
    progress.milestone(8);

    Ok(MapLoadInitial {
        map_data,
        map_source,
        mapgen_rng_continuation: None,
    })
}

pub(crate) fn load_map_from_initial(
    gpu: &GpuContext,
    batch: &BatchRenderer,
    asset_manager: &mut AssetManager,
    initial: MapLoadInitial,
    startup: LoadingStartup,
    preloaded_battle_start_plan: Option<PreloadedBattleStartPlan>,
    skirmish_settings: &crate::ui::main_menu::SkirmishSettings,
    theater_cache_mismatch: bool,
    runtime_color_scheme_count: usize,
    mut vxl_compute: Option<&mut crate::render::vxl_compute::VxlComputeRenderer>,
    shared_cell_dummy: crate::map::resolved_terrain::SharedCellDummy,
    tile_variant_selector_cache: &mut crate::map::tile_variant_selector::TileVariantSelectorCache,
    progress: &mut dyn crate::app::loading::pump::LoadingProgressSink,
) -> Result<MapLoadResult> {
    let MapLoadInitial {
        map_data,
        map_source,
        mapgen_rng_continuation,
    } = initial;
    let map_hash = match &map_source {
        LoadedMapSource::Loose { .. }
        | LoadedMapSource::Mix { .. }
        | LoadedMapSource::Generated { .. } => Some(map_data.ini.content_hash()),
        LoadedMapSource::LegacyFallback { .. } => None,
    };
    let skirmish_launch_session = startup.launch_session();
    // F09: the sim-owned launch descriptor. The shell close transaction
    // resolved every random country/color before this point, so validation
    // failing here is a programming error, not a user state.
    let match_launch_descriptor = skirmish_launch_session.map(|session| {
        crate::sim::scenario_bootstrap::MatchLaunchDescriptor::from_resolved(session.clone())
            .expect("shell close transaction resolves every random choice before launch")
    });
    // The scenario/Main pair and the one-time TMP selector construction share
    // the same single resolved seed word. Generic loads retain the explicitly
    // unverified fallback, but it is sampled exactly once.
    let match_seed = startup
        .seed_or_else(crate::app::loading::init_helpers::generate_unverified_legacy_match_seed);

    // Load theater INI for tileset lookup, palette, and LAT configuration.
    // Also loads theater-specific MIX archives (e.g., isotemmd.mix) at highest priority.
    let theater_result: Option<theater::TheaterData> =
        theater::load_theater(asset_manager, &map_data.header.theater);
    if theater_cache_mismatch {
        progress.milestone(12);
        // Native advances while rebuilding each color scheme. Rust's theater
        // loader is monolithic, so present the verified pre-load-count sequence
        // synchronously after that work instead of faking per-item callbacks.
        for value in
            crate::app::loading::pump::theater_ramp_changed_values(runtime_color_scheme_count)
        {
            progress.milestone(value);
        }
        progress.milestone(25);
    }
    progress.milestone(30);

    let theater_ext: &'static str = match &theater_result {
        Some(td) => td.extension,
        None => theater_ext_for(&map_data.header.theater),
    };

    // Native map loading always runs the LAT half. The terrain builder no-ops
    // when theater data or a complete LAT group configuration is unavailable.
    let lat_enabled = true;

    // Load rules.ini and art.ini before building resolved terrain so overlay
    // semantics and art-foundation data are available to the pipeline.
    // The selected game mode's override INI (MPModes roster row) applies its
    // rules payload between rulesmd and the map overrides — without it every
    // non-Battle mode silently plays with Battle rules.
    let mode_override_ini: Option<IniFile> = {
        let override_file = skirmish_launch_session
            .map(|s| s.mode.override_file.trim())
            .unwrap_or("");
        if override_file.is_empty() {
            None
        } else {
            asset_manager
                .get_with_source(override_file)
                .and_then(|(data, source)| {
                    log::info!(
                        "Loading game-mode rules override {} ({} bytes) from {}",
                        override_file,
                        data.len(),
                        source
                    );
                    IniFile::from_bytes(&data)
                        .map_err(|err| {
                            log::warn!("Failed to parse game-mode override {override_file}: {err}")
                        })
                        .ok()
                })
        }
    };
    let (loaded_rules, rules_ini) = load_rules_with_merged_ini(
        &asset_manager,
        mode_override_ini.as_ref(),
        Some(&map_data.ini),
    )
    .ok_or_else(|| anyhow::anyhow!("failed to load or validate merged game rules"))?
    .into_parts();
    let fixed_team_ai_ini =
        crate::app::loading::init_helpers::load_retail_team_ai_source(&asset_manager)
            .ok_or_else(|| anyhow::anyhow!("failed to load active YR aimd.ini"))?;
    let team_ai_registry = crate::rules::team_ai_ini::TeamAiIniRegistry::from_sources(
        &fixed_team_ai_ini,
        &map_data.ini,
        skirmish_launch_session.is_some(),
    );
    if !team_ai_registry.fixed_source_is_complete() {
        anyhow::bail!(
            "active YR aimd.ini failed structural validation: fixed_counts={:?}, diagnostics={:?}",
            team_ai_registry.fixed_counts,
            team_ai_registry.diagnostics
        );
    }
    for diagnostic in &team_ai_registry.diagnostics {
        log::warn!("Team AI INI diagnostic: {diagnostic:?}");
    }
    let mut rules: Option<RuleSet> = Some(loaded_rules);
    let art_result: Option<(ArtRegistry, IniFile)> = load_art_ini(&asset_manager);
    let (mut art, art_ini): (Option<ArtRegistry>, Option<IniFile>) = match art_result {
        Some((reg, ini)) => (Some(reg), Some(ini)),
        None => (None, None),
    };
    if let (Some(r), Some(a)) = (rules.as_mut(), art.as_mut()) {
        r.merge_art_data(a);
        // Eagerly populate per-anim SHP frame dimensions so the smudge
        // dispatcher can size-filter without falling back to the (30, 30)
        // default that always loses the threshold check.
        let (populated, fallback) =
            a.populate_anim_frame_dims(&asset_manager, theater_ext, &map_data.header.theater);
        log::info!(
            "Anim frame dims: {} populated, {} fallback (defaults to 30x30)",
            populated,
            fallback,
        );
        // Retain the art registry on RuleSet so dispatchers (e.g. smudge
        // spawning) can read per-anim spawn flags via &RuleSet alone.
        // Cloned because downstream consumers in this function still read
        // through the `art` Option (lighting, sidebar, sim spawn, etc.).
        r.art_registry = a.clone();
    }
    // Resolve warp animation rates from art.ini sections (e.g., [WARPOUT] Rate=120).
    if let (Some(r), Some(art_ini_file)) = (&mut rules, &art_ini) {
        r.general.resolve_art_rates(art_ini_file);
    }
    // Parse infantry animation sequence definitions from art.ini [*Sequence] sections.
    let infantry_sequences = if let Some(ref art_ini_file) = art_ini {
        crate::rules::infantry_sequence::parse_infantry_sequence_registry(art_ini_file)
    } else {
        HashMap::new()
    };
    // Rules + art parsed, merged, and processed (gamemd command-bar/CD/rules
    // milestones).
    progress.milestone(31);
    progress.milestone(35);
    progress.milestone(45);
    let csf = Some(load_csf(&asset_manager)?);
    let overlay_registry: OverlayTypeRegistry =
        OverlayTypeRegistry::from_ini(&rules_ini, art_ini.as_ref());

    // Compute playable area bounds from LocalSize (border filler hidden by shroud).
    let local_bounds: Option<LocalBounds> = Some(LocalBounds::from_header(&map_data.header));

    let cliff_back = rules
        .as_ref()
        .map(|r| r.general.cliff_back_impassability)
        .unwrap_or(2);
    let mut bootstrap_rng = ScenarioBootstrapRng::new(match_seed);
    if let Some(continuation) = mapgen_rng_continuation {
        bootstrap_rng.install_generated_mapgen_continuation(continuation);
    }
    if let Some(plan) = preloaded_battle_start_plan.as_ref() {
        // Native constructs houses and resolves Battle starts before the first
        // loading composition; terrain Fill continues that same Scenario
        // stream afterwards. Validate the launch cursor before installing the
        // immutable pre-render prefix so it can never be consumed twice.
        bootstrap_rng.install_preloaded_battle_plan(plan)?;
    }
    let (mut scenario_fill_rng, mut variant_main_rng) = bootstrap_rng.terrain_draws();
    let mut scenario_fill_ranged =
        |low, high| scenario_fill_rng.next_range_u32_inclusive(low, high);
    let mut variant_draw = || variant_main_rng.next_u32();
    let mut variant_selector = tile_variant_selector_cache.begin_load(&mut variant_draw);
    // Native `MapClass::Resize @ 0x00565C10` reconstructs the fixed fallback
    // CellClass through `CellClass::Constructor @ 0x0047BBF0` before Fill and
    // IsoMapPack materialize the new map. Reset its modeled bytes in place;
    // replacing this handle would break process-global pointer identity.
    shared_cell_dummy.reconstruct_for_map_resize();
    let mut resolved_terrain = ResolvedTerrainGrid::build_with_variant_selector_and_shared_dummy(
        &map_data,
        theater_result.as_ref(),
        Some(&asset_manager),
        rules.as_ref().map(|r| &r.terrain_rules),
        Some(&overlay_registry),
        rules.as_ref().map(|r| &r.terrain_object_types),
        lat_enabled,
        cliff_back,
        &mut scenario_fill_ranged,
        &mut variant_selector,
        shared_cell_dummy,
    );
    // Bind the complete scheduler closure only after theater Tile##Anim rows
    // have resolved, but before any atlas or AnimClass construction. Missing
    // tile art is a load error rather than a silently invisible map feature.
    if let (Some(r), Some(a)) = (rules.as_mut(), art.as_mut()) {
        let roots = scheduler_anim_roots(r, resolved_terrain.tile_animations());
        a.bind_scheduler_anim_assets(
            &roots,
            &asset_manager,
            theater_ext,
            &map_data.header.theater,
        )?;
        r.art_registry = a.clone();
        r.bind_effect_assets(&asset_manager, theater_ext, &map_data.header.theater);
        r.bind_terrain_spawner_assets(
            &rules_ini,
            &asset_manager,
            theater_ext,
            &map_data.header.theater,
        );
        r.bind_animation_sequences(&infantry_sequences);
    }
    let variant_table_generated = variant_selector.generated_table();
    let map_fill_scenario_advances = variant_selector.map_fill_scenario_advance_count();
    let variant_table_draws = variant_selector.raw_draw_count();
    drop(variant_selector);
    drop(variant_draw);
    drop(scenario_fill_ranged);
    drop(variant_main_rng);
    drop(scenario_fill_rng);
    // Native Fill snapshots prior process-global ClearTile/WaterSet values
    // before the current theater registry reload. Rust loads assets earlier,
    // so defer publishing current results until materialization is complete.
    if let Some(theater) = theater_result.as_ref() {
        tile_variant_selector_cache.complete_theater_registry_load(
            theater.rmg_tiles.clear_tile,
            theater.rmg_tiles.water_set,
        );
    }
    log::info!(
        "Map terrain load: {} Scenario Fill cursor advances; TMP variant table {} this load, {} raw Main draws",
        map_fill_scenario_advances,
        if variant_table_generated {
            "generated"
        } else if tile_variant_selector_cache.is_initialized() {
            "reused"
        } else {
            "not reached"
        },
        variant_table_draws,
    );
    let anchor_variant_table = theater_result
        .as_ref()
        .and_then(crate::map::theater::BridgeAnchorVariantTable::from_theater);
    let grid: TerrainGrid = terrain::build_terrain_grid_from_resolved(
        &resolved_terrain,
        local_bounds,
        anchor_variant_table,
    );
    // Side/house mix + resolved terrain grid ready.
    progress.milestone(50);
    progress.milestone(55);

    // Build per-cell lighting from map [Lighting] section.
    let lighting_config = lighting::parse_lighting(&map_data.ini);
    let lighting_profiles = lighting::parse_lighting_profiles(&map_data.ini);
    // [Basic]/lighting read complete (gamemd Read_INI_Basic milestones).
    progress.milestone(58);
    progress.milestone(60);

    let tile_atlas: Option<TileAtlas> = match &theater_result {
        Some(td) => build_tile_atlas(
            &asset_manager,
            &td.lookup,
            &td.iso_palette,
            td.extension,
            &grid,
            gpu,
            batch,
        ),
        None => None,
    };

    // Theater tileset / map-section surfaces built (gamemd map-section milestones).
    progress.milestone(63);
    progress.milestone(65);

    let art_fallback: ArtRegistry = ArtRegistry::empty();
    let overlay_shp_ids = resolved_overlay_shp_ids(
        &overlay_registry,
        &rules_ini,
        art.as_ref().unwrap_or(&art_fallback),
        &asset_manager,
        theater_ext,
        &map_data.header.theater,
    );
    let mut overlay_grid = crate::sim::overlay_grid::OverlayGrid::from_native_overlay_packs(
        &map_data.overlays,
        &map_data.overlay_data,
        &mut resolved_terrain,
        &overlay_registry,
        &overlay_shp_ids,
        skirmish_launch_session.is_some(),
    );
    let cleared_terrain_overlay_cells = rules.as_ref().map_or_else(BTreeSet::new, |rules| {
        crate::sim::terrain_spawn::clear_tiberium_source_cells_for_terrain(
            &mut overlay_grid,
            &mut resolved_terrain,
            &map_data.terrain_objects,
            rules,
            &overlay_registry,
        )
    });
    if !cleared_terrain_overlay_cells.is_empty() {
        log::info!(
            "Cleared {} same-cell tiberium overlay cell(s) for recognized terrain",
            cleared_terrain_overlay_cells.len(),
        );
    }

    // Parse house color assignments from map INI ([Houses] + per-house Color=).
    // Color=<name> resolves against the rules `[Colors]` list (entry index).
    let color_schemes: &[crate::rules::color_scheme::ColorSchemeEntry] = rules
        .as_ref()
        .map(|r| r.color_schemes.as_slice())
        .unwrap_or(&[]);
    let house_roster: HouseRoster = houses::parse_house_roster(&map_data.ini, color_schemes);
    let house_color_map: HouseColorMap = skirmish_launch_session.map_or_else(
        || house_roster.color_map(),
        |session| house_color_map_for_launch_session(session, &house_roster),
    );
    progress.milestone(67);

    // Build height lookup for entity/overlay elevation (shared between subsystems).
    let height_map: BTreeMap<(u16, u16), u8> = resolved_terrain.build_height_map();
    let bridge_height_map: BTreeMap<(u16, u16), u8> = resolved_terrain.build_bridge_height_map();
    let tactical_bridge_inverse_map: BTreeMap<(u16, u16), crate::map::terrain::TacticalBridgeCell> =
        resolved_terrain.build_tactical_bridge_inverse_map();

    let bridge_railing_tile_bases = theater_result
        .as_ref()
        .and_then(|td| td.bridge_railing_slope_starts())
        .map(
            |(slope_set_pieces_start, slope_set_pieces2_start)| BridgeRailingTileBases {
                slope_set_pieces_start,
                slope_set_pieces2_start,
            },
        );

    // Extract theater palettes for entity/overlay rendering.
    // Move palettes out of TheaterData (no longer needed after tile atlas is built).
    let (unit_palette, overlay_iso_palette, overlay_tiberium_palette) = match theater_result {
        Some(td) => (
            Some(td.unit_palette),
            Some(td.iso_palette),
            Some(td.tiberium_palette),
        ),
        None => (None, None, None),
    };
    // Map/overlay prelude ready (gamemd IsoMapPack/overlay milestones).
    progress.milestone(68);
    progress.milestone(69);
    progress.milestone(70);

    let bridge_destroyability_mode =
        skirmish_launch_session.map_or(BridgeDestroyabilityMode::CampaignOrEditor, |session| {
            BridgeDestroyabilityMode::SkirmishOrMultiplayer {
                bridge_destruction: session.options.bridges_destroyable,
            }
        });

    // Every shell startup carries its one fresh pre-loading seed unchanged.
    // Generic loading alone retains the explicitly noncertifying SystemTime
    // fallback. In every case the selected word exists before Simulation.
    let scenario_descriptor = crate::sim::scenario_session::ScenarioDescriptor {
        seed: match_seed,
        map_name: skirmish_launch_session
            .and_then(|s| s.selected_map_file.clone())
            .or_else(|| map_data.basic.name.clone())
            .unwrap_or_default(),
        theater: map_data.header.theater.clone(),
        game_mode_nonzero: skirmish_launch_session.is_some(),
        // Campaign/editor reads `[SpecialFlags] Inert=`. Nonzero game modes
        // replace active SpecialFlags from session staging; ordinary skirmish
        // has no writer for bit 0x20 and therefore starts false.
        no_damage: skirmish_launch_session.is_none()
            && map_data.special_flags.inert.unwrap_or(false),
        // CANONICAL CELL-ARRAY FRAME, not [Map] Size=. Sim cell coordinates
        // (entities, waypoints, vision) live in the iso array whose extent is
        // ~(SizeW+SizeH); seeding bounds from Size= verbatim leaves most of
        // the diamond — including start waypoints — outside the fog window.
        // The raw Size= width stays available on `Simulation.playfield_bounds`.
        map_width: resolved_terrain.width(),
        map_height: resolved_terrain.height(),
        local_left: map_data.header.local_left as u16,
        local_top: map_data.header.local_top as u16,
        local_width: map_data.header.local_width as u16,
        local_height: map_data.header.local_height as u16,
        mp_start_waypoints: waypoints::multiplayer_start_waypoints(&map_data.waypoints)
            .into_iter()
            .map(|wp| (wp.index, (wp.rx, wp.ry)))
            .collect(),
        lighting: crate::sim::scenario_session::ScenarioLightingState::new(
            crate::sim::scenario_session::ScenarioLightProfileUnits {
                ambient_percent: lighting_profiles.normal.ambient_percent,
                red_percent: lighting_profiles.normal.red_percent,
                green_percent: lighting_profiles.normal.green_percent,
                blue_percent: lighting_profiles.normal.blue_percent,
                ground_units: lighting_profiles.normal.ground_units,
                level_units: lighting_profiles.normal.level_units,
            },
            crate::sim::scenario_session::ScenarioLightProfileUnits {
                ambient_percent: lighting_profiles.ion.ambient_percent,
                red_percent: lighting_profiles.ion.red_percent,
                green_percent: lighting_profiles.ion.green_percent,
                blue_percent: lighting_profiles.ion.blue_percent,
                ground_units: lighting_profiles.ion.ground_units,
                level_units: lighting_profiles.ion.level_units,
            },
        ),
    };
    log::info!("Match seed: 0x{:08X}", scenario_descriptor.seed);

    let initialize_houses_before_objects = |sim: &mut Simulation| {
        if let Some(descriptor) = match_launch_descriptor.as_ref() {
            let ruleset = rules
                .as_ref()
                .expect("offline skirmish requires rules before House construction");
            initialize_skirmish_launch_houses(sim, &house_roster, ruleset, descriptor);
        } else {
            initialize_map_roster_houses(sim, &house_roster, rules.as_ref());
        }
    };
    // F09 seam: GPU-free construction first, then the presentation manifest
    // is derived from the constructed simulation — never fed back into it.
    let constructed = crate::app::loading::init_helpers::construct_app_scenario(
        &map_data,
        &resolved_terrain,
        &asset_manager,
        &map_data.header.theater,
        rules.as_ref(),
        art.as_ref(),
        &height_map,
        Some(&overlay_registry),
        Some(&overlay_grid),
        bridge_destroyability_mode,
        &scenario_descriptor,
        bootstrap_rng,
        initialize_houses_before_objects,
    );
    let manifest = crate::app::loading::init_helpers::build_presentation_manifest(
        &constructed,
        &asset_manager,
        gpu,
        batch,
        theater_ext,
        &map_data.header.theater,
        rules.as_ref(),
        art.as_ref(),
        &house_color_map,
        unit_palette.as_ref(),
        overlay_iso_palette.as_ref(),
        vxl_compute.as_deref_mut(),
    );
    let (mut unit_atlas, mut sprite_atlas, mut palette_set) = (
        manifest.unit_atlas,
        manifest.sprite_atlas,
        manifest.palette_set,
    );
    // Terrain/tiberium + units/infantry/buildings created from the map
    // (gamemd terrain/units/objects/buildings milestones).
    progress.milestone(72);
    progress.milestone(74);
    progress.milestone(76);
    progress.milestone(78);
    let mut simulation = Some(constructed);
    // Pre-intern all rule type IDs so that build_option_for_owner can resolve
    // InternedIds for types that haven't been spawned yet (e.g. GAPOWR).
    // Without this, sidebar cameo lookups fail because unspawned types get
    // InternedId(0) and resolve to the wrong string.
    if let (Some(sim), Some(ruleset)) = (&mut simulation, rules.as_ref()) {
        sim.intern_rule_type_ids(ruleset);
        // One-hop type resolution: build the handle table now that every type
        // id is interned. This also pre-resolves the `[CombatDamage]` bridge
        // warhead handles combat compares during the bridge-damage path;
        // resolution must happen before any combat tick.
        sim.resolve_type_handles(ruleset);
        let diagnostics = sim.install_team_ai_registry(&team_ai_registry, ruleset);
        if diagnostics
            .iter()
            .any(crate::sim::team_script_vm::TeamAiInstallDiagnostic::is_fixed_source_refusal)
        {
            anyhow::bail!(
                "active YR aimd.ini failed RuleSet resolution: diagnostics={diagnostics:?}"
            );
        }
        for diagnostic in diagnostics {
            log::warn!("Team AI install diagnostic: {diagnostic:?}");
        }
    }

    // SpawnPick phase is disabled — MCV always spawns directly at the chosen position.
    let spawn_pick_pending: bool = false;

    let mut initial_local_owner: Option<String> = None;
    if !spawn_pick_pending {
        if let (Some(sim), Some(ruleset)) = (&mut simulation, rules.as_ref()) {
            let should_rebuild_entity_atlases =
                if let Some(descriptor) = match_launch_descriptor.as_ref() {
                    // Complete standard-Battle vectors were resolved before the
                    // first loading frame and terrain continued their post-plan
                    // Scenario cursor. Other modes/deficient maps retain the
                    // terrain-dependent runtime resolver below.
                    let result = if let Some(plan) = preloaded_battle_start_plan.as_ref() {
                        apply_preloaded_battle_launch_session_with_overlay_registry(
                            sim,
                            &map_data,
                            &house_roster,
                            ruleset,
                            &height_map,
                            &resolved_terrain,
                            descriptor,
                            &overlay_registry,
                            plan,
                        )
                    } else {
                        apply_explicit_skirmish_launch_session_with_overlay_registry(
                            sim,
                            &map_data,
                            &house_roster,
                            ruleset,
                            &height_map,
                            &resolved_terrain,
                            descriptor,
                            &overlay_registry,
                        )
                    };
                    initial_local_owner = result.local_owner;
                    result.spawned_mcvs > 0
                } else {
                    initial_local_owner = seed_skirmish_opening_if_needed(
                        sim,
                        &map_data,
                        &house_roster,
                        ruleset,
                        &height_map,
                        &overlay_registry,
                        skirmish_settings,
                    );
                    // Set up AI players: all playable houses except the local (first) player.
                    if let Some(ref local_owner) = initial_local_owner {
                        sim.register_ai_players_from_roster(&house_roster, local_owner);
                    }
                    initial_local_owner.is_some()
                };

            if should_rebuild_entity_atlases {
                let (new_unit_atlas, new_sprite_atlas, new_palette_set) = build_entity_atlases(
                    sim,
                    &asset_manager,
                    gpu,
                    batch,
                    theater_ext,
                    &map_data.header.theater,
                    rules.as_ref(),
                    art.as_ref(),
                    &house_color_map,
                    unit_palette.as_ref(),
                    overlay_iso_palette.as_ref(),
                    vxl_compute.as_deref_mut(),
                );
                unit_atlas = new_unit_atlas;
                sprite_atlas = new_sprite_atlas;
                palette_set = new_palette_set;
            }
        }
    }

    // Optional debug spawn list for render testing.
    // Examples:
    //   RA2_DEBUG_SPAWN_UNITS=1                  -> default list (HTNK,MTNK,E1)
    //   RA2_DEBUG_SPAWN_UNITS=HTNK,MTNK,APOC
    if let (Some(sim), Some(ruleset), Some(debug_units)) = (
        &mut simulation,
        rules.as_ref(),
        parse_debug_spawn_units_env(),
    ) {
        let owner: String = house_color_map
            .keys()
            .find(|h| {
                let up = h.to_ascii_uppercase();
                up != "NEUTRAL" && up != "SPECIAL"
            })
            .cloned()
            .unwrap_or_else(|| "Americans".to_string());

        let (anchor_rx, anchor_ry): (u16, u16) = map_data
            .entities
            .iter()
            .find(|e| {
                e.category == crate::map::entities::EntityCategory::Structure
                    && e.owner.eq_ignore_ascii_case(&owner)
            })
            .map(|e| (e.cell_x, e.cell_y))
            .or_else(|| {
                waypoints::first_multiplayer_start(&map_data.waypoints).map(|wp| (wp.rx, wp.ry))
            })
            .or_else(|| map_data.cells.first().map(|c| (c.rx, c.ry)))
            .unwrap_or((50, 50));

        let offsets: &[(i32, i32)] = &[
            (2, 2),
            (4, 2),
            (6, 2),
            (2, 4),
            (4, 4),
            (6, 4),
            (2, 6),
            (4, 6),
        ];
        let mut spawned: u32 = 0;
        for (i, type_id) in debug_units.iter().enumerate() {
            let (ox, oy) = offsets[i % offsets.len()];
            let rx = (anchor_rx as i32 + ox).max(0) as u16;
            let ry = (anchor_ry as i32 + oy).max(0) as u16;
            if sim
                .spawn_object_with_overlay_registry(
                    type_id,
                    &owner,
                    rx,
                    ry,
                    64,
                    ruleset,
                    &height_map,
                    &overlay_registry,
                )
                .is_some()
            {
                spawned += 1;
            } else {
                log::warn!("Debug spawn failed for '{}'", type_id);
            }
        }
        if spawned > 0 {
            log::info!(
                "Debug-spawned {} unit(s) for owner={} near ({},{}): {:?}",
                spawned,
                owner,
                anchor_rx,
                anchor_ry,
                debug_units
            );
        }
    }

    let (
        overlay_atlas,
        bridge_atlas,
        bridge_railing_atlas,
        overlay_names,
        mut overlays_connected,
        overlay_radar_colors,
    ) = build_overlay_atlas_from_map(
        &map_data,
        &asset_manager,
        gpu,
        batch,
        theater_ext,
        &rules_ini,
        art.as_ref().unwrap_or(&art_fallback),
        overlay_iso_palette.as_ref(),
        unit_palette.as_ref(),
        overlay_tiberium_palette.as_ref(),
        rules.as_ref().map(|r| &r.smudge_types),
        bridge_railing_tile_bases,
    );

    // F09: the shared post-funnel finalization — spawner seed, map-wall owner
    // reconstruction, overlay-grid installation, smudge-grid seeding, and the
    // authoritative post-map tail — runs through the same sim-owned function
    // the headless loader uses.
    //
    // The overlay render entries are filtered BEFORE the call, against the
    // grid state the entries were derived from: the shared finalization's only
    // pre-install grid mutation is wall-owner reconstruction (wall_owner
    // fields, never overlay ids), while its post-map tail may place
    // scenario-start crates — new overlay occupancy this presentation filter
    // must not react to.
    overlays_connected
        .retain(|entry| overlay_grid.cell(entry.rx, entry.ry).overlay_id == Some(entry.overlay_id));
    let rules_for_post_map = rules
        .as_ref()
        .expect("merged rules were installed before post-map finalization");
    if let Some(sim) = &mut simulation {
        let output = crate::sim::runtime::finalize_constructed_scenario(
            sim,
            &map_data,
            rules_for_post_map,
            &overlay_registry,
            overlay_grid,
            &house_roster,
            match_launch_descriptor.as_ref(),
        );
        if let Some(stats) = output.tiberium_queues {
            log::info!(
                "Native tiberium queues rebuilt: {} growth entries, {} spread entries",
                stats.growth_entries,
                stats.spread_entries,
            );
        }
        if !output.navigation_published {
            log::error!("Initial navigation rebuild failed: resolved terrain is unavailable");
        }
    }

    // Anchor the opening view on the LOCAL player's start — retail opens a
    // skirmish looking at your own MCV, and anything else strands the player
    // staring at shroud. The local house's units are already spawned at the
    // assigned start slot by this point, so the MCV's actual cell is the
    // authoritative anchor. Falling back to the first multiplayer start
    // waypoint is wrong on any map with more than one start slot (it is some
    // OTHER player's corner unless you happened to draw slot one); it remains
    // only as the no-local-spawn fallback, then the middle of the playable
    // area for maps with no start waypoints at all.
    //
    // This is a **world point**, not a camera position: the sidebar's real width
    // depends on the UI scale, which only exists once `AppState` is built, so the
    // conversion to a camera top-left happens in `loading::transitions` where the
    // scaled layout spec is available.
    let local_start_cell: Option<(u16, u16)> = initial_local_owner
        .as_deref()
        .zip(simulation.as_ref())
        .and_then(|(owner, sim)| {
            let owner_id = sim.interner.get(owner)?;
            sim.substrate
                .entities
                .keys_sorted()
                .into_iter()
                .filter_map(|id| sim.substrate.entities.get(id))
                .find(|e| e.owner == owner_id)
                .map(|e| (e.position.rx, e.position.ry))
        });
    let (camera_anchor_x, camera_anchor_y): (f32, f32) = if let Some((rx, ry)) = local_start_cell {
        let z = height_map.get(&(rx, ry)).copied().unwrap_or(0);
        crate::app::input::camera::cell_centre_world_point(rx, ry, z)
    } else if let Some(start_wp) = waypoints::first_multiplayer_start(&map_data.waypoints) {
        let wp_z = height_map
            .get(&(start_wp.rx, start_wp.ry))
            .copied()
            .unwrap_or(0);
        crate::app::input::camera::cell_centre_world_point(start_wp.rx, start_wp.ry, wp_z)
    } else {
        let (area_x, area_y, area_w, area_h) = match local_bounds {
            Some(b) => (b.pixel_x, b.pixel_y, b.pixel_w, b.pixel_h),
            None => (
                grid.origin_x,
                grid.origin_y,
                grid.world_width,
                grid.world_height,
            ),
        };
        (area_x + area_w / 2.0, area_y + area_h / 2.0)
    };
    // Load cameo MIX archives so that *ICON.SHP files are findable.
    // These nested MIXes live inside local.mix/localmd.mix and aren't
    // auto-extracted by the two-level brute-force pass.
    for cameo_mix in ["cameomd.mix", "cameo.mix"] {
        match asset_manager.load_nested(cameo_mix) {
            Ok(()) => log::info!("Loaded nested {cameo_mix} for sidebar cameo icons"),
            Err(_) => log::debug!("{cameo_mix} not found (optional)"),
        }
    }
    let sidebar_cameo_atlas =
        build_sidebar_cameo_atlas(gpu, batch, &asset_manager, rules.as_ref(), art.as_ref());
    let sidebar_chrome =
        crate::render::sidebar_chrome::build_sidebar_chrome_set(gpu, batch, &asset_manager);
    let fnt_file = asset_manager.get_ref("GAME.FNT").and_then(|data| {
        crate::assets::fnt_file::FntFile::from_bytes(data)
            .map_err(|e| log::warn!("Failed to parse GAME.FNT: {e}"))
            .ok()
    });
    let software_cursor = cursor_atlas::build_software_cursor(gpu, batch, &asset_manager);
    if software_cursor.is_some() {
        log::info!("Software cursor loaded from mouse.sha — OS cursor will be hidden");
    } else {
        log::warn!("Software cursor NOT loaded (mouse.sha missing?) — using OS cursor");
    }
    let trigger_runtime = TriggerRuntime::from_map(&map_data.triggers, &map_data.local_variables);
    let lighting_grid = rebuild_lighting_grid_from_sim(
        &resolved_terrain,
        &lighting_config,
        simulation.as_ref(),
        rules.as_ref(),
        2,
    );
    // Final post-map-init milestones (cell attributes, beacon art, post-map
    // init, tactical cleanup, final pre-render refresh). 100 is emitted by the
    // pump at Finished.
    progress.milestone(82);
    progress.milestone(86);
    progress.milestone(90);
    progress.milestone(93);
    progress.milestone(96);
    progress.milestone(98);

    // Move fields out of map_data (last use) instead of cloning.
    let theater_name = map_data.header.theater;
    Ok(MapLoadResult {
        scenario: ScenarioLoadInputs {
            startup,
            map_source,
            map_hash,
            basic: map_data.basic,
            terrain_grid: Some(grid),
            resolved_terrain: Some(resolved_terrain),
            simulation,
            overlays: overlays_connected,
            terrain_objects: map_data.terrain_objects,
            waypoints: map_data.waypoints,
            cell_tags: map_data.cell_tags,
            tags: map_data.tags,
            triggers: map_data.triggers,
            events: map_data.events,
            actions: map_data.actions,
            trigger_graph: map_data.trigger_graph,
            trigger_runtime,
            overlay_registry,
            house_roster,
            height_map,
            bridge_height_map,
            tactical_bridge_inverse_map,
            rules,
            map_lighting_config: lighting_config,
            theater_name,
            theater_ext: theater_ext.to_string(),
            sandbox_full_visibility: false,
            spawn_pick_pending,
            initial_local_owner,
            camera_anchor_x,
            camera_anchor_y,
        },
        presentation: PresentationLoadAssets {
            tile_atlas,
            unit_atlas,
            palette_set,
            sprite_atlas,
            overlay_atlas,
            bridge_atlas,
            bridge_railing_atlas,
            sidebar_cameo_atlas,
            sidebar_chrome,
            software_cursor,
            overlay_names,
            overlay_radar_colors,
            house_color_map,
            lighting_grid,
            csf,
            fnt_file,
        },
        // The app loading job retains the one process-lifetime manager while
        // this borrowed phase runs, then moves it into the completed result.
        asset_manager: None,
    })
}

/// Register non-local playable houses as AI opponents.
#[cfg(test)]
mod random_map_retail_tests {
    //! Retail-input pass over the real `.SED` load path.
    //!
    //! This is the only place retail theater data meets the generator's
    //! emitted tile indices. The in-crate matrix cannot reach that clause at
    //! all: its synthetic tile-block stub answers every id, so resolvability
    //! is true there by construction rather than by generation being correct.
    //!
    //! It drives `load_map_initial_with_assets` — the production loop — instead
    //! of reassembling that function's resolution steps in test code. A
    //! mirrored sequence keeps passing after the production callsite drifts,
    //! which is exactly how every generated map shipped without a single
    //! neutral tech building while the phase itself was fully implemented and
    //! tested.
    //!
    //! Still not parity evidence. It shows the emitted surface is *loadable*,
    //! never that it matches the original.

    use super::*;
    use crate::map::entities::EntityCategory;
    use crate::map::rmg::RmgOptions;

    struct SilentProgress;
    impl crate::app::loading::pump::LoadingProgressSink for SilentProgress {
        fn milestone(&mut self, _percent: u32) {}
    }

    /// The configurations the dialog itself can emit.
    const RETAIL_MAP_TYPES: [i32; 4] = [1, 2, 3, 4];
    /// Comfortably above the river gate of 20, so the carved map types seed a
    /// lake *and* a river and both get their tiles resolved against retail
    /// theater data.
    const RETAIL_WATER_AMOUNT: i32 = 50;
    const RETAIL_THEATERS: [i32; 2] = [0, 1];

    /// A generated map must carry far more than this; the bar exists only to
    /// catch a run that emitted nothing real and would therefore pass the
    /// resolvability check without testing anything.
    const MIN_REAL_TILES_PER_MAP: usize = 1_000;

    fn retail_dir() -> Option<PathBuf> {
        let dir = PathBuf::from(std::env::var("RA2_DIR").ok()?);
        dir.is_dir().then_some(dir)
    }

    /// Write a `.SED` for `options` into a scratch dir, returning the dir the
    /// loader should read from and the seed file's name.
    ///
    /// The scratch dir is deliberately *not* the retail install: the loader
    /// takes `ra2_dir` and the asset manager as separate parameters, so the
    /// seed can come from a temp dir while the assets come from the real game.
    fn write_seed(options: &RmgOptions, tag: &str) -> (PathBuf, String) {
        let dir = std::env::temp_dir().join(format!("vera20k-rmg-retail-{tag}"));
        std::fs::create_dir_all(&dir).expect("scratch dir");
        let name = format!("{tag}.sed");
        std::fs::write(dir.join(&name), options.to_sed_bytes()).expect("write .SED");
        (dir, name)
    }

    #[test]
    #[ignore] // Requires RA2_DIR (retail game files).
    fn generated_maps_resolve_every_tile_against_retail_theaters() {
        let Some(ra2) = retail_dir() else {
            panic!("set RA2_DIR to the retail RA2/YR install directory");
        };

        let mut structures_placed = 0usize;
        let mut configurations = 0usize;
        for map_type in RETAIL_MAP_TYPES {
            for theater in RETAIL_THEATERS {
                configurations += 1;
                let tag = format!("t{map_type}-th{theater}");
                let options = RmgOptions {
                    map_type,
                    theater,
                    num_players: 4,
                    seed: 4242,
                    // Above the river gate, so the carved map types actually
                    // seed water. The default is zero, which skips their whole
                    // seeder — this pass validated neither lakes nor rivers
                    // until that was noticed.
                    water_amount: RETAIL_WATER_AMOUNT,
                    ..Default::default()
                };
                let (seed_dir, seed_name) = write_seed(&options, &tag);

                let mut asset_manager = AssetManager::new(&ra2).expect("AssetManager::new");
                let initial = load_map_initial_with_assets(
                    seed_dir,
                    &mut asset_manager,
                    Some(&seed_name),
                    &mut SilentProgress,
                )
                .unwrap_or_else(|err| panic!("{tag}: the .SED branch failed: {err}"));

                assert!(
                    !initial.map_data.cells.is_empty(),
                    "{tag}: the generated map has no cells"
                );

                let theater_name = crate::map::rmg::emit::theater_name(theater);
                let data = crate::map::theater::load_theater(&mut asset_manager, theater_name)
                    .unwrap_or_else(|| panic!("{tag}: theater {theater_name} unavailable"));

                // Guard against a vacuous pass: if every cell were NO_TILE the
                // resolvability filter below would empty and the assertion
                // would hold without ever having looked at a tile.
                let real_tiles: Vec<i32> = initial
                    .map_data
                    .cells
                    .iter()
                    .map(|cell| cell.tile_index)
                    .filter(|index| *index >= 0)
                    .collect();
                assert!(
                    real_tiles.len() > MIN_REAL_TILES_PER_MAP,
                    "{tag}: only {} cell(s) carry a real tile — the resolvability \
                     check below would be vacuous",
                    real_tiles.len()
                );

                // The completion-gate clause. `filename` returns None both for
                // out-of-range ids and for blank tileset slots, and either one
                // renders as a hole in the map, so both count as unresolved.
                let mut unresolved: Vec<i32> = real_tiles
                    .iter()
                    .copied()
                    .filter(|index| !data.lookup.filename(*index).is_some_and(|f| !f.is_empty()))
                    .collect();
                unresolved.sort_unstable();
                unresolved.dedup();
                let mut distinct = real_tiles.clone();
                distinct.sort_unstable();
                distinct.dedup();
                println!(
                    "{tag}: {} cells, {} real tiles ({} distinct), {} unresolved",
                    initial.map_data.cells.len(),
                    real_tiles.len(),
                    distinct.len(),
                    unresolved.len()
                );
                assert!(
                    unresolved.is_empty(),
                    "{tag}: {} distinct tile index(es) do not resolve against {theater_name}: {:?}",
                    unresolved.len(),
                    unresolved
                );

                structures_placed += initial
                    .map_data
                    .entities
                    .iter()
                    .filter(|entity| entity.category == EntityCategory::Structure)
                    .count();
            }
        }

        // The first check that the real `NeutralTechBuildings` catalog reaches
        // the generator through the app's own resolution rather than a test
        // stub. Asserted over the tier: how many a given map places is
        // configuration-dependent, but a starved catalog places none anywhere.
        assert!(
            structures_placed > 0,
            "no neutral tech building was placed on any retail configuration"
        );
        assert_eq!(
            configurations,
            RETAIL_MAP_TYPES.len() * RETAIL_THEATERS.len(),
            "the matrix did not visit every configuration"
        );
        println!(
            "{configurations} retail configurations checked, {structures_placed} neutral \
             structure(s) placed"
        );
    }
}
