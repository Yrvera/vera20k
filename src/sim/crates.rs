//! Scenario-start crate placement — gamemd `ScenarioClass::Post_Map_Init` step 3.
//!
//! When the lobby "Crates" option is on (stock default), the scenario clamps the
//! player count between `[CrateRules] CrateMinimum` and `CrateMaximum` and calls
//! the map's random-cell crate placer exactly that many times. Each placer call
//! first looks for a free entry in the map's 256-entry crate-slot array and
//! returns immediately — spending no draws — if there is none. Otherwise it walks
//! a bounded retry loop; every attempt draws a random X then a random Y inside
//! the map's `Size.Width + Size.Height` cell-coordinate extent, snaps the result
//! to a nearby passable cell, and
//! places the crate overlay there.
//!
//! The drawn cell decides the snap movement: water uses *float*, anything else
//! uses *track*. After snapping, the destination cell independently chooses
//! `WaterCrateImg` or `WoodCrateImg` from its own land type.
//!
//! This module owns *placement only*. Crate contents, pickup effects and
//! `CrateRegen` respawn belong to the crate-effect system and are not modelled
//! here — a placed crate is an ordinary overlay cell until that lands.
//!
//! ## Dependency rules
//! Part of `sim/` — depends on `rules/`, `map/` grid types and other `sim/`
//! modules only. Never on render/, ui/, sidebar/, audio/, net/.

use crate::map::overlay_types::OverlayTypeRegistry;
use crate::rules::locomotor_type::{MovementZone, SpeedType};
use crate::rules::ruleset::{CrateRules, RuleSet};
use crate::rules::terrain_rules::LandType;
use crate::sim::cell_rect::cell_is_in_playfield;
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

/// Native hard cap supplied to the nearby-passable-cell search.
const CRATE_SNAP_RADIUS_CAP: u16 = 32;

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
/// `scenario_rng`. Failed attempts spend X then Y. A successful overlay spends
/// one additional `RandomRanged(0, 0x7fff_fffe)` draw for its crate slot timer.
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
    let land_overlay_id = overlay_registry.id_for_name(&rules.crate_rules.wood_crate_img);
    let water_overlay_id = overlay_registry.id_for_name(&rules.crate_rules.water_crate_img);
    if land_overlay_id.is_none() {
        log::warn!(
            "No overlay type named '{}' ([CrateRules] WoodCrateImg)",
            rules.crate_rules.wood_crate_img
        );
    }
    if sim.overlay_grid.is_none() {
        log::warn!("No overlay grid — placing no scenario-start crates");
        return CratePlacement {
            requested,
            placed: 0,
        };
    }

    // This pass owns the fresh Post_Map_Init slot table. Runtime timer fields
    // are deferred, but the native 256-entry free-slot scan and its no-draw
    // full case are load-bearing here.
    let mut crate_slots = [false; CRATE_SLOT_CAPACITY];
    let mut placed = 0u32;
    for _ in 0..requested {
        let Some(crate_slot) = crate_slots.iter().position(|occupied| !*occupied) else {
            // No free crate slot: the native placer returns before drawing.
            continue;
        };
        let Some(cell) = draw_one_crate_cell(sim, path_grid) else {
            continue;
        };
        // PlaceCrate chooses Float/Track from the drawn cell, but CrateSlot
        // independently chooses Water/Wood from the snapped destination.
        let destination_surface = crate_surface_at(sim, cell);
        let overlay_id = match destination_surface {
            CrateSurface::Water => water_overlay_id,
            CrateSurface::Land => land_overlay_id,
        };
        let (Some(overlay_id), Some(grid)) = (overlay_id, sim.overlay_grid.as_mut()) else {
            continue;
        };
        // CrateSlot stamps the already-validated destination directly. It does
        // not run ordinary OverlayClass::Mark's terrain-object, occupation,
        // slope or Track-passability gates; in particular stock water has
        // Track=0% and must still accept a Float-snapped water crate.
        grid.place_overlay(cell.0, cell.1, overlay_id, u8::MAX);
        placed += 1;
        crate_slots[crate_slot] = true;
        // `CrateSlot` consumes the timer seed only after the overlay succeeds.
        // Timer storage/formula belongs to the runtime crate-system slice; the
        // scenario-stream draw does not.
        let _ = sim.scenario_rng.next_range_u32_inclusive(0, 0x7fff_fffe);
    }
    log::info!("Scenario-start crates: requested {requested}, placed {placed}");
    CratePlacement { requested, placed }
}

/// Native water-vs-land classification, evaluated independently at the drawn
/// cell for movement and at the snapped destination for image selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CrateSurface {
    Land,
    Water,
}

/// One placer call: up to [`MAX_PLACEMENT_ATTEMPTS`] attempts, each spending
/// exactly two draws (X then Y) on the scenario stream before the cell is judged.
fn draw_one_crate_cell(sim: &mut Simulation, path_grid: Option<&PathGrid>) -> Option<(u16, u16)> {
    // Production's canonical cell-array extent is Size.Width + Size.Height.
    // Both ranged calls use the same inclusive 1..extent-1 bounds; LocalSize
    // does not participate.
    let extent = sim.session.map_width;
    if extent <= 1 {
        return None;
    }

    for _ in 0..MAX_PLACEMENT_ATTEMPTS {
        // Draw order is load-bearing: X first, then Y.
        let x = sim
            .scenario_rng
            .next_range_u32_inclusive(1, u32::from(extent - 1)) as u16;
        let y = sim
            .scenario_rng
            .next_range_u32_inclusive(1, u32::from(extent - 1)) as u16;

        // The drawn cell's own land type selects the snap, before any snapping
        // happens. A draw outside the grid takes the land branch, matching the
        // native fallback cell.
        let movement_surface = crate_surface_at(sim, (x, y));

        let Some(cell) = snap_to_passable(sim, path_grid, (x, y), movement_surface) else {
            continue;
        };
        if cell == (0, 0)
            || !cell_is_in_playfield(
                (i32::from(cell.0), i32::from(cell.1)),
                sim.playfield_bounds,
                sim.resolved_terrain.as_ref(),
                Some((sim.session.map_width, sim.session.map_height)),
            )
        {
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
        return Some(cell);
    }
    None
}

fn crate_surface_at(sim: &Simulation, cell: (u16, u16)) -> CrateSurface {
    if sim
        .resolved_terrain
        .as_ref()
        .and_then(|terrain| terrain.cell(cell.0, cell.1))
        .is_some_and(|cell| cell.yr_cell_land_type == LandType::Water.as_index())
    {
        CrateSurface::Water
    } else {
        CrateSurface::Land
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
        allow_bridge_cells: true,
        check_height: false,
        check_occupancy: false,
        radius_cap: sim.session.map_width.min(CRATE_SNAP_RADIUS_CAP),
        target_cell: Some((0, 0)),
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
    use std::collections::BTreeSet;

    use crate::map::bridge_facts::BRIDGE_FLAG_STRUCTURAL;
    use crate::map::overlay_types::OverlayTypeRegistry;
    use crate::rules::ini_parser::IniFile;
    use crate::sim::cell_rect::PlayfieldBounds;
    use crate::sim::overlay_grid::OverlayGrid;
    use crate::sim::pathfinding::PathGrid;

    const MAP: u16 = 40;

    fn crate_registry() -> OverlayTypeRegistry {
        let ini = IniFile::from_str(
            "[OverlayTypes]\n0=TIB01\n1=SILVER\n2=WOOD\n3=WATER\n\
             [TIB01]\nTiberium=yes\n[SILVER]\nCrate=yes\n\
             [WOOD]\nCrate=yes\n[WATER]\nCrate=yes\n",
        );
        OverlayTypeRegistry::from_ini(&ini, None)
    }

    fn crate_ruleset(extra: &str) -> RuleSet {
        let ini = IniFile::from_str(&format!(
            "[InfantryTypes]\n[VehicleTypes]\n[AircraftTypes]\n[BuildingTypes]\n\
             [CrateRules]\nCrateImg=SILVER\nWoodCrateImg=WOOD\nWaterCrateImg=WATER\n{extra}",
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
        sim.resolved_terrain = Some(uniform_terrain(false));
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
        cells_with_overlay(sim, registry, "WOOD")
    }

    #[test]
    fn gsi_01_04_crate_count_applies_minimum_then_maximum() {
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
    fn gsi_01_04_crate_rules_read_all_three_image_keys() {
        let rules = crate_ruleset("CrateMinimum=2\nCrateMaximum=9\n");
        assert_eq!(rules.crate_rules.minimum, 2);
        assert_eq!(rules.crate_rules.maximum, 9);
        assert_eq!(rules.crate_rules.crate_img, "SILVER");
        assert_eq!(rules.crate_rules.wood_crate_img, "WOOD");
        assert_eq!(rules.crate_rules.water_crate_img, "WATER");

        let defaults = crate_ruleset("");
        assert_eq!(defaults.crate_rules.minimum, 1);
        assert_eq!(defaults.crate_rules.maximum, 255);
    }

    #[test]
    fn gsi_01_04_crate_placement_respects_clamped_count_and_distinct_cells() {
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
    fn gsi_01_04_crate_minimum_lifts_single_human_count() {
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
    fn gsi_01_04_crate_disabled_spends_no_rng_and_places_nothing() {
        let rules = crate_ruleset("CrateMinimum=1\nCrateMaximum=255\n");
        let registry = crate_registry();
        let grid = PathGrid::test_all_passable(MAP, MAP);
        let mut sim = sim_with_grid(7);
        sim.session.game_options.crates = false;
        let rng_before = sim.scenario_rng.state();

        let result = place_scenario_start_crates(&mut sim, &rules, &registry, Some(&grid), 4);

        assert_eq!(
            result,
            CratePlacement {
                requested: 0,
                placed: 0
            }
        );
        assert!(crate_cells(&sim, &registry).is_empty());
        assert_eq!(sim.scenario_rng.state(), rng_before);
    }

    /// A successful request spends X, Y, then the crate-slot timer draw.
    #[test]
    fn gsi_01_04_crate_success_spends_x_y_then_timer() {
        let rules = crate_ruleset("CrateMinimum=3\nCrateMaximum=255\n");
        let registry = crate_registry();
        let grid = PathGrid::test_all_passable(MAP, MAP);

        let mut first = sim_with_grid(0xABCD);
        let mut replay = first.scenario_rng.clone();
        place_scenario_start_crates(&mut first, &rules, &registry, Some(&grid), 3);

        // Three successful first attempts: X, Y, timer for each crate.
        for _ in 0..3 {
            replay.next_range_u32_inclusive(1, u32::from(MAP - 1));
            replay.next_range_u32_inclusive(1, u32::from(MAP - 1));
            replay.next_range_u32_inclusive(0, 0x7fff_fffe);
        }
        assert_eq!(
            first.scenario_rng.state(),
            replay.state(),
            "successful crates must leave the cursor after X, Y, timer"
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

    #[test]
    fn gsi_01_04_crate_failed_attempt_spends_only_x_y_before_success_timer() {
        let rules = crate_ruleset("CrateMinimum=1\nCrateMaximum=1\n");
        let registry = crate_registry();
        let grid = PathGrid::test_all_passable(MAP, MAP);
        let mut sim = sim_with_grid(0xA11C_E);
        let mut replay = sim.scenario_rng.clone();

        let first = (
            replay.next_range_u32_inclusive(1, u32::from(MAP - 1)) as u16,
            replay.next_range_u32_inclusive(1, u32::from(MAP - 1)) as u16,
        );
        let second = (
            replay.next_range_u32_inclusive(1, u32::from(MAP - 1)) as u16,
            replay.next_range_u32_inclusive(1, u32::from(MAP - 1)) as u16,
        );
        assert_ne!(first, second, "fixture needs a distinct second attempt");
        replay.next_range_u32_inclusive(0, 0x7fff_fffe);

        let blocker = registry.id_for_name("TIB01").expect("overlay type");
        sim.overlay_grid
            .as_mut()
            .expect("overlay grid")
            .place_overlay(first.0, first.1, blocker, 0);

        let result = place_scenario_start_crates(&mut sim, &rules, &registry, Some(&grid), 1);

        assert_eq!(result.placed, 1);
        assert_eq!(crate_cells(&sim, &registry), vec![second]);
        assert_eq!(sim.scenario_rng.state(), replay.state());
    }

    #[test]
    fn gsi_01_04_crate_second_request_sees_first_overlay_immediately() {
        let rules = crate_ruleset("CrateMinimum=2\nCrateMaximum=2\n");
        let registry = crate_registry();
        let grid = PathGrid::test_all_passable(MAP, MAP);

        // Seed 1671 yields (37,4), timer, (37,4), then retry (5,8).
        let mut sim = sim_with_grid(1671);
        let mut replay = sim.scenario_rng.clone();
        let first = (
            replay.next_range_u32_inclusive(1, u32::from(MAP - 1)) as u16,
            replay.next_range_u32_inclusive(1, u32::from(MAP - 1)) as u16,
        );
        assert_eq!(first, (37, 4));
        replay.next_range_u32_inclusive(0, 0x7fff_fffe);
        assert_eq!(
            (
                replay.next_range_u32_inclusive(1, u32::from(MAP - 1)) as u16,
                replay.next_range_u32_inclusive(1, u32::from(MAP - 1)) as u16,
            ),
            first
        );
        let retry = (
            replay.next_range_u32_inclusive(1, u32::from(MAP - 1)) as u16,
            replay.next_range_u32_inclusive(1, u32::from(MAP - 1)) as u16,
        );
        assert_eq!(retry, (5, 8));
        replay.next_range_u32_inclusive(0, 0x7fff_fffe);

        let result = place_scenario_start_crates(&mut sim, &rules, &registry, Some(&grid), 2);

        assert_eq!(result.placed, 2);
        assert_eq!(
            crate_cells(&sim, &registry)
                .into_iter()
                .collect::<BTreeSet<_>>(),
            BTreeSet::from([first, retry])
        );
        assert_eq!(sim.scenario_rng.state(), replay.state());
    }

    #[test]
    fn gsi_01_04_crate_one_thousand_failures_spend_exactly_two_thousand_draws() {
        let rules = crate_ruleset("CrateMinimum=1\nCrateMaximum=1\n");
        let registry = crate_registry();
        let grid = PathGrid::test_all_passable(MAP, MAP);
        let mut sim = sim_with_grid(0xF411_ED);
        // Empty diamond: every nearby result fails the separate playfield gate.
        sim.playfield_bounds = Some(PlayfieldBounds {
            base: 0,
            off_fc: 0,
            off_100: 0,
            off_104: 0,
            off_108: 0,
        });
        let mut replay = sim.scenario_rng.clone();
        for _ in 0..MAX_PLACEMENT_ATTEMPTS {
            replay.next_range_u32_inclusive(1, u32::from(MAP - 1));
            replay.next_range_u32_inclusive(1, u32::from(MAP - 1));
        }

        let result = place_scenario_start_crates(&mut sim, &rules, &registry, Some(&grid), 1);

        assert_eq!(result.placed, 0);
        assert!(crate_cells(&sim, &registry).is_empty());
        assert_eq!(sim.scenario_rng.state(), replay.state());
    }

    #[test]
    fn gsi_01_04_crate_full_slot_array_stops_before_more_rng() {
        let rules = crate_ruleset("CrateMinimum=300\nCrateMaximum=300\n");
        let registry = crate_registry();
        let grid = PathGrid::test_all_passable(MAP, MAP);
        let mut sim = sim_with_grid(0x5107_0256);
        let mut replay = sim.scenario_rng.clone();
        let mut occupied = BTreeSet::new();
        while occupied.len() < CRATE_SLOT_CAPACITY {
            let cell = (
                replay.next_range_u32_inclusive(1, u32::from(MAP - 1)) as u16,
                replay.next_range_u32_inclusive(1, u32::from(MAP - 1)) as u16,
            );
            if occupied.insert(cell) {
                replay.next_range_u32_inclusive(0, 0x7fff_fffe);
            }
        }

        let result = place_scenario_start_crates(&mut sim, &rules, &registry, Some(&grid), 300);

        assert_eq!(result.requested, 300);
        assert_eq!(result.placed, CRATE_SLOT_CAPACITY as u32);
        assert_eq!(sim.scenario_rng.state(), replay.state());
    }

    #[test]
    fn gsi_01_04_crate_slot_commit_skips_generic_mark_occupation_gate() {
        let rules = crate_ruleset("CrateMinimum=1\nCrateMaximum=1\n");
        let registry = crate_registry();
        let grid = PathGrid::test_all_passable(MAP, MAP);
        let mut sim = sim_with_grid(0x47C5_50);
        let mut replay = sim.scenario_rng.clone();
        let rx = replay.next_range_u32_inclusive(1, u32::from(MAP - 1)) as u16;
        let ry = replay.next_range_u32_inclusive(1, u32::from(MAP - 1)) as u16;
        replay.next_range_u32_inclusive(0, 0x7fff_fffe);
        sim.substrate.raw_cell_occupation.mark_ground(
            rx,
            ry,
            crate::sim::occupancy::OBJECT_OCCUPATION_BIT,
        );

        let result = place_scenario_start_crates(&mut sim, &rules, &registry, Some(&grid), 1);

        assert_eq!(result.requested, 1);
        assert_eq!(result.placed, 1);
        assert_eq!(crate_cells(&sim, &registry), vec![(rx, ry)]);
        assert_eq!(
            sim.overlay_grid
                .as_ref()
                .expect("overlay grid")
                .cell(rx, ry)
                .overlay_data,
            u8::MAX,
            "crate-slot stamp writes Cell+0x11E = 0xFF",
        );
        assert_eq!(sim.scenario_rng.state(), replay.state());
    }

    #[test]
    fn gsi_01_04_crate_water_track_zero_commits_and_spends_timer_draw() {
        let rules = crate_ruleset("CrateMinimum=1\nCrateMaximum=1\n");
        let registry = crate_registry();
        let grid = PathGrid::test_all_passable(MAP, MAP);
        let mut sim = sim_with_grid(0xF10A_7000);
        sim.resolved_terrain = Some(uniform_terrain(true));
        for cell in &mut sim.resolved_terrain.as_mut().expect("terrain").cells {
            cell.speed_costs.track = Some(0);
            cell.speed_costs.float = Some(100);
        }
        let mut replay = sim.scenario_rng.clone();
        let destination = (
            replay.next_range_u32_inclusive(1, u32::from(MAP - 1)) as u16,
            replay.next_range_u32_inclusive(1, u32::from(MAP - 1)) as u16,
        );
        replay.next_range_u32_inclusive(0, 0x7fff_fffe);

        let result = place_scenario_start_crates(&mut sim, &rules, &registry, Some(&grid), 1);

        assert_eq!(result.placed, 1);
        assert_eq!(
            cells_with_overlay(&sim, &registry, "WATER"),
            vec![destination]
        );
        assert_eq!(sim.scenario_rng.state(), replay.state());
    }

    /// With a uniform surface, destination image selection still follows the
    /// native LandType byte rather than the derived Rust convenience flag.
    #[test]
    fn gsi_01_04_crate_landtype_two_uses_water_otherwise_wood() {
        let rules = crate_ruleset("CrateMinimum=4\nCrateMaximum=255\n");
        let registry = crate_registry();
        let grid = PathGrid::test_all_passable(MAP, MAP);

        for water in [false, true] {
            let mut sim = sim_with_grid(0x5150);
            sim.resolved_terrain = Some(uniform_terrain(water));
            // Keep the convenience boolean deliberately opposite: selection
            // is by CellClass LandType == 2, not this derived Rust flag.
            for cell in &mut sim.resolved_terrain.as_mut().expect("terrain").cells {
                cell.is_water = !water;
            }

            let result = place_scenario_start_crates(&mut sim, &rules, &registry, Some(&grid), 4);
            assert_eq!(result.placed, 4);

            let silver_crates = cells_with_overlay(&sim, &registry, "SILVER");
            let land_crates = cells_with_overlay(&sim, &registry, "WOOD");
            let water_crates = cells_with_overlay(&sim, &registry, "WATER");
            assert!(silver_crates.is_empty(), "startup never uses CrateImg");
            if water {
                assert_eq!(water_crates.len(), 4, "water draws take WaterCrateImg");
                assert!(land_crates.is_empty());
            } else {
                assert_eq!(land_crates.len(), 4, "land draws take WoodCrateImg");
                assert!(water_crates.is_empty());
            }
        }
    }

    #[test]
    fn gsi_01_04_crate_mixed_surface_uses_drawn_movement_and_destination_image() {
        let rules = crate_ruleset("CrateMinimum=1\nCrateMaximum=1\n");
        let registry = crate_registry();
        let grid = PathGrid::test_all_passable(MAP, MAP);

        for drawn_is_water in [true, false] {
            let mut sim = sim_with_grid(0x1234_5678);
            let mut replay = sim.scenario_rng.clone();
            let drawn = (
                replay.next_range_u32_inclusive(1, u32::from(MAP - 1)) as u16,
                replay.next_range_u32_inclusive(1, u32::from(MAP - 1)) as u16,
            );
            assert_eq!(drawn, (14, 13));

            let destination = (15, 13);
            let wrong_movement_decoy = (13, 12);
            let mut terrain = uniform_terrain(false);
            for cell in &mut terrain.cells {
                cell.speed_costs.float = Some(0);
                cell.speed_costs.track = Some(0);
            }
            let drawn_cell = terrain.cell_mut(drawn.0, drawn.1).expect("drawn cell");
            drawn_cell.yr_cell_land_type = if drawn_is_water {
                LandType::Water.as_index()
            } else {
                LandType::Clear.as_index()
            };

            // The real destination admits only the drawn cell's movement type.
            // The nearer-to-(0,0) decoy admits only the opposite type, so
            // choosing movement from anywhere but the draw lands on the decoy.
            let destination_cell = terrain
                .cell_mut(destination.0, destination.1)
                .expect("destination cell");
            if drawn_is_water {
                destination_cell.speed_costs.float = Some(100);
            } else {
                destination_cell.speed_costs.track = Some(100);
            }
            destination_cell.yr_cell_land_type = if drawn_is_water {
                LandType::Clear.as_index()
            } else {
                LandType::Water.as_index()
            };
            let decoy = terrain
                .cell_mut(wrong_movement_decoy.0, wrong_movement_decoy.1)
                .expect("wrong-movement decoy");
            if drawn_is_water {
                decoy.speed_costs.track = Some(100);
            } else {
                decoy.speed_costs.float = Some(100);
            }
            sim.resolved_terrain = Some(terrain);

            let result = place_scenario_start_crates(&mut sim, &rules, &registry, Some(&grid), 1);

            assert_eq!(result.placed, 1);
            let expected_image = if drawn_is_water { "WOOD" } else { "WATER" };
            let drawn_image = if drawn_is_water { "WATER" } else { "WOOD" };
            assert_eq!(
                cells_with_overlay(&sim, &registry, expected_image),
                vec![destination],
                "image must follow the snapped destination surface"
            );
            assert!(
                cells_with_overlay(&sim, &registry, drawn_image).is_empty(),
                "image must not follow the drawn surface"
            );
            assert!(
                sim.overlay_grid
                    .as_ref()
                    .expect("overlay grid")
                    .cell(wrong_movement_decoy.0, wrong_movement_decoy.1)
                    .overlay_id
                    .is_none(),
                "movement type must follow the drawn surface"
            );
        }
    }

    #[test]
    fn gsi_01_04_crate_snap_uses_target_zero_bridge_allowance_no_occupancy_and_radius_over_eight() {
        let registry = crate_registry();
        let mut sim = sim_with_grid(1);
        let mut terrain = uniform_terrain(false);
        for cell in &mut terrain.cells {
            cell.speed_costs.track = Some(0);
        }
        // Both candidates are on ring 9. Ring order encounters (29,11)
        // first, but target (0,0) selects the nearer (11,20).
        terrain
            .cell_mut(29, 11)
            .expect("candidate A")
            .speed_costs
            .track = Some(100);
        let selected = terrain.cell_mut(11, 20).expect("candidate B");
        selected.speed_costs.track = Some(100);
        selected.bridge_facts.raw_flags |= BRIDGE_FLAG_STRUCTURAL;
        sim.resolved_terrain = Some(terrain);

        // An ordinary overlay would fail occupancy-rect checking, but this
        // caller passes occupancy_rect_check=false. Crate placement itself
        // performs its separate no-existing-overlay check after FNPC.
        let ore = registry.id_for_name("TIB01").expect("overlay type");
        sim.overlay_grid
            .as_mut()
            .expect("overlay grid")
            .place_overlay(11, 20, ore, 0);
        let grid = PathGrid::test_all_passable(MAP, MAP);

        assert_eq!(
            snap_to_passable(&sim, Some(&grid), (20, 20), CrateSurface::Land),
            Some((11, 20))
        );
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
    fn gsi_01_04_crate_one_human_and_seven_ai_requests_one() {
        use crate::sim::house_state::HouseState;

        let mut sim = sim_with_grid(0x1A17_0001);
        let mut add = |name: &str, is_human: bool, passive: bool| {
            let id = sim.interner.intern(name);
            let mut house = HouseState::new(id, 0, None, is_human, 10_000, 10);
            house.multiplay_passive = passive;
            sim.houses.insert(id, house);
        };
        add("Local", true, false);
        for index in 1..=7 {
            add(&format!("Computer{index}"), false, false);
        }
        add("Neutral", false, true);
        add("Special", false, true);

        let player_count = human_player_count(&sim);
        assert_eq!(player_count, 1);
        let rules = crate_ruleset("CrateMinimum=1\nCrateMaximum=255\n");
        let registry = crate_registry();
        let grid = PathGrid::test_all_passable(MAP, MAP);
        let result =
            place_scenario_start_crates(&mut sim, &rules, &registry, Some(&grid), player_count);
        assert_eq!(result.requested, 1);
        assert_eq!(result.placed, 1);
    }

    /// Both coordinates use the canonical Size.Width + Size.Height extent;
    /// LocalSize is irrelevant to the two ranged calls.
    #[test]
    fn gsi_01_04_crate_draws_use_size_extent_not_localsize() {
        let rules = crate_ruleset("CrateMinimum=1\nCrateMaximum=1\n");
        let registry = crate_registry();
        let grid = PathGrid::test_all_passable(MAP, MAP);
        let mut sim = sim_with_grid(0x2468);
        let mut replay = sim.scenario_rng.clone();
        let expected = (
            replay.next_range_u32_inclusive(1, u32::from(MAP - 1)) as u16,
            replay.next_range_u32_inclusive(1, u32::from(MAP - 1)) as u16,
        );
        replay.next_range_u32_inclusive(0, 0x7fff_fffe);
        sim.session.local_left = 0;
        sim.session.local_top = 0;
        sim.session.local_width = 1;
        sim.session.local_height = 1;

        let result = place_scenario_start_crates(&mut sim, &rules, &registry, Some(&grid), 1);

        assert_eq!(result.placed, 1);
        assert_eq!(crate_cells(&sim, &registry), vec![expected]);
        assert_ne!(expected, (0, 0), "draw lies outside the 1x1 LocalSize");
        assert_eq!(sim.scenario_rng.state(), replay.state());
    }
}
