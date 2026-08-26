//! Authoritative scenario initialization after map objects are installed.
//!
//! This owns the `ScenarioClass::Post_Map_Init @ 0x00686890` tail that must
//! complete before tick 0: native tiberium queue construction, navigation
//! publication, the skirmish AI credit grant, scenario-start crate RNG, and
//! the final HouseClass alliance pass. The app submits immutable map/session
//! inputs and consumes only the receipt.

use std::collections::BTreeSet;

use crate::map::basic::{BasicSection, SpecialFlagsSection};
use crate::map::houses::HouseRoster;
use crate::map::overlay_types::OverlayTypeRegistry;
use crate::rules::ruleset::RuleSet;
use crate::sim::crates::CratePlacement;
use crate::sim::ore_growth::NativeTiberiumRebuildStats;
use crate::sim::world::Simulation;
#[cfg(test)]
use crate::skirmish_launch::SkirmishLaunchSession;

/// Immutable inputs for the one-shot post-map scenario command.
pub(crate) struct ScenarioPostMapInput<'a> {
    pub(crate) map_width: u16,
    pub(crate) map_height: u16,
    pub(crate) basic: &'a BasicSection,
    pub(crate) special_flags: &'a SpecialFlagsSection,
    pub(crate) rules: &'a RuleSet,
    pub(crate) overlay_registry: &'a OverlayTypeRegistry,
    pub(crate) house_roster: &'a HouseRoster,
    pub(crate) skirmish_session: Option<&'a crate::sim::scenario_bootstrap::MatchLaunchDescriptor>,
}

/// Presentation/logging facts returned after authoritative initialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ScenarioPostMapOutput {
    pub(crate) tiberium_queues: Option<NativeTiberiumRebuildStats>,
    pub(crate) navigation_published: bool,
    pub(crate) crates: Option<CratePlacement>,
}

impl Simulation {
    /// Commit the post-map authority cone in native-observable order.
    ///
    /// This is a fresh-load-only command and is intentionally not idempotent:
    /// a second call would repeat the AI grant, crate RNG, and ore scheduler reset.
    /// Snapshot restore uses its narrower map-authority reconstruction path.
    pub(crate) fn finalize_scenario_post_map(
        &mut self,
        input: ScenarioPostMapInput<'_>,
    ) -> ScenarioPostMapOutput {
        self.production.ore_growth_config = crate::sim::ore_growth::OreGrowthConfig::from_ini(
            &input.rules.general,
            input.basic,
            input.special_flags,
        );
        self.production.ore_growth_state =
            crate::sim::ore_growth::OreGrowthState::new(input.map_width, input.map_height);

        let tiberium_queues = if let Some(overlay_grid) = self.overlay_grid.as_ref() {
            let source_object_cells: BTreeSet<(u16, u16)> = self
                .production
                .terrain_object_cells
                .keys()
                .copied()
                .collect();
            Some(
                self.production
                    .ore_growth_state
                    .rebuild_native_tiberium_queues_from_overlays(
                        overlay_grid,
                        input.overlay_registry,
                        &input.rules.tiberium_types,
                        self.resolved_terrain.as_ref(),
                        &source_object_cells,
                        input.basic.tiberium_growth_enabled.unwrap_or(true),
                        input.rules.general.tiberium_spreads
                            && input.special_flags.tiberium_spreads.unwrap_or(true),
                        self.session.binary_frame,
                    ),
            )
        } else {
            self.production
                .ore_growth_state
                .reset_native_tiberium_classes(0, self.session.binary_frame);
            None
        };

        // Runtime rebuilds use this same sim-owned publication seam. Crate
        // placement below pins the newly published path snapshot.
        let navigation_published = self.rebuild_dynamic_navigation(input.rules);

        let crates = if let Some(descriptor) = input.skirmish_session {
            let session = descriptor.session();
            crate::sim::scenario_bootstrap::apply_skirmish_ai_opening_credits(self);
            let player_count = crate::sim::crates::human_player_count(self);
            let initial_path = self.path_grid_snapshot();
            let placement = crate::sim::crates::place_scenario_start_crates(
                self,
                input.rules,
                input.overlay_registry,
                initial_path.as_deref(),
                player_count,
            );
            crate::sim::scenario_bootstrap::apply_skirmish_launch_alliances(
                self,
                input.house_roster,
                session,
            );
            Some(placement)
        } else {
            self.house_alliances = input.house_roster.alliance_map();
            None
        };

        ScenarioPostMapOutput {
            tiberium_queues,
            navigation_published,
            crates,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::BTreeSet;

    use crate::map::bridge_facts::BridgeCellFacts;
    use crate::map::houses::HouseDefinition;
    use crate::map::resolved_terrain::{ResolvedTerrainCell, ResolvedTerrainGrid, zone_class};
    use crate::rules::house_colors::HouseColorIndex;
    use crate::rules::ini_parser::IniFile;
    use crate::rules::terrain_rules::{LandType, SpeedCostProfile, TerrainClass};
    use crate::sim::ai::AiPlayerState;
    use crate::sim::house_state::HouseState;
    use crate::sim::overlay_grid::OverlayGrid;
    use crate::skirmish_launch::{
        AiDifficulty, LaunchCountry, LaunchStartPosition, LaunchTeam, SkirmishAiSlot,
        SkirmishLaunchMode, SkirmishLaunchOptions, SkirmishLocalSlot,
    };

    const MAP_SIZE: u16 = 8;

    fn post_map_rules_and_overlays() -> (RuleSet, OverlayTypeRegistry) {
        let ini = IniFile::from_str(
            "[General]\n\
             TiberiumGrows=yes\n\
             TiberiumSpreads=yes\n\
             [InfantryTypes]\n\
             [VehicleTypes]\n\
             [AircraftTypes]\n\
             [BuildingTypes]\n\
             [Tiberiums]\n\
             0=Riparius\n\
             [Riparius]\n\
             Image=1\n\
             Growth=2200\n\
             GrowthPercentage=.06\n\
             Spread=2200\n\
             SpreadPercentage=.06\n\
             [OverlayTypes]\n\
             0=TIBCELL\n\
             1=WOOD\n\
             2=WATER\n\
             [TIBCELL]\n\
             Tiberium=yes\n\
             [WOOD]\n\
             Crate=yes\n\
             [WATER]\n\
             Crate=yes\n\
             [CrateRules]\n\
             CrateImg=WOOD\n\
             WoodCrateImg=WOOD\n\
             WaterCrateImg=WATER\n\
             CrateMinimum=1\n\
             CrateMaximum=1\n",
        );
        (
            RuleSet::from_ini(&ini).expect("post-map rules"),
            OverlayTypeRegistry::from_ini(&ini, None),
        )
    }

    fn flat_terrain() -> ResolvedTerrainGrid {
        let land_type = LandType::Clear.as_index();
        let speed_costs = SpeedCostProfile {
            foot: Some(100),
            track: Some(100),
            wheel: Some(100),
            float: Some(100),
            amphibious: Some(100),
            float_beach: Some(100),
            hover: Some(100),
        };
        let mut cells = Vec::with_capacity(MAP_SIZE as usize * MAP_SIZE as usize);
        for ry in 0..MAP_SIZE {
            for rx in 0..MAP_SIZE {
                cells.push(ResolvedTerrainCell {
                    rx,
                    ry,
                    source_tile_index: 0,
                    source_sub_tile: 0,
                    final_tile_index: 0,
                    final_sub_tile: 0,
                    is_wood_bridge_repair_tile: false,
                    level: 0,
                    filled_clear: false,
                    tileset_index: Some(0),
                    land_type,
                    yr_cell_land_type: land_type,
                    slope_type: 0,
                    template_height: 0,
                    render_offset_x: 0,
                    render_offset_y: 0,
                    terrain_class: TerrainClass::Clear,
                    speed_costs,
                    is_water: false,
                    is_cliff_like: false,
                    is_rough: false,
                    is_road: false,
                    accepts_smudge: true,
                    allows_tiberium: true,
                    height_in_pixels: 0,
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
                    base_land_type: land_type,
                    base_yr_cell_land_type: land_type,
                    base_terrain_class: TerrainClass::Clear,
                    base_speed_costs: speed_costs,
                    build_blocked: false,
                    has_bridge_deck: false,
                    bridge_walkable: false,
                    bridge_transition: false,
                    bridge_deck_level: 0,
                    bridge_layer: None,
                    bridge_facts: BridgeCellFacts::default(),
                    tube_index: None,
                    radar_left: [0; 3],
                    radar_right: [0; 3],
                    has_damaged_data: false,
                    bridgehead_anchor_class_at_load: None,
                });
            }
        }
        ResolvedTerrainGrid::from_cells(MAP_SIZE, MAP_SIZE, cells)
    }

    fn allied_skirmish_session() -> SkirmishLaunchSession {
        SkirmishLaunchSession {
            mode: SkirmishLaunchMode {
                id: 1,
                ui_name_key: "GUI:Battle".to_string(),
                tooltip_key: "STT:ModeBattle".to_string(),
                override_file: "MPBattleMD.ini".to_string(),
                map_filter: "standard".to_string(),
                random_maps_allowed: true,
                allies_allowed: true,
                must_ally: false,
            },
            selected_map_file: Some("post-map-test.mmx".to_string()),
            player_name: "Player".to_string(),
            local: SkirmishLocalSlot {
                country: LaunchCountry::America,
                country_random: false,
                color_index: 1,
                color_random: false,
                start_position: LaunchStartPosition::Position(1),
                team: LaunchTeam::Team(0),
            },
            opponents: vec![SkirmishAiSlot {
                country: LaunchCountry::Russia,
                country_random: false,
                color_index: 2,
                color_random: false,
                start_position: LaunchStartPosition::Position(2),
                team: LaunchTeam::Team(0),
                difficulty: AiDifficulty::Easy,
            }],
            options: SkirmishLaunchOptions::default(),
        }
    }

    #[test]
    fn skirmish_post_map_finalizes_authority_in_one_call() {
        let (rules, overlays) = post_map_rules_and_overlays();
        let mut sim = Simulation::with_seed(0x51C0_0401);
        sim.session.map_width = MAP_SIZE;
        sim.session.map_height = MAP_SIZE;
        // The app installs normalized MapClass bounds before this production
        // post-map command. Model that prerequisite instead of relying on the
        // now-removed permissive headless fallback.
        sim.playfield_bounds = Some(crate::sim::cell_rect::PlayfieldBounds {
            base: 0,
            off_fc: -32,
            off_100: -32,
            off_104: 64,
            off_108: 64,
        });
        sim.session.game_options.crates = true;
        sim.resolved_terrain = Some(flat_terrain());
        sim.overlay_grid = Some(OverlayGrid::new(MAP_SIZE, MAP_SIZE));
        let tiberium = overlays.id_for_name("TIBCELL").expect("TIBCELL overlay");
        sim.overlay_grid
            .as_mut()
            .expect("overlay grid")
            .place_overlay(0, 0, tiberium, 10);

        let player = sim.interner.intern("Player");
        let computer = sim.interner.intern("Computer1");
        sim.houses
            .insert(player, HouseState::new(player, 0, None, true, 5_000, 10));
        sim.houses.insert(
            computer,
            HouseState::new(computer, 1, None, false, 5_000, 10),
        );
        sim.ai_players.push(AiPlayerState::new(computer));

        let mut expected_rng = sim.scenario_rng.clone();
        let expected_crate_cell = (
            expected_rng.next_range_u32_inclusive(1, u32::from(MAP_SIZE - 1)) as u16,
            expected_rng.next_range_u32_inclusive(1, u32::from(MAP_SIZE - 1)) as u16,
        );
        let _ = expected_rng.next_range_u32_inclusive(0, 0x7fff_fffe);
        let basic = BasicSection::default();
        let special_flags = SpecialFlagsSection::default();
        let roster = HouseRoster::default();
        let descriptor = crate::sim::scenario_bootstrap::MatchLaunchDescriptor::from_resolved(
            allied_skirmish_session(),
        )
        .expect("fixture session is fully resolved");

        let output = sim.finalize_scenario_post_map(ScenarioPostMapInput {
            map_width: MAP_SIZE,
            map_height: MAP_SIZE,
            basic: &basic,
            special_flags: &special_flags,
            rules: &rules,
            overlay_registry: &overlays,
            house_roster: &roster,
            skirmish_session: Some(&descriptor),
        });

        assert_eq!(
            output.tiberium_queues,
            Some(NativeTiberiumRebuildStats {
                growth_entries: 1,
                spread_entries: 1,
            })
        );
        let native = sim.production.ore_growth_state.native_tiberium_state();
        assert_eq!(native.classes.len(), 1);
        assert_eq!(native.classes[0].growth_bitmap, BTreeSet::from([(0, 0)]));
        assert_eq!(native.classes[0].spread_bitmap, BTreeSet::from([(0, 0)]));
        assert!(output.navigation_published);
        assert!(sim.path_grid().is_some());
        assert_eq!(sim.houses[&player].credits, 5_000);
        assert_eq!(sim.houses[&computer].credits, 10_000);
        assert_eq!(
            output.crates,
            Some(CratePlacement {
                requested: 1,
                placed: 1,
            })
        );
        let wood = overlays.id_for_name("WOOD").expect("WOOD overlay");
        let crate_cells: Vec<_> = sim
            .overlay_grid
            .as_ref()
            .expect("overlay grid")
            .iter_occupied()
            .filter_map(|(rx, ry, cell)| (cell.overlay_id == Some(wood)).then_some((rx, ry)))
            .collect();
        assert_eq!(crate_cells, vec![expected_crate_cell]);
        assert_eq!(sim.scenario_rng.state(), expected_rng.state());
        assert!(
            sim.house_alliances
                .get("PLAYER")
                .is_some_and(|allies| allies.contains("COMPUTER1"))
        );
        assert!(
            sim.house_alliances
                .get("COMPUTER1")
                .is_some_and(|allies| allies.contains("PLAYER"))
        );
    }

    #[test]
    fn generic_post_map_preserves_rng_and_credits_when_overlay_authority_is_absent() {
        let (rules, overlays) = post_map_rules_and_overlays();
        let mut sim = Simulation::with_seed(0x51C0_0402);
        sim.session.map_width = MAP_SIZE;
        sim.session.map_height = MAP_SIZE;
        sim.resolved_terrain = Some(flat_terrain());

        let owner = sim.interner.intern("HouseA");
        sim.houses
            .insert(owner, HouseState::new(owner, 0, None, false, 7_500, 10));
        sim.ai_players.push(AiPlayerState::new(owner));
        let rng_before = sim.scenario_rng.state();
        let roster = HouseRoster {
            houses: vec![
                HouseDefinition {
                    name: "HouseA".to_string(),
                    color: HouseColorIndex(0),
                    country: None,
                    side: None,
                    player_control: Some(false),
                    iq: None,
                    allies: vec!["HouseB".to_string()],
                    base_plan: Default::default(),
                },
                HouseDefinition {
                    name: "HouseB".to_string(),
                    color: HouseColorIndex(1),
                    country: None,
                    side: None,
                    player_control: Some(true),
                    iq: None,
                    allies: Vec::new(),
                    base_plan: Default::default(),
                },
            ],
        };

        let output = sim.finalize_scenario_post_map(ScenarioPostMapInput {
            map_width: MAP_SIZE,
            map_height: MAP_SIZE,
            basic: &BasicSection::default(),
            special_flags: &SpecialFlagsSection::default(),
            rules: &rules,
            overlay_registry: &overlays,
            house_roster: &roster,
            skirmish_session: None,
        });

        assert_eq!(output.tiberium_queues, None);
        assert!(output.navigation_published);
        assert_eq!(output.crates, None);
        assert_eq!(sim.scenario_rng.state(), rng_before);
        assert_eq!(sim.houses[&owner].credits, 7_500);
        assert!(
            sim.production
                .ore_growth_state
                .native_tiberium_state()
                .classes
                .is_empty()
        );
        assert!(
            sim.house_alliances
                .get("HOUSEA")
                .is_some_and(|allies| allies.contains("HOUSEB"))
        );
        assert!(
            sim.house_alliances
                .get("HOUSEB")
                .is_some_and(|allies| allies.contains("HOUSEA"))
        );
    }
}
