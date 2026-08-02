//! Offline Skirmish shell process state and close-transaction helpers.
//!
//! The UI owns raw controls, including Random sentinels. This app-layer owner
//! retains the durable `[Skirmish]` snapshot, the process-continuity Scenario
//! RNG, and Cooperative progress records across shell closes and matches.

use std::io;
use std::path::{Path, PathBuf};

use crate::app_init::MapMenuEntry;
use crate::assets::asset_manager::AssetManager;
use crate::rules::ini_parser::IniFile;
use crate::sim::rng::SimRng;
use crate::skirmish_cooperative::{
    CooperativeCountryRole, CooperativeCountryRosterEntry, CooperativeError,
    CooperativeProgressRecord, CooperativeProgressState, CooperativeRegistry,
    draw_country_for_progress,
};
use crate::skirmish_launch::{
    LaunchCountry, RandomCountryRole, ShellRandomAssignmentState, SkirmishLaunchSession,
};
use crate::skirmish_modes::{SkirmishGameMode, mode_by_id};
use crate::skirmish_persistence::{
    SKIRMISH_PERSISTED_SLOT_COUNT, SkirmishGlobalDefaults, SkirmishPersistedSlot,
    SkirmishPersistedSnapshot, read_skirmish_snapshot,
};
use crate::ui::main_menu::SkirmishCountry;
use crate::ui::skirmish_shell::{
    PlayerNameEditState, SkirmishAiRowType, SkirmishShellState, SkirmishTrackbarId,
};

const RA2MD_INI: &str = "RA2MD.INI";
const RANDOM_ITEM_DATA: i32 = -2;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct LocalMultiplayerPreferences {
    player_name: Option<String>,
    country: Option<SkirmishCountry>,
    color_index: Option<usize>,
}

/// App-owned state whose lifetime matches the front-end process, not one map.
pub(crate) struct OfflineSkirmishRuntime {
    snapshot: SkirmishPersistedSnapshot,
    local_preferences: LocalMultiplayerPreferences,
    scenario_rng: SimRng,
    ini_path: Option<PathBuf>,
    cooperative_registry: CooperativeRegistry,
    cooperative_progress: CooperativeProgressState,
    cooperative_country_roster: Vec<CooperativeCountryRosterEntry>,
    gameplay_rng_return_pending: bool,
}

impl OfflineSkirmishRuntime {
    /// Establish the front-end Scenario cursor, then construct Cooperative
    /// progress before reading the durable shell snapshot. Stock Cooperative
    /// data advances the freshly seeded cursor by ten logical `(0,2)` calls.
    pub(crate) fn initialize(
        seed: u32,
        ra2_dir: Option<&Path>,
        assets: Option<&AssetManager>,
        defaults: SkirmishGlobalDefaults,
    ) -> Self {
        let mut scenario_rng = SimRng::new(u64::from(seed));
        let cooperative_registry = assets
            .map(CooperativeRegistry::from_assets)
            .transpose()
            .unwrap_or_else(|err| {
                log::warn!("Could not load Cooperative campaign registry: {err}");
                None
            })
            .unwrap_or_default();
        let cooperative_progress =
            CooperativeProgressState::construct(&cooperative_registry, &mut scenario_rng)
                .unwrap_or_else(|err| {
                    log::warn!("Could not construct Cooperative progress state: {err}");
                    CooperativeProgressState::default()
                });
        let cooperative_country_roster = assets
            .map(cooperative_country_roster_from_assets)
            .unwrap_or_else(stock_cooperative_country_roster);
        let ini_path = ra2_dir.map(|root| root.join(RA2MD_INI));
        let (snapshot, local_preferences) = load_persistence(ini_path.as_deref(), defaults);

        Self {
            snapshot,
            local_preferences,
            scenario_rng,
            ini_path,
            cooperative_registry,
            cooperative_progress,
            cooperative_country_roster,
            gameplay_rng_return_pending: false,
        }
    }

    #[cfg(test)]
    fn snapshot(&self) -> &SkirmishPersistedSnapshot {
        &self.snapshot
    }

    /// Apply the one-time process snapshot to raw shell controls. The caller
    /// reconstructs Team defaults and map-capacity visibility afterward.
    pub(crate) fn hydrate_shell(
        &self,
        state: &mut SkirmishShellState,
        maps: &[MapMenuEntry],
        modes: &[SkirmishGameMode],
    ) {
        state.selected_mode_id = mode_by_id(modes, self.snapshot.game_mode)
            .or_else(|| modes.first())
            .map(|mode| mode.id)
            .unwrap_or(state.selected_mode_id);
        state.selected_map_idx = usize::try_from(self.snapshot.scenario_index)
            .ok()
            .filter(|index| *index < maps.len())
            .unwrap_or(0);

        let (speed_min, speed_max, _) = state
            .trackbar_bounds
            .range(SkirmishTrackbarId::GameSpeed0x529);
        let (credits_min, credits_max, _) = state
            .trackbar_bounds
            .range(SkirmishTrackbarId::Credits0x511);
        let (units_min, units_max, _) = state
            .trackbar_bounds
            .range(SkirmishTrackbarId::UnitCount0x50c);
        state.game_speed = clamp_between(self.snapshot.game_speed, speed_min, speed_max);
        state.starting_credits = clamp_between(self.snapshot.credits, credits_min, credits_max);
        state.unit_count = clamp_between(self.snapshot.unit_count, units_min, units_max);
        state.short_game = self.snapshot.short_game;
        state.super_weapons = self.snapshot.super_weapons_allowed;
        state.build_off_ally = self.snapshot.build_off_ally;
        state.mcv_redeploy = self.snapshot.mcv_repacks;
        state.crates = self.snapshot.crates_appear;

        if let Some(player_name) = self.local_preferences.player_name.as_deref() {
            state.player_name_edit = PlayerNameEditState::with_name(player_name);
        }
        if let Some(country) = self.local_preferences.country {
            state.player_country = country;
            state.player_country_random = false;
        }
        if let Some(color_index) = self.local_preferences.color_index {
            state.player_color_index = color_index;
            state.player_color_claimed = true;
        }

        for (opponent, persisted) in state.opponents.iter_mut().zip(self.snapshot.slots) {
            opponent.row_type = ai_row_type_from_persisted(persisted.row_type);
            opponent.enabled = opponent.row_type.is_active();
            if let Some(difficulty) = opponent.row_type.difficulty() {
                opponent.difficulty = difficulty;
            }

            opponent.country_random = persisted.country == RANDOM_ITEM_DATA;
            if let Some(country) = menu_country_from_item_data(persisted.country) {
                opponent.country = country;
                opponent.country_random = false;
            }

            opponent.color_claimed =
                (0..crate::skirmish_launch::HOUSE_COLOR_COUNT as i32).contains(&persisted.colour);
            if opponent.color_claimed {
                opponent.color_index = persisted.colour as usize;
            }
        }
    }

    /// Pack every durable raw control value without resolving a launch copy.
    /// Production Start/Back uses [`Self::close_shell_transaction`] so the
    /// same fields are committed at their verified native phase boundaries.
    #[cfg(test)]
    pub(crate) fn pack_shell_snapshot(
        &mut self,
        state: &SkirmishShellState,
        maps: &[MapMenuEntry],
        modes: &[SkirmishGameMode],
    ) {
        pack_snapshot_selection(&mut self.snapshot, state, maps, modes);
        pack_snapshot_slots(&mut self.snapshot, state);
        pack_snapshot_options(&mut self.snapshot, state);
    }

    /// Run the common Start/Back state transaction in the verified order:
    /// selection fields, local country, raw Slot triples, local colour, all AI
    /// assignments, then slider/checkbox mirrors. The raw UI state is borrowed
    /// throughout and never replaced by the resolved launch copy.
    pub(crate) fn close_shell_transaction(
        &mut self,
        state: &SkirmishShellState,
        maps: &[MapMenuEntry],
        modes: &[SkirmishGameMode],
        session: &SkirmishLaunchSession,
    ) -> Result<SkirmishLaunchSession, CooperativeError> {
        pack_snapshot_selection(&mut self.snapshot, state, maps, modes);

        // Validation may already have built a borrowed-input session, but it is
        // a pure local read. Material launch state deliberately starts without
        // AI arrays so their first write occurs after the local-country draw.
        let mut staged_session = SkirmishLaunchSession {
            mode: session.mode.clone(),
            selected_map_file: session.selected_map_file.clone(),
            player_name: session.player_name.clone(),
            local: session.local.clone(),
            opponents: Vec::new(),
            options: session.options.clone(),
        };
        let cooperative = is_cooperative_mode(session);
        if cooperative && let Some(map) = session.selected_map_file.as_deref() {
            if let Some(chosen_map) = self.ensure_cooperative_selection(map, maps)? {
                staged_session.selected_map_file = Some(chosen_map);
            }
        }

        let progress = cooperative.then(|| {
            self.cooperative_progress
                .active()
                .cloned()
                .unwrap_or_else(CooperativeProgressRecord::default)
        });
        let registry = &self.cooperative_registry;
        let roster = &self.cooperative_country_roster;
        let mut assignments = ShellRandomAssignmentState::new(&staged_session);
        let mut draw_country =
            |role, rng: &mut SimRng| -> Result<LaunchCountry, CooperativeError> {
                if let Some(progress) = progress.as_ref() {
                    let cooperative_role = match role {
                        RandomCountryRole::Human => CooperativeCountryRole::Player,
                        RandomCountryRole::Ai { .. } => CooperativeCountryRole::Enemy,
                    };
                    draw_country_for_progress(registry, progress, cooperative_role, roster, rng)
                        .map(|index| LaunchCountry::from_country_index(index as u32))
                } else {
                    Ok(LaunchCountry::from_country_index(
                        rng.next_range_u32_inclusive(0, 9),
                    ))
                }
            };

        assignments.resolve_local_country(&mut self.scenario_rng, &mut draw_country)?;
        assignments.pack_ai_assignments(&session.opponents);
        pack_snapshot_slots(&mut self.snapshot, state);
        assignments.resolve_local_color(&mut self.scenario_rng);
        assignments.resolve_ai(&mut self.scenario_rng, &mut draw_country)?;
        pack_snapshot_options(&mut self.snapshot, state);
        Ok(assignments.finish())
    }

    /// Resolve a borrowed raw session on the process-continuity Scenario RNG.
    /// The shell and durable snapshot remain unchanged.
    #[cfg(test)]
    pub(crate) fn resolve_launch_session(
        &mut self,
        session: &SkirmishLaunchSession,
        maps: &[MapMenuEntry],
    ) -> Result<SkirmishLaunchSession, CooperativeError> {
        if !is_cooperative_mode(session) {
            return Ok(session.resolve_shell_random_assignments(&mut self.scenario_rng));
        }

        let mut cooperative_session = session.clone();
        if let Some(map) = session.selected_map_file.as_deref() {
            if let Some(chosen_map) = self.ensure_cooperative_selection(map, maps)? {
                cooperative_session.selected_map_file = Some(chosen_map);
            }
        }
        let progress = self
            .cooperative_progress
            .active()
            .cloned()
            .unwrap_or_else(CooperativeProgressRecord::default);
        let registry = &self.cooperative_registry;
        let roster = &self.cooperative_country_roster;
        let rng = &mut self.scenario_rng;
        cooperative_session.try_resolve_shell_random_assignments_with(rng, |role, rng| {
            let cooperative_role = match role {
                RandomCountryRole::Human => CooperativeCountryRole::Player,
                RandomCountryRole::Ai { .. } => CooperativeCountryRole::Enemy,
            };
            draw_country_for_progress(registry, &progress, cooperative_role, roster, rng)
                .map(|index| LaunchCountry::from_country_index(index as u32))
        })
    }

    /// Bind/open the active Cooperative progress record for a highlighted map.
    /// Reopening an already active campaign consumes no map-variant draw.
    pub(crate) fn ensure_cooperative_selection(
        &mut self,
        scenario: &str,
        available_maps: &[MapMenuEntry],
    ) -> Result<Option<String>, CooperativeError> {
        let Some(campaign_index) = self.cooperative_registry.campaign_for_map(scenario) else {
            return Ok(None);
        };
        let rng_checkpoint = self.scenario_rng.clone();
        let progress_checkpoint = self.cooperative_progress.clone();
        let result = (|| {
            self.cooperative_progress.ensure_active(
                &self.cooperative_registry,
                campaign_index,
                &mut self.scenario_rng,
            )?;
            let chosen = self
                .cooperative_progress
                .active()
                .and_then(CooperativeProgressRecord::current_chosen_map)
                .map(str::to_string);
            validate_cooperative_map_binding(chosen, available_maps)
        })();
        if result.is_err() {
            self.scenario_rng = rng_checkpoint;
            self.cooperative_progress = progress_checkpoint;
        }
        result
    }

    /// Apply an accepted Cooperative campaign selection. Moving to another
    /// campaign swaps in its preconstructed record and initializes exactly one
    /// replacement reserve record on the same front-end RNG.
    pub(crate) fn accept_cooperative_selection(
        &mut self,
        scenario: &str,
        available_maps: &[MapMenuEntry],
    ) -> Result<Option<String>, CooperativeError> {
        let Some(campaign_index) = self.cooperative_registry.campaign_for_map(scenario) else {
            return Ok(None);
        };
        let rng_checkpoint = self.scenario_rng.clone();
        let progress_checkpoint = self.cooperative_progress.clone();
        let result = (|| {
            if self.cooperative_progress.active().is_none() {
                self.cooperative_progress.ensure_active(
                    &self.cooperative_registry,
                    campaign_index,
                    &mut self.scenario_rng,
                )?;
            } else if self
                .cooperative_progress
                .active()
                .is_some_and(|active| active.campaign_type != campaign_index as i32)
            {
                self.cooperative_progress.accept_campaign_swap(
                    &self.cooperative_registry,
                    campaign_index,
                    &mut self.scenario_rng,
                )?;
            }
            let chosen = self
                .cooperative_progress
                .active()
                .and_then(CooperativeProgressRecord::current_chosen_map)
                .map(str::to_string);
            validate_cooperative_map_binding(chosen, available_maps)
        })();
        if result.is_err() {
            self.scenario_rng = rng_checkpoint;
            self.cooperative_progress = progress_checkpoint;
        }
        result
    }

    /// Apply all snapshot keys to one in-memory buffer and perform one write.
    /// Failure is intentionally nonfatal, matching the native caller.
    pub(crate) fn persist_snapshot(&self) {
        let Some(path) = self.ini_path.as_deref() else {
            return;
        };
        let existing = match std::fs::read(path) {
            Ok(bytes) => bytes,
            Err(err) if err.kind() == io::ErrorKind::NotFound => Vec::new(),
            Err(err) => {
                log::warn!(
                    "Could not read {} before Skirmish persistence: {err}",
                    path.display()
                );
                return;
            }
        };
        let updated = self.snapshot.update_ini_bytes(&existing);
        if let Err(err) = std::fs::write(path, updated) {
            log::warn!(
                "Could not persist Skirmish settings to {}: {err}",
                path.display()
            );
        }
    }

    pub(crate) fn mark_gameplay_rng_return_pending(&mut self) {
        self.gameplay_rng_return_pending = true;
    }

    /// Restore the process Scenario cursor from a normally returning match.
    pub(crate) fn capture_returned_gameplay_rng(&mut self, gameplay_rng: SimRng) -> bool {
        if !self.gameplay_rng_return_pending {
            return false;
        }
        self.scenario_rng = gameplay_rng;
        self.gameplay_rng_return_pending = false;
        true
    }

    #[cfg(test)]
    fn scenario_rng_state(&self) -> u64 {
        self.scenario_rng.state()
    }
}

fn pack_snapshot_selection(
    snapshot: &mut SkirmishPersistedSnapshot,
    state: &SkirmishShellState,
    maps: &[MapMenuEntry],
    modes: &[SkirmishGameMode],
) {
    snapshot.game_mode = mode_by_id(modes, state.selected_mode_id)
        .or_else(|| modes.first())
        .map(|mode| mode.id)
        .unwrap_or(0);
    snapshot.scenario_index = if state.selected_map_idx < maps.len() {
        i32::try_from(state.selected_map_idx).unwrap_or(0)
    } else {
        0
    };
}

fn pack_snapshot_slots(snapshot: &mut SkirmishPersistedSnapshot, state: &SkirmishShellState) {
    for (index, destination) in snapshot.slots.iter_mut().enumerate() {
        let Some(opponent) = state.opponents.get(index) else {
            *destination = SkirmishPersistedSlot {
                row_type: 1,
                country: RANDOM_ITEM_DATA,
                colour: RANDOM_ITEM_DATA,
            };
            continue;
        };
        *destination = SkirmishPersistedSlot {
            row_type: persisted_row_type(opponent.row_type),
            country: if opponent.country_random {
                RANDOM_ITEM_DATA
            } else {
                menu_country_item_data(opponent.country)
            },
            colour: if opponent.color_claimed {
                i32::try_from(opponent.color_index).unwrap_or(RANDOM_ITEM_DATA)
            } else {
                RANDOM_ITEM_DATA
            },
        };
    }
}

fn pack_snapshot_options(snapshot: &mut SkirmishPersistedSnapshot, state: &SkirmishShellState) {
    snapshot.game_speed = state.game_speed;
    snapshot.credits = state.starting_credits;
    snapshot.unit_count = state.unit_count;
    snapshot.short_game = state.short_game;
    snapshot.super_weapons_allowed = state.super_weapons;
    snapshot.build_off_ally = state.build_off_ally;
    snapshot.mcv_repacks = state.mcv_redeploy;
    snapshot.crates_appear = state.crates;
}

fn validate_cooperative_map_binding(
    chosen: Option<String>,
    available_maps: &[MapMenuEntry],
) -> Result<Option<String>, CooperativeError> {
    if let Some(chosen_map) = chosen.as_deref()
        && !available_maps
            .iter()
            .any(|map| map.file_name.eq_ignore_ascii_case(chosen_map))
    {
        return Err(CooperativeError::MissingScenarioMap {
            scenario: chosen_map.to_string(),
        });
    }
    Ok(chosen)
}

pub(crate) fn skirmish_global_defaults(state: &SkirmishShellState) -> SkirmishGlobalDefaults {
    SkirmishGlobalDefaults {
        game_mode: state.selected_mode_id,
        scenario_index: i32::try_from(state.selected_map_idx).unwrap_or(0),
        game_speed: state.game_speed,
        credits: state.starting_credits,
        unit_count: state.unit_count,
        short_game: state.short_game,
        super_weapons_allowed: state.super_weapons,
        build_off_ally: state.build_off_ally,
        mcv_repacks: state.mcv_redeploy,
        crates_appear: state.crates,
    }
}

fn load_persistence(
    path: Option<&Path>,
    defaults: SkirmishGlobalDefaults,
) -> (SkirmishPersistedSnapshot, LocalMultiplayerPreferences) {
    let fallback = || {
        (
            SkirmishPersistedSnapshot::from_global_defaults(defaults),
            LocalMultiplayerPreferences::default(),
        )
    };
    let Some(path) = path else {
        return fallback();
    };
    match std::fs::read(path) {
        Ok(bytes) => {
            let snapshot = read_skirmish_snapshot(&bytes, defaults).unwrap_or_else(|err| {
                log::warn!(
                    "Could not parse Skirmish settings from {}: {err}",
                    path.display()
                );
                SkirmishPersistedSnapshot::from_global_defaults(defaults)
            });
            (snapshot, read_local_multiplayer_preferences(&bytes))
        }
        Err(err) if err.kind() == io::ErrorKind::NotFound => fallback(),
        Err(err) => {
            log::warn!(
                "Could not read Skirmish settings from {}: {err}",
                path.display()
            );
            fallback()
        }
    }
}

fn read_local_multiplayer_preferences(bytes: &[u8]) -> LocalMultiplayerPreferences {
    let Ok(ini) = IniFile::from_bytes(bytes) else {
        return LocalMultiplayerPreferences::default();
    };
    let Some(section) = ini.section("MultiPlayer") else {
        return LocalMultiplayerPreferences::default();
    };

    let player_name = section.get("Handle").and_then(decode_multiplayer_handle);
    let country = section.get("Side").and_then(|side| {
        SkirmishCountry::ALL
            .into_iter()
            .find(|country| country.country_name().eq_ignore_ascii_case(side.trim()))
    });
    let color_index = section
        .get_i32("Color")
        .and_then(|value| usize::try_from(value).ok())
        .filter(|value| *value < crate::skirmish_launch::HOUSE_COLOR_COUNT);

    LocalMultiplayerPreferences {
        player_name,
        country,
        color_index,
    }
}

fn decode_multiplayer_handle(value: &str) -> Option<String> {
    let mut bytes = Vec::new();
    for part in value.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let unit = u32::from_str_radix(part, 16).ok()? as u16;
        let byte = unit as u8;
        if byte == 0 {
            break;
        }
        bytes.push(byte);
    }
    if bytes.is_empty() {
        return None;
    }
    let decoded = crate::util::native_string::widen_bytes(&bytes);
    (!decoded.is_empty()).then_some(decoded)
}

fn is_cooperative_mode(session: &SkirmishLaunchSession) -> bool {
    session
        .mode
        .override_file
        .eq_ignore_ascii_case("MPCoopMD.ini")
}

fn clamp_between(value: i32, left: i32, right: i32) -> i32 {
    value.clamp(left.min(right), left.max(right))
}

fn ai_row_type_from_persisted(value: i32) -> SkirmishAiRowType {
    match value {
        4 => SkirmishAiRowType::Hard,
        5 => SkirmishAiRowType::Normal,
        6 => SkirmishAiRowType::Easy,
        _ => SkirmishAiRowType::None,
    }
}

fn persisted_row_type(value: SkirmishAiRowType) -> i32 {
    match value {
        SkirmishAiRowType::None => 1,
        SkirmishAiRowType::Hard => 4,
        SkirmishAiRowType::Normal => 5,
        SkirmishAiRowType::Easy => 6,
    }
}

fn menu_country_from_item_data(value: i32) -> Option<SkirmishCountry> {
    usize::try_from(value)
        .ok()
        .and_then(|index| SkirmishCountry::ALL.get(index).copied())
}

fn menu_country_item_data(country: SkirmishCountry) -> i32 {
    SkirmishCountry::ALL
        .iter()
        .position(|candidate| *candidate == country)
        .and_then(|index| i32::try_from(index).ok())
        .unwrap_or(0)
}

fn cooperative_country_roster_from_assets(
    assets: &AssetManager,
) -> Vec<CooperativeCountryRosterEntry> {
    let rules = crate::app_init_helpers::load_retail_rules_source(assets);
    SkirmishCountry::ALL
        .into_iter()
        .map(|country| {
            let id = country.country_name();
            let name = rules
                .as_ref()
                .and_then(|ini| ini.section(id))
                .and_then(|section| section.get("Name"));
            CooperativeCountryRosterEntry::new(id, name)
        })
        .collect()
}

fn stock_cooperative_country_roster() -> Vec<CooperativeCountryRosterEntry> {
    SkirmishCountry::ALL
        .into_iter()
        .map(|country| {
            CooperativeCountryRosterEntry::new(country.country_name(), Some(country.label()))
        })
        .collect()
}

const _: [(); SKIRMISH_PERSISTED_SLOT_COUNT] = [(); crate::skirmish_launch::SKIRMISH_AI_SLOT_COUNT];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::briefing::BriefingSection;
    use crate::map::preview::PreviewSection;
    use crate::skirmish_launch::{
        AiDifficulty, LaunchStartPosition, LaunchTeam, SkirmishAiSlot, SkirmishLaunchMode,
        SkirmishLaunchOptions, SkirmishLocalSlot,
    };
    use crate::ui::main_menu::StartPosition;

    fn map_named(file_name: &str) -> MapMenuEntry {
        MapMenuEntry {
            file_name: file_name.to_string(),
            display_name: "Test".to_string(),
            author: None,
            briefing: BriefingSection::default(),
            preview: PreviewSection::default(),
            multiplayer_start_waypoints: Vec::new(),
            player_capacity: 8,
            preview_source_bounds: None,
        }
    }

    fn map() -> MapMenuEntry {
        map_named("test.mmx")
    }

    fn cooperative_maps() -> Vec<MapMenuEntry> {
        ["A1", "A1B", "A1C", "A2", "A2B", "A2C", "W1", "W1B", "W1C"]
            .into_iter()
            .map(map_named)
            .collect()
    }

    fn mode() -> SkirmishGameMode {
        SkirmishGameMode {
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

    fn cooperative_mode() -> SkirmishGameMode {
        SkirmishGameMode {
            override_file: "MPCoopMD.ini".to_string(),
            ..mode()
        }
    }

    fn cooperative_registry() -> CooperativeRegistry {
        CooperativeRegistry::from_ini(&IniFile::from_str(
            "[Campaigns]\n1=Allied\n2=World\n\
             [Allied]\nNumberOfCampaignMaps=2\n\
             CampaignPlayer1=Americans\nCampaignEnemy1=Russians\n\
             CampaignPlayer2=Americans\nCampaignEnemy2=Russians\n\
             Map1=A1,A1B,A1C\nMap2=A2,A2B,A2C\n\
             [World]\nNumberOfCampaignMaps=1\n\
             CampaignPlayer1=Americans\nCampaignEnemy1=Russians\n\
             Map1=W1,W1B,W1C\n",
        ))
        .expect("Cooperative fixture")
    }

    fn cooperative_runtime(seed: u32) -> OfflineSkirmishRuntime {
        let cooperative_registry = cooperative_registry();
        let mut scenario_rng = SimRng::new(u64::from(seed));
        let cooperative_progress =
            CooperativeProgressState::construct(&cooperative_registry, &mut scenario_rng)
                .expect("preconstructed Cooperative records");
        OfflineSkirmishRuntime {
            snapshot: SkirmishPersistedSnapshot::from_global_defaults(defaults()),
            local_preferences: LocalMultiplayerPreferences::default(),
            scenario_rng,
            ini_path: None,
            cooperative_registry,
            cooperative_progress,
            cooperative_country_roster: stock_cooperative_country_roster(),
            gameplay_rng_return_pending: false,
        }
    }

    fn runtime(snapshot: SkirmishPersistedSnapshot, seed: u32) -> OfflineSkirmishRuntime {
        OfflineSkirmishRuntime {
            snapshot,
            local_preferences: LocalMultiplayerPreferences::default(),
            scenario_rng: SimRng::new(u64::from(seed)),
            ini_path: None,
            cooperative_registry: CooperativeRegistry::default(),
            cooperative_progress: CooperativeProgressState::default(),
            cooperative_country_roster: stock_cooperative_country_roster(),
            gameplay_rng_return_pending: false,
        }
    }

    fn defaults() -> SkirmishGlobalDefaults {
        skirmish_global_defaults(&SkirmishShellState::default())
    }

    fn random_session() -> SkirmishLaunchSession {
        SkirmishLaunchSession {
            mode: SkirmishLaunchMode::from_game_mode(&mode()),
            selected_map_file: Some("test.mmx".to_string()),
            player_name: "Player".to_string(),
            local: SkirmishLocalSlot {
                country: LaunchCountry::America,
                country_random: true,
                color_index: 0,
                color_random: true,
                start_position: LaunchStartPosition::Auto,
                team: LaunchTeam::None,
            },
            opponents: vec![SkirmishAiSlot {
                country: LaunchCountry::Russia,
                country_random: true,
                color_index: 1,
                color_random: true,
                start_position: LaunchStartPosition::Auto,
                team: LaunchTeam::None,
                difficulty: AiDifficulty::Easy,
            }],
            options: SkirmishLaunchOptions::default(),
        }
    }

    #[test]
    fn hydrate_and_pack_preserve_raw_random_slot_sentinels() {
        let mut snapshot = SkirmishPersistedSnapshot::from_global_defaults(defaults());
        snapshot.slots[0] = SkirmishPersistedSlot {
            row_type: 4,
            country: RANDOM_ITEM_DATA,
            colour: RANDOM_ITEM_DATA,
        };
        let expected_slot = snapshot.slots[0];
        let mut runtime = runtime(snapshot, 7);
        let mut shell = SkirmishShellState::default();

        runtime.hydrate_shell(&mut shell, &[map()], &[mode()]);
        assert_eq!(shell.opponents[0].row_type, SkirmishAiRowType::Hard);
        assert!(shell.opponents[0].country_random);
        assert!(!shell.opponents[0].color_claimed);

        runtime.pack_shell_snapshot(&shell, &[map()], &[mode()]);
        assert_eq!(runtime.snapshot().slots[0], expected_slot);
    }

    #[test]
    fn retail_multiplayer_preferences_decode_into_local_shell_state() {
        let preferences = read_local_multiplayer_preferences(
            b"[MultiPlayer]\n\
              Handle=5b,4e,65,77,20,50,6c,61,79,65,72,5d,\n\
              Color=2\n\
              ColorEx=-1\n\
              Side=Americans\n\
              SideEx=-1\n",
        );
        assert_eq!(
            preferences,
            LocalMultiplayerPreferences {
                player_name: Some("[New Player]".to_string()),
                country: Some(SkirmishCountry::America),
                color_index: Some(2),
            }
        );

        let mut runtime = runtime(
            SkirmishPersistedSnapshot::from_global_defaults(defaults()),
            7,
        );
        runtime.local_preferences = preferences;
        let mut shell = SkirmishShellState::default();
        runtime.hydrate_shell(&mut shell, &[map()], &[mode()]);

        assert_eq!(shell.player_name_edit.text, "[New Player]");
        assert_eq!(shell.player_country, SkirmishCountry::America);
        assert!(!shell.player_country_random);
        assert_eq!(shell.player_color_index, 2);
        assert!(shell.player_color_claimed);
    }

    #[test]
    fn malformed_multiplayer_preferences_fall_back_independently() {
        let preferences = read_local_multiplayer_preferences(
            b"[MultiPlayer]\nHandle=not-hex,\nColor=99\nSide=UnknownCountry\n",
        );
        assert_eq!(preferences, LocalMultiplayerPreferences::default());
        assert_eq!(decode_multiplayer_handle(""), None);
        assert_eq!(
            decode_multiplayer_handle("41,00,42,"),
            Some("A".to_string())
        );
        assert_eq!(decode_multiplayer_handle("e9,"), Some("\u{e9}".to_string()));
        assert_eq!(
            decode_multiplayer_handle("20ac,"),
            Some("\u{ac}".to_string())
        );
        assert_eq!(decode_multiplayer_handle("100,41,"), None);
    }

    #[test]
    fn resolving_launch_copy_does_not_mutate_raw_random_fields() {
        let snapshot = SkirmishPersistedSnapshot::from_global_defaults(defaults());
        let mut runtime = runtime(snapshot, 11);
        let session = random_session();

        let resolved = runtime
            .resolve_launch_session(&session, &[])
            .expect("ordinary resolver");

        assert!(session.local.country_random);
        assert!(session.local.color_random);
        assert!(session.opponents[0].country_random);
        assert!(session.opponents[0].color_random);
        assert!(!resolved.local.country_random);
        assert!(!resolved.local.color_random);
        assert!(!resolved.opponents[0].country_random);
        assert!(!resolved.opponents[0].color_random);
    }

    #[test]
    fn start_and_back_use_identical_shell_resolution_transcripts() {
        let snapshot = SkirmishPersistedSnapshot::from_global_defaults(defaults());
        let mut start_runtime = runtime(snapshot.clone(), 0x1234);
        let mut back_runtime = runtime(snapshot, 0x1234);
        let session = random_session();
        let shell = SkirmishShellState::default();

        let start_copy = start_runtime
            .close_shell_transaction(&shell, &[map()], &[mode()], &session)
            .expect("Start shell resolution");
        let back_copy = back_runtime
            .close_shell_transaction(&shell, &[map()], &[mode()], &session)
            .expect("Back shell resolution");

        assert_eq!(start_copy, back_copy);
        assert_eq!(start_runtime.snapshot(), back_runtime.snapshot());
        assert_eq!(
            start_runtime.scenario_rng_state(),
            back_runtime.scenario_rng_state()
        );
    }

    #[test]
    fn failed_start_validation_leaves_runtime_snapshot_and_rng_untouched() {
        let snapshot = SkirmishPersistedSnapshot::from_global_defaults(defaults());
        let runtime = runtime(snapshot.clone(), 0x5678);
        let rng_before = runtime.scenario_rng_state();
        let mut shell = SkirmishShellState::default();
        shell.selected_mode_id = mode().id;
        for opponent in &mut shell.opponents {
            opponent.row_type = SkirmishAiRowType::None;
            opponent.enabled = false;
        }

        let result = crate::ui::skirmish_shell::launch_session(&shell, &[map()], &[mode()]);

        assert_eq!(
            result,
            Err(crate::skirmish_launch::LaunchValidationError::NoEnabledOpponent)
        );
        assert_eq!(runtime.snapshot(), &snapshot);
        assert_eq!(runtime.scenario_rng_state(), rng_before);
    }

    #[test]
    fn cooperative_selection_uses_progress_chosen_map_not_clicked_variant() {
        let mut runtime = cooperative_runtime(0x2345);

        let first_campaign_map = runtime
            .ensure_cooperative_selection("A2B", &cooperative_maps())
            .expect("bind active campaign")
            .expect("chosen active map");
        assert!(["A1", "A1B", "A1C"].contains(&first_campaign_map.as_str()));
        assert_eq!(
            runtime
                .cooperative_progress
                .active()
                .map(|progress| (progress.campaign_type, progress.current_map)),
            Some((0, 0))
        );

        let switched_map = runtime
            .accept_cooperative_selection("W1B", &cooperative_maps())
            .expect("accept campaign swap")
            .expect("chosen switched map");
        assert!(["W1", "W1B", "W1C"].contains(&switched_map.as_str()));
        assert_eq!(
            runtime
                .cooperative_progress
                .active()
                .map(|progress| (progress.campaign_type, progress.current_map)),
            Some((1, 0))
        );
    }

    #[test]
    fn cooperative_resolution_replaces_clicked_variant_with_progress_choice() {
        let mut runtime = cooperative_runtime(0x3456);
        let mut session = random_session();
        session.mode = SkirmishLaunchMode::from_game_mode(&cooperative_mode());
        session.selected_map_file = Some("A2C".to_string());
        session.local.country_random = false;
        session.local.color_random = false;
        session.opponents[0].country_random = false;
        session.opponents[0].color_random = false;
        let shell = SkirmishShellState::default();

        let resolved = runtime
            .close_shell_transaction(&shell, &cooperative_maps(), &[cooperative_mode()], &session)
            .expect("Cooperative launch resolution");

        assert!(
            ["A1", "A1B", "A1C"]
                .contains(&resolved.selected_map_file.as_deref().unwrap_or_default())
        );
    }

    #[test]
    fn missing_cooperative_map_rolls_back_progress_and_rng() {
        let mut runtime = cooperative_runtime(0x4567);
        let rng_before = runtime.scenario_rng_state();
        let progress_before = runtime.cooperative_progress.clone();

        let error = runtime
            .ensure_cooperative_selection("A2B", &[map_named("A2B")])
            .expect_err("the stage-zero chosen variant is absent");

        assert!(matches!(error, CooperativeError::MissingScenarioMap { .. }));
        assert_eq!(runtime.scenario_rng_state(), rng_before);
        assert_eq!(runtime.cooperative_progress, progress_before);
    }

    #[test]
    fn failed_cooperative_swap_rolls_back_progress_and_rng() {
        let mut runtime = cooperative_runtime(0x5678);
        runtime
            .ensure_cooperative_selection("A2B", &cooperative_maps())
            .expect("bind first campaign");
        let rng_before = runtime.scenario_rng_state();
        let progress_before = runtime.cooperative_progress.clone();
        let allied_maps: Vec<_> = cooperative_maps()
            .into_iter()
            .filter(|map| map.file_name.starts_with('A'))
            .collect();

        let error = runtime
            .accept_cooperative_selection("W1B", &allied_maps)
            .expect_err("the selected World variant is absent");

        assert!(matches!(error, CooperativeError::MissingScenarioMap { .. }));
        assert_eq!(runtime.scenario_rng_state(), rng_before);
        assert_eq!(runtime.cooperative_progress, progress_before);
    }

    #[test]
    fn captured_match_rng_replaces_frontend_cursor_only_when_marked() {
        let snapshot = SkirmishPersistedSnapshot::from_global_defaults(defaults());
        let mut runtime = runtime(snapshot, 1);
        let replacement = SimRng::new(99);
        let expected = replacement.state();

        assert!(!runtime.capture_returned_gameplay_rng(replacement.clone()));
        assert_ne!(runtime.scenario_rng_state(), expected);
        runtime.mark_gameplay_rng_return_pending();
        assert!(runtime.capture_returned_gameplay_rng(replacement));
        assert_eq!(runtime.scenario_rng_state(), expected);
    }

    #[test]
    fn invalid_persisted_indices_clamp_to_first_mode_and_map() {
        let mut snapshot = SkirmishPersistedSnapshot::from_global_defaults(defaults());
        snapshot.game_mode = 999;
        snapshot.scenario_index = 999;
        let runtime = runtime(snapshot, 3);
        let mut shell = SkirmishShellState::default();
        shell.player_start_position = StartPosition::Auto;

        runtime.hydrate_shell(&mut shell, &[map()], &[mode()]);

        assert_eq!(shell.selected_mode_id, 1);
        assert_eq!(shell.selected_map_idx, 0);
    }
}
