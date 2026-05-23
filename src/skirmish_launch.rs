//! App-level skirmish launch contract.
//!
//! This module is intentionally data-only: it packages the lobby state the app
//! needs to create Battle-mode houses and initial spawns without making `sim/`
//! depend on UI, rendering, audio, or networking modules.

use crate::sim::game_options::GameOptions;

pub const SKIRMISH_PLAYER_SLOT_COUNT: usize = 8;
pub const SKIRMISH_AI_SLOT_COUNT: usize = SKIRMISH_PLAYER_SLOT_COUNT - 1;
pub const HOUSE_COLOR_COUNT: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkirmishLaunchMode {
    Battle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchCountry {
    America,
    Korea,
    France,
    Germany,
    GreatBritain,
    Libya,
    Iraq,
    Cuba,
    Russia,
    Yuri,
}

impl LaunchCountry {
    pub const fn country_name(self) -> &'static str {
        match self {
            Self::America => "Americans",
            Self::Korea => "Alliance",
            Self::France => "French",
            Self::Germany => "Germans",
            Self::GreatBritain => "British",
            Self::Libya => "Africans",
            Self::Iraq => "Arabs",
            Self::Cuba => "Confederation",
            Self::Russia => "Russians",
            Self::Yuri => "YuriCountry",
        }
    }

    pub const fn side_index(self) -> u8 {
        match self {
            Self::America | Self::Korea | Self::France | Self::Germany | Self::GreatBritain => 0,
            Self::Libya | Self::Iraq | Self::Cuba | Self::Russia => 1,
            Self::Yuri => 2,
        }
    }

    pub const fn opening_mcv_candidates(self) -> &'static [&'static str] {
        match self.side_index() {
            2 => &["PCV", "SMCV", "AMCV"],
            1 => &["SMCV", "AMCV", "PCV"],
            _ => &["AMCV", "SMCV", "PCV"],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchStartPosition {
    Auto,
    Position(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchTeam {
    None,
    Team(u8),
}

impl LaunchTeam {
    pub const fn from_shell_value(value: i32) -> Self {
        if value < 0 {
            Self::None
        } else {
            Self::Team(value as u8)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiDifficulty {
    Easy,
    Normal,
    Hard,
}

impl AiDifficulty {
    pub const fn as_i32(self) -> i32 {
        match self {
            Self::Easy => 0,
            Self::Normal => 1,
            Self::Hard => 2,
        }
    }
}

impl Default for AiDifficulty {
    fn default() -> Self {
        Self::Easy
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkirmishLaunchOptions {
    pub starting_credits: i32,
    pub unit_count: i32,
    pub tech_level: i32,
    pub game_speed: i32,
    pub short_game: bool,
    pub bases: bool,
    pub bridges_destroyable: bool,
    pub super_weapons: bool,
    pub build_off_ally: bool,
    pub crates: bool,
    pub mcv_redeploy: bool,
    pub fog_of_war: bool,
    pub shroud: bool,
    pub tiberium_grows: bool,
    pub multi_engineer: bool,
    pub harvester_truce: bool,
    pub ally_change_allowed: bool,
}

impl Default for SkirmishLaunchOptions {
    fn default() -> Self {
        let defaults = GameOptions::default();
        Self {
            starting_credits: defaults.starting_credits,
            unit_count: defaults.unit_count,
            tech_level: defaults.tech_level,
            game_speed: defaults.game_speed,
            short_game: defaults.short_game,
            bases: defaults.bases,
            bridges_destroyable: defaults.bridges_destroyable,
            super_weapons: defaults.super_weapons,
            build_off_ally: defaults.build_off_ally,
            crates: defaults.crates,
            mcv_redeploy: defaults.mcv_redeploy,
            fog_of_war: defaults.fog_of_war,
            shroud: defaults.shroud,
            tiberium_grows: defaults.tiberium_grows,
            multi_engineer: defaults.multi_engineer,
            harvester_truce: defaults.harvester_truce,
            ally_change_allowed: defaults.ally_change_allowed,
        }
    }
}

impl SkirmishLaunchOptions {
    pub fn to_game_options(&self, ai_players: i32, ai_difficulty: AiDifficulty) -> GameOptions {
        GameOptions {
            short_game: self.short_game,
            bases: self.bases,
            bridges_destroyable: self.bridges_destroyable,
            super_weapons: self.super_weapons,
            build_off_ally: self.build_off_ally,
            crates: self.crates,
            mcv_redeploy: self.mcv_redeploy,
            fog_of_war: self.fog_of_war,
            shroud: self.shroud,
            tiberium_grows: self.tiberium_grows,
            multi_engineer: self.multi_engineer,
            harvester_truce: self.harvester_truce,
            ally_change_allowed: self.ally_change_allowed,
            starting_credits: self.starting_credits,
            unit_count: self.unit_count,
            tech_level: self.tech_level,
            game_speed: self.game_speed,
            ai_difficulty: ai_difficulty.as_i32(),
            ai_players,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkirmishLocalSlot {
    pub country: LaunchCountry,
    pub color_index: u8,
    pub start_position: LaunchStartPosition,
    pub team: LaunchTeam,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkirmishAiSlot {
    pub country: LaunchCountry,
    pub color_index: u8,
    pub start_position: LaunchStartPosition,
    pub team: LaunchTeam,
    pub difficulty: AiDifficulty,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkirmishLaunchSession {
    pub mode: SkirmishLaunchMode,
    pub selected_map_file: Option<String>,
    pub local: SkirmishLocalSlot,
    pub opponents: Vec<SkirmishAiSlot>,
    pub options: SkirmishLaunchOptions,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchValidationError {
    NoSelectedMap,
    NoEnabledOpponent,
    MapCapacityExceeded {
        capacity: usize,
        requested_players: usize,
    },
    SameExplicitTeam {
        team: u8,
    },
    InvalidColorIndex {
        slot: usize,
        color_index: usize,
    },
    InvalidStartPosition {
        slot: usize,
        position: u8,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launch_options_preserve_build_off_ally_default() {
        let options = SkirmishLaunchOptions::default();
        let game_options = options.to_game_options(3, AiDifficulty::Hard);

        assert!(game_options.build_off_ally);
        assert_eq!(game_options.ai_players, 3);
        assert_eq!(game_options.ai_difficulty, 2);
    }

    #[test]
    fn yuri_country_uses_third_side_and_pcv_first() {
        assert_eq!(LaunchCountry::Yuri.side_index(), 2);
        assert_eq!(LaunchCountry::Yuri.opening_mcv_candidates()[0], "PCV");
    }

    #[test]
    fn shell_team_values_use_negative_none_and_zero_based_teams() {
        assert_eq!(LaunchTeam::from_shell_value(-2), LaunchTeam::None);
        assert_eq!(LaunchTeam::from_shell_value(0), LaunchTeam::Team(0));
        assert_eq!(LaunchTeam::from_shell_value(3), LaunchTeam::Team(3));
    }
}
