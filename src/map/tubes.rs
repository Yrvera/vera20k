//! Explicit `[Tubes]` map-section parsing.
//!
//! Map-authored tubes are full TubeClass records with entry/exit cells and a
//! path-step buffer. Automatic low-bridge shell tubes are constructed later in
//! resolved terrain from final land type and theater tile identity.

use crate::map::tube_facts::TubeFact;
use crate::rules::ini_parser::{IniFile, IniSection};

const MIN_TUBE_FIELDS: usize = 5;
const MAX_TUBE_PATH_STEPS: usize = 100;
const TUBE_PATH_SENTINEL: i32 = -1;

pub fn parse_tubes(ini: &IniFile) -> Vec<TubeFact> {
    let Some(section) = ini.section("Tubes") else {
        return Vec::new();
    };
    parse_tubes_section(section)
}

fn parse_tubes_section(section: &IniSection) -> Vec<TubeFact> {
    let mut tubes = Vec::new();
    for value in section.get_values() {
        let Some(tube) = parse_tube_entry(value) else {
            continue;
        };
        tubes.push(tube);
    }
    if !tubes.is_empty() {
        log::info!("Parsed {} explicit map tubes from [Tubes]", tubes.len());
    }
    tubes
}

fn parse_tube_entry(value: &str) -> Option<TubeFact> {
    let fields: Vec<&str> = value.split(',').map(str::trim).collect();
    if fields.len() < MIN_TUBE_FIELDS {
        log::warn!(
            "[Tubes] entry has {} fields; expected at least 5",
            fields.len()
        );
        return None;
    }

    let entry = (
        parse_u16(fields[0], "[Tubes] entry X")?,
        parse_u16(fields[1], "[Tubes] entry Y")?,
    );
    let direction = parse_direction(fields[2], "[Tubes] entry direction")?;
    let exit = (
        parse_u16(fields[3], "[Tubes] exit X")?,
        parse_u16(fields[4], "[Tubes] exit Y")?,
    );

    let mut path_steps = Vec::new();
    for raw in fields
        .iter()
        .skip(MIN_TUBE_FIELDS)
        .take(MAX_TUBE_PATH_STEPS)
    {
        let Ok(step) = raw.parse::<i32>() else {
            log::warn!("[Tubes] invalid path step '{}'", raw);
            return None;
        };
        if step == TUBE_PATH_SENTINEL {
            break;
        }
        if !(0..=7).contains(&step) {
            log::warn!("[Tubes] path step {} outside direction range 0..=7", step);
            return None;
        }
        path_steps.push(step as u8);
    }

    Some(TubeFact::explicit(entry, exit, direction, path_steps))
}

fn parse_u16(value: &str, label: &str) -> Option<u16> {
    match value.parse::<u16>() {
        Ok(parsed) => Some(parsed),
        Err(_) => {
            log::warn!("{} '{}' is not a valid cell coordinate", label, value);
            None
        }
    }
}

fn parse_direction(value: &str, label: &str) -> Option<u8> {
    match value.parse::<u8>() {
        Ok(parsed @ 0..=7) => Some(parsed),
        _ => {
            log::warn!("{} '{}' is outside direction range 0..=7", label, value);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::tube_facts::TubeSource;

    #[test]
    fn parse_tubes_preserves_entry_exit_direction_and_steps_until_sentinel() {
        let ini = IniFile::from_str("[Tubes]\n0=1,2,2,4,2,2,2,-1,6\n");

        let tubes = parse_tubes(&ini);

        assert_eq!(tubes.len(), 1);
        assert_eq!(tubes[0].entry, (1, 2));
        assert_eq!(tubes[0].exit, (4, 2));
        assert_eq!(tubes[0].direction, 2);
        assert_eq!(tubes[0].path_steps, vec![2, 2]);
        assert_eq!(tubes[0].source, TubeSource::ExplicitMap);
    }

    #[test]
    fn parse_tubes_sorts_numeric_keys_like_other_map_sections() {
        let ini = IniFile::from_str(
            "[Tubes]\n\
             7=7,0,6,4,0,6,6,-1\n\
             2=2,0,2,5,0,2,2,-1\n",
        );

        let tubes = parse_tubes(&ini);

        assert_eq!(tubes.len(), 2);
        assert_eq!(tubes[0].entry, (2, 0));
        assert_eq!(tubes[1].entry, (7, 0));
    }

    #[test]
    fn parse_tubes_caps_path_buffer_at_binary_limit() {
        let mut value = String::from("0,0,2,100,0");
        for _ in 0..105 {
            value.push_str(",2");
        }
        let ini = IniFile::from_str(&format!("[Tubes]\n0={value}\n"));

        let tubes = parse_tubes(&ini);

        assert_eq!(tubes.len(), 1);
        assert_eq!(tubes[0].path_len(), MAX_TUBE_PATH_STEPS);
    }

    #[test]
    fn missing_tubes_section_returns_empty_vec() {
        let ini = IniFile::from_str("[Map]\nTheater=TEMPERATE\n");

        assert!(parse_tubes(&ini).is_empty());
    }

    #[test]
    fn malformed_tube_entry_is_skipped() {
        let ini = IniFile::from_str("[Tubes]\n0=1,2,9,4,2,2,-1\n1=1,2,2,4,2,2,-1\n");

        let tubes = parse_tubes(&ini);

        assert_eq!(tubes.len(), 1);
        assert_eq!(tubes[0].direction, 2);
    }
}
