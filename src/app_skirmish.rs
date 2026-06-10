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
use crate::sim::house_state::HouseState;
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

pub(crate) fn apply_skirmish_launch_session(
    sim: &mut Simulation,
    map_data: &MapFile,
    house_roster: &HouseRoster,
    rules: &RuleSet,
    height_map: &BTreeMap<(u16, u16), u8>,
    resolved_terrain: &ResolvedTerrainGrid,
    session: &SkirmishLaunchSession,
) -> SkirmishLaunchApplyResult {
    // Resolve any "random country" slots into concrete countries before any
    // house/spawn state is built, drawing from the scenario stream so the
    // choice is deterministic for the game seed and identical across peers.
    let resolved_session = session.resolve_random_assignments(sim.random_assignment_rng());
    let session = &resolved_session;
    let slots = normalized_launch_slots(session);
    let ai_difficulty = session
        .opponents
        .first()
        .map(|slot| slot.difficulty)
        .unwrap_or_default();
    sim.session.game_options = session
        .options
        .to_game_options(session.opponents.len() as i32, ai_difficulty);

    sim.houses.clear();
    sim.ai_players.clear();
    populate_non_player_houses(sim, house_roster);
    populate_launch_houses(sim, &slots);
    sim.house_alliances = launch_alliance_map(house_roster, &slots);
    recount_house_owned_counts(sim);

    let starts = waypoints::multiplayer_start_waypoints(&map_data.waypoints);
    let assignments = assign_launch_starts(&slots, &starts, resolved_terrain);
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
        session.options.unit_count,
    );

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
    });
    for (idx, opponent) in session.opponents.iter().enumerate() {
        slots.push(NormalizedSkirmishSlot {
            owner_name: format!("Computer{}", idx + 1),
            country: opponent.country,
            color_index: opponent.color_index,
            start_position: opponent.start_position,
            team: opponent.team,
            is_human: false,
        });
    }
    slots
}

fn populate_non_player_houses(sim: &mut Simulation, house_roster: &HouseRoster) {
    for house in &house_roster.houses {
        if is_playable_faction_name(&house.name) {
            continue;
        }
        let name_id = sim.interner.intern(&house.name);
        let country_id = house.country.as_deref().map(|c| sim.interner.intern(c));
        let side_idx = crate::sim::house_state::side_index_from_name(house.side.as_deref());
        sim.houses.insert(
            name_id,
            HouseState::new(
                name_id,
                side_idx,
                country_id,
                false,
                sim.session.game_options.starting_credits,
                sim.session.game_options.tech_level,
            ),
        );
    }
}

fn populate_launch_houses(sim: &mut Simulation, slots: &[NormalizedSkirmishSlot]) {
    for slot in slots {
        let name_id = sim.interner.intern(&slot.owner_name);
        let country_id = sim.interner.intern(slot.country.country_name());
        sim.houses.insert(
            name_id,
            HouseState::new(
                name_id,
                slot.country.side_index(),
                Some(country_id),
                slot.is_human,
                sim.session.game_options.starting_credits,
                sim.session.game_options.tech_level,
            ),
        );
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

fn assign_launch_starts(
    slots: &[NormalizedSkirmishSlot],
    starts: &[Waypoint],
    resolved_terrain: &ResolvedTerrainGrid,
) -> Vec<(usize, Waypoint)> {
    let mut assignments: Vec<Option<Waypoint>> = vec![None; slots.len()];
    let mut used_start_indices = BTreeSet::new();
    let mut reserved_cells: BTreeSet<(u16, u16)> =
        starts.iter().map(|start| (start.rx, start.ry)).collect();
    let mut next_fallback_index = u32::MAX;

    for (idx, slot) in slots.iter().enumerate() {
        let LaunchStartPosition::Position(position) = slot.start_position else {
            continue;
        };
        let Some(start) = starts.iter().find(|start| start.index == position as u32) else {
            continue;
        };
        if used_start_indices.insert(start.index) {
            assignments[idx] = Some(*start);
            reserved_cells.insert((start.rx, start.ry));
        } else {
            continue;
        }
    }

    for (idx, slot) in slots.iter().enumerate() {
        if assignments[idx].is_some() || slot.start_position != LaunchStartPosition::Auto {
            continue;
        }
        let Some(start) = starts
            .iter()
            .find(|start| !used_start_indices.contains(&start.index))
        else {
            continue;
        };
        used_start_indices.insert(start.index);
        reserved_cells.insert((start.rx, start.ry));
        assignments[idx] = Some(*start);
    }

    for start in &mut assignments {
        if start.is_some() {
            continue;
        }
        let Some((rx, ry)) = find_deficient_start_fallback(resolved_terrain, &reserved_cells)
        else {
            log::warn!("No 8x8 Track-passable fallback start candidate found");
            continue;
        };
        *start = Some(Waypoint {
            index: next_fallback_index,
            rx,
            ry,
        });
        next_fallback_index = next_fallback_index.saturating_sub(1);
        reserved_cells.insert((rx, ry));
    }

    assignments
        .into_iter()
        .enumerate()
        .filter_map(|(idx, start)| start.map(|start| (idx, start)))
        .collect()
}

const DEFICIENT_START_RECT_W: u16 = 8;
const DEFICIENT_START_RECT_H: u16 = 8;

fn find_deficient_start_fallback(
    terrain: &ResolvedTerrainGrid,
    reserved_cells: &BTreeSet<(u16, u16)>,
) -> Option<(u16, u16)> {
    if terrain.width() < DEFICIENT_START_RECT_W || terrain.height() < DEFICIENT_START_RECT_H {
        return None;
    }
    let max_rx = terrain.width() - DEFICIENT_START_RECT_W;
    let max_ry = terrain.height() - DEFICIENT_START_RECT_H;
    for ry in 0..=max_ry {
        for rx in 0..=max_rx {
            if reserved_cells.contains(&(rx, ry)) {
                continue;
            }
            if deficient_start_rect_track_passable(terrain, rx, ry) {
                return Some((rx, ry));
            }
        }
    }
    None
}

fn deficient_start_rect_track_passable(terrain: &ResolvedTerrainGrid, rx: u16, ry: u16) -> bool {
    for y in ry..ry + DEFICIENT_START_RECT_H {
        for x in rx..rx + DEFICIENT_START_RECT_W {
            let Some(cell) = terrain.cell(x, y) else {
                return false;
            };
            if !deficient_start_cell_track_passable(cell) {
                return false;
            }
        }
    }
    true
}

fn deficient_start_cell_track_passable(
    cell: &crate::map::resolved_terrain::ResolvedTerrainCell,
) -> bool {
    if cell.has_bridge_deck && cell.bridge_walkable {
        return true;
    }
    // The verified deficient-start call uses MovementZone 0, whose reduced
    // movement-class row accepts only plain ground; overlay walls and outside
    // cells reject through this cached zone type.
    cell.zone_type == crate::map::resolved_terrain::zone_class::GROUND
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
    rules: &RuleSet,
    height_map: &BTreeMap<(u16, u16), u8>,
    resolved_terrain: &ResolvedTerrainGrid,
) -> Option<u64> {
    if starting_mcv_cell_placeable(sim, resolved_terrain, base_rx, base_ry) {
        return sim.spawn_object(type_id, owner, base_rx, base_ry, facing, rules, height_map);
    }

    for radius in start_radius..=STARTING_MCV_FALLBACK_MAX_RADIUS {
        for (dx, dy) in STARTING_MCV_FALLBACK_DIRECTIONS {
            let rx = base_rx as i32 + dx * radius;
            let ry = base_ry as i32 + dy * radius;
            if rx < 0 || ry < 0 {
                continue;
            }
            let (rx, ry) = (rx as u16, ry as u16);
            if !starting_mcv_cell_placeable(sim, resolved_terrain, rx, ry) {
                continue;
            }
            if let Some(id) = sim.spawn_object(type_id, owner, rx, ry, facing, rules, height_map) {
                return Some(id);
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

fn seed_starting_extra_units(
    sim: &mut Simulation,
    slots: &[NormalizedSkirmishSlot],
    rules: &RuleSet,
    height_map: &BTreeMap<(u16, u16), u8>,
    resolved_terrain: &ResolvedTerrainGrid,
    unit_count: i32,
) -> u32 {
    if unit_count <= 0 {
        return 0;
    }
    let budget = starting_unit_budget(slots, rules, unit_count, sim.session.game_options.tech_level);
    if budget <= 0 {
        return 0;
    }
    let spend_target = (budget.saturating_mul(2) / 3).max(1);
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
        let candidates =
            starting_unit_candidates_for_country(rules, slot.country, sim.session.game_options.tech_level);
        if candidates.is_empty() {
            continue;
        }
        let mut spent = 0;
        let mut index = 0;
        while spent < spend_target {
            let candidate = &candidates[index % candidates.len()];
            let candidate_cost = candidate.cost.max(1);
            if spent > 0 && spent.saturating_add(candidate_cost) > spend_target {
                break;
            }
            if place_starting_object_near_base(
                sim,
                &candidate.type_id,
                &slot.owner_name,
                base_center.0,
                base_center.1,
                STARTING_MCV_FACING,
                STARTING_EXTRA_UNIT_FALLBACK_START_RADIUS,
                rules,
                height_map,
                resolved_terrain,
            )
            .is_some()
            {
                spawned += 1;
                spent += candidate_cost;
            } else {
                break;
            }
            index += 1;
        }
    }

    spawned
}

fn starting_unit_budget(
    slots: &[NormalizedSkirmishSlot],
    rules: &RuleSet,
    unit_count: i32,
    tech_level: i32,
) -> i32 {
    if unit_count <= 0 {
        return 0;
    }
    let mut total_cost = 0;
    let mut eligible_count = 0;
    for id in rules.vehicle_ids.iter().chain(rules.infantry_ids.iter()) {
        let Some(object) = rules.object(id) else {
            continue;
        };
        if !starting_unit_candidate_allowed_for_any_slot(slots, rules, object, tech_level) {
            continue;
        }
        eligible_count += 1;
        total_cost += object.cost.max(0);
    }
    if eligible_count == 0 {
        return 0;
    }
    ((eligible_count / 2 + total_cost) / eligible_count) * unit_count
}

fn starting_unit_candidate_allowed_for_any_slot(
    slots: &[NormalizedSkirmishSlot],
    rules: &RuleSet,
    object: &crate::rules::object_type::ObjectType,
    tech_level: i32,
) -> bool {
    starting_unit_candidate_baseline_allowed(rules, object, tech_level)
        && slots
            .iter()
            .any(|slot| launch_country_can_own_object(slot.country, object))
}

fn starting_unit_candidates_for_country(
    rules: &RuleSet,
    country: LaunchCountry,
    tech_level: i32,
) -> Vec<StartingUnitCandidate> {
    rules
        .vehicle_ids
        .iter()
        .chain(rules.infantry_ids.iter())
        .filter_map(|id| {
            let object = rules.object(id)?;
            if !starting_unit_candidate_baseline_allowed(rules, object, tech_level)
                || !launch_country_can_own_object(country, object)
            {
                return None;
            }
            Some(StartingUnitCandidate {
                type_id: id.clone(),
                cost: object.cost.max(1),
            })
        })
        .collect()
}

fn starting_unit_candidate_baseline_allowed(
    rules: &RuleSet,
    object: &crate::rules::object_type::ObjectType,
    tech_level: i32,
) -> bool {
    object.allowed_to_start_in_multiplayer
        && object.tech_level <= tech_level
        && object.cost > 0
        && !rules
            .general
            .base_unit_types
            .iter()
            .any(|base_unit| base_unit.eq_ignore_ascii_case(&object.id))
        && matches!(
            object.category,
            crate::rules::object_type::ObjectCategory::Vehicle
                | crate::rules::object_type::ObjectCategory::Infantry
        )
}

fn starting_mcv_cell_placeable(
    sim: &Simulation,
    resolved_terrain: &ResolvedTerrainGrid,
    rx: u16,
    ry: u16,
) -> bool {
    if sim.occupancy().get(rx, ry).is_some() {
        return false;
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
        LaunchStartPosition, LaunchTeam, SkirmishAiSlot, SkirmishLaunchMode, SkirmishLaunchOptions,
        SkirmishLocalSlot,
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

    fn test_launch_starts() -> [Waypoint; 2] {
        [
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
            overlay_blocks: false,
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
    fn assign_launch_starts_places_explicit_slots_before_auto_slots() {
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
        let slots = normalized_launch_slots(&session);
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

        let terrain = test_terrain(16, 16);
        let assignments = assign_launch_starts(&slots, &starts, &terrain);

        assert_eq!(assignments[0], (0, starts[1]));
        assert_eq!(assignments[1], (1, starts[2]));
        assert_eq!(assignments[2], (2, starts[0]));
    }

    #[test]
    fn assign_launch_starts_generates_fallback_for_deficient_start_pool() {
        let mut session = test_session();
        session.opponents[0].start_position = LaunchStartPosition::Position(3);
        let slots = normalized_launch_slots(&session);
        let starts = [Waypoint {
            index: 3,
            rx: 30,
            ry: 30,
        }];
        let terrain = test_terrain(8, 8);

        let assignments = assign_launch_starts(&slots, &starts, &terrain);

        assert_eq!(assignments.len(), 2);
        assert_eq!(assignments[0], (0, starts[0]));
        assert_eq!(
            assignments[1],
            (
                1,
                Waypoint {
                    index: u32::MAX,
                    rx: 0,
                    ry: 0,
                }
            )
        );
    }

    #[test]
    fn assign_launch_starts_rejects_fallback_with_blocked_cell_inside_8x8_rect() {
        let slots = normalized_launch_slots(&test_session());
        let starts = [Waypoint {
            index: 3,
            rx: 30,
            ry: 30,
        }];
        let mut terrain = test_terrain(16, 8);
        let blocked = terrain.cell_mut(7, 7).unwrap();
        blocked.speed_costs.track = Some(0);
        blocked.base_speed_costs.track = Some(0);
        blocked.zone_type = zone_class::IMPASSABLE;

        let assignments = assign_launch_starts(&slots, &starts, &terrain);

        assert_eq!(assignments.len(), 2);
        assert_eq!(assignments[0], (0, starts[0]));
        assert_eq!(
            assignments[1],
            (
                1,
                Waypoint {
                    index: u32::MAX,
                    rx: 8,
                    ry: 0,
                }
            )
        );
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
        let alliances = launch_alliance_map(&roster_with_neutral_and_playable(), &slots);

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
        let slots = normalized_launch_slots(&test_session());
        let alliances = launch_alliance_map(&roster_with_neutral_and_playable(), &slots);

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
    fn skirmish_launch_start_position_and_team_are_independent() {
        let mut session = test_session();
        session.local.start_position = LaunchStartPosition::Position(3);
        session.local.team = LaunchTeam::Team(0);
        session.opponents[0].start_position = LaunchStartPosition::Position(0);
        session.opponents[0].team = LaunchTeam::Team(0);
        let slots = normalized_launch_slots(&session);
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
        ];
        let terrain = test_terrain(16, 16);

        let assignments = assign_launch_starts(&slots, &starts, &terrain);
        let alliances = launch_alliance_map(&roster_with_neutral_and_playable(), &slots);

        assert_eq!(assignments[0], (0, starts[1]));
        assert_eq!(assignments[1], (1, starts[0]));
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

        let result = apply_skirmish_launch_session(
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
            Some((10, 10))
        );
    }

    #[test]
    fn skirmish_bases_off_still_allows_unit_count_extra_units() {
        let mut sim = Simulation::new();
        let mut session = test_session();
        session.options.bases = false;
        session.options.unit_count = 1;
        let terrain = test_terrain(64, 64);
        let starts = test_launch_starts();
        let map = test_map_with_starts(&starts);
        let rules = test_starting_unit_rules();

        let result = apply_skirmish_launch_session(
            &mut sim,
            &map,
            &roster_with_neutral_and_playable(),
            &rules,
            &test_height_map(),
            &terrain,
            &session,
        );

        assert_eq!(result.spawned_mcvs, 0);
        assert_eq!(sim.entities().len(), 2);
        assert_eq!(entity_position_for_owner(&sim, "Player"), Some((30, 30)));
        assert_eq!(entity_position_for_owner(&sim, "Computer1"), Some((10, 10)));
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

        let result = apply_skirmish_launch_session(
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

        let result = apply_skirmish_launch_session(
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

        let result = apply_skirmish_launch_session(
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
        assert_eq!(entity_position_for_owner(&sim, "Player"), Some((30, 29)));
    }

    #[test]
    fn skirmish_start_unit_budget_filters_spawnable_tech_and_house_mask() {
        let rules = test_starting_unit_rules();
        let slots = normalized_launch_slots(&test_session());

        assert_eq!(starting_unit_budget(&slots, &rules, 2, 10), 334);

        let allied_candidates =
            starting_unit_candidates_for_country(&rules, LaunchCountry::America, 10);
        let allied_ids: Vec<&str> = allied_candidates
            .iter()
            .map(|candidate| candidate.type_id.as_str())
            .collect();
        assert_eq!(allied_ids, vec!["MTNK", "E1"]);

        let soviet_candidates =
            starting_unit_candidates_for_country(&rules, LaunchCountry::Russia, 10);
        let soviet_ids: Vec<&str> = soviet_candidates
            .iter()
            .map(|candidate| candidate.type_id.as_str())
            .collect();
        assert_eq!(soviet_ids, vec!["HTNK"]);
    }

    #[test]
    fn skirmish_start_unit_budget_excludes_baseunit_entries() {
        let rules = test_starting_unit_rules();

        let allied_candidates =
            starting_unit_candidates_for_country(&rules, LaunchCountry::America, 10);

        assert!(
            !allied_candidates
                .iter()
                .any(|candidate| candidate.type_id == "AMCV")
        );
    }

    #[test]
    fn skirmish_positive_unit_count_spawns_extra_starting_units() {
        let mut sim = Simulation::new();
        let mut session = test_session();
        session.options.unit_count = 1;
        let terrain = test_terrain(64, 64);
        let starts = test_launch_starts();
        let map = test_map_with_starts(&starts);
        let rules = test_starting_unit_rules();

        let result = apply_skirmish_launch_session(
            &mut sim,
            &map,
            &roster_with_neutral_and_playable(),
            &rules,
            &test_height_map(),
            &terrain,
            &session,
        );

        assert_eq!(result.spawned_mcvs, 2);
        assert_eq!(sim.entities().len(), 4);
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
        assert_eq!(player_units, 2);
        assert_eq!(ai_units, 2);
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
