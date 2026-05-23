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
use crate::map::waypoints;
use crate::map::waypoints::Waypoint;
use crate::render::batch::BatchRenderer;
use crate::render::bridge_atlas::{self, BridgeAtlas};
use crate::render::bridge_railing_atlas::{self, BridgeRailingAtlas, BridgeRailingTileBases};
use crate::render::gpu::GpuContext;
use crate::render::overlay_atlas::{self, OverlayAtlas};
use crate::rules::art_data::ArtRegistry;
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
    pub(crate) unsupported_deficient_starts: bool,
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
        colors.insert(slot.owner_name, HouseColorIndex(slot.color_index));
    }
    colors
}

pub(crate) fn apply_skirmish_launch_session(
    sim: &mut Simulation,
    map_data: &MapFile,
    house_roster: &HouseRoster,
    rules: &RuleSet,
    height_map: &BTreeMap<(u16, u16), u8>,
    session: &SkirmishLaunchSession,
) -> SkirmishLaunchApplyResult {
    let slots = normalized_launch_slots(session);
    let ai_difficulty = session
        .opponents
        .first()
        .map(|slot| slot.difficulty)
        .unwrap_or_default();
    sim.game_options = session
        .options
        .to_game_options(session.opponents.len() as i32, ai_difficulty);

    sim.houses.clear();
    sim.ai_players.clear();
    populate_non_player_houses(sim, house_roster);
    populate_launch_houses(sim, &slots);
    sim.house_alliances = launch_alliance_map(house_roster, &slots);
    recount_house_owned_counts(sim);

    let starts = waypoints::multiplayer_start_waypoints(&map_data.waypoints);
    let (assignments, unsupported_deficient_starts) = assign_launch_starts(&slots, &starts);
    let mut spawned_mcvs = 0;
    let mut local_owner = slots.first().map(|slot| slot.owner_name.clone());

    for (slot_idx, waypoint) in assignments {
        let Some(slot) = slots.get(slot_idx) else {
            continue;
        };
        let mcv_type = launch_mcv_type_for_country(slot.country, rules);
        if sim
            .spawn_object(
                mcv_type,
                &slot.owner_name,
                waypoint.rx,
                waypoint.ry,
                64,
                rules,
                height_map,
            )
            .is_some()
        {
            spawned_mcvs += 1;
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

    SkirmishLaunchApplyResult {
        local_owner,
        spawned_mcvs,
        active_slots: slots.len(),
        unsupported_deficient_starts,
    }
}

fn normalized_launch_slots(session: &SkirmishLaunchSession) -> Vec<NormalizedSkirmishSlot> {
    let mut slots = Vec::with_capacity(1 + session.opponents.len());
    slots.push(NormalizedSkirmishSlot {
        owner_name: "Player".to_string(),
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
                sim.game_options.starting_credits,
                sim.game_options.tech_level,
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
                sim.game_options.starting_credits,
                sim.game_options.tech_level,
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
        .entities
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
) -> (Vec<(usize, Waypoint)>, bool) {
    let mut assignments: Vec<Option<Waypoint>> = vec![None; slots.len()];
    let mut used = BTreeSet::new();
    let mut unsupported = starts.len() < slots.len();

    for (idx, slot) in slots.iter().enumerate() {
        let LaunchStartPosition::Position(position) = slot.start_position else {
            continue;
        };
        let Some(start) = starts.iter().find(|start| start.index == position as u32) else {
            unsupported = true;
            continue;
        };
        if used.insert(start.index) {
            assignments[idx] = Some(*start);
        } else {
            unsupported = true;
        }
    }

    for (idx, slot) in slots.iter().enumerate() {
        if assignments[idx].is_some() || slot.start_position != LaunchStartPosition::Auto {
            continue;
        }
        let Some(start) = starts.iter().find(|start| !used.contains(&start.index)) else {
            unsupported = true;
            continue;
        };
        used.insert(start.index);
        assignments[idx] = Some(*start);
    }

    (
        assignments
            .into_iter()
            .enumerate()
            .filter_map(|(idx, start)| start.map(|start| (idx, start)))
            .collect(),
        unsupported,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::houses::HouseDefinition;
    use crate::skirmish_launch::{
        LaunchStartPosition, LaunchTeam, SkirmishAiSlot, SkirmishLaunchMode, SkirmishLaunchOptions,
        SkirmishLocalSlot,
    };

    fn test_session() -> SkirmishLaunchSession {
        SkirmishLaunchSession {
            mode: SkirmishLaunchMode::Battle,
            selected_map_file: Some("test.mmx".to_string()),
            local: SkirmishLocalSlot {
                country: LaunchCountry::America,
                color_index: 1,
                start_position: LaunchStartPosition::Position(3),
                team: LaunchTeam::None,
            },
            opponents: vec![SkirmishAiSlot {
                country: LaunchCountry::Russia,
                color_index: 2,
                start_position: LaunchStartPosition::Auto,
                team: LaunchTeam::None,
                difficulty: Default::default(),
            }],
            options: SkirmishLaunchOptions::default(),
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

        assert_eq!(colors.get("Neutral"), Some(&HouseColorIndex(8)));
        assert_eq!(colors.get("Player"), Some(&HouseColorIndex(1)));
        assert_eq!(colors.get("Computer1"), Some(&HouseColorIndex(2)));
        assert!(!colors.contains_key("Americans"));
    }

    #[test]
    fn assign_launch_starts_places_explicit_slots_before_auto_slots() {
        let mut session = test_session();
        session.opponents.push(SkirmishAiSlot {
            country: LaunchCountry::Cuba,
            color_index: 3,
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

        let (assignments, unsupported) = assign_launch_starts(&slots, &starts);

        assert!(!unsupported);
        assert_eq!(assignments[0], (0, starts[1]));
        assert_eq!(assignments[1], (1, starts[2]));
        assert_eq!(assignments[2], (2, starts[0]));
    }

    #[test]
    fn assign_launch_starts_marks_deficient_start_pool() {
        let slots = normalized_launch_slots(&test_session());
        let starts = [Waypoint {
            index: 3,
            rx: 30,
            ry: 30,
        }];

        let (assignments, unsupported) = assign_launch_starts(&slots, &starts);

        assert!(unsupported);
        assert_eq!(assignments, vec![(0, starts[0])]);
    }
}

fn launch_mcv_type_for_country(country: LaunchCountry, rules: &RuleSet) -> &'static str {
    country
        .opening_mcv_candidates()
        .iter()
        .copied()
        .find(|id| rules.object(id).is_some())
        .unwrap_or("AMCV")
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
