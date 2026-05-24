//! Parser for the `[Basic]` map section.
//!
//! `[Basic]` carries scenario/map metadata used by the original games and tools.
//! This module keeps the first-pass support intentionally narrow: parse the most
//! useful metadata now and leave the broader scenario semantics for later work.

use crate::rules::ini_parser::IniFile;

/// Owner of the active `SpecialFlags::DestroyableBridges` bit during load.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeDestroyabilityMode {
    /// Campaign and map-editor loads let map `[SpecialFlags]` override reset defaults.
    CampaignOrEditor,
    /// Skirmish/multiplayer replaces the active flag with session staging state.
    SkirmishOrMultiplayer { bridge_destruction: bool },
}

/// Parsed metadata from a map's `[Basic]` section.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BasicSection {
    /// Human-facing map/scenario name when present.
    pub name: Option<String>,
    /// Author text when present.
    pub author: Option<String>,
    /// String-table key or raw intro/briefing hook.
    pub intro: Option<String>,
    /// String-table key or raw briefing hook.
    pub briefing: Option<String>,
    /// Theme/music id requested by the map.
    pub theme: Option<String>,
    /// Declared INI format version used by the map.
    pub new_ini_format: Option<i32>,
    /// Whether tiberium/ore growth is enabled for this map (TiberiumGrowthEnabled=).
    pub tiberium_growth_enabled: Option<bool>,
}

/// Parsed flags from a map's `[SpecialFlags]` section.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpecialFlagsSection {
    /// Parsed `MCVDeploy=` bit (0x0100); startup deployment logic is mode-specific elsewhere.
    pub mcv_deploy: Option<bool>,
    /// Map-level override: does ore grow denser? (TiberiumGrows=)
    pub tiberium_grows: Option<bool>,
    /// Map-level override: does ore spread to adjacent cells? (TiberiumSpreads=)
    pub tiberium_spreads: Option<bool>,
    /// Map-level override: are bridges destroyable? (DestroyableBridges=)
    pub destroyable_bridges: Option<bool>,
}

impl SpecialFlagsSection {
    /// Resolve the active `DestroyableBridges` bit using gamemd's mode ownership.
    pub fn effective_destroyable_bridges(&self, mode: BridgeDestroyabilityMode) -> bool {
        match mode {
            BridgeDestroyabilityMode::CampaignOrEditor => self.destroyable_bridges.unwrap_or(true),
            BridgeDestroyabilityMode::SkirmishOrMultiplayer { bridge_destruction } => {
                bridge_destruction
            }
        }
    }
}

/// Parse the `[Basic]` section from a map INI.
pub fn parse_basic_section(ini: &IniFile) -> BasicSection {
    let Some(section) = ini.section("Basic") else {
        return BasicSection::default();
    };

    BasicSection {
        name: section.get("Name").map(str::to_string),
        author: section.get("Author").map(str::to_string),
        intro: section.get("Intro").map(str::to_string),
        briefing: section.get("Brief").map(str::to_string),
        theme: section.get("Theme").map(str::to_string),
        new_ini_format: section.get_i32("NewINIFormat"),
        tiberium_growth_enabled: section.get_bool("TiberiumGrowthEnabled"),
    }
}

/// Parse the `[SpecialFlags]` section from a map INI.
pub fn parse_special_flags_section(ini: &IniFile) -> SpecialFlagsSection {
    let Some(section) = ini.section("SpecialFlags") else {
        return SpecialFlagsSection::default();
    };

    SpecialFlagsSection {
        mcv_deploy: section.get_bool("MCVDeploy"),
        tiberium_grows: section.get_bool("TiberiumGrows"),
        tiberium_spreads: section.get_bool("TiberiumSpreads"),
        destroyable_bridges: section.get_bool("DestroyableBridges"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::ini_parser::IniFile;

    #[test]
    fn parse_basic_metadata() {
        let ini = IniFile::from_str(
            "[Basic]\nName=Mission One\nAuthor=Westwood\nIntro=TXT_M01INTRO\n\
             Brief=TXT_M01BRIEF\nTheme=BIGF226M\nNewINIFormat=4\n",
        );

        let basic = parse_basic_section(&ini);
        assert_eq!(basic.name.as_deref(), Some("Mission One"));
        assert_eq!(basic.author.as_deref(), Some("Westwood"));
        assert_eq!(basic.intro.as_deref(), Some("TXT_M01INTRO"));
        assert_eq!(basic.briefing.as_deref(), Some("TXT_M01BRIEF"));
        assert_eq!(basic.theme.as_deref(), Some("BIGF226M"));
        assert_eq!(basic.new_ini_format, Some(4));
    }

    #[test]
    fn missing_basic_section_returns_defaults() {
        let ini = IniFile::from_str("[Map]\nTheater=TEMPERATE\n");
        let basic = parse_basic_section(&ini);
        assert_eq!(basic, BasicSection::default());
    }

    #[test]
    fn parse_special_flags_bridge_override() {
        let ini = IniFile::from_str(
            "[SpecialFlags]\nMCVDeploy=yes\nTiberiumGrows=yes\nTiberiumSpreads=no\nDestroyableBridges=no\n",
        );
        let flags = parse_special_flags_section(&ini);
        assert_eq!(flags.mcv_deploy, Some(true));
        assert_eq!(flags.tiberium_grows, Some(true));
        assert_eq!(flags.tiberium_spreads, Some(false));
        assert_eq!(flags.destroyable_bridges, Some(false));
    }

    #[test]
    fn parse_special_flags_mcvdeploy_no() {
        let ini = IniFile::from_str("[SpecialFlags]\nMCVDeploy=no\n");
        let flags = parse_special_flags_section(&ini);
        assert_eq!(flags.mcv_deploy, Some(false));
        assert_eq!(flags.tiberium_grows, None);
        assert_eq!(flags.tiberium_spreads, None);
        assert_eq!(flags.destroyable_bridges, None);
    }

    #[test]
    fn missing_special_flags_mcvdeploy_defaults_to_none() {
        let ini = IniFile::from_str("[SpecialFlags]\nDestroyableBridges=yes\n");
        let flags = parse_special_flags_section(&ini);
        assert_eq!(flags.mcv_deploy, None);
        assert_eq!(flags.destroyable_bridges, Some(true));

        let ini = IniFile::from_str("[Basic]\nName=No Special Flags\n");
        assert_eq!(parse_special_flags_section(&ini).mcv_deploy, None);
    }

    #[test]
    fn specialflags_destroyablebridges_map_override_campaign_only() {
        let flags = SpecialFlagsSection {
            destroyable_bridges: Some(false),
            ..SpecialFlagsSection::default()
        };

        assert!(!flags.effective_destroyable_bridges(BridgeDestroyabilityMode::CampaignOrEditor));
        assert!(flags.effective_destroyable_bridges(
            BridgeDestroyabilityMode::SkirmishOrMultiplayer {
                bridge_destruction: true,
            }
        ));
    }

    #[test]
    fn skirmish_bridge_destruction_option_controls_specialflags_bit_8000() {
        let flags = SpecialFlagsSection {
            destroyable_bridges: Some(true),
            ..SpecialFlagsSection::default()
        };

        assert!(!flags.effective_destroyable_bridges(
            BridgeDestroyabilityMode::SkirmishOrMultiplayer {
                bridge_destruction: false,
            }
        ));
        assert!(
            SpecialFlagsSection::default().effective_destroyable_bridges(
                BridgeDestroyabilityMode::SkirmishOrMultiplayer {
                    bridge_destruction: true,
                }
            )
        );
    }
}
