//! Source-ordered scenario records for the Skirmish Choose Map modal.
//!
//! Retail Choose Map consumes a scenario-record list in source append order.
//! This module models that list separately from the legacy display-sorted
//! `available_maps` menu list.

use crate::app_init::MapMenuEntry;
use crate::map::briefing::BriefingSection;
use crate::map::preview::{PreviewSection, PreviewSourceBounds};
use crate::map::waypoints::Waypoint;
use crate::rules::ini_parser::IniFile;
use crate::skirmish_modes::SkirmishGameMode;

pub const RANDMAP_SED: &str = "RandMap.Sed";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkirmishScenarioSource {
    MissionsMdPkt,
    LoosePkt(String),
    LooseYro(String),
    LooseYrm(String),
    Synthetic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkirmishScenarioKind {
    ConcreteMap,
    RandomMapSentinel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkirmishScenarioRecord {
    pub source_ordinal: usize,
    pub source: SkirmishScenarioSource,
    pub file_name: String,
    pub display_name: String,
    pub author: Option<String>,
    pub briefing: BriefingSection,
    pub preview: PreviewSection,
    pub multiplayer_start_waypoints: Vec<Waypoint>,
    pub preview_source_bounds: Option<PreviewSourceBounds>,
    pub game_modes: Vec<String>,
    pub min_players: Option<u8>,
    pub max_players: Option<u8>,
    pub official: bool,
    pub kind: SkirmishScenarioKind,
}

impl SkirmishScenarioRecord {
    pub fn concrete_from_ini(
        source_ordinal: usize,
        source: SkirmishScenarioSource,
        file_name: &str,
        ini: &IniFile,
    ) -> Self {
        let entry = crate::app_list_maps::read_map_menu_entry_from_ini(ini, file_name);
        let basic = ini.section("Basic");
        Self {
            source_ordinal,
            source,
            file_name: entry.file_name,
            display_name: entry.display_name,
            author: entry.author,
            briefing: entry.briefing,
            preview: entry.preview,
            multiplayer_start_waypoints: entry.multiplayer_start_waypoints,
            preview_source_bounds: entry.preview_source_bounds,
            game_modes: parse_game_modes(ini),
            min_players: basic
                .and_then(|section| section.get_i32("MinPlayers"))
                .and_then(valid_player_count),
            max_players: basic
                .and_then(|section| section.get_i32("MaxPlayers"))
                .and_then(valid_player_count),
            official: basic
                .and_then(|section| section.get_bool("Official"))
                .unwrap_or(false),
            kind: SkirmishScenarioKind::ConcreteMap,
        }
    }

    pub fn pkt_from_ini(
        source_ordinal: usize,
        source: SkirmishScenarioSource,
        file_name: &str,
        ini: &IniFile,
        display_name: impl Into<String>,
    ) -> Self {
        let mut record = Self::concrete_from_ini(source_ordinal, source, file_name, ini);
        record.display_name = display_name.into();
        if record.min_players.is_none() {
            record.min_players = Some(2);
        }
        if record.max_players.is_none() {
            record.max_players = Some(4);
        }
        record.official = ini
            .section("Basic")
            .and_then(|section| section.get_bool("Official"))
            .unwrap_or(true);
        record
    }

    pub fn random_map_sentinel(source_ordinal: usize, display_name: impl Into<String>) -> Self {
        Self {
            source_ordinal,
            source: SkirmishScenarioSource::Synthetic,
            file_name: RANDMAP_SED.to_string(),
            display_name: display_name.into(),
            author: None,
            briefing: BriefingSection::default(),
            preview: PreviewSection::default(),
            multiplayer_start_waypoints: Vec::new(),
            preview_source_bounds: None,
            game_modes: Vec::new(),
            min_players: None,
            max_players: None,
            official: false,
            kind: SkirmishScenarioKind::RandomMapSentinel,
        }
    }

    pub fn from_map_menu_entry(source_ordinal: usize, entry: &MapMenuEntry) -> Self {
        Self {
            source_ordinal,
            source: source_for_file_name(&entry.file_name),
            file_name: entry.file_name.clone(),
            display_name: entry.display_name.clone(),
            author: entry.author.clone(),
            briefing: entry.briefing.clone(),
            preview: entry.preview.clone(),
            multiplayer_start_waypoints: entry.multiplayer_start_waypoints.clone(),
            preview_source_bounds: entry.preview_source_bounds.clone(),
            game_modes: Vec::new(),
            min_players: None,
            max_players: None,
            official: false,
            kind: SkirmishScenarioKind::ConcreteMap,
        }
    }

    pub fn to_map_menu_entry(&self) -> MapMenuEntry {
        MapMenuEntry {
            file_name: self.file_name.clone(),
            display_name: self.display_name.clone(),
            author: self.author.clone(),
            briefing: self.briefing.clone(),
            preview: self.preview.clone(),
            multiplayer_start_waypoints: self.multiplayer_start_waypoints.clone(),
            preview_source_bounds: self.preview_source_bounds.clone(),
        }
    }
}

fn source_for_file_name(file_name: &str) -> SkirmishScenarioSource {
    let ext = std::path::Path::new(file_name)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(str::to_ascii_lowercase);
    match ext.as_deref() {
        Some("yro") => SkirmishScenarioSource::LooseYro(file_name.to_string()),
        Some("yrm") => SkirmishScenarioSource::LooseYrm(file_name.to_string()),
        Some("pkt") => SkirmishScenarioSource::LoosePkt(file_name.to_string()),
        _ => SkirmishScenarioSource::Synthetic,
    }
}

fn valid_player_count(value: i32) -> Option<u8> {
    (0..=u8::MAX as i32).contains(&value).then_some(value as u8)
}

pub fn parse_game_modes(ini: &IniFile) -> Vec<String> {
    ini.section("Basic")
        .and_then(|section| section.get_list("GameModes"))
        .unwrap_or_default()
        .into_iter()
        .map(str::trim)
        .filter(|mode| !mode.is_empty())
        .map(str::to_string)
        .collect()
}

pub fn filter_records_for_mode(
    records: &[SkirmishScenarioRecord],
    mode: &SkirmishGameMode,
) -> Vec<usize> {
    records
        .iter()
        .enumerate()
        .filter_map(|(idx, record)| record_matches_mode(record, mode).then_some(idx))
        .collect()
}

pub fn record_matches_mode(record: &SkirmishScenarioRecord, mode: &SkirmishGameMode) -> bool {
    match record.kind {
        SkirmishScenarioKind::RandomMapSentinel => mode.random_maps_allowed,
        SkirmishScenarioKind::ConcreteMap if record.game_modes.is_empty() => {
            mode.map_filter == "standard"
        }
        SkirmishScenarioKind::ConcreteMap => record
            .game_modes
            .iter()
            .any(|game_mode| game_mode == &mode.map_filter),
    }
}

pub fn upsert_random_map_sentinel(
    records: &mut Vec<SkirmishScenarioRecord>,
    display_name: impl Into<String>,
) -> usize {
    if let Some(idx) = records
        .iter()
        .position(|record| record.kind == SkirmishScenarioKind::RandomMapSentinel)
    {
        records[idx].display_name = display_name.into();
        return idx;
    }

    let idx = records.len();
    records.push(SkirmishScenarioRecord::random_map_sentinel(
        idx,
        display_name,
    ));
    idx
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skirmish_modes::stock_skirmish_modes;

    fn mode(id: i32) -> SkirmishGameMode {
        stock_skirmish_modes()
            .into_iter()
            .find(|mode| mode.id == id)
            .expect("stock mode")
    }

    fn record(source_ordinal: usize, name: &str, game_modes: &str) -> SkirmishScenarioRecord {
        let ini = IniFile::from_str(&format!(
            "[Basic]\nName={name}\nGameModes={game_modes}\nMinPlayers=2\nMaxPlayers=8\nOfficial=yes\n\
             [Waypoints]\n0=100011\n1=110012\n"
        ));
        SkirmishScenarioRecord::concrete_from_ini(
            source_ordinal,
            SkirmishScenarioSource::LooseYrm(format!("{name}.yrm")),
            &format!("{name}.yrm"),
            &ini,
        )
    }

    #[test]
    fn scenario_record_parses_game_modes() {
        let rec = record(0, "Duel Map", "duel, meatgrind");
        assert_eq!(rec.game_modes, vec!["duel", "meatgrind"]);
        assert_eq!(rec.min_players, Some(2));
        assert_eq!(rec.max_players, Some(8));
        assert!(rec.official);
    }

    #[test]
    fn scenario_record_projects_to_map_menu_entry() {
        let rec = record(3, "Projected", "standard");
        let entry = rec.to_map_menu_entry();
        assert_eq!(entry.file_name, "Projected.yrm");
        assert_eq!(entry.display_name, "Projected");
        assert_eq!(entry.multiplayer_start_waypoints.len(), 2);
    }

    #[test]
    fn pkt_record_uses_pkt_display_name_and_defaults() {
        let ini = IniFile::from_str("[Basic]\nName=Basic Name\nGameModes=standard\n");
        let rec = SkirmishScenarioRecord::pkt_from_ini(
            7,
            SkirmishScenarioSource::MissionsMdPkt,
            "Official.MAP",
            &ini,
            "PKT Display",
        );

        assert_eq!(rec.source_ordinal, 7);
        assert_eq!(rec.display_name, "PKT Display");
        assert_eq!(rec.file_name, "Official.MAP");
        assert_eq!(rec.min_players, Some(2));
        assert_eq!(rec.max_players, Some(4));
        assert!(rec.official);
    }

    #[test]
    fn choose_map_filters_by_selected_mpmode_game_modes() {
        let records = vec![
            record(0, "Battle", "standard"),
            record(1, "Team", "teamgame"),
            record(2, "Duel", "duel"),
        ];
        assert_eq!(filter_records_for_mode(&records, &mode(9)), vec![1]);
        assert_eq!(filter_records_for_mode(&records, &mode(6)), vec![2]);
    }

    #[test]
    fn choose_map_empty_game_modes_matches_standard_only() {
        let records = vec![record(0, "Empty", "")];
        assert_eq!(filter_records_for_mode(&records, &mode(1)), vec![0]);
        assert!(filter_records_for_mode(&records, &mode(9)).is_empty());
    }

    #[test]
    fn choose_map_filter_preserves_source_order_and_duplicates() {
        let records = vec![
            record(0, "Zoo", "standard"),
            record(1, "Alpha", "standard"),
            record(2, "Zoo", "standard"),
        ];
        let filtered = filter_records_for_mode(&records, &mode(1));
        assert_eq!(filtered, vec![0, 1, 2]);
        assert_eq!(records[0].display_name, records[2].display_name);
    }

    #[test]
    fn choose_map_filter_ignores_ui_label_and_category() {
        let battle = SkirmishGameMode {
            id: 42,
            ui_name_key: "GUI:TeamGame".to_string(),
            tooltip_key: String::new(),
            override_file: String::new(),
            map_filter: "standard".to_string(),
            random_maps_allowed: false,
            allies_allowed: true,
            must_ally: false,
        };
        let records = vec![record(0, "Standard", "standard")];
        assert_eq!(filter_records_for_mode(&records, &battle), vec![0]);
    }

    #[test]
    fn choose_map_filters_randmap_by_mode_random_allowed() {
        let records = vec![SkirmishScenarioRecord::random_map_sentinel(0, "Random Map")];
        assert_eq!(filter_records_for_mode(&records, &mode(1)), vec![0]);
        assert!(filter_records_for_mode(&records, &mode(9)).is_empty());
    }

    #[test]
    fn skirmish_random_map_command_adds_or_updates_single_sentinel_record() {
        let mut records = vec![record(0, "Concrete", "standard")];
        let first = upsert_random_map_sentinel(&mut records, "Random Map");
        let second = upsert_random_map_sentinel(&mut records, "Updated Random Map");
        assert_eq!(first, 1);
        assert_eq!(second, 1);
        assert_eq!(records.len(), 2);
        assert_eq!(records[1].file_name, RANDMAP_SED);
        assert_eq!(records[1].display_name, "Updated Random Map");
    }

    #[test]
    fn scenario_records_retain_explicit_source_ordinals() {
        let records = vec![
            record(10, "From Missions", "standard"),
            record(11, "From Loose Pkt", "standard"),
            record(12, "From Yro", "standard"),
            record(13, "From Yrm", "standard"),
        ];
        let ordinals: Vec<usize> = records.iter().map(|record| record.source_ordinal).collect();
        assert_eq!(ordinals, vec![10, 11, 12, 13]);
    }
}
