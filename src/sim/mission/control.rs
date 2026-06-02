//! `MissionControl` — the per-mission behaviour table parsed from the
//! `[<MissionName>]` INI sections (Rate / AARate / NoThreat / Zombie /
//! Recruitable / Paralyzed / Retaliate / Scatter).
//!
//! Each mission's section is read independently, starting from the documented
//! INI defaults; an absent key keeps that default. There is **no carry-forward**
//! between missions — the original reader stores each mission in its own table
//! slot and never copies a value from the previously-read mission. `AARate` is
//! the one special case: when absent (or zero) it copies the mission's own
//! `Rate`. Float appears only here, at parse time — the per-minute rate is
//! pre-converted to integer frames so no float ever reaches a tick path.
//! sim/ only.
use super::MissionType;
use crate::rules::ini_parser::IniFile;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Simulation frames per game-minute (15 fps × 60 s). `Rate=`/`AARate=` are
/// expressed in minutes; multiply by this and round to get integer frames.
const FRAMES_PER_MINUTE: f64 = 900.0;

/// Convert an INI rate (minutes between processings) to integer frames.
#[inline]
fn rate_to_frames(minutes: f64) -> u32 {
    (minutes * FRAMES_PER_MINUTE).round() as u32
}

#[inline]
fn parse_minutes(raw: &str) -> Option<f64> {
    raw.trim().parse::<f64>().ok()
}

/// One mission's processing cadence and behaviour flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MissionControlEntry {
    /// Frames between normal processings (`Rate=` × 900, rounded).
    pub rate_frames: u32,
    /// Frames between anti-aircraft processings (`AARate=`; copies `rate_frames`
    /// when the key is absent or zero).
    pub aa_rate_frames: u32,
    /// Weapons disabled → ignored as a target until it fires (`NoThreat=`, def no).
    pub no_threat: bool,
    /// Frozen forever, never recovers (`Zombie=`, def no).
    pub zombie: bool,
    /// Can be recruited into a team / base defence (`Recruitable=`, def yes).
    pub recruitable: bool,
    /// Frozen in place but can still fire and function (`Paralyzed=`, def no).
    pub paralyzed: bool,
    /// Allowed to retaliate while on this mission (`Retaliate=`, def yes).
    pub retaliate: bool,
    /// Allowed to scatter from threats (`Scatter=`, def yes).
    pub scatter: bool,
}

impl Default for MissionControlEntry {
    /// The documented INI header defaults — the values each table slot holds
    /// before its section is read (so an absent key keeps these).
    fn default() -> Self {
        Self {
            rate_frames: 0,
            aa_rate_frames: 0,
            no_threat: false,
            zombie: false,
            recruitable: true,
            paralyzed: false,
            retaliate: true,
            scatter: true,
        }
    }
}

/// The full mission-control table, one entry per dispatched mission id.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MissionControl {
    entries: BTreeMap<MissionType, MissionControlEntry>,
}

impl MissionControl {
    /// Parse every dispatched mission's `[<MissionName>]` section. A mission
    /// whose section is absent keeps the documented defaults (matching the
    /// original reader, which leaves an unread slot at its constructed value).
    pub fn from_ini(ini: &IniFile) -> Self {
        let mut entries = BTreeMap::new();
        for mission in MissionType::all() {
            let mut entry = MissionControlEntry::default();
            if let Some(section) = ini.section(mission.ini_section()) {
                if let Some(v) = section.get_bool("NoThreat") {
                    entry.no_threat = v;
                }
                if let Some(v) = section.get_bool("Zombie") {
                    entry.zombie = v;
                }
                if let Some(v) = section.get_bool("Recruitable") {
                    entry.recruitable = v;
                }
                if let Some(v) = section.get_bool("Paralyzed") {
                    entry.paralyzed = v;
                }
                if let Some(v) = section.get_bool("Retaliate") {
                    entry.retaliate = v;
                }
                if let Some(v) = section.get_bool("Scatter") {
                    entry.scatter = v;
                }
                if let Some(rate) = section.get("Rate").and_then(parse_minutes) {
                    entry.rate_frames = rate_to_frames(rate);
                }
                // AARate: present and non-zero overrides; absent or zero copies Rate.
                match section.get("AARate").and_then(parse_minutes) {
                    Some(aa) if aa != 0.0 => entry.aa_rate_frames = rate_to_frames(aa),
                    _ => entry.aa_rate_frames = entry.rate_frames,
                }
            } else {
                entry.aa_rate_frames = entry.rate_frames;
            }
            entries.insert(mission, entry);
        }
        Self { entries }
    }

    /// The control entry for a mission (present for every dispatched id).
    #[inline]
    pub fn entry(&self, mission: MissionType) -> Option<&MissionControlEntry> {
        self.entries.get(&mission)
    }

    /// Processing cadence in frames for a mission (0 if unknown).
    #[inline]
    pub fn rate_frames(&self, mission: MissionType) -> u32 {
        self.entries.get(&mission).map_or(0, |e| e.rate_frames)
    }

    /// Number of populated mission entries.
    #[inline]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the table holds no entries.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ini(text: &str) -> IniFile {
        IniFile::from_str(text)
    }

    #[test]
    fn rate_to_frames_uses_900_per_minute() {
        assert_eq!(rate_to_frames(1.0), 900);
        assert_eq!(rate_to_frames(0.016), 14); // 14.4 -> 14
        assert_eq!(rate_to_frames(0.030), 27);
        assert_eq!(rate_to_frames(0.040), 36);
    }

    #[test]
    fn aarate_absent_copies_rate_present_overrides() {
        let mc = MissionControl::from_ini(&ini(
            "[Move]\nRate=.016\n[Guard]\nRate=.030\nAARate=.016\n",
        ));
        let mv = mc.entry(MissionType::Move).unwrap();
        assert_eq!(mv.rate_frames, 14);
        assert_eq!(mv.aa_rate_frames, 14); // copied from Rate
        let gd = mc.entry(MissionType::Guard).unwrap();
        assert_eq!(gd.rate_frames, 27);
        assert_eq!(gd.aa_rate_frames, 14); // overridden by AARate
    }

    #[test]
    fn explicit_zero_aarate_copies_rate() {
        let mc = MissionControl::from_ini(&ini("[Guard]\nRate=.030\nAARate=0\n"));
        let gd = mc.entry(MissionType::Guard).unwrap();
        assert_eq!(gd.aa_rate_frames, gd.rate_frames);
        assert_eq!(gd.aa_rate_frames, 27);
    }

    #[test]
    fn bools_use_documented_defaults() {
        // [Move] specifies only Rate → every flag keeps its documented default.
        let mc = MissionControl::from_ini(&ini("[Move]\nRate=.016\n"));
        let mv = mc.entry(MissionType::Move).unwrap();
        assert!(!mv.no_threat);
        assert!(!mv.zombie);
        assert!(mv.recruitable); // def yes
        assert!(!mv.paralyzed);
        assert!(mv.retaliate); // def yes
        assert!(mv.scatter); // def yes
    }

    #[test]
    fn present_bool_overrides_default() {
        let mc = MissionControl::from_ini(&ini(
            "[Sleep]\nRecruitable=no\nZombie=yes\nRetaliate=no\nScatter=no\nRate=1\n",
        ));
        let sl = mc.entry(MissionType::Sleep).unwrap();
        assert!(!sl.recruitable);
        assert!(sl.zombie);
        assert!(!sl.retaliate);
        assert!(!sl.scatter);
        assert_eq!(sl.rate_frames, 900); // Rate=1 minute -> 900 frames
    }

    #[test]
    fn no_carry_forward_between_missions() {
        // Guard sets AARate/Rate; a keyless mission must NOT inherit them.
        let mc = MissionControl::from_ini(&ini("[Guard]\nRate=.030\nAARate=.016\n"));
        let stop = mc.entry(MissionType::Stop).unwrap(); // no [Stop] section
        assert_eq!(stop.rate_frames, 0);
        assert_eq!(stop.aa_rate_frames, 0);
        assert!(stop.recruitable); // documented defaults, not Guard's values
        assert!(stop.retaliate);
        assert!(stop.scatter);
    }

    #[test]
    fn table_is_fully_populated_even_with_empty_ini() {
        let mc = MissionControl::from_ini(&ini(""));
        assert_eq!(mc.len(), MissionType::all().count());
        for m in MissionType::all() {
            assert!(mc.entry(m).is_some(), "missing entry for {m:?}");
        }
    }
}
