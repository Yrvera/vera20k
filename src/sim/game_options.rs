//! Per-match game settings — the lobby options card.
//!
//! Parsed from `[MultiplayerDialogSettings]` in rulesmd.ini. Set once at game
//! start, read-only during gameplay. Included in the deterministic state
//! hash for lockstep correctness.

use crate::rules::ini_parser::IniFile;

/// Per-match game settings from the lobby / `[MultiplayerDialogSettings]`.
///
/// Set once at game start, read-only during gameplay.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GameOptions {
    // --- Runtime-checked by gameplay systems ---
    /// Defeat when all buildings lost (vs all objects lost). Rules+0x14B6.
    pub short_game: bool,
    /// Construction yards / base building enabled. Rules+0x14AF.
    pub bases: bool,
    /// Bridges can be destroyed. Rules+0x14AC.
    pub bridges_destroyable: bool,
    /// Superweapons can be built. Rules+0x14B9.
    pub super_weapons: bool,
    /// Build adjacent to allied buildings. Rules+0x14BA.
    pub build_off_ally: bool,
    /// Random crate spawning. Rules+0x14B1.
    pub crates: bool,
    /// MCV can repack into vehicle. Rules+0x14B8.
    pub mcv_redeploy: bool,
    /// TS-legacy semi-transparent fog. Default false in YR. Rules+0x14B7.
    pub fog_of_war: bool,
    /// Unexplored cells are black. Rules+0x14AE.
    pub shroud: bool,
    /// Ore/gems regenerate on the map. Rules+0x14B0.
    pub tiberium_grows: bool,
    /// Engineers capture at reduced HP only. Rules+0x14B4.
    pub multi_engineer: bool,
    /// Harvesters immune to enemy fire. Rules+0x14B3.
    pub harvester_truce: bool,
    /// Alliances can be changed mid-game. Rules+0x14BB (YR addition).
    pub ally_change_allowed: bool,

    // --- Used at init (Create_Houses, spawn units) ---
    /// Default starting credits per player. Rules+0x1484.
    pub starting_credits: i32,
    /// Number of starting units to spawn. Rules+0x1494.
    pub unit_count: i32,
    /// Maximum tech level for this match. Rules+0x149C.
    pub tech_level: i32,
    /// Game speed (0=fastest, 6=slowest). Rules+0x14A0.
    pub game_speed: i32,
    /// AI difficulty (0=Easy, 1=Normal, 2=Hard). Rules+0x14A4.
    pub ai_difficulty: i32,
    /// Number of AI opponents. Rules+0x14A8.
    pub ai_players: i32,
}

impl Default for GameOptions {
    /// Defaults from `[MultiplayerDialogSettings]` in rulesmd.ini (YR).
    fn default() -> Self {
        Self {
            short_game: true,
            bases: true,
            bridges_destroyable: true,
            super_weapons: true,
            build_off_ally: true,
            crates: true,
            mcv_redeploy: true,
            fog_of_war: false,
            shroud: true,
            tiberium_grows: true,
            multi_engineer: false,
            harvester_truce: false,
            ally_change_allowed: true,
            starting_credits: 10000,
            unit_count: 10,
            tech_level: 10,
            game_speed: 1,
            ai_difficulty: 0,
            ai_players: 0,
        }
    }
}

impl GameOptions {
    /// Override the per-match defaults from a merged rules INI's
    /// `[MultiplayerDialogSettings]` section.
    ///
    /// This section is read once into the rules data that both the skirmish
    /// setup dialog and the launched match draw from. Each key is optional: a
    /// missing key (or a missing section) keeps the corresponding
    /// [`GameOptions::default`] value, so the stock INI — whose values equal the
    /// defaults — parses to an unchanged result, and only a mod that edits a key
    /// shifts behaviour.
    ///
    /// `GameSpeed` is stored as parsed (0 = fastest); the setup trackbar inverts
    /// it only for display. The setup dialog exposes superweapons and
    /// build-off-ally as checkboxes even though the stock section omits both
    /// keys, so they fall back to the enabled defaults until a mod sets
    /// `SuperWeaponsAllowed` / `BuildOffAlly`.
    ///
    /// Three keys this section also carries are intentionally not mapped here:
    /// `ShadowGrow` and `CaptureTheFlag` drive systems this engine does not
    /// model, and the per-mode allies setting is sourced from the selected game
    /// mode rather than this global default. `AIDifficulty` / `AIPlayers` are
    /// parsed for a faithful round-trip, but a skirmish launch overrides both
    /// from the configured opponent slots.
    pub fn from_multiplayer_dialog_settings(ini: &IniFile) -> Self {
        let mut options = Self::default();
        let Some(section) = ini.section("MultiplayerDialogSettings") else {
            return options;
        };

        if let Some(value) = section.get_i32("Money") {
            options.starting_credits = value;
        }
        if let Some(value) = section.get_i32("UnitCount") {
            options.unit_count = value;
        }
        if let Some(value) = section.get_i32("TechLevel") {
            options.tech_level = value;
        }
        if let Some(value) = section.get_i32("GameSpeed") {
            options.game_speed = value;
        }
        if let Some(value) = section.get_i32("AIDifficulty") {
            options.ai_difficulty = value;
        }
        if let Some(value) = section.get_i32("AIPlayers") {
            options.ai_players = value;
        }

        if let Some(value) = section.get_bool("BridgeDestruction") {
            options.bridges_destroyable = value;
        }
        if let Some(value) = section.get_bool("Shroud") {
            options.shroud = value;
        }
        if let Some(value) = section.get_bool("Bases") {
            options.bases = value;
        }
        if let Some(value) = section.get_bool("TiberiumGrows") {
            options.tiberium_grows = value;
        }
        if let Some(value) = section.get_bool("Crates") {
            options.crates = value;
        }
        if let Some(value) = section.get_bool("HarvesterTruce") {
            options.harvester_truce = value;
        }
        if let Some(value) = section.get_bool("MultiEngineer") {
            options.multi_engineer = value;
        }
        if let Some(value) = section.get_bool("AllyChangeAllowed") {
            options.ally_change_allowed = value;
        }
        if let Some(value) = section.get_bool("ShortGame") {
            options.short_game = value;
        }
        if let Some(value) = section.get_bool("SuperWeaponsAllowed") {
            options.super_weapons = value;
        }
        if let Some(value) = section.get_bool("BuildOffAlly") {
            options.build_off_ally = value;
        }
        if let Some(value) = section.get_bool("FogOfWar") {
            options.fog_of_war = value;
        }
        if let Some(value) = section.get_bool("MCVRedeploys") {
            options.mcv_redeploy = value;
        }

        options
    }
}

#[cfg(test)]
mod tests {
    use super::GameOptions;
    use crate::rules::ini_parser::IniFile;

    #[test]
    fn build_off_ally_default_matches_yr_enabled() {
        assert!(GameOptions::default().build_off_ally);
    }

    #[test]
    fn stock_multiplayer_dialog_settings_match_hardcoded_defaults() {
        // The merged stock section equals every default, so the parse is a
        // no-op on the stock INI — the change is invisible in stock skirmishes
        // and only a mod that edits a key diverges.
        let ini = IniFile::from_str(
            "[MultiplayerDialogSettings]\n\
             Money=10000\nUnitCount=10\nTechLevel=10\nGameSpeed=1\n\
             AIDifficulty=0\nAIPlayers=0\n\
             BridgeDestruction=yes\nShroud=yes\nBases=yes\nTiberiumGrows=yes\n\
             Crates=yes\nHarvesterTruce=no\nMultiEngineer=no\nAllyChangeAllowed=yes\n\
             ShortGame=yes\nFogOfWar=no\nMCVRedeploys=yes\n",
        );
        let parsed = GameOptions::from_multiplayer_dialog_settings(&ini);
        let default = GameOptions::default();
        assert_eq!(parsed.starting_credits, default.starting_credits);
        assert_eq!(parsed.unit_count, default.unit_count);
        assert_eq!(parsed.tech_level, default.tech_level);
        assert_eq!(parsed.game_speed, default.game_speed);
        assert_eq!(parsed.bridges_destroyable, default.bridges_destroyable);
        assert_eq!(parsed.shroud, default.shroud);
        assert_eq!(parsed.bases, default.bases);
        assert_eq!(parsed.tiberium_grows, default.tiberium_grows);
        assert_eq!(parsed.crates, default.crates);
        assert_eq!(parsed.harvester_truce, default.harvester_truce);
        assert_eq!(parsed.multi_engineer, default.multi_engineer);
        assert_eq!(parsed.ally_change_allowed, default.ally_change_allowed);
        assert_eq!(parsed.short_game, default.short_game);
        assert_eq!(parsed.fog_of_war, default.fog_of_war);
        assert_eq!(parsed.mcv_redeploy, default.mcv_redeploy);
        // The stock section omits both these keys, so they keep the enabled
        // defaults rather than reading as false.
        assert_eq!(parsed.super_weapons, default.super_weapons);
        assert_eq!(parsed.build_off_ally, default.build_off_ally);
    }

    #[test]
    fn absent_section_keeps_all_defaults() {
        let ini = IniFile::from_str("[General]\nFoo=1\n");
        let parsed = GameOptions::from_multiplayer_dialog_settings(&ini);
        let default = GameOptions::default();
        assert_eq!(parsed.starting_credits, default.starting_credits);
        assert_eq!(parsed.tech_level, default.tech_level);
        assert!(parsed.bases);
        assert!(parsed.super_weapons);
    }

    #[test]
    fn modded_numeric_and_bool_keys_override_defaults() {
        let ini = IniFile::from_str(
            "[MultiplayerDialogSettings]\n\
             Money=7400\nUnitCount=4\nTechLevel=3\nGameSpeed=4\n\
             Bases=no\nCrates=no\nShortGame=no\nMCVRedeploys=no\n\
             TiberiumGrows=no\nFogOfWar=yes\n",
        );
        let parsed = GameOptions::from_multiplayer_dialog_settings(&ini);
        assert_eq!(parsed.starting_credits, 7400);
        assert_eq!(parsed.unit_count, 4);
        assert_eq!(parsed.tech_level, 3);
        // GameSpeed is stored as parsed; the display inversion is not baked in.
        assert_eq!(parsed.game_speed, 4);
        assert!(!parsed.bases);
        assert!(!parsed.crates);
        assert!(!parsed.short_game);
        assert!(!parsed.mcv_redeploy);
        assert!(!parsed.tiberium_grows);
        assert!(parsed.fog_of_war);
    }

    #[test]
    fn super_weapons_uses_allowed_suffix_key() {
        // The superweapons checkbox is backed by `SuperWeaponsAllowed`, not the
        // bare `SuperWeapons` key — the latter must be ignored.
        let wrong = IniFile::from_str("[MultiplayerDialogSettings]\nSuperWeapons=no\n");
        assert!(
            GameOptions::from_multiplayer_dialog_settings(&wrong).super_weapons,
            "the bare SuperWeapons key must not disable superweapons"
        );
        let right = IniFile::from_str("[MultiplayerDialogSettings]\nSuperWeaponsAllowed=no\n");
        assert!(!GameOptions::from_multiplayer_dialog_settings(&right).super_weapons);
    }

    #[test]
    fn build_off_ally_key_overrides_default() {
        let ini = IniFile::from_str("[MultiplayerDialogSettings]\nBuildOffAlly=no\n");
        assert!(!GameOptions::from_multiplayer_dialog_settings(&ini).build_off_ally);
    }
}
