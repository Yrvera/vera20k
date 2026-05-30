//! Launch settings and session conversion for the skirmish shell.

use crate::app_init::MapMenuEntry;
use crate::skirmish_launch::{
    HOUSE_COLOR_COUNT, LaunchCountry, LaunchStartPosition, LaunchTeam, LaunchValidationError,
    SKIRMISH_PLAYER_SLOT_COUNT, SkirmishAiSlot, SkirmishLaunchMode, SkirmishLaunchOptions,
    SkirmishLaunchSession, SkirmishLocalSlot,
};
use crate::skirmish_modes::{SkirmishGameMode, mode_by_id};
use crate::ui::main_menu::{SkirmishCountry, SkirmishSettings, StartPosition};

use super::SkirmishShellState;

/// Map the menu country selection onto a launch country. When the slot is set
/// to Random this still returns the currently-shown menu country as a
/// placeholder; the caller flags the slot via `country_random` so the concrete
/// country is drawn later (see `SkirmishLaunchSession::resolve_random_assignments`).
fn launch_country_from_menu(country: SkirmishCountry) -> LaunchCountry {
    match country {
        SkirmishCountry::America => LaunchCountry::America,
        SkirmishCountry::Korea => LaunchCountry::Korea,
        SkirmishCountry::France => LaunchCountry::France,
        SkirmishCountry::Germany => LaunchCountry::Germany,
        SkirmishCountry::GreatBritain => LaunchCountry::GreatBritain,
        SkirmishCountry::Libya => LaunchCountry::Libya,
        SkirmishCountry::Iraq => LaunchCountry::Iraq,
        SkirmishCountry::Cuba => LaunchCountry::Cuba,
        SkirmishCountry::Russia => LaunchCountry::Russia,
        SkirmishCountry::Yuri => LaunchCountry::Yuri,
    }
}

fn launch_start_position(
    slot: usize,
    start_position: StartPosition,
) -> Result<LaunchStartPosition, LaunchValidationError> {
    match start_position {
        StartPosition::Auto => Ok(LaunchStartPosition::Auto),
        StartPosition::Position(position) if position < SKIRMISH_PLAYER_SLOT_COUNT as u8 => {
            Ok(LaunchStartPosition::Position(position))
        }
        StartPosition::Position(position) => {
            Err(LaunchValidationError::InvalidStartPosition { slot, position })
        }
    }
}

fn launch_color_index(slot: usize, color_index: usize) -> Result<u8, LaunchValidationError> {
    if color_index < HOUSE_COLOR_COUNT {
        Ok(color_index as u8)
    } else {
        Err(LaunchValidationError::InvalidColorIndex { slot, color_index })
    }
}

pub fn launch_settings(state: &SkirmishShellState) -> SkirmishSettings {
    let ai_country = state
        .opponents
        .iter()
        .find(|opponent| opponent.is_active())
        .map(|opponent| opponent.country)
        .unwrap_or(SkirmishCountry::Russia);

    SkirmishSettings {
        selected_map_idx: state.selected_map_idx,
        player_country: state.player_country,
        ai_country,
        starting_credits: state.starting_credits,
        start_position: state.player_start_position,
        short_game: state.short_game,
        zoom_enabled: state.zoom_enabled,
    }
}

pub fn launch_session(
    state: &SkirmishShellState,
    maps: &[MapMenuEntry],
    modes: &[SkirmishGameMode],
) -> Result<SkirmishLaunchSession, LaunchValidationError> {
    let selected_map = maps
        .get(state.selected_map_idx)
        .ok_or(LaunchValidationError::NoSelectedMap)?;

    let active_count = state
        .opponents
        .iter()
        .filter(|opponent| opponent.is_active())
        .count();
    let requested_players = active_count + 1;
    let capacity = selected_map.multiplayer_start_waypoints.len();
    if capacity < requested_players {
        return Err(LaunchValidationError::MapCapacityExceeded {
            capacity,
            requested_players,
        });
    }
    if active_count == 0 {
        return Err(LaunchValidationError::NoEnabledOpponent);
    }
    if state.player_team >= 0 {
        let local_team = state.player_team as u8;
        let all_active_ai_same_team = state
            .opponents
            .iter()
            .filter(|opponent| opponent.is_active())
            .all(|opponent| {
                LaunchTeam::from_shell_value(opponent.team) == LaunchTeam::Team(local_team)
            });
        if all_active_ai_same_team {
            return Err(LaunchValidationError::SameExplicitTeam { team: local_team });
        }
    }

    let selected_mode =
        mode_by_id(modes, state.selected_mode_id).ok_or(LaunchValidationError::NoSelectedMode {
            mode_id: state.selected_mode_id,
        })?;

    let local = SkirmishLocalSlot {
        country: launch_country_from_menu(state.player_country),
        country_random: state.player_country_random,
        color_index: launch_color_index(0, state.player_color_index)?,
        start_position: launch_start_position(0, state.player_start_position)?,
        team: LaunchTeam::from_shell_value(state.player_team),
    };

    let mut opponents = Vec::new();
    for (idx, opponent) in state.opponents.iter().enumerate() {
        let Some(difficulty) = opponent.row_type.difficulty() else {
            continue;
        };
        let slot = idx + 1;
        opponents.push(SkirmishAiSlot {
            country: launch_country_from_menu(opponent.country),
            country_random: opponent.country_random,
            color_index: launch_color_index(slot, opponent.color_index)?,
            start_position: launch_start_position(slot, opponent.start_position)?,
            team: LaunchTeam::from_shell_value(opponent.team),
            difficulty,
        });
    }

    let mut options = SkirmishLaunchOptions::default();
    options.starting_credits = state.starting_credits;
    options.unit_count = state.unit_count;
    options.game_speed = state.game_speed;
    options.short_game = state.short_game;
    options.super_weapons = state.super_weapons;
    options.build_off_ally = state.build_off_ally;
    options.crates = state.crates;
    options.mcv_redeploy = state.mcv_redeploy;

    Ok(SkirmishLaunchSession {
        mode: SkirmishLaunchMode::from_game_mode(selected_mode),
        selected_map_file: Some(selected_map.file_name.clone()),
        player_name: state.player_name_edit.text.clone(),
        local,
        opponents,
        options,
    })
}
