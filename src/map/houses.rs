//! Map house parsing â€” extracts active house definitions and color assignments.
//!
//! RA2 map files have a `[Houses]` section that lists active factions/owners.
//! Each house also has its own section with keys such as `Color=`, `Country=`,
//! `Side=`, `Allies=`, and sometimes `PlayerControl=`.
//!
//! This module parses those sections into a `HouseRoster` plus the derived
//! `HouseColorMap`. The roster keeps the original map order and the most useful
//! ownership metadata for later simulation/UI work.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use crate::rules::color_scheme::{ColorSchemeEntry, scheme_entry_by_name};
use crate::rules::house_colors::{DEFAULT_SCHEME_ENTRY, HouseColorIndex};
use crate::rules::ini_parser::IniFile;

/// Mapping from owner name (e.g., "Americans") to house color index.
///
/// Used at atlas build time to determine which palette ramp to apply,
/// and at render time for minimap dot colors.
pub type HouseColorMap = HashMap<String, HouseColorIndex>;
/// Normalized alliance graph keyed by uppercase house name.
pub type HouseAllianceMap = BTreeMap<String, BTreeSet<String>>;

/// Parsed metadata for one active house listed in `[Houses]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HouseDefinition {
    /// House section name as referenced by entities and triggers.
    pub name: String,
    /// House color selection used for remap rendering.
    pub color: HouseColorIndex,
    /// Optional country/country-like identity from `Country=`.
    pub country: Option<String>,
    /// Optional side/faction grouping from `Side=`.
    pub side: Option<String>,
    /// Optional player-control hint from `PlayerControl=`.
    pub player_control: Option<bool>,
    /// Allies listed in the house section.
    pub allies: Vec<String>,
}

/// Ordered active-house list from the map's `[Houses]` section.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HouseRoster {
    /// Houses in the same order they appear in `[Houses]`.
    pub houses: Vec<HouseDefinition>,
}

impl HouseRoster {
    /// Convert roster entries to the color map used by render code.
    pub fn color_map(&self) -> HouseColorMap {
        self.houses
            .iter()
            .map(|house| (house.name.clone(), house.color))
            .collect()
    }

    /// Collect uppercase names of all human-controlled houses (PlayerControl=yes).
    /// Used during init to set `HouseState.is_human` for the sim-layer equivalent
    /// of the original engine's IsHumanPlayer.
    pub fn human_house_names(&self) -> HashSet<String> {
        self.houses
            .iter()
            .filter(|h| h.player_control == Some(true))
            .map(|h| h.name.to_ascii_uppercase())
            .collect()
    }

    /// Convert roster entries to a symmetric alliance graph.
    pub fn alliance_map(&self) -> HouseAllianceMap {
        let mut map: HouseAllianceMap = BTreeMap::new();
        for house in &self.houses {
            map.entry(normalize_house_name(&house.name)).or_default();
        }
        for house in &self.houses {
            let source = normalize_house_name(&house.name);
            for ally in &house.allies {
                let target = normalize_house_name(ally);
                map.entry(source.clone())
                    .or_default()
                    .insert(target.clone());
                map.entry(target).or_default().insert(source.clone());
            }
        }
        map
    }
}

/// Returns true when two house names should be treated as friendly.
pub fn are_houses_friendly(alliances: &HouseAllianceMap, a: &str, b: &str) -> bool {
    if a.eq_ignore_ascii_case(b) {
        return true;
    }
    let a_norm = normalize_house_name(a);
    let b_norm = normalize_house_name(b);
    alliances
        .get(&a_norm)
        .is_some_and(|set| set.contains(&b_norm))
        || alliances
            .get(&b_norm)
            .is_some_and(|set| set.contains(&a_norm))
}

fn normalize_house_name(name: &str) -> String {
    name.trim().to_ascii_uppercase()
}

/// Parse house color assignments from a map's INI data.
///
/// This remains as a compatibility helper for systems that only need color.
/// `schemes` is the parsed `[Colors]` list used to resolve each house's
/// `Color=<name>` to a `[Colors]` entry index.
pub fn parse_house_colors(ini: &IniFile, schemes: &[ColorSchemeEntry]) -> HouseColorMap {
    parse_house_roster(ini, schemes).color_map()
}

/// Parse the ordered active-house roster from a map's INI data.
///
/// `schemes` is the parsed `[Colors]` list; a house's `Color=<name>` resolves to
/// that entry's index (case-insensitive). Houses with no/unknown color fall back
/// to [`DEFAULT_SCHEME_ENTRY`].
pub fn parse_house_roster(ini: &IniFile, schemes: &[ColorSchemeEntry]) -> HouseRoster {
    let houses_section = match ini.section("Houses") {
        Some(s) => s,
        None => {
            log::info!("No [Houses] section in map â€” all entities use default Gold color");
            return HouseRoster::default();
        }
    };

    let mut houses = Vec::new();

    // [Houses] has numbered keys: 0=Americans, 1=Russians, etc.
    for key in houses_section.keys() {
        let Some(house_name) = houses_section.get(key) else {
            continue;
        };
        let house_name = house_name.trim().to_string();
        if house_name.is_empty() {
            continue;
        }

        let section = ini.section(&house_name);
        let color = section
            .and_then(|s| s.get("Color"))
            .and_then(|name| scheme_entry_by_name(schemes, name))
            .map(|entry| HouseColorIndex(entry as u8))
            .unwrap_or(HouseColorIndex(DEFAULT_SCHEME_ENTRY as u8));
        let country = section.and_then(|s| s.get("Country")).map(str::to_string);
        let side = section.and_then(|s| s.get("Side")).map(str::to_string);
        let player_control = section.and_then(|s| s.get_bool("PlayerControl"));
        let allies = section
            .and_then(|s| s.get_list("Allies"))
            .unwrap_or_default()
            .into_iter()
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect();

        houses.push(HouseDefinition {
            name: house_name,
            color,
            country,
            side,
            player_control,
            allies,
        });
    }

    log::info!("HouseRoster: {} entries parsed from map", houses.len());
    HouseRoster { houses }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Retail `[Colors]` list (declaration order) — only the entries the tests
    /// reference need exact positions: DarkRed = entry 5, DarkBlue = entry 10.
    fn test_schemes() -> Vec<ColorSchemeEntry> {
        let raw: &[(&str, [u8; 3])] = &[
            ("LightGold", [25, 255, 255]),  // 0
            ("Gold", [43, 239, 255]),       // 1
            ("LightGrey", [0, 0, 240]),     // 2
            ("Grey", [0, 0, 131]),          // 3
            ("Red", [20, 255, 184]),        // 4
            ("DarkRed", [0, 230, 255]),     // 5
            ("Orange", [25, 230, 255]),     // 6
            ("Magenta", [221, 102, 255]),   // 7
            ("Purple", [201, 201, 189]),    // 8
            ("LightBlue", [119, 143, 255]), // 9
            ("DarkBlue", [153, 214, 212]),  // 10
            ("NeonBlue", [185, 156, 238]),  // 11
            ("DarkSky", [131, 200, 230]),   // 12
            ("Green", [104, 241, 195]),     // 13
            ("DarkGreen", [81, 200, 210]),  // 14
        ];
        raw.iter()
            .map(|(name, hsv)| ColorSchemeEntry {
                name: name.to_string(),
                hsv: *hsv,
            })
            .collect()
    }

    #[test]
    fn test_parse_standard_houses() {
        let ini: IniFile = IniFile::from_str(
            "[Houses]\n0=Americans\n1=Russians\n\
             [Americans]\nColor=DarkBlue\nSide=Allies\nCountry=America\nPlayerControl=yes\n\
             [Russians]\nColor=DarkRed\nSide=Soviet\nCountry=Russia\nAllies=Confederation,YuriCountry\n",
        );
        let roster = parse_house_roster(&ini, &test_schemes());
        let map = roster.color_map();
        assert_eq!(map.len(), 2);
        // Color=name resolves to the [Colors] entry index.
        assert_eq!(map["Americans"], HouseColorIndex(10)); // DarkBlue
        assert_eq!(map["Russians"], HouseColorIndex(5)); // DarkRed
        assert_eq!(roster.houses[0].side.as_deref(), Some("Allies"));
        assert_eq!(roster.houses[0].country.as_deref(), Some("America"));
        assert_eq!(roster.houses[0].player_control, Some(true));
        assert_eq!(
            roster.houses[1].allies,
            vec!["Confederation".to_string(), "YuriCountry".to_string()]
        );
        let alliances = roster.alliance_map();
        assert!(are_houses_friendly(&alliances, "Russians", "Confederation"));
        assert!(are_houses_friendly(&alliances, "YuriCountry", "Russians"));
        assert!(!are_houses_friendly(&alliances, "Americans", "Russians"));
    }

    #[test]
    fn test_missing_color_defaults_to_default_scheme() {
        let ini: IniFile = IniFile::from_str("[Houses]\n0=Neutral\n[Neutral]\nIQ=5\n");
        let map = parse_house_colors(&ini, &test_schemes());
        assert_eq!(map["Neutral"], HouseColorIndex(DEFAULT_SCHEME_ENTRY as u8));
    }

    #[test]
    fn test_unknown_color_defaults_to_default_scheme() {
        let ini: IniFile =
            IniFile::from_str("[Houses]\n0=Neutral\n[Neutral]\nColor=PinkPolkaDot\n");
        let map = parse_house_colors(&ini, &test_schemes());
        assert_eq!(map["Neutral"], HouseColorIndex(DEFAULT_SCHEME_ENTRY as u8));
    }

    #[test]
    fn test_missing_houses_section() {
        let ini: IniFile = IniFile::from_str("[General]\nKey=Value\n");
        let roster = parse_house_roster(&ini, &test_schemes());
        assert!(roster.houses.is_empty());
    }

    #[test]
    fn test_house_without_section() {
        let ini: IniFile = IniFile::from_str("[Houses]\n0=Ghost\n");
        let map = parse_house_colors(&ini, &test_schemes());
        assert_eq!(map["Ghost"], HouseColorIndex(DEFAULT_SCHEME_ENTRY as u8));
    }
}
