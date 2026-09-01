//! Scenario-start crate placement — gamemd `ScenarioClass::Post_Map_Init`.
//!
//! When the lobby "Crates" option is on (stock default), the scenario clamps the
//! player count between `[CrateRules] CrateMinimum` and `CrateMaximum` and calls
//! the map's random-cell crate placer exactly that many times. Each placer call
//! first looks for a free entry in the map's 256-entry crate-slot array and
//! returns immediately — spending no draws — if there is none. Otherwise it walks
//! a bounded retry loop; every attempt draws a random X then a random Y inside
//! the active Map rectangle, snaps the result to a nearby passable cell, and
//! submits the destination to the crate-specific native Mark transaction.
//!
//! The drawn cell decides the snap movement: water uses *float*, anything else
//! uses *track*. After snapping, the destination cell independently chooses
//! `WaterCrateImg` or `WoodCrateImg` from its own land type.
//!
//! This module owns startup placement and its persistent slot/timer result.
//! Crate contents, pickup effects, removal, and `CrateRegen` scanning remain
//! later crate-system mechanisms.
//!
//! ## Dependency rules
//! Part of `sim/` — depends on `rules/`, `map/` grid types and other `sim/`
//! modules only. Never on render/, ui/, sidebar/, audio/, net/.

mod state;

pub use state::{CrateAuthority, CrateSlot};

use crate::map::bridge_facts::BRIDGE_FLAG_STRUCTURAL;
use crate::map::overlay_types::OverlayTypeRegistry;
use crate::rules::crate_rules::CrateRules;
use crate::rules::locomotor_type::{MovementZone, SpeedType};
use crate::rules::ruleset::RuleSet;
use crate::rules::terrain_rules::LandType;
use crate::sim::cell_rect::cell_is_in_playfield_height_aware;
use crate::sim::find_nearby_cell::{
    NearbyAnchorGate, NearbyFootprint, NearbyQuery, PassabilityArgs, find_nearby_passable_cell,
};
use crate::sim::pathfinding::PathGrid;
use crate::sim::world::Simulation;

/// Retry budget inside one placer call. Every attempt spends its two draws
/// whether or not the cell works out; after this many failures the call gives up
/// and no crate is placed.
const MAX_PLACEMENT_ATTEMPTS: u32 = 1000;

/// Native hard cap supplied to the nearby-passable-cell search.
const CRATE_SNAP_RADIUS_CAP: u16 = 32;

/// Outcome of one scenario-start crate pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CratePlacement {
    /// Signed attempts selected by native minimum/human/maximum comparisons.
    pub requested: i32,
    /// Accepted calls, including timed ghosts whose overlay Mark failed.
    pub accepted: u32,
    /// Accepted calls that installed a visible overlay.
    pub visible: u32,
}

/// `min(max(CrateMinimum, player_count), CrateMaximum)`.
///
/// gamemd applies the floor first and the ceiling second, so a `CrateMaximum`
/// below `CrateMinimum` wins — the clamp is not symmetric and must not be
/// rewritten as a single `clamp()` call with the arguments in rules order.
pub fn scenario_start_crate_count(rules: &CrateRules, player_count: i32) -> i32 {
    rules.maximum.min(rules.minimum.max(player_count))
}

/// The human seat count the crate clamp is applied to.
///
/// gamemd's pregame setup assigns the crate-count global straight from the value
/// it logs as "Pregame setup for N players", and the sibling `Post_Map_Init`
/// budget gate adds the AI count to that same value — so it is human seats only.
/// In a 1v3 skirmish this is 1, not 4. Passive houses (Neutral, Special) are
/// never seats.
pub fn human_player_count(sim: &Simulation) -> i32 {
    i32::try_from(
        sim.houses
            .values()
            .filter(|house| house.is_human && !house.multiplay_passive)
            .count(),
    )
    .unwrap_or(i32::MAX)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OneCrateResult {
    HardRejected,
    AcceptedGhost,
    AcceptedVisible,
}

/// Native allocation, Unlimbo, and Mark failures all collapse to the same
/// gameplay result after the two hard prechecks: an accepted timed ghost.
/// Production invariants select `None`; focused tests inject the other cases
/// without importing gamemd's object allocator into simulation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[cfg_attr(not(test), allow(dead_code))]
enum ForcedPostPrecheckFailure {
    #[default]
    None,
    Allocation,
    Unlimbo,
    Mark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AcceptedCellResult {
    Ghost,
    Visible,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CrateRandomFrame {
    left: i32,
    top: i32,
    width: i32,
    height: i32,
    radius_cap: u16,
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
    player_count: i32,
) -> CratePlacement {
    place_scenario_start_crates_with_failure(
        sim,
        rules,
        overlay_registry,
        path_grid,
        player_count,
        ForcedPostPrecheckFailure::None,
    )
}

fn place_scenario_start_crates_with_failure(
    sim: &mut Simulation,
    rules: &RuleSet,
    overlay_registry: &OverlayTypeRegistry,
    path_grid: Option<&PathGrid>,
    player_count: i32,
    forced_failure: ForcedPostPrecheckFailure,
) -> CratePlacement {
    if !sim.session.game_options.crates {
        return CratePlacement {
            requested: 0,
            accepted: 0,
            visible: 0,
        };
    }
    let requested = scenario_start_crate_count(&rules.crate_rules, player_count);
    if requested <= 0 {
        return CratePlacement {
            requested,
            accepted: 0,
            visible: 0,
        };
    }

    let mut accepted = 0u32;
    let mut visible = 0u32;
    for _ in 0..requested {
        match place_one_random_crate(
            sim,
            &rules.crate_rules,
            overlay_registry,
            path_grid,
            forced_failure,
        ) {
            OneCrateResult::HardRejected => {}
            OneCrateResult::AcceptedGhost => accepted = accepted.wrapping_add(1),
            OneCrateResult::AcceptedVisible => {
                accepted = accepted.wrapping_add(1);
                visible = visible.wrapping_add(1);
            }
        }
    }
    log::info!(
        "Scenario-start crates: requested {requested}, accepted {accepted}, visible {visible}"
    );
    CratePlacement {
        requested,
        accepted,
        visible,
    }
}

/// Native water-vs-land classification, evaluated independently at the drawn
/// cell for movement and at the snapped destination for image selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CrateSurface {
    Land,
    Water,
}

/// One `MapClass__PlaceCrateAtRandomCell @ 0x0056BD40` call.
fn place_one_random_crate(
    sim: &mut Simulation,
    rules: &CrateRules,
    overlay_registry: &OverlayTypeRegistry,
    path_grid: Option<&PathGrid>,
    forced_failure: ForcedPostPrecheckFailure,
) -> OneCrateResult {
    let Some(slot_index) = sim.crate_authority.first_empty_index() else {
        // The native full-table return precedes every random draw.
        return OneCrateResult::HardRejected;
    };
    let Some(frame) = crate_random_frame(sim) else {
        return OneCrateResult::HardRejected;
    };
    for _ in 0..MAX_PLACEMENT_ATTEMPTS {
        let drawn = draw_crate_candidate(&mut sim.scenario_rng, frame);

        // The drawn cell's own land type selects the snap, before any snapping
        // happens. A draw outside the grid takes the land branch, matching the
        // native fallback cell.
        let movement_surface = crate_surface_at_packed(sim, drawn);

        let Some(cell) = snap_to_passable_with_radius(
            sim,
            path_grid,
            drawn,
            movement_surface,
            frame.radius_cap,
        ) else {
            continue;
        };
        if cell == (0, 0)
            || !cell_is_in_playfield_height_aware(
                (i32::from(cell.0), i32::from(cell.1)),
                sim.playfield_bounds,
                sim.resolved_terrain.as_ref(),
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
        let accepted =
            validate_and_stamp_candidate(sim, rules, overlay_registry, cell, forced_failure);

        // `CrateSlot__PlaceOverlayAndInitTimer @ 0x004A17C0`: accepted ghosts
        // and visible overlays both write the packed coordinate before drawing
        // and installing start/aux/duration timer words.
        {
            let slot = sim.crate_authority.slot_mut(slot_index);
            slot.cell_x = cell.0 as i16;
            slot.cell_y = cell.1 as i16;
        }
        let timer_draw = sim.scenario_rng.next_range_u32_inclusive(0, 0x7fff_fffe);
        let (start_frame, aux, duration) =
            state::crate_timer_words(rules.regen, timer_draw, sim.session.binary_frame as i32);
        let slot = sim.crate_authority.slot_mut(slot_index);
        slot.start_frame = start_frame;
        slot.aux = aux;
        slot.duration = duration;
        return match accepted {
            AcceptedCellResult::Ghost => OneCrateResult::AcceptedGhost,
            AcceptedCellResult::Visible => OneCrateResult::AcceptedVisible,
        };
    }
    OneCrateResult::HardRejected
}

/// Active Map rectangle used by random placement. `MapClass` constructs it as
/// `(1, 1, SizeW + SizeH - 1, SizeW + SizeH - 1)`; LocalSize and the canonical
/// Rust cell-array extent do not own these draws.
fn crate_random_frame(sim: &Simulation) -> Option<CrateRandomFrame> {
    let size_width = sim.playfield_bounds?.base;
    let size_height = sim.playfield_size_height?;
    let size_sum = size_width.wrapping_add(size_height);
    let width = size_sum.wrapping_sub(1);
    Some(CrateRandomFrame {
        left: 1,
        top: 1,
        width,
        height: width,
        // Native forwards min(SizeW + SizeH, 32) as a signed loop cap. A
        // nonpositive cap scans no FNPC rings, but the X/Y draws above still
        // occur. Zero is the Rust-native representation of that empty scan.
        radius_cap: size_sum.clamp(0, i32::from(CRATE_SNAP_RADIUS_CAP)) as u16,
    })
}

/// Signed `Random__RandomRanged` projection used by the crate rectangle.
/// Reversed endpoints are compared and swapped as signed dwords before the
/// shared native mask/rejection sampler sees their nonnegative span.
fn crate_random_ranged_i32(
    rng: &mut crate::sim::rng::SimRng,
    low: i32,
    high: i32,
) -> i32 {
    let (lo, hi) = if low <= high {
        (low, high)
    } else {
        (high, low)
    };
    let span = (i64::from(hi) - i64::from(lo)) as u32;
    lo.wrapping_add(rng.next_range_u32_inclusive(0, span) as i32)
}

/// `MapClass__PlaceCrateAtRandomCell @ 0x0056BD8B..0x0056BDD3` always draws
/// X then Y, adds the signed rectangle origin with dword wrapping, and stores
/// each result through a 16-bit word before the later MOVSX reads it back.
fn draw_crate_candidate(
    rng: &mut crate::sim::rng::SimRng,
    frame: CrateRandomFrame,
) -> (i32, i32) {
    let x = frame.left.wrapping_add(crate_random_ranged_i32(
        rng,
        0,
        frame.width.wrapping_sub(1),
    ));
    let y = frame.top.wrapping_add(crate_random_ranged_i32(
        rng,
        0,
        frame.height.wrapping_sub(1),
    ));
    (x as i16 as i32, y as i16 as i32)
}

fn validate_and_stamp_candidate(
    sim: &mut Simulation,
    rules: &CrateRules,
    overlay_registry: &OverlayTypeRegistry,
    cell: (u16, u16),
    forced_failure: ForcedPostPrecheckFailure,
) -> AcceptedCellResult {
    let water_id = rules
        .water_crate_img
        .as_deref()
        .and_then(|name| overlay_registry.id_for_name(name));
    let wood_id = rules
        .wood_crate_img
        .as_deref()
        .and_then(|name| overlay_registry.id_for_name(name));
    let crate_id = rules
        .crate_img
        .as_deref()
        .and_then(|name| overlay_registry.id_for_name(name));
    let selected_id = match crate_surface_at(sim, cell) {
        CrateSurface::Water => water_id,
        CrateSurface::Land => wood_id,
    };
    let Some(selected_id) = selected_id else {
        return AcceptedCellResult::Ghost;
    };

    // `OverlayClass__Mark @ 0x005FC570` compares live Rules pointers, with
    // Water first. Numeric registry identity is the Rust-native pointer alias.
    let mark_speed = if Some(selected_id) == water_id {
        SpeedType::Float
    } else if Some(selected_id) == crate_id || Some(selected_id) == wood_id {
        SpeedType::Track
    } else {
        return AcceptedCellResult::Ghost;
    };

    let Some(terrain_cell) = sim
        .resolved_terrain
        .as_ref()
        .and_then(|terrain| terrain.cell(cell.0, cell.1))
    else {
        return AcceptedCellResult::Ghost;
    };
    if sim.production.terrain_object_cells.contains_key(&cell) {
        return AcceptedCellResult::Ghost;
    }
    if terrain_cell.slope_type > 4 && selected_id != 0xB2 {
        return AcceptedCellResult::Ghost;
    }
    let bridge_layer_selected = terrain_cell.bridge_facts.raw_flags & BRIDGE_FLAG_STRUCTURAL != 0;
    let selected_occupation = if bridge_layer_selected {
        sim.substrate.raw_cell_occupation.deck_bits(cell.0, cell.1)
    } else {
        sim.substrate
            .raw_cell_occupation
            .ground_bits(cell.0, cell.1)
    };
    if selected_occupation != 0 {
        return AcceptedCellResult::Ghost;
    }
    if !bridge_layer_selected && terrain_cell.speed_costs.cost_for_speed_type(mark_speed) == Some(0)
    {
        return AcceptedCellResult::Ghost;
    }
    if forced_failure != ForcedPostPrecheckFailure::None {
        return AcceptedCellResult::Ghost;
    }

    let (Some(overlay_grid), Some(resolved_terrain)) =
        (sim.overlay_grid.as_mut(), sim.resolved_terrain.as_mut())
    else {
        return AcceptedCellResult::Ghost;
    };
    if overlay_grid.place_crate_overlay(
        resolved_terrain,
        overlay_registry,
        cell.0,
        cell.1,
        selected_id,
    ) {
        AcceptedCellResult::Visible
    } else {
        AcceptedCellResult::Ghost
    }
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

fn crate_surface_at_packed(sim: &Simulation, cell: (i32, i32)) -> CrateSurface {
    let Some((rx, ry)) = crate::map::cell_index::canonical_cell_coord(cell.0, cell.1) else {
        return CrateSurface::Land;
    };
    crate_surface_at(sim, (rx, ry))
}

/// Snap a drawn cell onto a nearby passable cell of the matching surface.
fn snap_to_passable(
    sim: &Simulation,
    path_grid: Option<&PathGrid>,
    drawn: (i32, i32),
    surface: CrateSurface,
) -> Option<(u16, u16)> {
    let radius_cap = crate_random_frame(sim)?.radius_cap;
    snap_to_passable_with_radius(sim, path_grid, drawn, surface, radius_cap)
}

fn snap_to_passable_with_radius(
    sim: &Simulation,
    path_grid: Option<&PathGrid>,
    drawn: (i32, i32),
    surface: CrateSurface,
    radius_cap: u16,
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
        footprint: NearbyFootprint::SINGLE,
        // gamemd-derived: every FNPC candidate path, including crate placement,
        // calls Is_Cell_In_Playfield_CellClass(cell, 1) @ 0x00578540 before
        // CellRect passability (caller PlaceCrateAtRandomCell @ 0x0056BD40).
        anchor_gate: NearbyAnchorGate::NativeHeightAware,
        allow_bridge_cells: true,
        check_height: false,
        check_occupancy: false,
        radius_cap,
        // gamemd-derived: PlaceCrateAtRandomCell @ 0x0056BD40 initializes
        // this reference to the engine zero cell. FNPC @ 0x0056DC20 treats
        // that zero sentinel as "no target" and selects by live frame modulo.
        target_cell: None,
        path_grid,
        resolved_terrain: sim.resolved_terrain.as_ref(),
        overlay_grid: sim.overlay_grid.as_ref(),
        occupancy: Some(&sim.substrate.occupancy),
        entities: Some(&sim.substrate.entities),
        zone_grid: sim.zone_grid.as_ref(),
        playfield_bounds: sim.playfield_bounds,
    };
    find_nearby_passable_cell(
        drawn,
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

    fn crate_ruleset_with_images(wood: &str, common: &str, water: &str, extra: &str) -> RuleSet {
        let ini = IniFile::from_str(&format!(
            "[InfantryTypes]\n[VehicleTypes]\n[AircraftTypes]\n[BuildingTypes]\n\
             [CrateRules]\nCrateImg={common}\nWoodCrateImg={wood}\nWaterCrateImg={water}\n{extra}",
        ));
        RuleSet::from_ini(&ini).expect("rules")
    }

    fn crate_registry_with_raw_b2() -> OverlayTypeRegistry {
        use std::fmt::Write as _;

        let mut ini_text = String::from("[OverlayTypes]\n");
        for index in 0..=0xB2u16 {
            let name = if index == 0xB2 {
                "STEEPCRATE".to_owned()
            } else {
                format!("OV{index:03}")
            };
            writeln!(&mut ini_text, "{index}={name}").expect("string write");
        }
        ini_text.push_str("[STEEPCRATE]\nCrate=yes\n");
        OverlayTypeRegistry::from_ini(&IniFile::from_str(&ini_text), None)
    }

    /// `seed` positions the scenario cursor; the placer draws from there.
    fn sim_with_grid(seed: u64) -> Simulation {
        let mut sim = Simulation::new();
        sim.session.seed = seed;
        sim.scenario_rng = crate::sim::rng::SimRng::new(seed);
        sim.session.map_width = MAP;
        sim.session.map_height = MAP;
        // Production installs MapClass authority before Post_Map_Init reaches
        // crate placement. Keep this focused fixture intentionally broad while
        // exercising the same required mode-one gate.
        sim.playfield_bounds = Some(PlayfieldBounds {
            base: 20,
            off_fc: -128,
            off_100: -128,
            off_104: 256,
            off_108: 256,
        });
        sim.playfield_size_height = Some(20);
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

    fn first_random_cell(sim: &Simulation) -> ((u16, u16), crate::sim::rng::SimRng) {
        let frame = crate_random_frame(sim).expect("valid crate random frame");
        let mut replay = sim.scenario_rng.clone();
        let cell =
            (
                frame.left.wrapping_add(
                    replay.next_range_u32_inclusive(0, (frame.width - 1) as u32) as i32,
                ) as u16,
                frame.top.wrapping_add(
                    replay.next_range_u32_inclusive(0, (frame.height - 1) as u32) as i32,
                ) as u16,
            );
        (cell, replay)
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

        let signed = CrateRules {
            minimum: -9,
            maximum: -3,
            ..CrateRules::default()
        };
        assert_eq!(scenario_start_crate_count(&signed, -7), -7);
        assert_eq!(scenario_start_crate_count(&signed, 4), -3);

        let uncapped_slot_count = CrateRules {
            minimum: 300,
            maximum: i32::MAX,
            ..CrateRules::default()
        };
        assert_eq!(scenario_start_crate_count(&uncapped_slot_count, 1), 300);
    }

    #[test]
    fn gsi_01_04_crate_rules_read_all_three_image_keys() {
        let rules = crate_ruleset("CrateMinimum=2\nCrateMaximum=9\n");
        assert_eq!(rules.crate_rules.minimum, 2);
        assert_eq!(rules.crate_rules.maximum, 9);
        assert_eq!(rules.crate_rules.crate_img.as_deref(), Some("SILVER"));
        assert_eq!(rules.crate_rules.wood_crate_img.as_deref(), Some("WOOD"));
        assert_eq!(rules.crate_rules.water_crate_img.as_deref(), Some("WATER"));

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
        assert_eq!((result.accepted, result.visible), (3, 3));
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
        assert_eq!((result.accepted, result.visible), (4, 4));
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
                accepted: 0,
                visible: 0,
            }
        );
        assert!(crate_cells(&sim, &registry).is_empty());
        assert_eq!(sim.scenario_rng.state(), rng_before);
    }

    #[test]
    fn scenario_start_crate_missing_map_size_authority_invents_no_rectangle() {
        let rules = crate_ruleset("CrateMinimum=1\nCrateMaximum=1\n");
        let registry = crate_registry();
        let grid = PathGrid::test_all_passable(MAP, MAP);
        let mut sim = sim_with_grid(0xBAD5_1E00);
        sim.playfield_size_height = None;
        let before = sim.scenario_rng.state();

        let result = place_scenario_start_crates(&mut sim, &rules, &registry, Some(&grid), 1);

        assert_eq!((result.accepted, result.visible), (0, 0));
        assert_eq!(sim.scenario_rng.state(), before);
        assert!(sim.crate_authority.slots()[0].is_empty());
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

        assert_eq!((result.accepted, result.visible), (1, 1));
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

        assert_eq!((result.accepted, result.visible), (2, 2));
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
        // Empty diamond: every FNPC candidate fails the mandatory anchor gate
        // (and the independent final playfield gate would reject it too).
        sim.playfield_bounds = Some(PlayfieldBounds {
            base: 20,
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

        assert_eq!((result.accepted, result.visible), (0, 0));
        assert!(crate_cells(&sim, &registry).is_empty());
        assert_eq!(sim.crate_authority.slots()[0], CrateSlot::default());
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
        while occupied.len() < state::CRATE_SLOT_CAPACITY {
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
        assert_eq!(
            (result.accepted, result.visible),
            (
                state::CRATE_SLOT_CAPACITY as u32,
                state::CRATE_SLOT_CAPACITY as u32,
            )
        );
        assert_eq!(sim.scenario_rng.state(), replay.state());
    }

    #[test]
    fn scenario_start_crate_preexisting_full_slot_table_spends_no_rng() {
        let rules = crate_ruleset("CrateMinimum=4\nCrateMaximum=4\n");
        let registry = crate_registry();
        let grid = PathGrid::test_all_passable(MAP, MAP);
        let mut sim = sim_with_grid(0x5107_F011);
        for index in 0..state::CRATE_SLOT_CAPACITY {
            sim.crate_authority.slot_mut(index).cell_x = (index as i16).wrapping_add(1);
        }
        let before = sim.scenario_rng.state();

        let result = place_scenario_start_crates(&mut sim, &rules, &registry, Some(&grid), 1);

        assert_eq!(result.requested, 4);
        assert_eq!((result.accepted, result.visible), (0, 0));
        assert_eq!(sim.scenario_rng.state(), before);
    }

    #[test]
    fn scenario_start_crate_any_ground_occupation_bit_accepts_a_timed_ghost() {
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
        assert_eq!((result.accepted, result.visible), (1, 0));
        assert!(crate_cells(&sim, &registry).is_empty());
        assert_eq!(
            sim.overlay_grid
                .as_ref()
                .expect("overlay grid")
                .cell(rx, ry)
                .overlay_id,
            None,
            "a post-precheck Mark failure leaves the Cell unchanged",
        );
        assert_eq!(sim.scenario_rng.state(), replay.state());
        let slot = sim.crate_authority.slots()[0];
        assert_eq!((slot.cell_x, slot.cell_y), (rx as i16, ry as i16));
        assert_eq!(slot.start_frame, sim.session.binary_frame as i32);
        assert_eq!(
            slot.aux, 0x40d1_9400,
            "constructor regen 10 owns upper=18000"
        );
        assert!(slot.duration > 0);
    }

    #[test]
    fn scenario_start_crate_every_nonzero_ground_occupation_bit_ghosts() {
        let rules = crate_ruleset("CrateMinimum=1\nCrateMaximum=1\n");
        let registry = crate_registry();
        let grid = PathGrid::test_all_passable(MAP, MAP);

        for bit in [1, 2, 4, 8, 16, 32, 64, 128] {
            let mut sim = sim_with_grid(0x0CC0_0000 + u64::from(bit));
            let (cell, _) = first_random_cell(&sim);
            sim.substrate
                .raw_cell_occupation
                .mark_ground(cell.0, cell.1, bit);
            let result = place_scenario_start_crates(&mut sim, &rules, &registry, Some(&grid), 1);
            assert_eq!(
                (result.accepted, result.visible),
                (1, 0),
                "raw occupation bit {bit:#04x} must reject Mark"
            );
            assert!(sim.crate_authority.slots()[0].cell_x != 0);
        }
    }

    #[test]
    fn scenario_start_crate_null_image_and_terrain_object_are_accepted_ghosts() {
        let null_rules = crate_ruleset_with_images(
            "none",
            "SILVER",
            "WATER",
            "CrateMinimum=1\nCrateMaximum=1\nCrateRegen=3\n",
        );
        let ordinary_rules = crate_ruleset("CrateMinimum=1\nCrateMaximum=1\nCrateRegen=3\n");
        let registry = crate_registry();
        let grid = PathGrid::test_all_passable(MAP, MAP);

        let mut null_sim = sim_with_grid(0xA011_0001);
        null_sim.session.binary_frame = u32::MAX;
        let (null_cell, mut null_replay) = first_random_cell(&null_sim);
        let timer_draw = null_replay.next_range_u32_inclusive(0, 0x7fff_fffe);
        let expected_timer = state::crate_timer_words(
            null_rules.crate_rules.regen,
            timer_draw,
            null_sim.session.binary_frame as i32,
        );
        let null_result =
            place_scenario_start_crates(&mut null_sim, &null_rules, &registry, Some(&grid), 1);
        assert_eq!((null_result.accepted, null_result.visible), (1, 0));
        assert!(
            null_sim
                .overlay_grid
                .as_ref()
                .unwrap()
                .iter_occupied()
                .next()
                .is_none()
        );
        let null_slot = null_sim.crate_authority.slots()[0];
        assert_eq!(
            (null_slot.cell_x, null_slot.cell_y),
            (null_cell.0 as i16, null_cell.1 as i16)
        );
        assert_eq!(
            (null_slot.start_frame, null_slot.aux, null_slot.duration),
            expected_timer
        );
        assert_eq!(null_sim.scenario_rng.state(), null_replay.state());

        let mut terrain_sim = sim_with_grid(0xA011_0002);
        let (cell, _) = first_random_cell(&terrain_sim);
        terrain_sim.production.terrain_object_cells.insert(cell, 77);
        let terrain_result = place_scenario_start_crates(
            &mut terrain_sim,
            &ordinary_rules,
            &registry,
            Some(&grid),
            1,
        );
        assert_eq!((terrain_result.accepted, terrain_result.visible), (1, 0));
        assert_eq!(
            (
                terrain_sim.crate_authority.slots()[0].cell_x,
                terrain_sim.crate_authority.slots()[0].cell_y
            ),
            (cell.0 as i16, cell.1 as i16)
        );
    }

    #[test]
    fn crate_placement_forced_post_precheck_failures_are_timed_ghosts() {
        let rules = crate_ruleset("CrateMinimum=1\nCrateMaximum=1\nCrateRegen=3\n");
        let registry = crate_registry();
        let grid = PathGrid::test_all_passable(MAP, MAP);

        for failure in [
            ForcedPostPrecheckFailure::Allocation,
            ForcedPostPrecheckFailure::Unlimbo,
            ForcedPostPrecheckFailure::Mark,
        ] {
            let mut sim = sim_with_grid(0xFA11_0000 + failure as u64);
            let result = place_scenario_start_crates_with_failure(
                &mut sim,
                &rules,
                &registry,
                Some(&grid),
                1,
                failure,
            );
            assert_eq!((result.accepted, result.visible), (1, 0));
            let slot = sim.crate_authority.slots()[0];
            assert!(!slot.is_empty());
            assert_eq!(slot.aux, 0x40b5_1800);
            assert!(
                sim.overlay_grid
                    .as_ref()
                    .unwrap()
                    .iter_occupied()
                    .next()
                    .is_none()
            );
        }
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

        assert_eq!((result.accepted, result.visible), (1, 1));
        assert_eq!(
            cells_with_overlay(&sim, &registry, "WATER"),
            vec![destination]
        );
        assert_eq!(sim.scenario_rng.state(), replay.state());
    }

    #[test]
    fn crate_placement_mark_uses_water_first_numeric_identity_alias() {
        let rules =
            crate_ruleset_with_images("WOOD", "SILVER", "WOOD", "CrateMinimum=1\nCrateMaximum=1\n");
        let registry = crate_registry();
        let mut sim = sim_with_grid(0xA11A_5001);
        let cell = (7, 9);
        let terrain_cell = sim
            .resolved_terrain
            .as_mut()
            .unwrap()
            .cell_mut(cell.0, cell.1)
            .unwrap();
        terrain_cell.speed_costs.track = Some(0);
        terrain_cell.speed_costs.float = Some(100);

        let result = validate_and_stamp_candidate(
            &mut sim,
            &rules.crate_rules,
            &registry,
            cell,
            ForcedPostPrecheckFailure::None,
        );

        assert_eq!(result, AcceptedCellResult::Visible);
        assert_eq!(
            sim.overlay_grid
                .as_ref()
                .unwrap()
                .cell(cell.0, cell.1)
                .overlay_id,
            registry.id_for_name("WOOD")
        );

        let mut zero_float = sim_with_grid(0xA11A_5002);
        let zero_cell = zero_float
            .resolved_terrain
            .as_mut()
            .unwrap()
            .cell_mut(cell.0, cell.1)
            .unwrap();
        zero_cell.speed_costs.track = Some(100);
        zero_cell.speed_costs.float = Some(0);
        assert_eq!(
            validate_and_stamp_candidate(
                &mut zero_float,
                &rules.crate_rules,
                &registry,
                cell,
                ForcedPostPrecheckFailure::None,
            ),
            AcceptedCellResult::Ghost,
            "the aliased selected identity must use Water/Float, not Wood/Track"
        );
    }

    #[test]
    fn crate_placement_bridge_selects_full_deck_byte_and_bypasses_speed_zero() {
        let rules = crate_ruleset("CrateMinimum=1\nCrateMaximum=1\n");
        let registry = crate_registry();
        let cell = (8, 11);

        let mut visible = sim_with_grid(0xB21D_0001);
        let terrain_cell = visible
            .resolved_terrain
            .as_mut()
            .unwrap()
            .cell_mut(cell.0, cell.1)
            .unwrap();
        terrain_cell.bridge_facts.raw_flags = BRIDGE_FLAG_STRUCTURAL;
        terrain_cell.speed_costs.track = Some(0);
        visible
            .substrate
            .raw_cell_occupation
            .mark_ground(cell.0, cell.1, 0x80);
        assert_eq!(
            validate_and_stamp_candidate(
                &mut visible,
                &rules.crate_rules,
                &registry,
                cell,
                ForcedPostPrecheckFailure::None,
            ),
            AcceptedCellResult::Visible,
            "structural bridge chooses empty deck and bypasses ground speed zero"
        );

        let mut ghost = sim_with_grid(0xB21D_0002);
        let terrain_cell = ghost
            .resolved_terrain
            .as_mut()
            .unwrap()
            .cell_mut(cell.0, cell.1)
            .unwrap();
        terrain_cell.bridge_facts.raw_flags = BRIDGE_FLAG_STRUCTURAL;
        terrain_cell.speed_costs.track = Some(100);
        ghost
            .substrate
            .raw_cell_occupation
            .mark_deck(cell.0, cell.1, 0x01);
        assert_eq!(
            validate_and_stamp_candidate(
                &mut ghost,
                &rules.crate_rules,
                &registry,
                cell,
                ForcedPostPrecheckFailure::None,
            ),
            AcceptedCellResult::Ghost,
            "any nonzero selected deck byte rejects Mark"
        );
    }

    #[test]
    fn crate_placement_steep_slope_uses_exact_raw_b2_exception() {
        let ordinary_rules = crate_ruleset("CrateMinimum=1\nCrateMaximum=1\n");
        let ordinary_registry = crate_registry();
        let cell = (12, 12);
        let mut ordinary = sim_with_grid(0xB200_0001);
        ordinary
            .resolved_terrain
            .as_mut()
            .unwrap()
            .cell_mut(cell.0, cell.1)
            .unwrap()
            .slope_type = 5;
        assert_eq!(
            validate_and_stamp_candidate(
                &mut ordinary,
                &ordinary_rules.crate_rules,
                &ordinary_registry,
                cell,
                ForcedPostPrecheckFailure::None,
            ),
            AcceptedCellResult::Ghost
        );

        let b2_registry = crate_registry_with_raw_b2();
        assert_eq!(b2_registry.id_for_name("STEEPCRATE"), Some(0xB2));
        let b2_rules = CrateRules {
            wood_crate_img: Some("STEEPCRATE".to_owned()),
            crate_img: Some("STEEPCRATE".to_owned()),
            water_crate_img: Some("STEEPCRATE".to_owned()),
            ..CrateRules::default()
        };
        let mut exempt = sim_with_grid(0xB200_0002);
        exempt
            .resolved_terrain
            .as_mut()
            .unwrap()
            .cell_mut(cell.0, cell.1)
            .unwrap()
            .slope_type = u8::MAX;
        assert_eq!(
            validate_and_stamp_candidate(
                &mut exempt,
                &b2_rules,
                &b2_registry,
                cell,
                ForcedPostPrecheckFailure::None,
            ),
            AcceptedCellResult::Visible
        );
    }

    #[test]
    fn place_crate_overlay_preserves_wall_owner_and_publishes_one_dirty_cell() {
        let rules = crate_ruleset("CrateMinimum=1\nCrateMaximum=1\n");
        let registry = crate_registry();
        let mut sim = sim_with_grid(0xD117_0001);
        let cell = (6, 13);
        let owner = sim.interner.intern("Owner");
        let grid = sim.overlay_grid.as_mut().unwrap();
        grid.cell_mut(cell.0, cell.1).wall_owner = Some(owner);
        assert!(grid.take_dirty_cells().is_empty());

        assert_eq!(
            validate_and_stamp_candidate(
                &mut sim,
                &rules.crate_rules,
                &registry,
                cell,
                ForcedPostPrecheckFailure::None,
            ),
            AcceptedCellResult::Visible
        );
        let grid = sim.overlay_grid.as_mut().unwrap();
        let written = grid.cell(cell.0, cell.1);
        assert_eq!(written.overlay_id, registry.id_for_name("WOOD"));
        assert_eq!(written.overlay_data, u8::MAX);
        assert_eq!(written.wall_owner, Some(owner));
        assert_eq!(grid.take_dirty_cells(), vec![cell]);
    }

    #[test]
    fn configured_noncrate_images_keep_native_mark_zero_and_road_data() {
        let registry = OverlayTypeRegistry::from_ini(
            &IniFile::from_str(
                "[OverlayTypes]\n0=PLAIN\n1=ROADBOX\n[PLAIN]\n[ROADBOX]\nLand=Road\n",
            ),
            None,
        );
        for (name, expected_data) in [("PLAIN", 0), ("ROADBOX", 1)] {
            let rules = CrateRules {
                wood_crate_img: Some(name.to_owned()),
                crate_img: Some(name.to_owned()),
                water_crate_img: Some(name.to_owned()),
                ..CrateRules::default()
            };
            let mut sim = sim_with_grid(0xDA7A_0000 + u64::from(expected_data));
            let cell = (6, 13);

            assert_eq!(
                validate_and_stamp_candidate(
                    &mut sim,
                    &rules,
                    &registry,
                    cell,
                    ForcedPostPrecheckFailure::None,
                ),
                AcceptedCellResult::Visible
            );
            let written = sim.overlay_grid.as_ref().unwrap().cell(cell.0, cell.1);
            assert_eq!(written.overlay_id, registry.id_for_name(name));
            assert_eq!(written.overlay_data, expected_data, "configured {name}");
        }
    }

    #[test]
    fn crate_random_rectangle_draws_signed_reversed_ranges_then_narrows_to_i16() {
        let mut sum_one = crate::sim::rng::SimRng::new(0x1401);
        let mut sum_one_expected = sum_one.clone();
        let sum_one_frame = CrateRandomFrame {
            left: 1,
            top: 1,
            width: 0,
            height: 0,
            radius_cap: 1,
        };
        let sum_one_cell = draw_crate_candidate(&mut sum_one, sum_one_frame);
        let expected_sum_one = (
            sum_one_expected.next_range_u32_inclusive(0, 1) as i32,
            sum_one_expected.next_range_u32_inclusive(0, 1) as i32,
        );
        assert_eq!(sum_one_cell, expected_sum_one);
        assert_eq!(sum_one.state(), sum_one_expected.state());

        let mut negative = crate::sim::rng::SimRng::new(0x1402);
        let mut negative_expected = negative.clone();
        let negative_frame = CrateRandomFrame {
            left: i32::MAX,
            top: i32::MIN,
            width: -2,
            height: -2,
            radius_cap: 0,
        };
        let negative_cell = draw_crate_candidate(&mut negative, negative_frame);
        let expected_negative = (
            i32::MAX
                .wrapping_add(-3 + negative_expected.next_range_u32_inclusive(0, 3) as i32)
                as i16 as i32,
            i32::MIN
                .wrapping_add(-3 + negative_expected.next_range_u32_inclusive(0, 3) as i32)
                as i16 as i32,
        );
        assert_eq!(negative_cell, expected_negative);
        assert_eq!(negative.state(), negative_expected.state());

        let mut wide = crate::sim::rng::SimRng::new(0x1403);
        let mut wide_expected = wide.clone();
        let wide_frame = CrateRandomFrame {
            left: 30_000,
            top: -30_000,
            width: 70_000,
            height: 70_000,
            radius_cap: CRATE_SNAP_RADIUS_CAP,
        };
        let wide_cell = draw_crate_candidate(&mut wide, wide_frame);
        let expected_wide = (
            30_000i32
                .wrapping_add(wide_expected.next_range_u32_inclusive(0, 69_999) as i32)
                as i16 as i32,
            (-30_000i32)
                .wrapping_add(wide_expected.next_range_u32_inclusive(0, 69_999) as i32)
                as i16 as i32,
        );
        assert_eq!(wide_cell, expected_wide);
        assert_eq!(wide.state(), wide_expected.state());
    }

    #[test]
    fn crate_random_frame_retains_nonpositive_signed_size_for_rng_before_empty_snap() {
        let mut sim = sim_with_grid(1);
        sim.playfield_bounds.as_mut().unwrap().base = 1;
        sim.playfield_size_height = Some(0);
        assert_eq!(
            crate_random_frame(&sim),
            Some(CrateRandomFrame {
                left: 1,
                top: 1,
                width: 0,
                height: 0,
                radius_cap: 1,
            })
        );

        sim.playfield_bounds.as_mut().unwrap().base = -3;
        sim.playfield_size_height = Some(-2);
        assert_eq!(
            crate_random_frame(&sim),
            Some(CrateRandomFrame {
                left: 1,
                top: 1,
                width: -6,
                height: -6,
                radius_cap: 0,
            })
        );
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
            assert_eq!((result.accepted, result.visible), (4, 4));

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

            // The real destination admits the drawn-cell FNPC movement and the
            // independently selected image's Mark movement. The nearer-to-(0,0)
            // decoy admits only the opposite FNPC type, so choosing the search
            // movement from anywhere but the draw lands on the decoy.
            let destination_cell = terrain
                .cell_mut(destination.0, destination.1)
                .expect("destination cell");
            destination_cell.speed_costs.float = Some(100);
            destination_cell.speed_costs.track = Some(100);
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

            assert_eq!((result.accepted, result.visible), (1, 1));
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
    fn gsi_01_04_crate_snap_zero_reference_uses_live_frame_modulo() {
        let registry = crate_registry();
        let mut sim = sim_with_grid(1);
        let mut terrain = uniform_terrain(false);
        for cell in &mut terrain.cells {
            cell.speed_costs.track = Some(0);
        }
        // Both preferred candidates are on ring 9. Engine ring order encounters
        // (29,11) first and (11,20) second, while nearest-to-origin would choose
        // (11,20). Native's zero reference is a sentinel, not a real target.
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

        sim.session.binary_frame = 0;
        assert_eq!(
            snap_to_passable(&sim, Some(&grid), (20, 20), CrateSurface::Land),
            Some((29, 11)),
            "frame zero selects the first preferred survivor, not nearest to (0,0)"
        );

        sim.session.binary_frame = 1;
        assert_eq!(
            snap_to_passable(&sim, Some(&grid), (20, 20), CrateSurface::Land),
            Some((11, 20)),
            "the live frame advances modulo the preferred pool"
        );
    }

    #[test]
    fn gsi_01_04_crate_snap_requires_native_candidate_anchor_diamond() {
        let mut sim = sim_with_grid(1);
        let mut terrain = uniform_terrain(false);
        for cell in &mut terrain.cells {
            cell.speed_costs.track = Some(0);
        }
        // Both candidates are valid cells inside the 40x40 terrain rectangle.
        // Ring order visits (8,3) first, but the native diamond below admits
        // (8,5) only: on flat terrain it requires 12 < x+y <= 26,
        // x-y < 14, and y-x < 6.
        terrain
            .cell_mut(8, 3)
            .expect("rectangular-only candidate")
            .speed_costs
            .track = Some(100);
        terrain
            .cell_mut(8, 5)
            .expect("in-diamond candidate")
            .speed_costs
            .track = Some(100);
        sim.resolved_terrain = Some(terrain);
        sim.playfield_bounds = Some(PlayfieldBounds {
            base: 10,
            off_fc: 2,
            off_100: 1,
            off_104: 10,
            off_108: 6,
        });
        sim.session.binary_frame = 0;
        let grid = PathGrid::test_all_passable(MAP, MAP);

        assert_eq!(
            snap_to_passable(&sim, Some(&grid), (9, 4), CrateSurface::Land),
            Some((8, 5)),
            "the earlier rectangularly valid but off-diamond anchor must be rejected"
        );

        sim.playfield_bounds = None;
        assert_eq!(
            snap_to_passable(&sim, Some(&grid), (9, 4), CrateSurface::Land),
            None,
            "missing MapClass playfield authority must reject, not bypass"
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
        assert_eq!((result.accepted, result.visible), (1, 1));
    }

    #[test]
    fn scenario_start_crate_draws_exact_active_rectangle_not_session_extent_or_localsize() {
        let rules = crate_ruleset("CrateMinimum=1\nCrateMaximum=1\n");
        let registry = crate_registry();
        let grid = PathGrid::test_all_passable(MAP, MAP);
        let mut sim = sim_with_grid(0x2468);
        sim.playfield_bounds.as_mut().unwrap().base = 14;
        sim.playfield_size_height = Some(9);
        sim.session.map_width = 37;
        let mut replay = sim.scenario_rng.clone();
        let expected = (
            1 + replay.next_range_u32_inclusive(0, 21) as u16,
            1 + replay.next_range_u32_inclusive(0, 21) as u16,
        );
        replay.next_range_u32_inclusive(0, 0x7fff_fffe);
        sim.session.local_left = 0;
        sim.session.local_top = 0;
        sim.session.local_width = 1;
        sim.session.local_height = 1;

        let result = place_scenario_start_crates(&mut sim, &rules, &registry, Some(&grid), 1);

        assert_eq!((result.accepted, result.visible), (1, 1));
        assert_eq!(crate_cells(&sim, &registry), vec![expected]);
        assert!((1..=22).contains(&expected.0) && (1..=22).contains(&expected.1));
        assert_ne!(expected, (0, 0), "draw lies outside the 1x1 LocalSize");
        assert_eq!(sim.scenario_rng.state(), replay.state());
    }
}
