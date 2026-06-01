//! App-level skirmish launch contract.
//!
//! This module is intentionally data-only: it packages the lobby state the app
//! needs to create Battle-mode houses and initial spawns without making `sim/`
//! depend on UI, rendering, audio, or networking modules.

use crate::sim::game_options::GameOptions;
use crate::sim::rng::SimRng;
use crate::skirmish_modes::SkirmishGameMode;

pub const SKIRMISH_PLAYER_SLOT_COUNT: usize = 8;
pub const SKIRMISH_AI_SLOT_COUNT: usize = SKIRMISH_PLAYER_SLOT_COUNT - 1;
pub const HOUSE_COLOR_COUNT: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkirmishLaunchMode {
    pub id: i32,
    pub ui_name_key: String,
    pub tooltip_key: String,
    pub override_file: String,
    pub map_filter: String,
    pub random_maps_allowed: bool,
    pub allies_allowed: bool,
    pub must_ally: bool,
}

impl SkirmishLaunchMode {
    pub fn from_game_mode(mode: &SkirmishGameMode) -> Self {
        Self {
            id: mode.id,
            ui_name_key: mode.ui_name_key.clone(),
            tooltip_key: mode.tooltip_key.clone(),
            override_file: mode.override_file.clone(),
            map_filter: mode.map_filter.clone(),
            random_maps_allowed: mode.random_maps_allowed,
            allies_allowed: mode.allies_allowed,
            must_ally: mode.must_ally,
        }
    }
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

    /// Map a ranged country index (0..=9) onto a concrete country, in the
    /// country-list order used by the setup UI. Values above the last index
    /// clamp to the final country so the mapping is total.
    pub const fn from_country_index(index: u32) -> Self {
        match index {
            0 => Self::America,
            1 => Self::Korea,
            2 => Self::France,
            3 => Self::Germany,
            4 => Self::GreatBritain,
            5 => Self::Libya,
            6 => Self::Iraq,
            7 => Self::Cuba,
            8 => Self::Russia,
            _ => Self::Yuri,
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
    /// Build the launch base from per-match options parsed once from
    /// `[MultiplayerDialogSettings]`. The setup dialog later overrides the
    /// values it exposes as widgets; the remaining fields — tech level and the
    /// non-widget toggles (bases, shroud, tiberium growth, …) — flow straight
    /// through to the match from this base. AI difficulty and player count are
    /// supplied separately at launch from the configured opponent slots, so
    /// they are not carried on this struct.
    pub fn from_game_options(options: &GameOptions) -> Self {
        Self {
            starting_credits: options.starting_credits,
            unit_count: options.unit_count,
            tech_level: options.tech_level,
            game_speed: options.game_speed,
            short_game: options.short_game,
            bases: options.bases,
            bridges_destroyable: options.bridges_destroyable,
            super_weapons: options.super_weapons,
            build_off_ally: options.build_off_ally,
            crates: options.crates,
            mcv_redeploy: options.mcv_redeploy,
            fog_of_war: options.fog_of_war,
            shroud: options.shroud,
            tiberium_grows: options.tiberium_grows,
            multi_engineer: options.multi_engineer,
            harvester_truce: options.harvester_truce,
            ally_change_allowed: options.ally_change_allowed,
        }
    }

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
    /// When true, `country` is a placeholder to be replaced by a random draw
    /// during session resolution; see [`SkirmishLaunchSession::resolve_random_assignments`].
    pub country_random: bool,
    pub color_index: u8,
    pub start_position: LaunchStartPosition,
    pub team: LaunchTeam,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkirmishAiSlot {
    pub country: LaunchCountry,
    /// When true, `country` is a placeholder to be replaced by a random draw
    /// during session resolution; see [`SkirmishLaunchSession::resolve_random_assignments`].
    pub country_random: bool,
    pub color_index: u8,
    pub start_position: LaunchStartPosition,
    pub team: LaunchTeam,
    pub difficulty: AiDifficulty,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkirmishLaunchSession {
    pub mode: SkirmishLaunchMode,
    pub selected_map_file: Option<String>,
    pub player_name: String,
    pub local: SkirmishLocalSlot,
    pub opponents: Vec<SkirmishAiSlot>,
    pub options: SkirmishLaunchOptions,
}

impl SkirmishLaunchSession {
    /// Resolve every slot left on "random country" into a concrete country by
    /// drawing from the supplied scenario RNG. The selection order matches the
    /// original setup handoff: the local (human) slot is resolved first, then
    /// each AI slot in order, with one inclusive `(0, 9)` ranged draw per
    /// random slot. Slots that already hold a concrete country are left
    /// untouched and consume no draw, so the RNG stream only advances for the
    /// slots that actually requested a random country.
    ///
    /// Drawing from the scenario stream (rather than a side RNG) keeps the
    /// resolution deterministic for a given game seed and identical across
    /// lockstep peers. Color is always concrete in the current setup UI, so no
    /// color draw is made here.
    pub fn resolve_random_assignments(&self, rng: &mut SimRng) -> Self {
        let mut resolved = self.clone();
        if resolved.local.country_random {
            let index = rng.next_range_u32_inclusive(0, 9);
            resolved.local.country = LaunchCountry::from_country_index(index);
            resolved.local.country_random = false;
        }
        for opponent in &mut resolved.opponents {
            if opponent.country_random {
                let index = rng.next_range_u32_inclusive(0, 9);
                opponent.country = LaunchCountry::from_country_index(index);
                opponent.country_random = false;
            }
        }
        resolved
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchValidationError {
    NoSelectedMap,
    NoSelectedMode {
        mode_id: i32,
    },
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
    fn yuri_country_uses_third_side() {
        assert_eq!(LaunchCountry::Yuri.side_index(), 2);
    }

    #[test]
    fn from_game_options_carries_non_widget_fields_to_the_match() {
        // The launch base is the path by which parsed-INI fields the setup
        // dialog does not expose (tech level, bases, shroud, …) reach the
        // launched match. Build a base from modded options and confirm those
        // fields survive the round-trip through to_game_options.
        let mut parsed = GameOptions::default();
        parsed.tech_level = 3;
        parsed.bases = false;
        parsed.shroud = false;
        parsed.tiberium_grows = false;
        parsed.harvester_truce = true;
        parsed.multi_engineer = true;
        parsed.bridges_destroyable = false;
        parsed.ally_change_allowed = false;
        parsed.fog_of_war = true;

        let base = SkirmishLaunchOptions::from_game_options(&parsed);
        let launched = base.to_game_options(2, AiDifficulty::Normal);

        assert_eq!(launched.tech_level, 3);
        assert!(!launched.bases);
        assert!(!launched.shroud);
        assert!(!launched.tiberium_grows);
        assert!(launched.harvester_truce);
        assert!(launched.multi_engineer);
        assert!(!launched.bridges_destroyable);
        assert!(!launched.ally_change_allowed);
        assert!(launched.fog_of_war);
        // AI difficulty / player count come from the opponent slots, not the
        // parsed defaults.
        assert_eq!(launched.ai_players, 2);
        assert_eq!(launched.ai_difficulty, 1);
    }

    #[test]
    fn launch_mode_carries_selected_mpmode_data() {
        let mode = SkirmishGameMode {
            id: 9,
            ui_name_key: "GUI:TeamGame".to_string(),
            tooltip_key: "STT:ModeTeamGame".to_string(),
            override_file: "MPTeamMD.ini".to_string(),
            map_filter: "teamgame".to_string(),
            random_maps_allowed: false,
            allies_allowed: true,
            must_ally: true,
        };

        let launch_mode = SkirmishLaunchMode::from_game_mode(&mode);

        assert_eq!(launch_mode.id, 9);
        assert_eq!(launch_mode.ui_name_key, "GUI:TeamGame");
        assert_eq!(launch_mode.override_file, "MPTeamMD.ini");
        assert!(launch_mode.allies_allowed);
        assert!(launch_mode.must_ally);
    }

    #[test]
    fn shell_team_values_use_negative_none_and_zero_based_teams() {
        assert_eq!(LaunchTeam::from_shell_value(-2), LaunchTeam::None);
        assert_eq!(LaunchTeam::from_shell_value(-1), LaunchTeam::None);
        assert_eq!(LaunchTeam::from_shell_value(0), LaunchTeam::Team(0));
        assert_eq!(LaunchTeam::from_shell_value(3), LaunchTeam::Team(3));
    }

    fn random_test_session() -> SkirmishLaunchSession {
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
            selected_map_file: Some("test.mmx".to_string()),
            player_name: "Player".to_string(),
            local: SkirmishLocalSlot {
                country: LaunchCountry::America,
                country_random: true,
                color_index: 0,
                start_position: LaunchStartPosition::Auto,
                team: LaunchTeam::None,
            },
            opponents: vec![
                SkirmishAiSlot {
                    country: LaunchCountry::Russia,
                    country_random: true,
                    color_index: 1,
                    start_position: LaunchStartPosition::Auto,
                    team: LaunchTeam::None,
                    difficulty: AiDifficulty::Easy,
                },
                SkirmishAiSlot {
                    country: LaunchCountry::Cuba,
                    country_random: false,
                    color_index: 2,
                    start_position: LaunchStartPosition::Auto,
                    team: LaunchTeam::None,
                    difficulty: AiDifficulty::Easy,
                },
            ],
            options: SkirmishLaunchOptions::default(),
        }
    }

    #[test]
    fn resolve_random_assignments_is_deterministic_for_a_seed() {
        let session = random_test_session();
        let mut rng_a = SimRng::new(0xC0FFEE);
        let mut rng_b = SimRng::new(0xC0FFEE);

        let first = session.resolve_random_assignments(&mut rng_a);
        let second = session.resolve_random_assignments(&mut rng_b);

        assert_eq!(first, second, "same seed must yield the same assignment");
        assert!(!first.local.country_random);
        assert!(!first.opponents[0].country_random);
    }

    #[test]
    fn resolve_random_assignments_only_draws_for_random_slots_in_order() {
        let session = random_test_session();

        // Two random slots (local + opponent 0); opponent 1 is concrete and must
        // not consume a draw or change. The draw order is local first, then AI,
        // so resolving by hand in that order must reproduce the same result.
        let mut rng = SimRng::new(7);
        let resolved = session.resolve_random_assignments(&mut rng);

        let mut expected_rng = SimRng::new(7);
        let expected_local =
            LaunchCountry::from_country_index(expected_rng.next_range_u32_inclusive(0, 9));
        let expected_ai0 =
            LaunchCountry::from_country_index(expected_rng.next_range_u32_inclusive(0, 9));

        assert_eq!(resolved.local.country, expected_local);
        assert_eq!(resolved.opponents[0].country, expected_ai0);
        // The concrete slot is untouched and the stream is now exhausted of the
        // two expected draws — both RNGs must be in the same state.
        assert_eq!(resolved.opponents[1].country, LaunchCountry::Cuba);
        assert!(!resolved.opponents[1].country_random);
        assert_eq!(rng.state(), expected_rng.state());
    }

    #[test]
    fn resolve_random_assignments_leaves_concrete_session_untouched() {
        let mut session = random_test_session();
        session.local.country_random = false;
        session.opponents[0].country_random = false;

        let before = SimRng::new(42).state();
        let mut rng = SimRng::new(42);
        let resolved = session.resolve_random_assignments(&mut rng);

        assert_eq!(resolved, session, "no random slots means no change");
        assert_eq!(rng.state(), before, "no random slots means no draws");
    }
}
