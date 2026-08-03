//! Scenario-start crate placement — gamemd `ScenarioClass::Post_Map_Init` step 3.
//!
//! When the lobby "Crates" option is on (stock default), the scenario clamps the
//! player count between `[CrateRules] CrateMinimum` and `CrateMaximum` and calls
//! the map's random-cell crate placer exactly that many times. Each placer call
//! first looks for a free entry in the map's 256-entry crate-slot array and
//! returns immediately — spending no draws — if there is none. Otherwise it walks
//! a bounded retry loop; every attempt draws a random X then a random Y inside
//! the map's visible rectangle, snaps the result to a nearby passable cell, and
//! places the crate overlay there.
//!
//! The drawn cell decides the snap: a water cell is snapped with the *float*
//! speed type and takes `WaterCrateImg`, anything else uses the *track* speed
//! type and `CrateImg`.
//!
//! This module owns *placement only*. Crate contents, pickup effects and
//! `CrateRegen` respawn belong to the crate-effect system and are not modelled
//! here — a placed crate is an ordinary overlay cell until that lands.
//!
//! ## Dependency rules
//! Part of `sim/` — depends on `rules/`, `map/` grid types and other `sim/`
//! modules only. Never on render/, ui/, sidebar/, audio/, net/.

use std::collections::BTreeSet;

use crate::map::overlay_types::OverlayTypeRegistry;
use crate::rules::locomotor_type::{MovementZone, SpeedType};
use crate::rules::ruleset::{CrateRules, RuleSet};
use crate::sim::find_nearby_cell::{NearbyQuery, PassabilityArgs, find_nearby_passable_cell};
use crate::sim::pathfinding::PathGrid;
use crate::sim::world::Simulation;

/// Entries in the map's crate-slot array. The placer scans it for a free slot
/// before drawing anything, and gives up without a single draw when all 256 are
/// taken.
///
/// The array is a MapClass member, *not* derived from overlay contents. Whether
/// a crate the map preplaced in its overlay pack registers a slot is
/// **UNCHECKED**, so this pass counts only the crates it places itself — at
/// scenario start the array is empty, which is the only case this pass runs in.
const CRATE_SLOT_CAPACITY: usize = 256;

/// Retry budget inside one placer call. Every attempt spends its two draws
/// whether or not the cell works out; after this many failures the call gives up
/// and no crate is placed.
const MAX_PLACEMENT_ATTEMPTS: u32 = 1000;

/// Ring radius for the passable-cell snap.
///
/// VERA-internal: gamemd routes the drawn cell through its nearby-passable-cell
/// helper. The speed type it passes is verified (float over water, track
/// otherwise); the radius and the remaining filter arguments are UNCHECKED. A
/// drawn cell that is already passable snaps to itself, so this only shapes
/// where a crate lands when the draw hits a cliff or a building.
const CRATE_SNAP_RADIUS: u16 = 8;

/// Outcome of one scenario-start crate pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CratePlacement {
    /// Crates the clamp asked for (0 when the lobby option is off).
    pub requested: u32,
    /// Crates actually placed; lower than `requested` only when the placer ran
    /// out of retries or crate slots.
    pub placed: u32,
}

/// `min(max(CrateMinimum, player_count), CrateMaximum)`.
///
/// gamemd applies the floor first and the ceiling second, so a `CrateMaximum`
/// below `CrateMinimum` wins — the clamp is not symmetric and must not be
/// rewritten as a single `clamp()` call with the arguments in rules order.
pub fn scenario_start_crate_count(rules: &CrateRules, player_count: u32) -> u32 {
    rules.maximum.min(rules.minimum.max(player_count))
}

/// The human seat count the crate clamp is applied to.
///
/// gamemd's pregame setup assigns the crate-count global straight from the value
/// it logs as "Pregame setup for N players", and the sibling `Post_Map_Init`
/// budget gate adds the AI count to that same value — so it is human seats only.
/// In a 1v3 skirmish this is 1, not 4. Passive houses (Neutral, Special) are
/// never seats.
pub fn human_player_count(sim: &Simulation) -> u32 {
    sim.houses
        .values()
        .filter(|house| house.is_human && !house.multiplay_passive)
        .count() as u32
}

/// Place the scenario-start crates.
///
/// `player_count` is the lobby *human* player count. gamemd clamps a session
/// global that the pregame-setup routine assigns from the same value it logs as
/// "Pregame setup for N players", and the sibling budget gate in `Post_Map_Init`
/// adds the AI count to it separately — so it counts human seats only, not
/// human + AI. Callers pass VERA's human, non-passive house count.
///
/// ## RNG
/// Both coordinates are drawn from the scenario stream — the placer loads the
/// scenario instance pointer and adds `0x218` before each `RandomRanged` call,
/// the same member `Gather_Start_Positions` binds and the one VERA models as
/// `scenario_rng`. Two draws per attempt, X then Y, and the retry loop spends
/// them again on every retry.
pub fn place_scenario_start_crates(
    sim: &mut Simulation,
    rules: &RuleSet,
    overlay_registry: &OverlayTypeRegistry,
    path_grid: Option<&PathGrid>,
    player_count: u32,
) -> CratePlacement {
    if !sim.session.game_options.crates {
        return CratePlacement {
            requested: 0,
            placed: 0,
        };
    }
    let requested = scenario_start_crate_count(&rules.crate_rules, player_count);
    if requested == 0 {
        return CratePlacement {
            requested,
            placed: 0,
        };
    }
    let land_overlay_id = overlay_registry.id_for_name(&rules.crate_rules.crate_img);
    let water_overlay_id = overlay_registry.id_for_name(&rules.crate_rules.water_crate_img);
    if land_overlay_id.is_none() {
        log::warn!(
            "No overlay type named '{}' ([CrateRules] CrateImg) — placing no crates",
            rules.crate_rules.crate_img
        );
        return CratePlacement {
            requested,
            placed: 0,
        };
    }
    if sim.overlay_grid.is_none() {
        log::warn!("No overlay grid — placing no scenario-start crates");
        return CratePlacement {
            requested,
            placed: 0,
        };
    }

    let mut chosen: Vec<((u16, u16), CrateSurface)> = Vec::new();
    let mut taken: BTreeSet<(u16, u16)> = BTreeSet::new();

    for _ in 0..requested {
        if chosen.len() >= CRATE_SLOT_CAPACITY {
            // No free crate slot: the native placer returns before drawing.
            break;
        }
        let Some(placement) = draw_one_crate_cell(sim, path_grid, &taken) else {
            continue;
        };
        taken.insert(placement.0);
        chosen.push(placement);
    }

    let placed = chosen.len() as u32;
    if let Some(grid) = sim.overlay_grid.as_mut() {
        for (cell, surface) in chosen {
            let overlay_id = match surface {
                // A water draw with no WaterCrateImg overlay type falls back to
                // the land image rather than dropping the crate.
                CrateSurface::Water => water_overlay_id.or(land_overlay_id),
                CrateSurface::Land => land_overlay_id,
            };
            if let Some(overlay_id) = overlay_id {
                // Overlay data byte for a fresh crate is UNCHECKED; 0 is the
                // first frame of the crate SHP.
                grid.place_overlay(cell.0, cell.1, overlay_id, 0);
            }
        }
    }
    log::info!("Scenario-start crates: requested {requested}, placed {placed}");
    CratePlacement { requested, placed }
}

/// Which crate image a drawn cell earns, and which speed type snapped it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CrateSurface {
    Land,
    Water,
}

/// One placer call: up to [`MAX_PLACEMENT_ATTEMPTS`] attempts, each spending
/// exactly two draws (X then Y) on the scenario stream before the cell is judged.
fn draw_one_crate_cell(
    sim: &mut Simulation,
    path_grid: Option<&PathGrid>,
    taken: &BTreeSet<(u16, u16)>,
) -> Option<((u16, u16), CrateSurface)> {
    // The draw rectangle is the map's visible rect — the same four globals
    // `Gather_Start_Positions` reads, which VERA resolves to `[Map] LocalSize`.
    let (left, top, width, height) = crate_draw_rect(sim);
    if width == 0 || height == 0 {
        return None;
    }

    for _ in 0..MAX_PLACEMENT_ATTEMPTS {
        // Draw order is load-bearing: X (width, then + left) first, then Y
        // (height, then + top), both inclusive of the far edge, exactly as the
        // native placer spends its two draws.
        let x = sim
            .scenario_rng
            .next_range_u32_inclusive(0, u32::from(width - 1)) as u16
            + left;
        let y = sim
            .scenario_rng
            .next_range_u32_inclusive(0, u32::from(height - 1)) as u16
            + top;

        // The drawn cell's own land type selects the snap, before any snapping
        // happens. A draw outside the grid takes the land branch, matching the
        // native fallback cell.
        let surface = if sim
            .resolved_terrain
            .as_ref()
            .and_then(|terrain| terrain.cell(x, y))
            .is_some_and(|cell| cell.is_water)
        {
            CrateSurface::Water
        } else {
            CrateSurface::Land
        };

        let Some(cell) = snap_to_passable(sim, path_grid, (x, y), surface) else {
            continue;
        };
        if taken.contains(&cell) {
            continue;
        }
        // A cell that already carries ore, a wall, a bridge piece or another
        // crate cannot take one.
        if sim
            .overlay_grid
            .as_ref()
            .is_some_and(|grid| grid.cell(cell.0, cell.1).overlay_id.is_some())
        {
            continue;
        }
        return Some((cell, surface));
    }
    None
}

/// The rectangle the two draws are taken from.
///
/// gamemd reads a left/top/width/height quad that `Gather_Start_Positions` also
/// reads; VERA already resolves that quad to `[Map] LocalSize` for the
/// start-position path, so crates use the same session fields, with the same
/// full-grid fallback when the map declares no LocalSize.
fn crate_draw_rect(sim: &Simulation) -> (u16, u16, u16, u16) {
    if sim.session.local_width != 0 && sim.session.local_height != 0 {
        (
            sim.session.local_left,
            sim.session.local_top,
            sim.session.local_width,
            sim.session.local_height,
        )
    } else {
        (0, 0, sim.session.map_width, sim.session.map_height)
    }
}

/// Snap a drawn cell onto a nearby passable cell of the matching surface.
fn snap_to_passable(
    sim: &Simulation,
    path_grid: Option<&PathGrid>,
    drawn: (u16, u16),
    surface: CrateSurface,
) -> Option<(u16, u16)> {
    let query = NearbyQuery {
        passability: PassabilityArgs {
            // Verified: the placer passes native speed type 5 (float) when the
            // drawn cell is water and 1 (track) otherwise.
            speed_type: match surface {
                CrateSurface::Water => SpeedType::Float,
                CrateSurface::Land => SpeedType::Track,
            },
            required_zone_id: None,
            movement_zone: MovementZone::Normal,
            bridge_aware_zone: false,
        },
        allow_bridge_cells: false,
        check_height: false,
        check_occupancy: true,
        radius_cap: CRATE_SNAP_RADIUS,
        // Nearest-to-the-draw selection keeps the snap deterministic without
        // reading the frame counter, which is 0 for every call at load.
        target_cell: Some((i32::from(drawn.0), i32::from(drawn.1))),
        path_grid,
        resolved_terrain: sim.resolved_terrain.as_ref(),
        overlay_grid: sim.overlay_grid.as_ref(),
        occupancy: Some(&sim.substrate.occupancy),
        entities: Some(&sim.substrate.entities),
        zone_grid: sim.zone_grid.as_ref(),
        map_size: Some((sim.session.map_width, sim.session.map_height)),
        playfield_bounds: sim.playfield_bounds,
    };
    find_nearby_passable_cell(
        (i32::from(drawn.0), i32::from(drawn.1)),
        &query,
        sim.session.binary_frame,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::overlay_types::OverlayTypeRegistry;
    use crate::rules::ini_parser::IniFile;
    use crate::sim::overlay_grid::OverlayGrid;
    use crate::sim::pathfinding::PathGrid;

    const MAP: u16 = 40;

    fn crate_registry() -> OverlayTypeRegistry {
        let ini = IniFile::from_str(
            "[OverlayTypes]\n0=TIB01\n1=CRATE\n2=WCRATE\n\
             [TIB01]\nTiberium=yes\n[CRATE]\n[WCRATE]\n",
        );
        OverlayTypeRegistry::from_ini(&ini, None)
    }

    fn crate_ruleset(extra: &str) -> RuleSet {
        let ini = IniFile::from_str(&format!(
            "[InfantryTypes]\n[VehicleTypes]\n[AircraftTypes]\n[BuildingTypes]\n\
             [CrateRules]\nCrateImg=CRATE\nWaterCrateImg=WCRATE\n{extra}",
        ));
        RuleSet::from_ini(&ini).expect("rules")
    }

    /// `seed` positions the scenario cursor; the placer draws from there.
    fn sim_with_grid(seed: u64) -> Simulation {
        let mut sim = Simulation::new();
        sim.session.seed = seed;
        sim.scenario_rng = crate::sim::rng::SimRng::new(seed);
        sim.session.map_width = MAP;
        sim.session.map_height = MAP;
        sim.session.game_options.crates = true;
        sim.overlay_grid = Some(OverlayGrid::new(MAP, MAP));
        sim
    }

    fn cells_with_overlay(
        sim: &Simulation,
        registry: &OverlayTypeRegistry,
        name: &str,
    ) -> Vec<(u16, u16)> {
        let id = registry.id_for_name(name).expect("overlay type");
        let grid = sim.overlay_grid.as_ref().expect("overlay grid");
        let mut cells = Vec::new();
        for ry in 0..grid.height() {
            for rx in 0..grid.width() {
                if grid.cell(rx, ry).overlay_id == Some(id) {
                    cells.push((rx, ry));
                }
            }
        }
        cells
    }

    fn crate_cells(sim: &Simulation, registry: &OverlayTypeRegistry) -> Vec<(u16, u16)> {
        cells_with_overlay(sim, registry, "CRATE")
    }

    #[test]
    fn crate_count_applies_the_minimum_floor_then_the_maximum_ceiling() {
        let rules = CrateRules {
            minimum: 1,
            maximum: 255,
            ..CrateRules::default()
        };
        // Stock shape: one crate per player, never below CrateMinimum.
        assert_eq!(scenario_start_crate_count(&rules, 0), 1);
        assert_eq!(scenario_start_crate_count(&rules, 1), 1);
        assert_eq!(scenario_start_crate_count(&rules, 4), 4);
        assert_eq!(scenario_start_crate_count(&rules, 900), 255);

        let floored = CrateRules {
            minimum: 6,
            maximum: 255,
            ..CrateRules::default()
        };
        assert_eq!(scenario_start_crate_count(&floored, 2), 6);

        // The ceiling is applied after the floor, so a maximum under the
        // minimum wins outright.
        let inverted = CrateRules {
            minimum: 10,
            maximum: 3,
            ..CrateRules::default()
        };
        assert_eq!(scenario_start_crate_count(&inverted, 1), 3);
    }

    #[test]
    fn crate_rules_read_the_stock_section_keys() {
        let rules = crate_ruleset("CrateMinimum=2\nCrateMaximum=9\n");
        assert_eq!(rules.crate_rules.minimum, 2);
        assert_eq!(rules.crate_rules.maximum, 9);
        assert_eq!(rules.crate_rules.crate_img, "CRATE");
        assert_eq!(rules.crate_rules.water_crate_img, "WCRATE");

        let defaults = crate_ruleset("");
        assert_eq!(defaults.crate_rules.minimum, 1);
        assert_eq!(defaults.crate_rules.maximum, 255);
    }

    #[test]
    fn placement_respects_the_clamped_count_and_lands_on_distinct_free_cells() {
        let rules = crate_ruleset("CrateMinimum=1\nCrateMaximum=3\n");
        let registry = crate_registry();
        let grid = PathGrid::test_all_passable(MAP, MAP);
        let mut sim = sim_with_grid(0x1234_5678);

        // Five players, ceiling 3.
        let result = place_scenario_start_crates(&mut sim, &rules, &registry, Some(&grid), 5);

        assert_eq!(result.requested, 3);
        assert_eq!(result.placed, 3);
        let cells = crate_cells(&sim, &registry);
        assert_eq!(cells.len(), 3);
        let unique: BTreeSet<(u16, u16)> = cells.iter().copied().collect();
        assert_eq!(unique.len(), 3, "no two crates share a cell");
        for (rx, ry) in cells {
            assert!(rx < MAP && ry < MAP, "crate inside the map rectangle");
        }
    }

    #[test]
    fn placement_floor_lifts_a_single_player_match_to_crate_minimum() {
        let rules = crate_ruleset("CrateMinimum=4\nCrateMaximum=255\n");
        let registry = crate_registry();
        let grid = PathGrid::test_all_passable(MAP, MAP);
        let mut sim = sim_with_grid(99);

        let result = place_scenario_start_crates(&mut sim, &rules, &registry, Some(&grid), 1);

        assert_eq!(result.requested, 4);
        assert_eq!(result.placed, 4);
        assert_eq!(crate_cells(&sim, &registry).len(), 4);
    }

    #[test]
    fn crates_off_places_nothing() {
        let rules = crate_ruleset("CrateMinimum=1\nCrateMaximum=255\n");
        let registry = crate_registry();
        let grid = PathGrid::test_all_passable(MAP, MAP);
        let mut sim = sim_with_grid(7);
        sim.session.game_options.crates = false;

        let result = place_scenario_start_crates(&mut sim, &rules, &registry, Some(&grid), 4);

        assert_eq!(
            result,
            CratePlacement {
                requested: 0,
                placed: 0
            }
        );
        assert!(crate_cells(&sim, &registry).is_empty());
    }

    /// Native spends exactly two scenario-stream draws per attempt, X then Y,
    /// and the cursor it leaves behind is part of the match state every peer
    /// must agree on.
    #[test]
    fn placement_spends_two_scenario_draws_per_crate_and_is_cursor_deterministic() {
        let rules = crate_ruleset("CrateMinimum=3\nCrateMaximum=255\n");
        let registry = crate_registry();
        let grid = PathGrid::test_all_passable(MAP, MAP);

        let mut first = sim_with_grid(0xABCD);
        let mut replay = first.scenario_rng.clone();
        place_scenario_start_crates(&mut first, &rules, &registry, Some(&grid), 3);

        // Replay the exact draw sequence the placer must have spent: three
        // crates, each one attempt, each attempt X then Y over the draw rect.
        for _ in 0..3 {
            replay.next_range_u32_inclusive(0, u32::from(MAP - 1));
            replay.next_range_u32_inclusive(0, u32::from(MAP - 1));
        }
        assert_eq!(
            first.scenario_rng.state(),
            replay.state(),
            "placement must advance the scenario cursor by exactly two draws per crate"
        );

        let mut same_cursor = sim_with_grid(0xABCD);
        place_scenario_start_crates(&mut same_cursor, &rules, &registry, Some(&grid), 3);
        let mut other_cursor = sim_with_grid(0x1111);
        place_scenario_start_crates(&mut other_cursor, &rules, &registry, Some(&grid), 3);

        assert_eq!(
            crate_cells(&first, &registry),
            crate_cells(&same_cursor, &registry),
            "the same scenario cursor must scatter crates identically"
        );
        assert_ne!(
            crate_cells(&first, &registry),
            crate_cells(&other_cursor, &registry),
            "a different scenario cursor must scatter crates differently"
        );
    }

    /// The drawn cell's own land type picks the crate image, before any snap:
    /// water takes `WaterCrateImg`, everything else takes `CrateImg`.
    #[test]
    fn a_water_draw_places_the_water_crate_image() {
        let rules = crate_ruleset("CrateMinimum=4\nCrateMaximum=255\n");
        let registry = crate_registry();
        let grid = PathGrid::test_all_passable(MAP, MAP);

        for water in [false, true] {
            let mut sim = sim_with_grid(0x5150);
            sim.resolved_terrain = Some(uniform_terrain(water));

            let result = place_scenario_start_crates(&mut sim, &rules, &registry, Some(&grid), 4);
            assert_eq!(result.placed, 4);

            let land_crates = cells_with_overlay(&sim, &registry, "CRATE");
            let water_crates = cells_with_overlay(&sim, &registry, "WCRATE");
            if water {
                assert_eq!(water_crates.len(), 4, "water draws take WaterCrateImg");
                assert!(land_crates.is_empty());
            } else {
                assert_eq!(land_crates.len(), 4, "land draws take CrateImg");
                assert!(water_crates.is_empty());
            }
        }
    }

    /// Every cell the same land type, every speed type passable, so the only
    /// thing the water flag can change is which crate image is chosen.
    fn uniform_terrain(water: bool) -> crate::map::resolved_terrain::ResolvedTerrainGrid {
        use crate::map::resolved_terrain::{ResolvedTerrainCell, ResolvedTerrainGrid, zone_class};
        use crate::rules::terrain_rules::{LandType, SpeedCostProfile, TerrainClass};

        let speed_costs = SpeedCostProfile {
            foot: Some(100),
            track: Some(100),
            wheel: Some(100),
            float: Some(100),
            amphibious: Some(100),
            float_beach: Some(100),
            hover: Some(100),
        };
        let land_type = if water {
            LandType::Water.as_index()
        } else {
            LandType::Clear.as_index()
        };
        let template = ResolvedTerrainCell {
            rx: 0,
            ry: 0,
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
            is_water: water,
            is_cliff_like: false,
            is_rough: false,
            is_road: false,
            accepts_smudge: false,
            allows_tiberium: true,
            is_cliff_redraw: false,
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
            bridge_facts: crate::map::bridge_facts::BridgeCellFacts::default(),
            tube_index: None,
            radar_left: [0; 3],
            radar_right: [0; 3],
            has_damaged_data: false,
            bridgehead_anchor_class_at_load: None,
        };
        let mut cells = Vec::with_capacity(MAP as usize * MAP as usize);
        for ry in 0..MAP {
            for rx in 0..MAP {
                let mut cell = template.clone();
                cell.rx = rx;
                cell.ry = ry;
                cells.push(cell);
            }
        }
        ResolvedTerrainGrid::from_cells(MAP, MAP, cells)
    }

    /// The count the loader feeds the clamp: human seats only. A 1v3 skirmish
    /// asks for one crate, not four.
    #[test]
    fn human_player_count_excludes_ai_and_passive_houses() {
        use crate::sim::house_state::HouseState;

        let mut sim = Simulation::new();
        let mut add = |name: &str, is_human: bool, passive: bool| {
            let id = sim.interner.intern(name);
            let mut house = HouseState::new(id, 0, None, is_human, 10_000, 10);
            house.multiplay_passive = passive;
            sim.houses.insert(id, house);
        };
        add("Local", true, false);
        add("Computer1", false, false);
        add("Computer2", false, false);
        add("Computer3", false, false);
        add("Neutral", false, true);
        add("Special", false, true);

        assert_eq!(human_player_count(&sim), 1);
    }

    /// The draw rect is `[Map] LocalSize`, so no crate can land in the border
    /// strip outside it.
    #[test]
    fn draws_come_from_the_localsize_rect_not_the_full_cell_array() {
        let rules = crate_ruleset("CrateMinimum=12\nCrateMaximum=255\n");
        let registry = crate_registry();
        let grid = PathGrid::test_all_passable(MAP, MAP);
        let mut sim = sim_with_grid(0x2468);
        sim.session.local_left = 10;
        sim.session.local_top = 12;
        sim.session.local_width = 6;
        sim.session.local_height = 5;

        let result = place_scenario_start_crates(&mut sim, &rules, &registry, Some(&grid), 12);

        assert_eq!(result.requested, 12);
        assert!(result.placed > 0);
        for (rx, ry) in crate_cells(&sim, &registry) {
            assert!(
                (10..16).contains(&rx) && (12..17).contains(&ry),
                "crate at ({rx},{ry}) escaped the LocalSize rect"
            );
        }
    }
}
