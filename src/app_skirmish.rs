//! Skirmish opening seeding, deployable building detection, and overlay atlas construction.
//!
//! Extracted from app_init_helpers.rs for file-size limits.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::assets::asset_manager::AssetManager;
use crate::assets::pal_file::Palette;
use crate::map::entities::EntityCategory;
use crate::map::houses::{HouseColorMap, HouseRoster};
use crate::map::map_file::MapFile;
use crate::map::overlay::OverlayEntry;
use crate::map::overlay_types::{OverlayTypeRegistry, resolve_overlay_name_for_render};
use crate::map::resolved_terrain::ResolvedTerrainGrid;
use crate::map::waypoints;
use crate::map::waypoints::Waypoint;
use crate::render::batch::BatchRenderer;
use crate::render::bridge_atlas::{self, BridgeAtlas};
use crate::render::bridge_railing_atlas::{self, BridgeRailingAtlas, BridgeRailingTileBases};
use crate::render::gpu::GpuContext;
use crate::render::overlay_atlas::{self, OverlayAtlas};
use crate::rules::art_data::ArtRegistry;
use crate::rules::color_scheme::scheme_entry_for_priority;
use crate::rules::house_colors::HouseColorIndex;
use crate::rules::ini_parser::IniFile;
use crate::rules::ruleset::RuleSet;
use crate::sim::ai::AiPlayerState;
use crate::sim::house_state::{HouseDifficulty, HouseState};
use crate::sim::mission::{MissionId, MissionType};
use crate::sim::rng::SimRng;
use crate::sim::world::Simulation;
use crate::skirmish_launch::{
    LaunchCountry, LaunchStartPosition, LaunchTeam, SkirmishLaunchSession,
};
use crate::ui::main_menu::{SkirmishSettings, StartPosition};

pub(crate) fn seed_skirmish_opening_if_needed(
    sim: &mut Simulation,
    map_data: &MapFile,
    house_roster: &HouseRoster,
    rules: &RuleSet,
    height_map: &BTreeMap<(u16, u16), u8>,
    settings: &SkirmishSettings,
) -> Option<String> {
    // Seed MCVs whenever multiplayer start waypoints exist, even if the map
    // has pre-placed entities (e.g., oil derricks on Dustbowl). The waypoint
    // check is sufficient to distinguish multiplayer maps from campaign missions.
    let mut starts = waypoints::multiplayer_start_waypoints(&map_data.waypoints);
    if starts.len() < 2 {
        return None;
    }
    let houses = skirmish_house_candidates(house_roster);
    if houses.is_empty() {
        return None;
    }

    // If the player chose a specific start position, swap that waypoint to index 0
    // so the local player spawns there.
    if let StartPosition::Position(pos) = settings.start_position {
        let idx: usize = pos as usize;
        if idx < starts.len() && idx != 0 {
            starts.swap(0, idx);
        }
    }

    // Reorder houses so the player's chosen side is first (becomes local owner).
    let selected_side = settings.player_country.side();
    let houses = reorder_houses_for_side(houses, selected_side);

    let credits: i32 = settings.starting_credits;
    let pairings = starts.into_iter().zip(houses.into_iter());
    let mut spawned_mcvs: u32 = 0;
    let mut local_owner: Option<String> = None;
    for (start, house) in pairings.take(2) {
        if let Some(h) = crate::sim::house_state::house_state_for_owner_mut(
            &mut sim.houses,
            &house.name,
            &sim.interner,
        ) {
            h.credits = credits;
        }
        let mcv_type: &str = skirmish_mcv_type_for_house(house, rules);
        if sim
            .spawn_object(
                mcv_type,
                &house.name,
                start.rx,
                start.ry,
                64,
                rules,
                height_map,
            )
            .is_some()
        {
            spawned_mcvs += 1;
            if local_owner.is_none() {
                local_owner = Some(house.name.clone());
            }
            if let Some(h) = crate::sim::house_state::house_state_for_owner_mut(
                &mut sim.houses,
                &house.name,
                &sim.interner,
            ) {
                h.base_center = Some((start.rx, start.ry));
                h.waypoint_edge = crate::sim::house_state::closest_edge_for(
                    (start.rx, start.ry),
                    sim.fog.width as u32,
                    sim.fog.height as u32,
                );
            }
        } else {
            log::warn!(
                "Failed to seed opening MCV '{}' for {} at waypoint {} ({},{})",
                mcv_type,
                house.name,
                start.index,
                start.rx,
                start.ry
            );
        }
    }
    if spawned_mcvs > 0 {
        log::info!(
            "Seeded {} skirmish opening MCV(s) with {} credits each",
            spawned_mcvs,
            credits
        );
    }
    local_owner
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkirmishLaunchApplyResult {
    pub(crate) local_owner: Option<String>,
    pub(crate) spawned_mcvs: u32,
    pub(crate) active_slots: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NormalizedSkirmishSlot {
    owner_name: String,
    country: LaunchCountry,
    color_index: u8,
    start_position: LaunchStartPosition,
    team: LaunchTeam,
    is_human: bool,
    difficulty: HouseDifficulty,
}

pub(crate) fn house_color_map_for_launch_session(
    session: &SkirmishLaunchSession,
    house_roster: &HouseRoster,
) -> HouseColorMap {
    let mut colors = HouseColorMap::new();
    for house in &house_roster.houses {
        if !is_playable_faction_name(&house.name) {
            colors.insert(house.name.clone(), house.color);
        }
    }
    for slot in normalized_launch_slots(session) {
        // `slot.color_index` is the gamemd color *priority* (lobby slot order);
        // resolve it to a `[Colors]` entry index via the priority LUT + /2 doubling.
        let entry = scheme_entry_for_priority(slot.color_index as i32) as u8;
        colors.insert(slot.owner_name, HouseColorIndex(entry));
    }
    colors
}

/// Apply an already validated explicit session without placeholder RNG draws.
pub(crate) fn apply_explicit_skirmish_launch_session(
    sim: &mut Simulation,
    map_data: &MapFile,
    house_roster: &HouseRoster,
    rules: &RuleSet,
    height_map: &BTreeMap<(u16, u16), u8>,
    resolved_terrain: &ResolvedTerrainGrid,
    session: &SkirmishLaunchSession,
) -> SkirmishLaunchApplyResult {
    apply_resolved_skirmish_launch_session(
        sim,
        map_data,
        house_roster,
        rules,
        height_map,
        resolved_terrain,
        session,
    )
}

fn apply_resolved_skirmish_launch_session(
    sim: &mut Simulation,
    map_data: &MapFile,
    house_roster: &HouseRoster,
    rules: &RuleSet,
    height_map: &BTreeMap<(u16, u16), u8>,
    resolved_terrain: &ResolvedTerrainGrid,
    session: &SkirmishLaunchSession,
) -> SkirmishLaunchApplyResult {
    let slots = normalized_launch_slots(session);
    sim.session.game_options = session
        .options
        .to_game_options(session.opponents.len() as i32);

    sim.houses.clear();
    sim.session.house_order.clear();
    sim.ai_players.clear();
    populate_launch_houses(sim, &slots, rules);
    populate_special_houses(sim, house_roster, rules);
    sim.house_alliances = launch_alliance_map(house_roster, &slots, &session.mode);
    recount_house_owned_counts(sim);

    let bounds = NativeStartBounds::from_session(sim, resolved_terrain);
    let starts = native_gather_start_positions(
        &map_data.waypoints,
        slots.len(),
        resolved_terrain,
        &sim.substrate.occupancy,
        bounds,
        &mut sim.scenario_rng,
    );
    let assignments = if session
        .mode
        .override_file
        .eq_ignore_ascii_case("MPCoopMD.ini")
    {
        let human_start_spots = map_data
            .ini
            .section("Header")
            .and_then(|header| header.get_i32("NumCoopHumanStartSpots"))
            .unwrap_or(0)
            .max(0) as usize;
        native_assign_cooperative_launch_starts(
            session,
            &starts,
            human_start_spots,
            &mut sim.scenario_rng,
        )
    } else {
        native_assign_launch_starts(session, &starts, &mut sim.scenario_rng)
    };
    // Session start-slot -> house table: filled after the random-assignment
    // draws, before tick 0 — lockstep state (hashed + serialized).
    sim.session.start_slot_houses.clear();
    for (slot_idx, waypoint) in &assignments {
        if let Some(slot) = slots.get(*slot_idx) {
            let owner = sim.interner.intern(&slot.owner_name);
            sim.session.start_slot_houses.insert(waypoint.index, owner);
        }
    }
    let mut spawned_mcvs = 0;
    let mut local_owner = slots.first().map(|slot| slot.owner_name.clone());

    assign_launch_base_centers(sim, &slots, &assignments);

    if session.options.bases {
        for (slot_idx, waypoint) in &assignments {
            let Some(slot) = slots.get(*slot_idx) else {
                continue;
            };
            let mcv_type = launch_mcv_type_for_country(slot.country, rules);
            if place_starting_mcv(
                sim,
                mcv_type,
                &slot.owner_name,
                waypoint.rx,
                waypoint.ry,
                bounds,
                rules,
                height_map,
                resolved_terrain,
            )
            .is_some()
            {
                spawned_mcvs += 1;
            } else {
                log::warn!(
                    "Failed to seed session MCV '{}' for {} at waypoint {} ({},{})",
                    mcv_type,
                    slot.owner_name,
                    waypoint.index,
                    waypoint.rx,
                    waypoint.ry
                );
                if slot.is_human {
                    local_owner = None;
                }
            }
        }

        if spawned_mcvs > 0 {
            log::info!(
                "Seeded {} session skirmish MCV(s) for {} active slot(s)",
                spawned_mcvs,
                slots.len()
            );
        }
    }

    seed_starting_extra_units(
        sim,
        &slots,
        rules,
        height_map,
        resolved_terrain,
        bounds,
        session.options.unit_count,
        map_data
            .special_flags
            .initial_veteran
            .unwrap_or(rules.initial_veteran),
    );

    // The selected-mode starting-force orchestrator advances Scenario RNG once
    // after the complete house pass, including zero-budget runs.
    let _ = sim.scenario_rng.next_range_u32_inclusive(0, 0xffff);

    SkirmishLaunchApplyResult {
        local_owner,
        spawned_mcvs,
        active_slots: slots.len(),
    }
}

fn assign_launch_base_centers(
    sim: &mut Simulation,
    slots: &[NormalizedSkirmishSlot],
    assignments: &[(usize, Waypoint)],
) {
    for (slot_idx, waypoint) in assignments {
        let Some(slot) = slots.get(*slot_idx) else {
            continue;
        };
        if let Some(house) = crate::sim::house_state::house_state_for_owner_mut(
            &mut sim.houses,
            &slot.owner_name,
            &sim.interner,
        ) {
            house.base_center = Some((waypoint.rx, waypoint.ry));
            house.waypoint_edge = crate::sim::house_state::closest_edge_for(
                (waypoint.rx, waypoint.ry),
                sim.fog.width as u32,
                sim.fog.height as u32,
            );
        }
    }
}

fn normalized_launch_slots(session: &SkirmishLaunchSession) -> Vec<NormalizedSkirmishSlot> {
    let mut slots = Vec::with_capacity(1 + session.opponents.len());
    slots.push(NormalizedSkirmishSlot {
        owner_name: session.player_name.clone(),
        country: session.local.country,
        color_index: session.local.color_index,
        start_position: session.local.start_position,
        team: session.local.team,
        is_human: true,
        difficulty: HouseDifficulty::Normal,
    });
    for (idx, opponent) in session.opponents.iter().enumerate() {
        slots.push(NormalizedSkirmishSlot {
            owner_name: format!("Computer{}", idx + 1),
            country: opponent.country,
            color_index: opponent.color_index,
            start_position: opponent.start_position,
            team: opponent.team,
            is_human: false,
            difficulty: HouseDifficulty::from_native(opponent.difficulty.as_i32())
                .expect("AiDifficulty uses native HouseClass discriminants"),
        });
    }
    slots
}

fn populate_special_houses(sim: &mut Simulation, house_roster: &HouseRoster, rules: &RuleSet) {
    for special_name in ["Neutral", "Special"] {
        let definition = house_roster
            .houses
            .iter()
            .find(|house| house.name.eq_ignore_ascii_case(special_name));
        let name = definition.map_or(special_name, |house| house.name.as_str());
        let name_id = sim.interner.intern(name);
        let country_name = definition
            .and_then(|house| house.country.as_deref())
            .unwrap_or(special_name);
        let country_id = Some(sim.interner.intern(country_name));
        let declared_side = definition.and_then(|house| house.side.as_deref());
        let side_idx = crate::sim::house_state::resolve_house_side_index(
            rules,
            Some(country_name),
            declared_side,
            crate::sim::house_state::side_index_from_name(declared_side),
        );
        let mut house = HouseState::new(
            name_id,
            side_idx,
            country_id,
            false,
            sim.session.game_options.starting_credits,
            sim.session.game_options.tech_level,
        );
        // Stock Neutral/Special are MultiplayPassive, which keeps them out of
        // defeat evaluation and out of the game-over alive scan. `country_name`
        // is always concrete here — the roster's `Country=` when the map named
        // one, otherwise the special house's own `[Countries]` entry, which is
        // what this house is being built as.
        house.multiplay_passive =
            crate::sim::house_state::resolve_multiplay_passive(Some(rules), Some(country_name));
        sim.houses.insert(name_id, house);
        sim.session.house_order.push(name_id);
    }
}

fn populate_launch_houses(sim: &mut Simulation, slots: &[NormalizedSkirmishSlot], rules: &RuleSet) {
    for slot in slots {
        let name_id = sim.interner.intern(&slot.owner_name);
        let country_name = slot.country.country_name();
        let country_id = sim.interner.intern(country_name);
        let side_index = crate::sim::house_state::resolve_house_side_index(
            rules,
            Some(country_name),
            None,
            slot.country.side_index(),
        );
        let mut house = HouseState::new(
            name_id,
            side_index,
            Some(country_id),
            slot.is_human,
            sim.session.game_options.starting_credits,
            sim.session.game_options.tech_level,
        );
        house.difficulty = slot.difficulty;
        house.multiplay_passive =
            crate::sim::house_state::resolve_multiplay_passive(Some(rules), Some(country_name));
        sim.houses.insert(name_id, house);
        sim.session.house_order.push(name_id);
        if !slot.is_human {
            sim.ai_players.push(AiPlayerState::new(name_id));
            log::info!("AI player registered: {}", slot.owner_name);
        }
    }
}

fn normalize_house_key(name: &str) -> String {
    name.trim().to_ascii_uppercase()
}

fn launch_alliance_map(
    house_roster: &HouseRoster,
    slots: &[NormalizedSkirmishSlot],
    mode: &crate::skirmish_launch::SkirmishLaunchMode,
) -> crate::map::houses::HouseAllianceMap {
    let mut alliances = house_roster.alliance_map();
    for slot in slots {
        alliances
            .entry(normalize_house_key(&slot.owner_name))
            .or_default();
    }
    for left in slots {
        let LaunchTeam::Team(team) = left.team else {
            continue;
        };
        for right in slots {
            if left.owner_name == right.owner_name || right.team != LaunchTeam::Team(team) {
                continue;
            }
            let left_key = normalize_house_key(&left.owner_name);
            let right_key = normalize_house_key(&right.owner_name);
            alliances
                .entry(left_key.clone())
                .or_default()
                .insert(right_key.clone());
            alliances.entry(right_key).or_default().insert(left_key);
        }
    }
    if mode.override_file.eq_ignore_ascii_case("MPCoopMD.ini") {
        for left in slots {
            for right in slots {
                if left.owner_name == right.owner_name || left.is_human != right.is_human {
                    continue;
                }
                let left_key = normalize_house_key(&left.owner_name);
                let right_key = normalize_house_key(&right.owner_name);
                alliances
                    .entry(left_key.clone())
                    .or_default()
                    .insert(right_key.clone());
                alliances.entry(right_key).or_default().insert(left_key);
            }
        }
    }
    alliances
}

fn recount_house_owned_counts(sim: &mut Simulation) {
    for house in sim.houses.values_mut() {
        house.owned_building_count = 0;
        house.owned_unit_count = 0;
    }
    let counts: Vec<_> = sim
        .entities()
        .values()
        .map(|entity| (entity.owner, entity.category))
        .collect();
    for (owner, category) in counts {
        let Some(house) = sim.houses.get_mut(&owner) else {
            continue;
        };
        match category {
            EntityCategory::Structure => house.owned_building_count += 1,
            _ => house.owned_unit_count += 1,
        }
    }
}

/// Assign participants to original map waypoints without terrain fallback.
///
/// Explicit requests reserve their waypoint first. Remaining `Auto` requests
/// then take the first still-free waypoint in participant order. This helper
/// intentionally performs no randomized/distance selection and consumes no RNG.
pub(crate) fn original_launch_start_assignments(
    session: &SkirmishLaunchSession,
    starts: &[Waypoint],
) -> Vec<(usize, Waypoint)> {
    let requested_starts: Vec<LaunchStartPosition> = std::iter::once(session.local.start_position)
        .chain(
            session
                .opponents
                .iter()
                .map(|opponent| opponent.start_position),
        )
        .collect();
    let mut assignments: Vec<Option<Waypoint>> = vec![None; requested_starts.len()];
    let mut used_start_indices = BTreeSet::new();

    for (idx, requested) in requested_starts.iter().enumerate() {
        let LaunchStartPosition::Position(position) = requested else {
            continue;
        };
        let Some(start) = starts
            .iter()
            .find(|start| start.index == u32::from(*position))
        else {
            continue;
        };
        if used_start_indices.insert(start.index) {
            assignments[idx] = Some(*start);
        }
    }

    for (idx, requested) in requested_starts.iter().enumerate() {
        if assignments[idx].is_some() || *requested != LaunchStartPosition::Auto {
            continue;
        }
        let Some(start) = starts
            .iter()
            .find(|start| !used_start_indices.contains(&start.index))
        else {
            continue;
        };
        used_start_indices.insert(start.index);
        assignments[idx] = Some(*start);
    }

    assignments
        .into_iter()
        .enumerate()
        .filter_map(|(idx, start)| start.map(|start| (idx, start)))
        .collect()
}

#[derive(Debug, Clone, Copy)]
struct NativeStartBounds {
    min_rx: u16,
    min_ry: u16,
    width: u16,
    height: u16,
}

impl NativeStartBounds {
    fn from_session(sim: &Simulation, terrain: &ResolvedTerrainGrid) -> Self {
        if sim.session.local_width != 0 && sim.session.local_height != 0 {
            Self {
                min_rx: sim.session.local_left,
                min_ry: sim.session.local_top,
                width: sim.session.local_width,
                height: sim.session.local_height,
            }
        } else {
            Self {
                min_rx: 0,
                min_ry: 0,
                width: terrain.width(),
                height: terrain.height(),
            }
        }
    }

    fn max_rx(self) -> u16 {
        self.min_rx.saturating_add(self.width.saturating_sub(1))
    }

    fn max_ry(self) -> u16 {
        self.min_ry.saturating_add(self.height.saturating_sub(1))
    }

    fn clamp(self, rx: i32, ry: i32) -> (u16, u16) {
        (
            rx.clamp(i32::from(self.min_rx), i32::from(self.max_rx())) as u16,
            ry.clamp(i32::from(self.min_ry), i32::from(self.max_ry())) as u16,
        )
    }

    fn contains(self, rx: u16, ry: u16) -> bool {
        rx >= self.min_rx && rx <= self.max_rx() && ry >= self.min_ry && ry <= self.max_ry()
    }
}

/// Build the native multiplayer-start vector before assigning houses.
///
/// Only waypoint indices below the computed target count are examined. When
/// authored starts are deficient, each retry consumes the two asymmetric
/// ranged draws and runs the 8x8 nearby-passable search; invalid searches do
/// not advance the vector and have no artificial retry cap.
fn native_gather_start_positions(
    waypoints: &HashMap<u32, Waypoint>,
    participant_count: usize,
    terrain: &ResolvedTerrainGrid,
    occupancy: &crate::sim::occupancy::OccupancyGrid,
    bounds: NativeStartBounds,
    rng: &mut SimRng,
) -> Vec<Waypoint> {
    let authored_prefix = (0..8u32)
        .take_while(|index| waypoints.contains_key(index))
        .count();
    let target_count = authored_prefix.max(participant_count);
    let mut starts = Vec::with_capacity(target_count);

    for index in 0..target_count as u32 {
        if let Some(waypoint) = waypoints.get(&index) {
            starts.push(*waypoint);
        }
    }

    while starts.len() < target_count {
        let x_span = u32::from(bounds.width.saturating_sub(10));
        let y_high = u32::from(bounds.height.saturating_sub(10));
        let seed_rx = rng
            .next_range_u32_inclusive(0, x_span)
            .wrapping_add(u32::from(bounds.min_rx))
            .wrapping_add(10) as u16;
        let seed_ry = rng
            .next_range_u32_inclusive(10, y_high)
            .wrapping_add(u32::from(bounds.min_ry)) as u16;

        let Some((rx, ry)) = find_nearby_start_rect(terrain, occupancy, bounds, seed_rx, seed_ry)
        else {
            continue;
        };
        starts.push(Waypoint {
            index: starts.len() as u32,
            rx,
            ry,
        });
    }

    starts
}

/// Reduced active call-shape of FootClass::Find_Nearby_Passable_Cell used by
/// start gathering: scan square rings from the seed, finish the first ring
/// containing an 8x8 passable candidate, and select its first candidate at
/// frame zero.
fn find_nearby_start_rect(
    terrain: &ResolvedTerrainGrid,
    occupancy: &crate::sim::occupancy::OccupancyGrid,
    bounds: NativeStartBounds,
    seed_rx: u16,
    seed_ry: u16,
) -> Option<(u16, u16)> {
    for radius in 0..32i32 {
        let mut ring = Vec::new();
        let r = radius;
        for dx in -r..=r {
            ring.push((i32::from(seed_rx) + dx, i32::from(seed_ry) - r));
            if r != 0 {
                ring.push((i32::from(seed_rx) + dx, i32::from(seed_ry) + r));
            }
        }
        for dy in (1 - r)..r {
            ring.push((i32::from(seed_rx) - r, i32::from(seed_ry) + dy));
            if r != 0 {
                ring.push((i32::from(seed_rx) + r, i32::from(seed_ry) + dy));
            }
        }

        let mut accepted = Vec::new();
        for (rx, ry) in ring {
            if rx < i32::from(bounds.min_rx)
                || ry < i32::from(bounds.min_ry)
                || rx + i32::from(DEFICIENT_START_RECT_W) - 1 > i32::from(bounds.max_rx())
                || ry + i32::from(DEFICIENT_START_RECT_H) - 1 > i32::from(bounds.max_ry())
            {
                continue;
            }
            let (rx, ry) = (rx as u16, ry as u16);
            if deficient_start_rect_track_passable(terrain, occupancy, rx, ry) {
                accepted.push((rx, ry));
                if accepted.len() == 24 {
                    break;
                }
            }
        }
        if let Some(first) = accepted.first().copied() {
            return Some(first);
        }
    }
    None
}

/// Assign the gathered vector in HouseClass order: every human first, then
/// AI. Explicit starts populate a last-writer-wins table; automatic selection
/// follows the retail random/near/far branches and their exact draw points.
fn native_assign_launch_starts(
    session: &SkirmishLaunchSession,
    starts: &[Waypoint],
    rng: &mut SimRng,
) -> Vec<(usize, Waypoint)> {
    if starts.is_empty() {
        return Vec::new();
    }

    let requested: Vec<LaunchStartPosition> = std::iter::once(session.local.start_position)
        .chain(
            session
                .opponents
                .iter()
                .map(|opponent| opponent.start_position),
        )
        .collect();
    let mut explicit_owner = vec![None; starts.len()];
    for (slot, request) in requested.iter().enumerate() {
        let LaunchStartPosition::Position(index) = request else {
            continue;
        };
        if let Some(owner) = explicit_owner.get_mut(usize::from(*index)) {
            *owner = Some(slot);
        }
    }

    let mut occupied = vec![false; starts.len()];
    let mut assigned = vec![None; requested.len()];
    for (slot, is_human) in (0..requested.len())
        .map(|slot| (slot, slot == 0))
        .collect::<Vec<_>>()
    {
        let explicit = explicit_owner.iter().position(|owner| *owner == Some(slot));
        let start_index =
            explicit.unwrap_or_else(|| choose_automatic_start(starts, &occupied, is_human, rng));
        occupied[start_index] = true;
        assigned[slot] = Some(starts[start_index]);
    }

    assigned
        .into_iter()
        .enumerate()
        .filter_map(|(slot, start)| start.map(|start| (slot, start)))
        .collect()
}

/// Cooperative's custom start callback partitions authored positions into a
/// human prefix and an AI suffix. The explicit Scenario start table reserves
/// all of its entries before HouseClass iteration; automatic human placement
/// draws within the prefix and probes forward, while the remaining houses
/// take the first free suffix entry without a draw.
fn native_assign_cooperative_launch_starts(
    session: &SkirmishLaunchSession,
    starts: &[Waypoint],
    human_start_spots: usize,
    rng: &mut SimRng,
) -> Vec<(usize, Waypoint)> {
    if starts.is_empty() {
        return Vec::new();
    }

    let requested: Vec<LaunchStartPosition> = std::iter::once(session.local.start_position)
        .chain(
            session
                .opponents
                .iter()
                .map(|opponent| opponent.start_position),
        )
        .collect();
    let mut explicit_owner = vec![None; starts.len()];
    for (slot, request) in requested.iter().enumerate() {
        let LaunchStartPosition::Position(index) = request else {
            continue;
        };
        if let Some(owner) = explicit_owner.get_mut(usize::from(*index)) {
            *owner = Some(slot);
        }
    }

    let mut occupied: Vec<bool> = explicit_owner.iter().map(Option::is_some).collect();
    let human_house_count = 1usize;
    let human_start_spots = human_start_spots.min(starts.len());
    let mut assigned = vec![None; requested.len()];

    for slot in 0..requested.len() {
        let explicit = explicit_owner.iter().position(|owner| *owner == Some(slot));
        let start_index = if let Some(explicit) = explicit {
            explicit
        } else {
            let occupied_count = occupied.iter().filter(|used| **used).count();
            if occupied_count < human_house_count && human_start_spots != 0 {
                let mut candidate =
                    rng.next_range_u32_inclusive(0, human_start_spots as u32 - 1) as usize;
                while occupied[candidate] {
                    candidate = (candidate + 1) % human_start_spots;
                }
                candidate
            } else {
                (human_start_spots..starts.len())
                    .find(|candidate| !occupied[*candidate])
                    .or_else(|| occupied.iter().position(|used| !*used))
                    .expect("participant count cannot exceed gathered starts")
            }
        };
        occupied[start_index] = true;
        assigned[slot] = Some(starts[start_index]);
    }

    assigned
        .into_iter()
        .enumerate()
        .filter_map(|(slot, start)| start.map(|start| (slot, start)))
        .collect()
}

fn choose_automatic_start(
    starts: &[Waypoint],
    occupied: &[bool],
    is_human: bool,
    rng: &mut SimRng,
) -> usize {
    let used_count = occupied.iter().filter(|used| **used).count();
    if used_count == 0 && is_human {
        return rng.next_range_u32_inclusive(0, starts.len() as u32 - 1) as usize;
    }

    let free: Vec<usize> = occupied
        .iter()
        .enumerate()
        .filter_map(|(index, used)| (!*used).then_some(index))
        .collect();
    if used_count == 2 && !is_human {
        let ordinal = rng.next_range_u32_inclusive(0, free.len() as u32 - 1) as usize;
        return free[ordinal];
    }

    let distance_sum = |candidate: usize| -> i32 {
        occupied
            .iter()
            .enumerate()
            .filter(|(_, used)| **used)
            .map(|(other, _)| native_start_distance(starts[candidate], starts[other]))
            .sum()
    };
    let mut iter = free.into_iter();
    let mut selected = iter
        .next()
        .expect("participant count cannot exceed gathered starts");
    let mut selected_sum = distance_sum(selected);
    for candidate in iter {
        let sum = distance_sum(candidate);
        let replace = if used_count == 1 {
            sum < selected_sum
        } else {
            sum > selected_sum
        };
        if replace {
            selected = candidate;
            selected_sum = sum;
        }
    }
    selected
}

fn native_start_distance(left: Waypoint, right: Waypoint) -> i32 {
    let dx = i32::from((left.rx as i16).wrapping_sub(right.rx as i16));
    let dy = i32::from((left.ry as i16).wrapping_sub(right.ry as i16));
    crate::map::rmg::x87::approx_sqrt_f32(dx.wrapping_mul(dx).wrapping_add(dy.wrapping_mul(dy)))
        as i32
}

const DEFICIENT_START_RECT_W: u16 = 8;
const DEFICIENT_START_RECT_H: u16 = 8;

fn deficient_start_rect_track_passable(
    terrain: &ResolvedTerrainGrid,
    occupancy: &crate::sim::occupancy::OccupancyGrid,
    rx: u16,
    ry: u16,
) -> bool {
    crate::sim::cell_rect::check_passability_rect(
        crate::sim::cell_rect::CellRectPassabilityContext {
            rect: crate::sim::cell_rect::CellRect::new(
                i32::from(rx),
                i32::from(ry),
                i32::from(DEFICIENT_START_RECT_W),
                i32::from(DEFICIENT_START_RECT_H),
            ),
            speed_type: crate::rules::locomotor_type::SpeedType::Track,
            required_zone_id: None,
            movement_zone: crate::rules::locomotor_type::MovementZone::Normal,
            required_height_or_level: None,
            bridge_aware_zone: false,
            reject_any_overlay: false,
            path_grid: None,
            resolved_terrain: Some(terrain),
            overlay_grid: None,
            occupancy: Some(occupancy),
            zone_grid: None,
        },
    )
}

const STARTING_MCV_FACING: u8 = 64;
const STARTING_MCV_FALLBACK_MAX_RADIUS: i32 = 31;
const STARTING_EXTRA_UNIT_FALLBACK_START_RADIUS: i32 = 4;
const STARTING_MCV_FALLBACK_DIRECTIONS: &[(i32, i32)] = &[
    (0, -1),
    (1, -1),
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
    (-1, -1),
];

fn place_starting_mcv(
    sim: &mut Simulation,
    mcv_type: &str,
    owner: &str,
    base_rx: u16,
    base_ry: u16,
    bounds: NativeStartBounds,
    rules: &RuleSet,
    height_map: &BTreeMap<(u16, u16), u8>,
    resolved_terrain: &ResolvedTerrainGrid,
) -> Option<u64> {
    place_starting_object_near_base(
        sim,
        mcv_type,
        owner,
        base_rx,
        base_ry,
        STARTING_MCV_FACING,
        1,
        bounds,
        rules,
        height_map,
        resolved_terrain,
    )
}

fn place_starting_object_near_base(
    sim: &mut Simulation,
    type_id: &str,
    owner: &str,
    base_rx: u16,
    base_ry: u16,
    facing: u8,
    start_radius: i32,
    bounds: NativeStartBounds,
    rules: &RuleSet,
    height_map: &BTreeMap<(u16, u16), u8>,
    resolved_terrain: &ResolvedTerrainGrid,
) -> Option<u64> {
    let category = rules.object(type_id)?.category;
    if starting_object_cell_placeable(sim, resolved_terrain, bounds, base_rx, base_ry, category) {
        return sim.spawn_object(type_id, owner, base_rx, base_ry, facing, rules, height_map);
    }

    for radius in start_radius..=STARTING_MCV_FALLBACK_MAX_RADIUS {
        let start_direction = sim.scenario_rng.next_range_u32_inclusive(0, 7) as usize;
        for jitter_pass in 0..2 {
            for offset in 0..8 {
                let direction = (start_direction + offset) & 7;
                let (dx, dy) = STARTING_MCV_FALLBACK_DIRECTIONS[direction];
                let (mut rx, mut ry) = bounds.clamp(
                    i32::from(base_rx) + dx * radius,
                    i32::from(base_ry) + dy * radius,
                );

                if jitter_pass != 0 {
                    let x_amount = sim.scenario_rng.next_range_u32_inclusive(0, 1) as i32;
                    let x_sign = sim.scenario_rng.next_range_u32_inclusive(0, 99);
                    let y_amount = sim.scenario_rng.next_range_u32_inclusive(0, 1) as i32;
                    let y_sign = sim.scenario_rng.next_range_u32_inclusive(0, 99);
                    let jittered = bounds.clamp(
                        i32::from(rx) + if x_sign < 50 { x_amount } else { -x_amount },
                        i32::from(ry) + if y_sign < 50 { y_amount } else { -y_amount },
                    );
                    rx = jittered.0;
                    ry = jittered.1;
                }

                if (rx, ry) == (base_rx, base_ry)
                    || !starting_object_cell_placeable(
                        sim,
                        resolved_terrain,
                        bounds,
                        rx,
                        ry,
                        category,
                    )
                {
                    continue;
                }
                if let Some(id) =
                    sim.spawn_object(type_id, owner, rx, ry, facing, rules, height_map)
                {
                    return Some(id);
                }
            }
        }
    }

    None
}

#[derive(Debug, Clone)]
struct StartingUnitCandidate {
    type_id: String,
    cost: i32,
}

const STARTING_EXTRA_UNIT_MAX_PLACEMENT_FAILURES: u8 = 20;

fn starting_unit_prefers_vehicle(spent: i32, budget: i32) -> bool {
    // gamemd 0x005D7337..0x005D7349 compares remaining budget against
    // trunc(initial_budget / 3). Expressed through `spent`, the strict
    // boundary is initial_budget - trunc(initial_budget / 3).
    spent < budget.wrapping_sub(budget / 3)
}

fn seed_starting_extra_units(
    sim: &mut Simulation,
    slots: &[NormalizedSkirmishSlot],
    rules: &RuleSet,
    height_map: &BTreeMap<(u16, u16), u8>,
    resolved_terrain: &ResolvedTerrainGrid,
    bounds: NativeStartBounds,
    unit_count: i32,
    initial_veteran: bool,
) -> u32 {
    if unit_count <= 0 {
        return 0;
    }
    let budget = starting_unit_budget(
        rules,
        slots,
        sim.session.game_options.tech_level,
        unit_count,
    );
    if budget <= 0 {
        return 0;
    }
    let mut spawned = 0;

    for slot in slots {
        let Some(base_center) = crate::sim::house_state::house_state_for_owner(
            &sim.houses,
            &slot.owner_name,
            &sim.interner,
        )
        .and_then(|house| house.base_center) else {
            continue;
        };
        let candidates = starting_unit_candidates_for_country(
            rules,
            slot.country,
            sim.session.game_options.tech_level,
        );
        if candidates.vehicles.is_empty() && candidates.infantry.is_empty() {
            continue;
        }
        let mut spent = 0i32;
        let mut placement_failures_left = STARTING_EXTRA_UNIT_MAX_PLACEMENT_FAILURES;
        while spent < budget && placement_failures_left != 0 {
            let prefer_vehicle = starting_unit_prefers_vehicle(spent, budget);
            let preferred = if prefer_vehicle {
                &candidates.vehicles
            } else {
                &candidates.infantry
            };
            let pool = if preferred.is_empty() {
                if prefer_vehicle {
                    &candidates.infantry
                } else {
                    break;
                }
            } else {
                preferred
            };
            let candidate_index =
                sim.scenario_rng
                    .next_range_u32_inclusive(0, pool.len() as u32 - 1) as usize;
            let candidate = &pool[candidate_index];
            let Some(stable_id) = place_starting_object_near_base(
                sim,
                &candidate.type_id,
                &slot.owner_name,
                base_center.0,
                base_center.1,
                STARTING_MCV_FACING,
                STARTING_EXTRA_UNIT_FALLBACK_START_RADIUS,
                bounds,
                rules,
                height_map,
                resolved_terrain,
            ) else {
                placement_failures_left -= 1;
                continue;
            };

            if initial_veteran {
                if let Some(entity) = sim.entities_mut().get_mut(stable_id) {
                    entity.veterancy = 200;
                }
            }
            let mission = if slot.is_human {
                MissionType::Guard
            } else {
                MissionType::AreaGuard
            };
            let _ = sim.mission_assign_exact(
                stable_id,
                MissionId::from_known(mission),
                sim.session.binary_frame,
            );
            spawned += 1;
            spent = spent.wrapping_add(candidate.cost);
        }
    }

    spawned
}

fn starting_unit_budget(
    rules: &RuleSet,
    slots: &[NormalizedSkirmishSlot],
    tech_level: i32,
    unit_count: i32,
) -> i32 {
    if unit_count <= 0 {
        return 0;
    }
    let mut total_cost: i32 = 0;
    let mut eligible_count: i32 = 0;
    for id in &rules.vehicle_ids {
        let Some(object) = rules.object(id) else {
            continue;
        };
        if !object.allowed_to_start_in_multiplayer
            || object.tech_level > tech_level
            || !slots
                .iter()
                .any(|slot| launch_country_can_own_object(slot.country, object))
            || rules
                .general
                .base_unit_types
                .iter()
                .any(|base_unit| base_unit.eq_ignore_ascii_case(&object.id))
        {
            continue;
        }
        eligible_count += 1i32;
        total_cost = total_cost.wrapping_add(object.cost);
    }
    for id in &rules.infantry_ids {
        let Some(object) = rules.object(id) else {
            continue;
        };
        if !object.allowed_to_start_in_multiplayer
            || object.tech_level > tech_level
            || !slots
                .iter()
                .any(|slot| launch_country_can_own_object(slot.country, object))
        {
            continue;
        }
        eligible_count += 1i32;
        total_cost = total_cost.wrapping_add(object.cost);
    }
    eligible_count = eligible_count.max(1);
    (eligible_count / 2)
        .wrapping_add(total_cost)
        .wrapping_div(eligible_count)
        .wrapping_mul(unit_count)
}

#[derive(Debug, Default)]
struct StartingUnitCandidates {
    vehicles: Vec<StartingUnitCandidate>,
    infantry: Vec<StartingUnitCandidate>,
}

fn starting_unit_candidates_for_country(
    rules: &RuleSet,
    country: LaunchCountry,
    tech_level: i32,
) -> StartingUnitCandidates {
    let vehicles = rules
        .vehicle_ids
        .iter()
        .filter_map(|id| {
            let object = rules.object(id)?;
            if !starting_unit_vehicle_allowed(rules, object, tech_level)
                || !launch_country_can_own_object(country, object)
            {
                return None;
            }
            Some(StartingUnitCandidate {
                type_id: id.clone(),
                cost: object.cost,
            })
        })
        .collect();
    let infantry = rules
        .infantry_ids
        .iter()
        .filter_map(|id| {
            let object = rules.object(id)?;
            if !starting_unit_infantry_allowed(object, tech_level)
                || !launch_country_can_own_object(country, object)
            {
                return None;
            }
            Some(StartingUnitCandidate {
                type_id: id.clone(),
                cost: object.cost,
            })
        })
        .collect();
    StartingUnitCandidates { vehicles, infantry }
}

fn starting_unit_vehicle_allowed(
    rules: &RuleSet,
    object: &crate::rules::object_type::ObjectType,
    tech_level: i32,
) -> bool {
    object.allowed_to_start_in_multiplayer
        && object.tech_level <= tech_level
        && !rules
            .general
            .base_unit_types
            .iter()
            .any(|base_unit| base_unit.eq_ignore_ascii_case(&object.id))
}

fn starting_unit_infantry_allowed(
    object: &crate::rules::object_type::ObjectType,
    tech_level: i32,
) -> bool {
    object.allowed_to_start_in_multiplayer && object.tech_level <= tech_level
}

fn starting_object_cell_placeable(
    sim: &Simulation,
    resolved_terrain: &ResolvedTerrainGrid,
    bounds: NativeStartBounds,
    rx: u16,
    ry: u16,
    category: crate::rules::object_type::ObjectCategory,
) -> bool {
    if !bounds.contains(rx, ry) {
        return false;
    }
    if let Some(occupancy) = sim.occupancy().get(rx, ry) {
        let compatible_infantry = category == crate::rules::object_type::ObjectCategory::Infantry
            && occupancy
                .occupants
                .first()
                .and_then(|occupant| sim.entities().get(occupant.entity_id))
                .is_some_and(|entity| entity.category == EntityCategory::Infantry)
            && occupancy
                .occupants
                .iter()
                .filter(|occupant| occupant.sub_cell.is_some())
                .count()
                < 5;
        if !compatible_infantry {
            return false;
        }
    }
    let Some(cell) = resolved_terrain.cell(rx, ry) else {
        return false;
    };
    if cell.overlay_blocks || cell.terrain_object_blocks {
        return false;
    }
    if cell.has_bridge_deck && cell.bridge_walkable {
        return true;
    }
    cell.zone_type == crate::map::resolved_terrain::zone_class::GROUND
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::houses::HouseDefinition;
    use crate::map::resolved_terrain::{ResolvedTerrainCell, zone_class};
    use crate::rules::ini_parser::IniFile;
    use crate::rules::terrain_rules::{SpeedCostProfile, TerrainClass};
    use crate::skirmish_launch::{
        AiDifficulty, LaunchStartPosition, LaunchTeam, SkirmishAiSlot, SkirmishLaunchMode,
        SkirmishLaunchOptions, SkirmishLocalSlot,
    };

    fn test_session() -> SkirmishLaunchSession {
        SkirmishLaunchSession {
            mode: test_battle_mode(),
            selected_map_file: Some("test.mmx".to_string()),
            player_name: "Player".to_string(),
            local: SkirmishLocalSlot {
                country: LaunchCountry::America,
                country_random: false,
                color_index: 1,
                color_random: false,
                start_position: LaunchStartPosition::Position(3),
                team: LaunchTeam::None,
            },
            opponents: vec![SkirmishAiSlot {
                country: LaunchCountry::Russia,
                country_random: false,
                color_index: 2,
                color_random: false,
                start_position: LaunchStartPosition::Auto,
                team: LaunchTeam::None,
                difficulty: Default::default(),
            }],
            options: SkirmishLaunchOptions::default(),
        }
    }

    fn test_battle_mode() -> SkirmishLaunchMode {
        SkirmishLaunchMode {
            id: 1,
            ui_name_key: "GUI:Battle".to_string(),
            tooltip_key: "STT:ModeBattle".to_string(),
            override_file: "MPBattleMD.ini".to_string(),
            map_filter: "standard".to_string(),
            random_maps_allowed: true,
            allies_allowed: true,
            must_ally: false,
        }
    }

    fn test_cooperative_mode() -> SkirmishLaunchMode {
        SkirmishLaunchMode {
            id: 3,
            ui_name_key: "GUI:Cooperative".to_string(),
            tooltip_key: "STT:ModeCooperative".to_string(),
            override_file: "MPCoopMD.ini".to_string(),
            map_filter: "cooperative".to_string(),
            random_maps_allowed: false,
            allies_allowed: true,
            must_ally: false,
        }
    }

    fn test_rules_with_base_units(
        base_units: &str,
        objects: &[(&str, &str, Option<&str>, Option<&str>)],
    ) -> RuleSet {
        let mut text = format!("[General]\nBaseUnit={base_units}\n\n[VehicleTypes]\n");
        for (idx, (id, _, _, _)) in objects.iter().enumerate() {
            text.push_str(&format!("{}={id}\n", idx + 1));
        }
        for (id, owner, required, forbidden) in objects {
            text.push_str(&format!("\n[{id}]\nOwner={owner}\n"));
            if let Some(required) = required {
                text.push_str(&format!("RequiredHouses={required}\n"));
            }
            if let Some(forbidden) = forbidden {
                text.push_str(&format!("ForbiddenHouses={forbidden}\n"));
            }
        }
        let ini = IniFile::from_str(&text);
        RuleSet::from_ini(&ini).expect("test rules parse")
    }

    fn test_standard_launch_rules() -> RuleSet {
        test_rules_with_base_units(
            "AMCV,SMCV,PCV",
            &[
                (
                    "AMCV",
                    "British,French,Germans,Americans,Alliance",
                    None,
                    None,
                ),
                ("SMCV", "Russians,Confederation,Africans,Arabs", None, None),
                ("PCV", "YuriCountry", None, None),
            ],
        )
    }

    fn ordered_country_side_rules() -> RuleSet {
        let ini = IniFile::from_str(
            "[Countries]\n\
             0=Americans\n1=Alliance\n2=French\n3=Germans\n4=British\n\
             5=Africans\n6=Arabs\n7=Confederation\n8=Russians\n9=YuriCountry\n\
             10=GDI\n11=Nod\n12=Neutral\n13=Special\n\
             [Sides]\n\
             GDI=British,French,Germans,Americans,Alliance\n\
             Nod=Russians,Africans,Confederation,Arabs\n\
             ThirdSide=YuriCountry\nCivilian=Neutral\nMutant=Special\n",
        );
        RuleSet::from_ini(&ini).expect("ordered country/side rules parse")
    }

    fn test_starting_unit_rules() -> RuleSet {
        let ini = IniFile::from_str(
            "[General]\nBaseUnit=AMCV,SMCV,PCV\n\
             [VehicleTypes]\n1=AMCV\n2=SMCV\n3=MTNK\n4=HTNK\n5=HARV\n6=YTNK\n\
             [InfantryTypes]\n1=E1\n2=SHK\n\
             [AMCV]\nOwner=Americans\nCost=1000\nTechLevel=1\nAllowedToStartInMultiplayer=yes\n\
             [SMCV]\nOwner=Russians\nCost=1000\nTechLevel=1\nAllowedToStartInMultiplayer=yes\n\
             [MTNK]\nOwner=Americans\nCost=100\nTechLevel=1\nAllowedToStartInMultiplayer=yes\n\
             [HTNK]\nOwner=Russians\nCost=100\nTechLevel=1\nAllowedToStartInMultiplayer=yes\n\
             [HARV]\nOwner=Americans,Russians\nCost=500\nTechLevel=1\nAllowedToStartInMultiplayer=no\n\
             [YTNK]\nOwner=YuriCountry\nCost=700\nTechLevel=1\nAllowedToStartInMultiplayer=yes\n\
             [E1]\nOwner=Americans\nCost=300\nTechLevel=1\nAllowedToStartInMultiplayer=yes\n\
             [SHK]\nOwner=Russians\nCost=200\nTechLevel=11\nAllowedToStartInMultiplayer=yes\n",
        );
        RuleSet::from_ini(&ini).expect("starting unit rules parse")
    }

    fn test_map_with_starts(starts: &[Waypoint]) -> MapFile {
        let waypoints = starts
            .iter()
            .map(|waypoint| (waypoint.index, *waypoint))
            .collect();
        MapFile {
            header: crate::map::map_file::MapHeader {
                theater: "TEMPERATE".to_string(),
                fill: "Clear".to_string(),
                level: 0,
                width: 64,
                height: 64,
                local_left: 0,
                local_top: 0,
                local_width: 64,
                local_height: 64,
            },
            basic: crate::map::basic::BasicSection::default(),
            briefing: crate::map::briefing::BriefingSection::default(),
            preview: crate::map::preview::PreviewSection::default(),
            cells: Vec::new(),
            entities: Vec::new(),
            overlays: Vec::new(),
            overlay_data: crate::map::overlay::OverlayDataPack::default(),
            smudges: Vec::new(),
            terrain_objects: Vec::new(),
            waypoints,
            cell_tags: std::collections::HashMap::new(),
            tags: std::collections::HashMap::new(),
            triggers: std::collections::HashMap::new(),
            events: std::collections::HashMap::new(),
            actions: std::collections::HashMap::new(),
            local_variables: std::collections::HashMap::new(),
            trigger_graph: crate::map::trigger_graph::TriggerGraph::default(),
            special_flags: crate::map::basic::SpecialFlagsSection::default(),
            explicit_tubes: Vec::new(),
            ini: IniFile::from_str(""),
        }
    }

    fn test_launch_starts() -> [Waypoint; 4] {
        [
            Waypoint {
                index: 0,
                rx: 10,
                ry: 10,
            },
            Waypoint {
                index: 1,
                rx: 30,
                ry: 10,
            },
            Waypoint {
                index: 2,
                rx: 10,
                ry: 30,
            },
            Waypoint {
                index: 3,
                rx: 30,
                ry: 30,
            },
        ]
    }

    fn test_height_map() -> BTreeMap<(u16, u16), u8> {
        BTreeMap::new()
    }

    fn entity_position_for_owner(sim: &Simulation, owner: &str) -> Option<(u16, u16)> {
        sim.entities().values().find_map(|entity| {
            (sim.interner.resolve(entity.owner) == owner)
                .then_some((entity.position.rx, entity.position.ry))
        })
    }

    fn test_terrain(width: u16, height: u16) -> ResolvedTerrainGrid {
        let mut cells = Vec::with_capacity(width as usize * height as usize);
        for ry in 0..height {
            for rx in 0..width {
                cells.push(test_terrain_cell(rx, ry, Some(100)));
            }
        }
        ResolvedTerrainGrid::from_cells(width, height, cells)
    }

    fn test_terrain_cell(rx: u16, ry: u16, track_cost: Option<u8>) -> ResolvedTerrainCell {
        ResolvedTerrainCell {
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
            land_type: 0,
            yr_cell_land_type: 0,
            slope_type: 0,
            template_height: 0,
            render_offset_x: 0,
            render_offset_y: 0,
            terrain_class: TerrainClass::Clear,
            speed_costs: SpeedCostProfile {
                track: track_cost,
                ..Default::default()
            },
            is_water: false,
            is_cliff_like: false,
            is_rough: false,
            is_road: false,
            accepts_smudge: false,
            allows_tiberium: false,
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
            base_land_type: 0,
            base_yr_cell_land_type: 0,
            base_terrain_class: TerrainClass::Clear,
            base_speed_costs: SpeedCostProfile {
                track: track_cost,
                ..Default::default()
            },
            build_blocked: false,
            has_bridge_deck: false,
            bridge_walkable: false,
            bridge_transition: false,
            bridge_deck_level: 0,
            bridge_layer: None,
            bridge_facts: crate::map::bridge_facts::BridgeCellFacts::default(),
            tube_index: None,
            radar_left: [0, 0, 0],
            radar_right: [0, 0, 0],
            has_damaged_data: false,
            bridgehead_anchor_class_at_load: None,
        }
    }

    fn roster_with_neutral_and_playable() -> HouseRoster {
        HouseRoster {
            houses: vec![
                HouseDefinition {
                    name: "Neutral".to_string(),
                    color: HouseColorIndex(8),
                    country: None,
                    side: None,
                    player_control: None,
                    allies: Vec::new(),
                },
                HouseDefinition {
                    name: "Americans".to_string(),
                    color: HouseColorIndex(4),
                    country: Some("Americans".to_string()),
                    side: Some("Allies".to_string()),
                    player_control: Some(true),
                    allies: Vec::new(),
                },
            ],
        }
    }

    #[test]
    fn launch_color_map_keeps_non_players_and_uses_session_slots() {
        let colors = house_color_map_for_launch_session(
            &test_session(),
            &roster_with_neutral_and_playable(),
        );

        // Neutral keeps the roster-assigned index; players resolve their lobby
        // priority through the [Colors] entry LUT (priority 1 → entry 5 DarkRed,
        // priority 2 → entry 10 DarkBlue).
        assert_eq!(colors.get("Neutral"), Some(&HouseColorIndex(8)));
        assert_eq!(colors.get("Player"), Some(&HouseColorIndex(5)));
        assert_eq!(colors.get("Computer1"), Some(&HouseColorIndex(10)));
        assert!(!colors.contains_key("Americans"));
    }

    #[test]
    fn launch_session_uses_player_name_for_local_owner() {
        let mut session = test_session();
        session.player_name = "Commander".to_string();

        let slots = normalized_launch_slots(&session);

        assert_eq!(slots[0].owner_name, "Commander");
        assert!(slots[0].is_human);
    }

    #[test]
    fn original_launch_start_assignments_preserve_explicit_and_auto_slot_identity() {
        let mut session = test_session();
        session.opponents.push(SkirmishAiSlot {
            country: LaunchCountry::Cuba,
            country_random: false,
            color_index: 3,
            color_random: false,
            start_position: LaunchStartPosition::Position(0),
            team: LaunchTeam::None,
            difficulty: Default::default(),
        });
        let starts = [
            Waypoint {
                index: 0,
                rx: 10,
                ry: 10,
            },
            Waypoint {
                index: 3,
                rx: 30,
                ry: 30,
            },
            Waypoint {
                index: 5,
                rx: 50,
                ry: 50,
            },
        ];

        let assignments = original_launch_start_assignments(&session, &starts);

        assert_eq!(assignments[0], (0, starts[1]));
        assert_eq!(assignments[1], (1, starts[2]));
        assert_eq!(assignments[2], (2, starts[0]));
    }

    #[test]
    fn native_gather_generates_deficient_start_with_two_ranged_draws() {
        let authored = Waypoint {
            index: 0,
            rx: 30,
            ry: 30,
        };
        let waypoints = HashMap::from([(0, authored)]);
        let terrain = test_terrain(64, 64);
        let bounds = NativeStartBounds {
            min_rx: 0,
            min_ry: 0,
            width: 64,
            height: 64,
        };
        let mut rng = SimRng::new(9);
        let mut expected = rng.clone();
        let _ = expected.next_range_u32_inclusive(0, 54);
        let _ = expected.next_range_u32_inclusive(10, 54);
        let occupancy = crate::sim::occupancy::OccupancyGrid::new();

        let starts =
            native_gather_start_positions(&waypoints, 2, &terrain, &occupancy, bounds, &mut rng);

        assert_eq!(starts.len(), 2);
        assert_eq!(starts[0], authored);
        assert_eq!(starts[1].index, 1);
        assert!(deficient_start_rect_track_passable(
            &terrain,
            &occupancy,
            starts[1].rx,
            starts[1].ry
        ));
        assert_eq!(rng.logical_state(), expected.logical_state());
    }

    #[test]
    fn native_gather_rejects_a_blocked_cell_inside_the_8x8_rect() {
        let mut terrain = test_terrain(16, 8);
        let blocked = terrain.cell_mut(7, 7).unwrap();
        blocked.speed_costs.track = Some(0);
        blocked.base_speed_costs.track = Some(0);
        blocked.zone_type = zone_class::IMPASSABLE;
        let occupancy = crate::sim::occupancy::OccupancyGrid::new();

        assert!(!deficient_start_rect_track_passable(
            &terrain, &occupancy, 0, 0
        ));
        assert!(deficient_start_rect_track_passable(
            &terrain, &occupancy, 8, 0
        ));
    }

    #[test]
    fn native_gather_rejects_live_occupation_inside_the_8x8_rect() {
        let terrain = test_terrain(16, 8);
        let mut occupancy = crate::sim::occupancy::OccupancyGrid::new();
        occupancy.add(
            7,
            7,
            1,
            crate::sim::movement::locomotor::MovementLayer::Ground,
            None,
            crate::sim::occupancy::CellListInsertion::PrependNonBuilding,
        );

        assert!(!deficient_start_rect_track_passable(
            &terrain, &occupancy, 0, 0
        ));
        assert!(deficient_start_rect_track_passable(
            &terrain, &occupancy, 8, 0
        ));
    }

    #[test]
    fn launch_registration_orders_participants_before_guaranteed_special_houses() {
        let session = test_session();
        let mut sim = Simulation::new();
        let rules = test_standard_launch_rules();

        populate_launch_houses(&mut sim, &normalized_launch_slots(&session), &rules);
        populate_special_houses(&mut sim, &HouseRoster::default(), &rules);

        let order: Vec<_> = sim
            .session
            .house_order
            .iter()
            .map(|id| sim.interner.resolve(*id))
            .collect();
        assert_eq!(order, ["Player", "Computer1", "Neutral", "Special"]);
        assert_eq!(sim.houses.len(), 4);
    }

    #[test]
    fn ordered_country_side_stock_identities_reach_skirmish_house_state() {
        let mut session = test_session();
        session.opponents.push(SkirmishAiSlot {
            country: LaunchCountry::Yuri,
            country_random: false,
            color_index: 3,
            color_random: false,
            start_position: LaunchStartPosition::Auto,
            team: LaunchTeam::None,
            difficulty: Default::default(),
        });
        let rules = ordered_country_side_rules();
        let mut sim = Simulation::new();

        populate_launch_houses(&mut sim, &normalized_launch_slots(&session), &rules);
        populate_special_houses(&mut sim, &HouseRoster::default(), &rules);

        let side = |owner: &str| {
            crate::sim::house_state::house_state_for_owner(&sim.houses, owner, &sim.interner)
                .map(|house| house.side_index)
        };
        assert_eq!(side("Player"), Some(0));
        assert_eq!(side("Computer1"), Some(1));
        assert_eq!(side("Computer2"), Some(2));
        assert_eq!(side("Neutral"), Some(3));
        assert_eq!(side("Special"), Some(4));

        let country = |owner: &str| {
            crate::sim::house_state::house_state_for_owner(&sim.houses, owner, &sim.interner)
                .and_then(|house| house.country)
                .map(|country| sim.interner.resolve(country).to_string())
        };
        assert_eq!(country("Neutral"), Some("Neutral".to_string()));
        assert_eq!(country("Special"), Some("Special".to_string()));
    }

    #[test]
    fn skirmish_launch_same_explicit_team_creates_mutual_alliance() {
        let mut session = test_session();
        session.local.team = LaunchTeam::Team(0);
        session.opponents[0].team = LaunchTeam::Team(0);
        session.opponents.push(SkirmishAiSlot {
            country: LaunchCountry::Cuba,
            country_random: false,
            color_index: 3,
            color_random: false,
            start_position: LaunchStartPosition::Auto,
            team: LaunchTeam::Team(1),
            difficulty: Default::default(),
        });
        let slots = normalized_launch_slots(&session);
        let alliances =
            launch_alliance_map(&roster_with_neutral_and_playable(), &slots, &session.mode);

        assert!(
            alliances
                .get("PLAYER")
                .is_some_and(|allies| allies.contains("COMPUTER1"))
        );
        assert!(
            alliances
                .get("COMPUTER1")
                .is_some_and(|allies| allies.contains("PLAYER"))
        );
        assert!(
            !alliances
                .get("PLAYER")
                .is_some_and(|allies| allies.contains("COMPUTER2"))
        );
    }

    #[test]
    fn skirmish_launch_team_sentinels_do_not_auto_ally() {
        let session = test_session();
        let slots = normalized_launch_slots(&session);
        let alliances =
            launch_alliance_map(&roster_with_neutral_and_playable(), &slots, &session.mode);

        assert!(
            !alliances
                .get("PLAYER")
                .is_some_and(|allies| allies.contains("COMPUTER1"))
        );
        assert!(
            !alliances
                .get("COMPUTER1")
                .is_some_and(|allies| allies.contains("PLAYER"))
        );
    }

    #[test]
    fn cooperative_mode_allies_nonspecial_houses_by_control_group() {
        let mut session = test_session();
        session.mode = test_cooperative_mode();
        session.opponents.push(SkirmishAiSlot {
            country: LaunchCountry::Cuba,
            country_random: false,
            color_index: 3,
            color_random: false,
            start_position: LaunchStartPosition::Auto,
            team: LaunchTeam::None,
            difficulty: Default::default(),
        });
        let slots = normalized_launch_slots(&session);

        let alliances =
            launch_alliance_map(&roster_with_neutral_and_playable(), &slots, &session.mode);

        assert!(
            alliances
                .get("COMPUTER1")
                .is_some_and(|allies| allies.contains("COMPUTER2"))
        );
        assert!(
            !alliances
                .get("PLAYER")
                .is_some_and(|allies| allies.contains("COMPUTER1"))
        );
    }

    #[test]
    fn automatic_start_distance_uses_the_retail_lut_tie() {
        let starts = [
            Waypoint {
                index: 0,
                rx: 100,
                ry: 100,
            },
            Waypoint {
                index: 1,
                rx: 100,
                ry: 229,
            },
            Waypoint {
                index: 2,
                rx: 228,
                ry: 100,
            },
        ];
        assert_eq!(native_start_distance(starts[1], starts[0]), 128);
        assert_eq!(native_start_distance(starts[2], starts[0]), 128);

        let mut rng = SimRng::new(7);
        assert_eq!(
            choose_automatic_start(&starts, &[true, false, false], false, &mut rng),
            1
        );
    }

    #[test]
    fn cooperative_start_uses_human_prefix_then_ai_suffix() {
        let mut session = test_session();
        session.local.start_position = LaunchStartPosition::Auto;
        let starts = test_launch_starts();
        let mut expected_rng = SimRng::new(17);
        let expected_human = expected_rng.next_range_u32_inclusive(0, 1) as usize;
        let mut rng = SimRng::new(17);

        let assignments = native_assign_cooperative_launch_starts(&session, &starts, 2, &mut rng);

        assert_eq!(assignments[0], (0, starts[expected_human]));
        assert_eq!(assignments[1], (1, starts[2]));
        assert_eq!(rng.logical_state(), expected_rng.logical_state());
    }

    #[test]
    fn cooperative_explicit_table_is_reserved_before_house_iteration() {
        let mut session = test_session();
        session.local.start_position = LaunchStartPosition::Auto;
        session.opponents[0].start_position = LaunchStartPosition::Position(0);
        let starts = test_launch_starts();
        let mut rng = SimRng::new(17);
        let before = rng.logical_state();

        let assignments = native_assign_cooperative_launch_starts(&session, &starts, 2, &mut rng);

        assert_eq!(assignments[0], (0, starts[2]));
        assert_eq!(assignments[1], (1, starts[0]));
        assert_eq!(rng.logical_state(), before);
    }

    #[test]
    fn skirmish_launch_start_position_and_team_are_independent() {
        let mut session = test_session();
        session.local.start_position = LaunchStartPosition::Position(3);
        session.local.team = LaunchTeam::Team(0);
        session.opponents[0].start_position = LaunchStartPosition::Position(0);
        session.opponents[0].team = LaunchTeam::Team(0);
        let slots = normalized_launch_slots(&session);
        let starts = test_launch_starts();
        let mut rng = SimRng::new(17);
        let before = rng.logical_state();
        let assignments = native_assign_launch_starts(&session, &starts, &mut rng);
        let alliances =
            launch_alliance_map(&roster_with_neutral_and_playable(), &slots, &session.mode);

        assert_eq!(assignments[0], (0, starts[3]));
        assert_eq!(assignments[1], (1, starts[0]));
        assert_eq!(rng.logical_state(), before);
        assert!(
            alliances
                .get("PLAYER")
                .is_some_and(|allies| allies.contains("COMPUTER1"))
        );
    }

    #[test]
    fn skirmish_bases_off_skips_standard_mcv_callback() {
        let mut sim = Simulation::new();
        let mut session = test_session();
        session.options.bases = false;
        session.options.unit_count = 0;
        let terrain = test_terrain(64, 64);
        let starts = test_launch_starts();
        let map = test_map_with_starts(&starts);
        let rules = test_standard_launch_rules();

        let result = apply_explicit_skirmish_launch_session(
            &mut sim,
            &map,
            &roster_with_neutral_and_playable(),
            &rules,
            &test_height_map(),
            &terrain,
            &session,
        );

        assert_eq!(result.spawned_mcvs, 0);
        assert_eq!(result.active_slots, 2);
        assert_eq!(sim.entities().len(), 0);
        assert_eq!(
            crate::sim::house_state::house_state_for_owner(&sim.houses, "Player", &sim.interner)
                .and_then(|house| house.base_center),
            Some((30, 30))
        );
        assert_eq!(
            crate::sim::house_state::house_state_for_owner(&sim.houses, "Computer1", &sim.interner)
                .and_then(|house| house.base_center),
            Some((30, 10))
        );
    }

    #[test]
    fn explicit_launch_local_owner_drives_production_sidebar_theme() {
        use crate::render::sidebar_chrome::SidebarTheme;

        let terrain = test_terrain(64, 64);
        let starts = test_launch_starts();
        let map = test_map_with_starts(&starts);
        let rules = test_standard_launch_rules();
        let roster = roster_with_neutral_and_playable();

        for (country, expected_theme) in [
            (LaunchCountry::America, SidebarTheme::Allied),
            (LaunchCountry::Russia, SidebarTheme::Soviet),
            (LaunchCountry::Yuri, SidebarTheme::Yuri),
        ] {
            let mut sim = Simulation::new();
            let mut session = test_session();
            session.player_name = "Commander".to_string();
            session.local.country = country;
            session.options.bases = false;
            session.options.unit_count = 0;

            let result = apply_explicit_skirmish_launch_session(
                &mut sim,
                &map,
                &roster,
                &rules,
                &test_height_map(),
                &terrain,
                &session,
            );
            let owner = result
                .local_owner
                .as_deref()
                .expect("explicit launch must return its pinned local owner");
            assert_eq!(owner, "Commander");

            let actual = crate::app_sidebar_render::sidebar_theme_for_owner_sources(
                Some(&sim),
                &roster,
                owner,
            )
            .unwrap_or(SidebarTheme::Allied);
            assert_eq!(
                actual, expected_theme,
                "{country:?} launch owner must reach its live side theme"
            );
        }
    }

    #[test]
    fn sidebar_theme_sources_preserve_roster_path_on_owner_name_collision() {
        use crate::render::sidebar_chrome::SidebarTheme;

        let roster = HouseRoster {
            houses: vec![HouseDefinition {
                name: "MapPlayer".to_string(),
                color: HouseColorIndex(4),
                country: Some("Russians".to_string()),
                side: Some("Soviet".to_string()),
                player_control: Some(true),
                allies: Vec::new(),
            }],
        };

        assert_eq!(
            crate::app_sidebar_render::sidebar_theme_for_owner_sources(None, &roster, "MapPlayer",),
            Some(SidebarTheme::Soviet),
            "an absent live owner must preserve the existing roster resolver"
        );

        let mut sim = Simulation::new();
        let owner_id = sim.interner.intern("MapPlayer");
        sim.houses
            .insert(owner_id, HouseState::new(owner_id, 2, None, true, 0, 10));
        assert_eq!(
            crate::app_sidebar_render::sidebar_theme_for_owner_sources(
                Some(&sim),
                &roster,
                "MapPlayer",
            ),
            Some(SidebarTheme::Soviet),
            "an explicit-name collision must preserve the roster-resolved map path"
        );
    }

    #[test]
    fn sidebar_theme_sources_reject_unknown_live_side() {
        let mut sim = Simulation::new();
        let owner_id = sim.interner.intern("DynamicOwner");
        sim.houses
            .insert(owner_id, HouseState::new(owner_id, 7, None, true, 0, 10));

        assert_eq!(
            crate::app_sidebar_render::sidebar_theme_for_owner_sources(
                Some(&sim),
                &HouseRoster::default(),
                "DynamicOwner",
            ),
            None,
            "unknown live side must defer to the caller's explicit fallback"
        );
    }

    #[test]
    fn explicit_skirmish_application_bypasses_legacy_resolver_even_with_poison_random_flag() {
        let mut sim = Simulation::new();
        let mut session = test_session();
        session.local.country_random = true;
        session.opponents[0].start_position = LaunchStartPosition::Position(0);
        session.options.bases = false;
        session.options.unit_count = 0;
        let terrain = test_terrain(64, 64);
        let starts = test_launch_starts();
        let map = test_map_with_starts(&starts);
        let rules = test_standard_launch_rules();
        let mut expected_scenario = sim.scenario_rng.clone();
        let _ = expected_scenario.next_range_u32_inclusive(0, 0xffff);

        let result = apply_explicit_skirmish_launch_session(
            &mut sim,
            &map,
            &roster_with_neutral_and_playable(),
            &rules,
            &test_height_map(),
            &terrain,
            &session,
        );

        assert_eq!(result.spawned_mcvs, 0);
        assert_eq!(result.active_slots, 2);
        assert_eq!(sim.rng_state().scenario, expected_scenario.logical_state());
        assert!(
            session.local.country_random,
            "the borrowed poison fixture must remain marked random"
        );
    }

    #[test]
    fn launch_population_copies_each_ai_row_difficulty_to_its_house() {
        let mut session = test_session();
        let template = session.opponents[0].clone();
        session.opponents = [AiDifficulty::Hard, AiDifficulty::Normal, AiDifficulty::Easy]
            .into_iter()
            .enumerate()
            .map(|(index, difficulty)| SkirmishAiSlot {
                color_index: (index + 2) as u8,
                difficulty,
                ..template.clone()
            })
            .collect();

        let mut sim = Simulation::new();
        let rules = test_standard_launch_rules();
        populate_launch_houses(&mut sim, &normalized_launch_slots(&session), &rules);

        let difficulty = |owner: &str| {
            crate::sim::house_state::house_state_for_owner(&sim.houses, owner, &sim.interner)
                .map(|house| house.difficulty)
        };
        assert_eq!(difficulty("Player"), Some(HouseDifficulty::Normal));
        assert_eq!(difficulty("Computer1"), Some(HouseDifficulty::Hard));
        assert_eq!(difficulty("Computer2"), Some(HouseDifficulty::Normal));
        assert_eq!(difficulty("Computer3"), Some(HouseDifficulty::Easy));
    }

    #[test]
    fn skirmish_bases_off_still_allows_unit_count_extra_units() {
        let mut sim = Simulation::new();
        let mut session = test_session();
        session.options.bases = false;
        session.options.unit_count = 1;
        let terrain = test_terrain(64, 64);
        let starts = test_launch_starts();
        let mut map = test_map_with_starts(&starts);
        map.special_flags.initial_veteran = Some(true);
        let rules = test_starting_unit_rules();

        let result = apply_explicit_skirmish_launch_session(
            &mut sim,
            &map,
            &roster_with_neutral_and_playable(),
            &rules,
            &test_height_map(),
            &terrain,
            &session,
        );

        assert_eq!(result.spawned_mcvs, 0);
        assert_eq!(sim.entities().len(), 4);
        assert_eq!(entity_position_for_owner(&sim, "Player"), Some((30, 30)));
        assert_eq!(entity_position_for_owner(&sim, "Computer1"), Some((30, 10)));
        assert!(
            sim.entities()
                .values()
                .all(|entity| entity.veterancy == 200)
        );
        assert!(sim.entities().values().all(|entity| {
            let expected = if sim.interner.resolve(entity.owner) == "Player" {
                MissionType::Guard
            } else {
                MissionType::AreaGuard
            };
            entity.mission.current().known() == Some(expected)
        }));
    }

    #[test]
    fn skirmish_unit_count_zero_spawns_mcv_only_when_bases_enabled() {
        let mut sim = Simulation::new();
        let mut session = test_session();
        session.options.unit_count = 0;
        session.options.bases = true;
        let terrain = test_terrain(64, 64);
        let starts = test_launch_starts();
        let map = test_map_with_starts(&starts);
        let rules = test_standard_launch_rules();

        let result = apply_explicit_skirmish_launch_session(
            &mut sim,
            &map,
            &roster_with_neutral_and_playable(),
            &rules,
            &test_height_map(),
            &terrain,
            &session,
        );

        assert_eq!(result.spawned_mcvs, 2);
        assert_eq!(sim.session.game_options.unit_count, 0);
        assert_eq!(sim.entities().len(), 2);
    }

    #[test]
    fn skirmish_assigned_start_sets_house_base_cell_before_mcv_spawn() {
        let mut sim = Simulation::new();
        let session = test_session();
        let terrain = test_terrain(64, 64);
        let starts = test_launch_starts();
        let map = test_map_with_starts(&starts);
        let rules = test_standard_launch_rules();

        let result = apply_explicit_skirmish_launch_session(
            &mut sim,
            &map,
            &roster_with_neutral_and_playable(),
            &rules,
            &test_height_map(),
            &terrain,
            &session,
        );

        assert_eq!(result.spawned_mcvs, 2);
        assert_eq!(
            crate::sim::house_state::house_state_for_owner(&sim.houses, "Player", &sim.interner)
                .and_then(|house| house.base_center),
            Some((30, 30))
        );
        assert_eq!(entity_position_for_owner(&sim, "Player"), Some((30, 30)));
    }

    #[test]
    fn skirmish_mcv_start_uses_radius_fallback_when_start_cell_blocked() {
        let mut sim = Simulation::new();
        let session = test_session();
        let terrain = test_terrain(64, 64);
        let starts = test_launch_starts();
        let map = test_map_with_starts(&starts);
        let rules = test_standard_launch_rules();
        sim.spawn_object(
            "AMCV",
            "Neutral",
            30,
            30,
            STARTING_MCV_FACING,
            &rules,
            &test_height_map(),
        )
        .expect("blocker");
        let mut expected_rng = sim.scenario_rng.clone();
        let direction = expected_rng.next_range_u32_inclusive(0, 7) as usize;
        let (dx, dy) = STARTING_MCV_FALLBACK_DIRECTIONS[direction];
        let expected_position = ((30i32 + dx) as u16, (30i32 + dy) as u16);
        let _ = expected_rng.next_range_u32_inclusive(0, 0xffff);

        let result = apply_explicit_skirmish_launch_session(
            &mut sim,
            &map,
            &roster_with_neutral_and_playable(),
            &rules,
            &test_height_map(),
            &terrain,
            &session,
        );

        assert_eq!(result.spawned_mcvs, 2);
        assert_eq!(
            crate::sim::house_state::house_state_for_owner(&sim.houses, "Player", &sim.interner)
                .and_then(|house| house.base_center),
            Some((30, 30))
        );
        assert_eq!(
            entity_position_for_owner(&sim, "Player"),
            Some(expected_position)
        );
        assert_eq!(
            sim.rng_state().scenario,
            expected_rng.logical_state(),
            "one direction draw plus the final sync draw"
        );
    }

    #[test]
    fn skirmish_start_unit_budget_is_global_but_house_pools_filter_tech_and_mask() {
        let rules = test_starting_unit_rules();
        let slots = normalized_launch_slots(&test_session());

        assert_eq!(starting_unit_budget(&rules, &slots, 10, 2), 334);

        let allied_candidates =
            starting_unit_candidates_for_country(&rules, LaunchCountry::America, 10);
        let allied_ids: Vec<&str> = allied_candidates
            .vehicles
            .iter()
            .chain(allied_candidates.infantry.iter())
            .map(|candidate| candidate.type_id.as_str())
            .collect();
        assert_eq!(allied_ids, vec!["MTNK", "E1"]);

        let soviet_candidates =
            starting_unit_candidates_for_country(&rules, LaunchCountry::Russia, 10);
        let soviet_ids: Vec<&str> = soviet_candidates
            .vehicles
            .iter()
            .chain(soviet_candidates.infantry.iter())
            .map(|candidate| candidate.type_id.as_str())
            .collect();
        assert_eq!(soviet_ids, vec!["HTNK"]);
    }

    #[test]
    fn skirmish_start_unit_budget_excludes_baseunit_entries() {
        let rules = test_starting_unit_rules();
        let slots = normalized_launch_slots(&test_session());

        let allied_candidates =
            starting_unit_candidates_for_country(&rules, LaunchCountry::America, 10);
        assert_eq!(starting_unit_budget(&rules, &slots, 10, 1), 167);

        assert!(
            !allied_candidates
                .vehicles
                .iter()
                .any(|candidate| candidate.type_id == "AMCV")
        );
    }

    #[test]
    fn skirmish_start_unit_vehicle_phase_preserves_native_third_rounding() {
        assert!(starting_unit_prefers_vehicle(0, 4));
        assert!(
            starting_unit_prefers_vehicle(2, 4),
            "remaining 2 is still greater than trunc(4 / 3)"
        );
        assert!(!starting_unit_prefers_vehicle(3, 4));
    }

    #[test]
    fn skirmish_start_unit_blocked_placement_stops_after_twenty_failures() {
        let mut sim = Simulation::new();
        let session = test_session();
        sim.session.game_options = session.options.to_game_options(0);
        let all_slots = normalized_launch_slots(&session);
        let slots = &all_slots[..1];
        let rules = test_starting_unit_rules();
        populate_launch_houses(&mut sim, slots, &rules);
        crate::sim::house_state::house_state_for_owner_mut(
            &mut sim.houses,
            &slots[0].owner_name,
            &sim.interner,
        )
        .expect("launch house")
        .base_center = Some((32, 32));

        let mut terrain = test_terrain(64, 64);
        for ry in 0..terrain.height() {
            for rx in 0..terrain.width() {
                terrain
                    .cell_mut(rx, ry)
                    .expect("terrain cell")
                    .overlay_blocks = true;
            }
        }
        let bounds = NativeStartBounds::from_session(&sim, &terrain);
        let mut expected_rng = sim.scenario_rng.clone();
        assert_eq!(STARTING_EXTRA_UNIT_MAX_PLACEMENT_FAILURES, 20);
        for _ in 0..20 {
            // The only eligible vehicle has a one-entry pool. RandomRanged(0,0)
            // is still called but does not advance the native RNG.
            let _ = expected_rng.next_range_u32_inclusive(0, 0);
            for _radius in
                STARTING_EXTRA_UNIT_FALLBACK_START_RADIUS..=STARTING_MCV_FALLBACK_MAX_RADIUS
            {
                let _ = expected_rng.next_range_u32_inclusive(0, 7);
                for _direction in 0..8 {
                    let _ = expected_rng.next_range_u32_inclusive(0, 1);
                    let _ = expected_rng.next_range_u32_inclusive(0, 99);
                    let _ = expected_rng.next_range_u32_inclusive(0, 1);
                    let _ = expected_rng.next_range_u32_inclusive(0, 99);
                }
            }
        }

        let spawned = seed_starting_extra_units(
            &mut sim,
            slots,
            &rules,
            &test_height_map(),
            &terrain,
            bounds,
            1,
            false,
        );

        assert_eq!(spawned, 0);
        assert!(sim.entities().is_empty());
        assert_eq!(
            sim.rng_state().scenario,
            expected_rng.logical_state(),
            "candidate plus fallback draws stop after the twentieth failed placement"
        );
    }

    #[test]
    fn skirmish_positive_unit_count_spawns_extra_starting_units() {
        let mut sim = Simulation::new();
        let mut session = test_session();
        session.options.unit_count = 2;
        let terrain = test_terrain(64, 64);
        let starts = test_launch_starts();
        let map = test_map_with_starts(&starts);
        let rules = test_starting_unit_rules();

        let result = apply_explicit_skirmish_launch_session(
            &mut sim,
            &map,
            &roster_with_neutral_and_playable(),
            &rules,
            &test_height_map(),
            &terrain,
            &session,
        );

        assert_eq!(result.spawned_mcvs, 2);
        assert_eq!(sim.entities().len(), 9);
        let player_units = sim
            .entities()
            .values()
            .filter(|entity| sim.interner.resolve(entity.owner) == "Player")
            .count();
        let ai_units = sim
            .entities()
            .values()
            .filter(|entity| sim.interner.resolve(entity.owner) == "Computer1")
            .count();
        assert_eq!(player_units, 5);
        assert_eq!(ai_units, 4);
    }

    #[test]
    fn skirmish_baseunit_vector_selects_side_matching_mcv() {
        let rules = test_rules_with_base_units(
            "SMCV,AMCV,PCV",
            &[
                ("SMCV", "Russians,Confederation,Africans,Arabs", None, None),
                (
                    "AMCV",
                    "British,French,Germans,Americans,Alliance",
                    None,
                    None,
                ),
                ("PCV", "YuriCountry", None, None),
            ],
        );

        assert_eq!(
            launch_mcv_type_for_country(LaunchCountry::America, &rules),
            "AMCV"
        );
        assert_eq!(
            launch_mcv_type_for_country(LaunchCountry::Russia, &rules),
            "SMCV"
        );
        assert_eq!(
            launch_mcv_type_for_country(LaunchCountry::Yuri, &rules),
            "PCV"
        );
    }

    #[test]
    fn skirmish_baseunit_selection_uses_rules_order() {
        let rules = test_rules_with_base_units(
            "ALTMCV,AMCV",
            &[
                ("AMCV", "Americans", None, None),
                ("ALTMCV", "Americans", None, None),
            ],
        );

        assert_eq!(
            launch_mcv_type_for_country(LaunchCountry::America, &rules),
            "ALTMCV"
        );
    }

    #[test]
    fn skirmish_baseunit_selection_respects_required_and_forbidden_houses() {
        let rules = test_rules_with_base_units(
            "BLOCKED,REQUIRED,FORBIDDEN,AMCV",
            &[
                ("BLOCKED", "Americans", Some("French"), None),
                ("REQUIRED", "Americans", Some("Americans"), None),
                ("FORBIDDEN", "Americans", None, Some("Americans")),
                ("AMCV", "Americans", None, None),
            ],
        );

        assert_eq!(
            launch_mcv_type_for_country(LaunchCountry::America, &rules),
            "REQUIRED"
        );
    }

    #[test]
    fn skirmish_launch_does_not_use_country_hardcoded_mcv_for_parity_path() {
        let rules =
            test_rules_with_base_units("CUSTOMMCV", &[("CUSTOMMCV", "Americans", None, None)]);

        assert_eq!(
            launch_mcv_type_for_country(LaunchCountry::America, &rules),
            "CUSTOMMCV"
        );
    }
}

fn launch_mcv_type_for_country<'a>(country: LaunchCountry, rules: &'a RuleSet) -> &'a str {
    rules
        .general
        .base_unit_types
        .iter()
        .find_map(|id| {
            let object = rules.object(id)?;
            launch_country_can_own_object(country, object).then_some(id.as_str())
        })
        .or_else(|| {
            rules
                .general
                .base_unit_types
                .iter()
                .find(|id| rules.object(id).is_some())
                .map(String::as_str)
        })
        .unwrap_or("AMCV")
}

fn launch_country_can_own_object(
    country: LaunchCountry,
    object: &crate::rules::object_type::ObjectType,
) -> bool {
    let country_name = country.country_name();
    if !object.owner.iter().any(|owner| owner == country_name) {
        return false;
    }
    if !object.required_houses.is_empty()
        && !object
            .required_houses
            .iter()
            .any(|required| required == country_name)
    {
        return false;
    }
    !object
        .forbidden_houses
        .iter()
        .any(|forbidden| forbidden == country_name)
}

pub(crate) fn skirmish_house_candidates(
    house_roster: &HouseRoster,
) -> Vec<&crate::map::houses::HouseDefinition> {
    // First pass: prefer houses without explicit PlayerControl=no.
    let preferred: Vec<&crate::map::houses::HouseDefinition> = house_roster
        .houses
        .iter()
        .filter(|house| {
            is_playable_faction_name(&house.name) && house.player_control != Some(false)
        })
        .collect();
    if preferred.len() >= 2 {
        return preferred;
    }
    // Second pass: include all playable factions (even PlayerControl=no)
    // so skirmish maps can seed at least 2 MCVs for AI opponents.
    house_roster
        .houses
        .iter()
        .filter(|house| is_playable_faction_name(&house.name))
        .collect()
}

/// Reorder house candidates so the player's chosen side appears first.
///
/// Matches houses by their Side= field (Allies/Soviet). If no exact match,
/// falls back to original order.
fn reorder_houses_for_side<'a>(
    houses: Vec<&'a crate::map::houses::HouseDefinition>,
    side: crate::ui::main_menu::SkirmishSide,
) -> Vec<&'a crate::map::houses::HouseDefinition> {
    use crate::ui::main_menu::SkirmishSide;

    let target_side: &str = match side {
        SkirmishSide::Allied => "ALLIES",
        SkirmishSide::Soviet => "SOVIET",
    };

    // Find index of a house matching the player's chosen side.
    let matching_idx = houses.iter().position(|h| {
        h.side
            .as_deref()
            .is_some_and(|s| s.to_ascii_uppercase().contains(target_side))
    });

    let Some(idx) = matching_idx else {
        return houses;
    };
    if idx == 0 {
        return houses;
    }

    // Swap the matching house to position 0 (local player slot).
    let mut reordered = houses;
    reordered.swap(0, idx);
    reordered
}

/// Returns true for faction names that represent real players (not neutral/civilian).
fn is_playable_faction_name(name: &str) -> bool {
    let up = name.to_ascii_uppercase();
    !matches!(
        up.as_str(),
        "NEUTRAL" | "SPECIAL" | "CIVILIAN" | "GOODGUY" | "BADGUY" | "JP"
    )
}

pub(crate) fn skirmish_mcv_type_for_house(
    house: &crate::map::houses::HouseDefinition,
    rules: &RuleSet,
) -> &'static str {
    let mut candidates = Vec::new();
    if let Some(country) = house.country.as_deref() {
        let upper = country.to_ascii_uppercase();
        if upper.contains("YURI") {
            candidates.push("PCV");
        } else if upper.contains("RUSS")
            || upper.contains("CONFED")
            || upper.contains("IRAQ")
            || upper.contains("CUBA")
            || upper.contains("LIBYA")
        {
            candidates.push("SMCV");
        } else {
            candidates.push("AMCV");
        }
    }
    if let Some(side) = house.side.as_deref() {
        let upper = side.to_ascii_uppercase();
        if upper.contains("YURI") {
            candidates.push("PCV");
        } else if upper.contains("SOV") {
            candidates.push("SMCV");
        } else if upper.contains("ALL") {
            candidates.push("AMCV");
        }
    }
    candidates.extend(["AMCV", "SMCV", "PCV"]);
    candidates
        .into_iter()
        .find(|id| rules.object(id).is_some())
        .unwrap_or("AMCV")
}

/// Collect building type IDs that can be spawned at runtime and need atlas pre-loading.
///
/// Scans all objects with `DeploysInto=` set in rules.ini to find deploy targets
/// (e.g., AMCV→GACNST). Data-driven — no hardcoded MCV/ConYard type pairs.
pub fn deployable_building_types<'a>(
    entities: &crate::sim::entity_store::EntityStore,
    rules: Option<&'a RuleSet>,
    interner: Option<&crate::sim::intern::StringInterner>,
) -> Vec<&'a str> {
    let Some(rules) = rules else {
        return Vec::new();
    };
    let mut result: Vec<&str> = Vec::new();

    // Collect deploy targets for any units currently on the map.
    for entity in entities.values() {
        let type_str = interner.map_or("", |i| i.resolve(entity.type_ref));
        if let Some(obj) = rules.object(type_str) {
            if let Some(ref target_id) = obj.deploys_into {
                if let Some(target_obj) = rules.object(target_id) {
                    let id_str: &str = &target_obj.id;
                    if !result.iter().any(|r| r.eq_ignore_ascii_case(id_str)) {
                        result.push(id_str);
                    }
                }
            }
        }
    }

    // Also include deploy targets from ALL objects in rules — units with DeploysInto
    // can appear via production or scripted events even without being on the map yet.
    for obj in rules.all_objects() {
        if let Some(ref target_id) = obj.deploys_into {
            if let Some(target_obj) = rules.object(target_id) {
                let id_str: &str = &target_obj.id;
                if !result.iter().any(|r| r.eq_ignore_ascii_case(id_str)) {
                    result.push(id_str);
                }
            }
        }
    }

    result
}

/// Build overlay sprite atlas and name mapping from map data + rules.ini.
pub(crate) fn build_overlay_atlas_from_map(
    map_data: &MapFile,
    asset_manager: &AssetManager,
    gpu: &GpuContext,
    batch: &BatchRenderer,
    theater_ext: &str,
    rules_ini: &IniFile,
    art_registry: &ArtRegistry,
    theater_iso_palette: Option<&Palette>,
    theater_unit_palette: Option<&Palette>,
    theater_tiberium_palette: Option<&Palette>,
    smudge_types: Option<&crate::rules::smudge_type::SmudgeTypeRegistry>,
    bridge_railing_tile_bases: Option<BridgeRailingTileBases>,
) -> (
    Option<OverlayAtlas>,
    Option<BridgeAtlas>,
    Option<BridgeRailingAtlas>,
    BTreeMap<u8, String>,
    Vec<OverlayEntry>,
    HashMap<(u8, u8), [u8; 3]>,
) {
    let empty_names: BTreeMap<u8, String> = BTreeMap::new();
    let force_tib_remap_enabled: bool = std::env::var("RA2_FORCE_TIB3_TO_TIB01")
        .ok()
        .map(|v| {
            let n = v.trim().to_ascii_lowercase();
            n == "1" || n == "true" || n == "yes" || n == "on"
        })
        .unwrap_or(false);
    if force_tib_remap_enabled {
        log::warn!("Debug overlay remap enabled: TIB3_20 -> TIB01");
    }
    let tib_id_offset: isize = std::env::var("RA2_TIB_ID_OFFSET")
        .ok()
        .and_then(|s| s.parse::<isize>().ok())
        .unwrap_or(0);
    if tib_id_offset != 0 {
        log::warn!(
            "Debug resource ID offset enabled: RA2_TIB_ID_OFFSET={}",
            tib_id_offset
        );
    }

    if map_data.overlays.is_empty() && map_data.terrain_objects.is_empty() {
        return (
            None,
            None,
            None,
            empty_names,
            map_data.overlays.clone(),
            HashMap::new(),
        );
    }
    let overlay_registry: OverlayTypeRegistry = OverlayTypeRegistry::from_ini(rules_ini, None);

    // Compute wall connectivity bitmasks on a mutable clone so the atlas
    // and AppState see correct auto-tiled frames (0–15 per wall type).
    let mut wall_overlays: Vec<OverlayEntry> = map_data.overlays.clone();
    let walls_updated: u32 =
        crate::map::overlay::compute_wall_connectivity(&mut wall_overlays, &overlay_registry);
    if walls_updated > 0 {
        log::info!("Wall connectivity: {} wall entries updated", walls_updated);
    }

    // Log first 20 overlay types for diagnostic verification.
    let max_diag: usize = 20.min(overlay_registry.len());
    for i in 0..max_diag {
        if let Some(name) = overlay_registry.name(i as u8) {
            let mapped = resolve_overlay_name_for_render(&overlay_registry, i as u8)
                .unwrap_or_else(|| name.to_string());
            let flags = overlay_registry.flags(i as u8);
            let tib: bool = flags.map(|f| f.tiberium).unwrap_or(false);
            let wall: bool = flags.map(|f| f.wall).unwrap_or(false);
            log::info!(
                "  OverlayType[{:3}] = {:20} mapped={:20} tib={} wall={}",
                i,
                name,
                mapped,
                tib,
                wall
            );
        }
    }

    // Build ID → name mapping for render-time lookups.
    let mut overlay_names: BTreeMap<u8, String> = BTreeMap::new();
    let mut unmapped_count: u32 = 0;
    let mut unmapped_ids: std::collections::HashSet<u8> = std::collections::HashSet::new();
    for entry in &map_data.overlays {
        if let Some(mapped_name) =
            resolve_overlay_name_for_render(&overlay_registry, entry.overlay_id)
        {
            overlay_names.entry(entry.overlay_id).or_insert(mapped_name);
        } else {
            unmapped_count += 1;
            unmapped_ids.insert(entry.overlay_id);
        }
    }
    if !unmapped_ids.is_empty() {
        let mut ids: Vec<u8> = unmapped_ids.into_iter().collect();
        ids.sort();
        log::warn!("Unmapped overlay IDs (not in registry): {:?}", ids,);
    }
    log::info!(
        "Overlay name mapping: {} IDs mapped, {} unmapped entries",
        overlay_names.len(),
        unmapped_count,
    );
    // Always register all wall types from the registry, even if not present on
    // this map's OverlayPack. This ensures player-built walls (injected into
    // state.overlays at runtime) have overlay_names entries for the renderer.
    let mut wall_ids_added: u32 = 0;
    for overlay_id in 0u8..=u8::MAX {
        let is_wall: bool = overlay_registry
            .flags(overlay_id)
            .map(|f| f.wall)
            .unwrap_or(false);
        if is_wall {
            if let Some(name) = resolve_overlay_name_for_render(&overlay_registry, overlay_id) {
                overlay_names.entry(overlay_id).or_insert_with(|| {
                    wall_ids_added += 1;
                    name
                });
            }
        }
    }
    if wall_ids_added > 0 {
        log::info!(
            "Pre-registered {} wall overlay type(s) in overlay_names for player placement",
            wall_ids_added
        );
    }

    // Log resource overlays for diagnostic visibility.
    for (id, name) in &overlay_names {
        let flags = overlay_registry.flags(*id);
        let tib: bool = flags.map(|f| f.tiberium).unwrap_or(false);
        if tib {
            log::info!("  Resource overlay: id={} name={}", id, name);
        }
    }

    // Use theater-provided palettes if available, otherwise fall back to search.
    let theater_palette: Option<Palette> = theater_iso_palette.cloned().or_else(|| {
        let pal_names: &[&str] = &["isotem.pal", "isosno.pal", "isourb.pal", "temperat.pal"];
        pal_names.iter().find_map(|name| {
            let data: Vec<u8> = asset_manager.get(name)?;
            Palette::from_bytes(&data).ok()
        })
    });
    let unit_palette: Option<Palette> = theater_unit_palette.cloned().or_else(|| {
        let pal_names: &[&str] = &["unittem.pal", "unitsno.pal", "uniturb.pal", "unit.pal"];
        pal_names.iter().find_map(|name| {
            let data: Vec<u8> = asset_manager.get(name)?;
            Palette::from_bytes(&data).ok()
        })
    });
    // Tiberium palette: the original engine uses a dedicated palette (e.g., temperat.pal) for
    // ore/gem overlays, distinct from both the iso palette and the unit palette.
    let tiberium_palette: Option<Palette> = theater_tiberium_palette.cloned().or_else(|| {
        let pal_names: &[&str] = &["temperat.pal", "snow.pal", "urban.pal"];
        pal_names.iter().find_map(|name| {
            let data: Vec<u8> = asset_manager.get(name)?;
            Palette::from_bytes(&data).ok()
        })
    });

    // Compute tiberium radar colors from SHP frame pixel averages.
    let tiberium_radar_colors: HashMap<(u8, u8), [u8; 3]> =
        if let Some(tib_pal) = tiberium_palette.as_ref() {
            overlay_atlas::compute_tiberium_radar_colors(
                asset_manager,
                tib_pal,
                &overlay_registry,
                &wall_overlays,
                &overlay_names,
                theater_ext,
                rules_ini,
                art_registry,
            )
        } else {
            HashMap::new()
        };

    let atlas: Option<OverlayAtlas> = theater_palette.as_ref().and_then(|theater_pal| {
        // If no unit palette, fall back to theater palette for everything.
        let unit_pal: &Palette = unit_palette.as_ref().unwrap_or(theater_pal);
        let tib_pal: &Palette = tiberium_palette.as_ref().unwrap_or(theater_pal);
        overlay_atlas::build_overlay_atlas(
            gpu,
            batch,
            &wall_overlays,
            &map_data.terrain_objects,
            asset_manager,
            theater_pal,
            unit_pal,
            tib_pal,
            theater_ext,
            &map_data.header.theater,
            &overlay_registry,
            rules_ini,
            art_registry,
            smudge_types,
        )
    });

    let bridge_atlas: Option<BridgeAtlas> = theater_palette.as_ref().and_then(|theater_pal| {
        let unit_pal: &Palette = unit_palette.as_ref().unwrap_or(theater_pal);
        bridge_atlas::build_bridge_atlas(
            gpu,
            batch,
            &wall_overlays,
            &overlay_names,
            asset_manager,
            theater_pal,
            unit_pal,
            theater_ext,
            &map_data.header.theater,
            &overlay_registry,
            rules_ini,
            art_registry,
        )
    });

    let bridge_railing_atlas: Option<BridgeRailingAtlas> =
        theater_palette.as_ref().and_then(|theater_pal| {
            bridge_railing_atlas::build_bridge_railing_atlas(
                gpu,
                batch,
                asset_manager,
                theater_pal,
                theater_ext,
                bridge_railing_tile_bases,
            )
        });

    (
        atlas,
        bridge_atlas,
        bridge_railing_atlas,
        overlay_names,
        wall_overlays,
        tiberium_radar_colors,
    )
}
