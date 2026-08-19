//! Explicit `[Tubes]` map-section parsing.
//!
//! Map-authored tubes are full TubeClass records with entry/exit cells and a
//! path-step buffer. Automatic low-bridge shell tubes are constructed later in
//! resolved terrain from final land type and theater tile identity.

use crate::map::tube_facts::TubeFact;
use crate::rules::ini_parser::{IniFile, IniSection};

const MIN_TUBE_FIELDS: usize = 5;
/// `MapClass::ReadTubesINI` 0x007283C0 runs its path loop at most 100 times
/// (`while (iVar6 < 100)`), so at most 99 steps are ever counted into the
/// TubeClass record's +0x1C0 length field.
const NATIVE_PATH_LOOP_ITERATIONS: usize = 100;
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

    // `MapClass::ReadTubesINI` 0x007283C0 path loop. The native loop presets
    // the count at +0x1C0 to -1 and increments it for every value it writes,
    // including the sentinel, so the count always lands one below the number of
    // slots touched. It runs at most 100 times (`iVar6 < 100`), which means an
    // over-long path is silently truncated to 99 counted steps rather than
    // rejected: the 100th value is written into slot 99 and never counted.
    let mut path_steps = Vec::new();
    let mut terminated = false;
    for (slot, raw) in fields.iter().skip(MIN_TUBE_FIELDS).enumerate() {
        let step = crt_atoi(raw);
        if step == TUBE_PATH_SENTINEL {
            terminated = true;
            break;
        }
        if slot + 1 == NATIVE_PATH_LOOP_ITERATIONS {
            // Native `while (iVar6 < 100)` exit. The row survives with the
            // steps it accumulated; the remaining fields are never read.
            terminated = true;
            break;
        }
        path_steps.push(step);
    }
    if !terminated {
        // VERA-internal. gamemd has no check here: `CRT__strtok` 0x007C9CC2
        // returns NULL once the row runs out of fields and `CRT__atoi`
        // 0x007C9B72 dereferences its argument without a null test, so a row
        // with no `-1` inside 100 fields faults the process. Dropping the row
        // is a deliberate divergence from an access violation, not from a
        // behavior. gamemd equivalent UNCHECKED against a live run.
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

    /// `MapClass::ReadTubesINI` 0x007283C0 does not reject an over-long path.
    /// Its `while (iVar6 < 100)` loop stops after 100 iterations, and the
    /// +0x1C0 counter it maintains lands on 99, so the row survives with 99
    /// steps and every later field is ignored.
    #[test]
    fn gsi_04_15_over_long_path_truncates_to_the_native_loop_bound() {
        let mut value = String::from("0,0,2,100,0");
        for _ in 0..105 {
            value.push_str(",2");
        }
        let ini = IniFile::from_str(&format!("[Tubes]\n0={value}\n"));

        let tubes = parse_tubes(&ini);

        assert_eq!(tubes.len(), 1, "gamemd keeps the row; it does not drop it");
        assert_eq!(tubes[0].path_len(), 99);
        assert_eq!(tubes[0].entry, (0, 0));
        assert_eq!(tubes[0].exit, (100, 0));
    }

    /// A sentinel sitting exactly on the last slot the native loop reaches is
    /// still a sentinel: iteration 100 writes -1 and breaks with the counter on
    /// 99, the same length the truncation path produces.
    #[test]
    fn gsi_04_15_sentinel_on_the_final_native_slot_yields_99_steps() {
        let mut value = String::from("0,0,2,100,0");
        for _ in 0..99 {
            value.push_str(",2");
        }
        value.push_str(",-1,7,7");
        let ini = IniFile::from_str(&format!("[Tubes]\n0={value}\n"));

        let tubes = parse_tubes(&ini);

        assert_eq!(tubes.len(), 1);
        assert_eq!(tubes[0].path_len(), 99);
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

    /// VERA-internal. See the comment on the `!terminated` arm: gamemd walks
    /// off the end of the row and faults inside `CRT__atoi` 0x007C9B72, so
    /// there is no native behavior to be faithful to here.
    #[test]
    fn gsi_04_15_missing_path_sentinel_is_rejected_safely() {
        let ini = IniFile::from_str("[Tubes]\n0=1,2,2,4,2,2,2\n");
        assert!(parse_tubes(&ini).is_empty());
    }

    /// RESIDUAL — gamemd address 0x007283C0, `MapClass::ReadTubesINI`.
    ///
    /// Trigger: a `[Tubes]` row with fewer than six comma-separated fields, or
    /// with no `-1` inside its first 100 path fields.
    ///
    /// Effect: gamemd calls `CRT__strtok` 0x007C9CC2 past the end of the row,
    /// gets NULL, and passes it straight to `CRT__atoi` 0x007C9B72, which
    /// dereferences it with no null test — an access violation during map
    /// load. VERA drops the row and keeps loading. Every other tube on the map
    /// therefore exists in VERA and does not exist in gamemd, because gamemd
    /// never finishes the map.
    ///
    /// Frequency: never in a retail map — the YR map editor always emits the
    /// sentinel. Reachable only through a hand-edited or third-party map.
    ///
    /// Left divergent deliberately: matching gamemd means crashing. Recorded
    /// so the difference is not silent.
    #[test]
    #[ignore = "gamemd 0x007283C0 faults on a sentinel-less [Tubes] row; VERA drops the row"]
    fn gsi_04_15_sentinel_less_row_diverges_from_a_native_crash() {
        panic!("intentional divergence: gamemd access-faults where VERA drops the row");
    }
}
