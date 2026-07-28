//! Map waypoint parsing.
//!
//! Waypoints are numbered cell anchors used by mission logic, spawns, AI teams,
//! and other map-authored behavior. Parsing them now gives later trigger/team
//! work a stable source of truth.

use std::collections::HashMap;

use crate::rules::ini_parser::IniFile;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Waypoint {
    pub index: u32,
    pub rx: u16,
    pub ry: u16,
}

/// Standard multiplayer/skirmish start waypoints in RA2/YR.
pub const MULTIPLAYER_START_WAYPOINTS: std::ops::RangeInclusive<u32> = 0..=7;

/// Native fallback when a map defines neither a usable `[Waypoints]` start nor
/// a non-zero `[RandomMap] NumPlayers` value.
pub const DEFAULT_SKIRMISH_PLAYER_CAPACITY: i32 = 8;

/// Return the player capacity used by the Skirmish setup shell.
///
/// `gamemd` reads the eight numeric `[Waypoints]` keys with an integer default
/// of `-1` and counts every result other than `-1`; the coordinate itself is not
/// validated by this UI query. Only when that count is zero does it consult
/// `[RandomMap] NumPlayers`, with zero/missing falling back to eight.
pub fn skirmish_player_capacity(ini: &IniFile) -> i32 {
    let waypoint_count = ini.section("Waypoints").map_or(0, |section| {
        MULTIPLAYER_START_WAYPOINTS
            .filter(|index| section.get_i32(&index.to_string()).unwrap_or(-1) != -1)
            .count()
    });
    if waypoint_count != 0 {
        return i32::try_from(waypoint_count).expect("the native query examines only eight keys");
    }

    let random_map_players = ini
        .section("RandomMap")
        .and_then(|section| section.get_i32("NumPlayers"))
        .unwrap_or(0);
    if random_map_players == 0 {
        DEFAULT_SKIRMISH_PLAYER_CAPACITY
    } else {
        random_map_players
    }
}

/// Parse `[Waypoints]` into a waypoint index -> cell mapping.
///
/// RA2/YR maps typically use `NewINIFormat=5`, which packs coordinates as
/// `ry * 1000 + rx`. Older formats use `ry * 128 + rx`.
pub fn parse_waypoints(ini: &IniFile) -> HashMap<u32, Waypoint> {
    let Some(section) = ini.section("Waypoints") else {
        return HashMap::new();
    };

    let coord_factor: u32 = waypoint_coord_factor(ini);
    let mut waypoints: HashMap<u32, Waypoint> = HashMap::new();
    for key in section.keys() {
        let Ok(index) = key.parse::<u32>() else {
            continue;
        };
        let Some(raw_value) = section.get(key) else {
            continue;
        };
        let Ok(coords) = raw_value.trim().parse::<u32>() else {
            continue;
        };
        let rx = (coords % coord_factor) as u16;
        let ry = (coords / coord_factor) as u16;
        waypoints.insert(index, Waypoint { index, rx, ry });
    }

    if !waypoints.is_empty() {
        log::info!("Parsed {} waypoints from [Waypoints]", waypoints.len());
    }
    waypoints
}

/// Return multiplayer start waypoints (0..=7) sorted by waypoint index.
pub fn multiplayer_start_waypoints(waypoints: &HashMap<u32, Waypoint>) -> Vec<Waypoint> {
    let mut starts: Vec<Waypoint> = waypoints
        .values()
        .copied()
        .filter(|wp| MULTIPLAYER_START_WAYPOINTS.contains(&wp.index))
        .collect();
    starts.sort_by_key(|wp| wp.index);
    starts
}

/// Return the first multiplayer/skirmish start waypoint if present.
pub fn first_multiplayer_start(waypoints: &HashMap<u32, Waypoint>) -> Option<Waypoint> {
    multiplayer_start_waypoints(waypoints).into_iter().next()
}

fn waypoint_coord_factor(ini: &IniFile) -> u32 {
    let new_ini_format = ini
        .section("Basic")
        .and_then(|section| section.get("NewINIFormat"))
        .and_then(|value| value.trim().parse::<u32>().ok())
        .unwrap_or(5);
    if new_ini_format >= 4 { 1000 } else { 128 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_waypoints_ra2_format() {
        let ini = IniFile::from_str("[Basic]\nNewINIFormat=5\n[Waypoints]\n0=140034\n99=55098\n");
        let waypoints = parse_waypoints(&ini);
        assert_eq!(waypoints.len(), 2);
        assert_eq!(
            waypoints.get(&0),
            Some(&Waypoint {
                index: 0,
                rx: 34,
                ry: 140
            })
        );
        assert_eq!(
            waypoints.get(&99),
            Some(&Waypoint {
                index: 99,
                rx: 98,
                ry: 55
            })
        );
    }

    #[test]
    fn test_parse_waypoints_old_format() {
        let ini = IniFile::from_str("[Basic]\nNewINIFormat=3\n[Waypoints]\n7=261\n");
        let waypoints = parse_waypoints(&ini);
        assert_eq!(
            waypoints.get(&7),
            Some(&Waypoint {
                index: 7,
                rx: 5,
                ry: 2
            })
        );
    }

    #[test]
    fn test_missing_waypoints_is_empty() {
        let ini = IniFile::from_str("[Map]\nTheater=TEMPERATE\n");
        assert!(parse_waypoints(&ini).is_empty());
    }

    #[test]
    fn test_multiplayer_start_waypoints_are_sorted_and_filtered() {
        let ini = IniFile::from_str(
            "[Basic]\nNewINIFormat=5\n[Waypoints]\n11=200300\n3=100200\n0=100050\n99=55098\n",
        );
        let waypoints = parse_waypoints(&ini);
        let starts = multiplayer_start_waypoints(&waypoints);
        assert_eq!(starts.len(), 2);
        assert_eq!(starts[0].index, 0);
        assert_eq!(starts[1].index, 3);
        assert_eq!(first_multiplayer_start(&waypoints), Some(starts[0]));
    }

    #[test]
    fn skirmish_capacity_counts_native_non_minus_one_waypoint_values() {
        let ini = IniFile::from_str("[Waypoints]\n0=100011\n1=-1\n2=-2\n7=120034\n8=130040\n");

        assert_eq!(skirmish_player_capacity(&ini), 3);
    }

    #[test]
    fn skirmish_capacity_uses_random_map_fallback_only_when_no_starts_exist() {
        let fallback = IniFile::from_str("[Waypoints]\n0=-1\n7=-1\n[RandomMap]\nNumPlayers=4\n");
        assert_eq!(skirmish_player_capacity(&fallback), 4);

        let concrete = IniFile::from_str("[Waypoints]\n0=100011\n[RandomMap]\nNumPlayers=6\n");
        assert_eq!(skirmish_player_capacity(&concrete), 1);
    }

    #[test]
    fn skirmish_capacity_zero_or_missing_falls_back_to_eight() {
        assert_eq!(
            skirmish_player_capacity(&IniFile::from_str("[RandomMap]\nNumPlayers=0\n")),
            DEFAULT_SKIRMISH_PLAYER_CAPACITY
        );
        assert_eq!(
            skirmish_player_capacity(&IniFile::from_str("[Basic]\nName=No Starts\n")),
            DEFAULT_SKIRMISH_PLAYER_CAPACITY
        );
    }

    #[test]
    fn skirmish_capacity_preserves_nonzero_signed_random_map_value() {
        assert_eq!(
            skirmish_player_capacity(&IniFile::from_str("[RandomMap]\nNumPlayers=-2\n")),
            -2
        );
    }
}
