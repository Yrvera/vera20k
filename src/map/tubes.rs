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

    let entry = (crt_atoi(fields[0]) as u16, crt_atoi(fields[1]) as u16);
    let direction = crt_atoi(fields[2]);
    let exit = (crt_atoi(fields[3]) as u16, crt_atoi(fields[4]) as u16);

    let mut path_steps = Vec::new();
    let mut terminated = false;
    for raw in fields.iter().skip(MIN_TUBE_FIELDS) {
        let step = crt_atoi(raw);
        if step == TUBE_PATH_SENTINEL {
            terminated = true;
            break;
        }
        if path_steps.len() == MAX_TUBE_PATH_STEPS {
            log::warn!("[Tubes] path buffer has no -1 sentinel within 100 entries");
            return None;
        }
        path_steps.push(step);
    }
    if !terminated {
        log::warn!("[Tubes] path buffer is missing its -1 sentinel");
        return None;
    }

    Some(TubeFact::explicit(entry, exit, direction, path_steps))
}

/// The supported, deterministic part of CRT `atoi`: leading ASCII whitespace,
/// one optional sign, then the maximal decimal digit prefix. Missing digits
/// return zero. Accumulation wraps in the same 32-bit domain used by TubeClass.
fn crt_atoi(value: &str) -> i32 {
    let bytes = value.as_bytes();
    let mut index = 0;
    while bytes.get(index).is_some_and(u8::is_ascii_whitespace) {
        index += 1;
    }
    let mut negative = false;
    if let Some(sign) = bytes.get(index) {
        if *sign == b'-' || *sign == b'+' {
            negative = *sign == b'-';
            index += 1;
        }
    }
    let mut value = 0_i32;
    let mut saw_digit = false;
    while let Some(&digit) = bytes.get(index) {
        if !digit.is_ascii_digit() {
            break;
        }
        saw_digit = true;
        value = value.wrapping_mul(10).wrapping_add(i32::from(digit - b'0'));
        index += 1;
    }
    if !saw_digit {
        0
    } else if negative {
        value.wrapping_neg()
    } else {
        value
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
    fn parse_tubes_preserves_source_order() {
        let ini = IniFile::from_str(
            "[Tubes]\n\
             7=7,0,6,4,0,6,6,-1\n\
             2=2,0,2,5,0,2,2,-1\n",
        );

        let tubes = parse_tubes(&ini);

        assert_eq!(tubes.len(), 2);
        assert_eq!(tubes[0].entry, (7, 0));
        assert_eq!(tubes[1].entry, (2, 0));
    }

    #[test]
    fn gsi_04_15_parse_tubes_rejects_path_without_in_bounds_sentinel() {
        let mut value = String::from("0,0,2,100,0");
        for _ in 0..105 {
            value.push_str(",2");
        }
        let ini = IniFile::from_str(&format!("[Tubes]\n0={value}\n"));

        let tubes = parse_tubes(&ini);

        assert!(tubes.is_empty());
    }

    #[test]
    fn missing_tubes_section_returns_empty_vec() {
        let ini = IniFile::from_str("[Map]\nTheater=TEMPERATE\n");

        assert!(parse_tubes(&ini).is_empty());
    }

    #[test]
    fn gsi_04_15_raw_atoi_values_and_low16_coordinates_are_preserved() {
        let ini = IniFile::from_str("[Tubes]\nname=-1,65537,9tail,4,2,10,-2,15,-1,6\n");

        let tubes = parse_tubes(&ini);

        assert_eq!(tubes.len(), 1);
        assert_eq!(tubes[0].entry, (u16::MAX, 1));
        assert_eq!(tubes[0].direction, 9);
        assert_eq!(tubes[0].path_steps, vec![10, -2, 15]);
    }

    #[test]
    fn gsi_04_15_missing_path_sentinel_is_rejected_safely() {
        let ini = IniFile::from_str("[Tubes]\n0=1,2,2,4,2,2,2\n");
        assert!(parse_tubes(&ini).is_empty());
    }
}
