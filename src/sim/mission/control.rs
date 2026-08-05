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
/// expressed in minutes; multiply by this and truncate (ftol) to get integer frames.
const FRAMES_PER_MINUTE: f64 = 900.0;

/// The `Rate=`/`AARate=` value a mission-control slot holds *before* any INI is
/// read: 0.016 minutes, i.e. `ftol(0.016 x 900) = 14` frames.
///
/// The original's `MissionControlClass` constructor stores this same double
/// into both the `Rate` and `AARate` fields of all 32 slots at process start;
/// its reader then passes the *current* double as the `Rate` default (so an
/// absent `Rate=` keeps it) and returns without touching the slot at all when
/// the `[<MissionName>]` section is absent. Eleven stock missions — Return,
/// Stop, Ambush, Construction, Selling, both Paradrop legs, Wait, Attack Move
/// and both Spyplane legs — reach their cadence through one of those two paths
/// and therefore resolve to 14 frames, never 0.
const CONSTRUCTED_RATE_MINUTES: f64 = 0.016;

/// Convert an INI rate (minutes between processings) to integer frames,
/// modelling gamemd's `Math::ftol(Rate * 900)` truncate-toward-zero (the
/// per-minute domain is non-negative, so `as u32` == floor == ftol here).
#[inline]
fn rate_to_frames(minutes: f64) -> u32 {
    (minutes * FRAMES_PER_MINUTE) as u32
}

#[inline]
fn parse_minutes(raw: &str) -> Option<f64> {
    raw.trim().parse::<f64>().ok()
}

/// One mission's processing cadence and behaviour flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MissionControlEntry {
    /// Frames between normal processings (`Rate=` × 900, ftol-truncated).
    pub rate_frames: u32,
    /// Frames between anti-aircraft processings (`AARate=`; copies `rate_frames`
    /// when the key is absent or zero).
    pub aa_rate_frames: u32,
    /// Weapons disabled → ignored as a target until it fires (`NoThreat=`, def no).
    pub no_threat: bool,
    /// Frozen forever, never recovers (`Zombie=`, def no).
    pub zombie: bool,
    /// Can be recruited into a team / base defence (`Recruitable=`, def yes).
    ///
    /// **No consumer, either side.** Its only retail readers are team
    /// recruitment, and VERA has no AI opponent to run a team. `[Sticky]` and
    /// `[Area Guard]` are the two stock sections that set it explicitly.
    pub recruitable: bool,
    /// Frozen in place but can still fire and function (`Paralyzed=`, def no).
    ///
    /// Two verified consumer families, both indexing the object's **own
    /// current** mission slot:
    /// 1. the Unit and Infantry arrival selectors — modelled, on the infantry
    ///    side, in the Move handler's arrival arm. Note the vehicle side reads
    ///    it only on the Area-Guard-promotion branch, not on the ordinary Guard
    ///    fall, so a `Paralyzed=` mission does not stop a vehicle falling to
    ///    Guard;
    /// 2. the scatter paths, which live in `sim/movement` — **not consumed
    ///    there yet**. That is where `[Sticky] Paralyzed=yes` would actually
    ///    bite: a Sticky civilian can currently be shoved off its cell, which
    ///    is the opposite of the section comment's "cannot move".
    pub paralyzed: bool,
    /// Allowed to retaliate while on this mission (`Retaliate=`, def yes).
    pub retaliate: bool,
    /// Allowed to scatter from threats (`Scatter=`, def yes).
    ///
    /// Verified consumers are the Infantry, Unit and aircraft scatter paths
    /// plus the damage handler — all in `sim/movement` / `sim/combat` damage,
    /// none of which read this table yet. `Scatter=no` covers Sleep, Sticky,
    /// Attack, Capture, **Harvest**, Unload, Construction and Selling in stock
    /// rules, so the live gap is on every miner in every match, not just on
    /// Sticky.
    pub scatter: bool,
}

impl Default for MissionControlEntry {
    /// The constructed defaults — the values each table slot holds before its
    /// section is read (so an absent key, or an absent section, keeps these).
    /// The six bools match the `; def=` comments in the RULESMD header block;
    /// the rate pair has no INI comment and comes from the constructor
    /// (see [`CONSTRUCTED_RATE_MINUTES`]).
    fn default() -> Self {
        Self {
            rate_frames: rate_to_frames(CONSTRUCTED_RATE_MINUTES),
            aa_rate_frames: rate_to_frames(CONSTRUCTED_RATE_MINUTES),
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

    /// Anti-air processing cadence in frames for a mission (0 if unknown).
    ///
    /// This is a *different* number from [`MissionControl::rate_frames`] on the
    /// two stock missions that declare both: Guard resolves 27 / **14** and
    /// Area Guard 36 / **28**, so a consumer that reaches for the wrong field
    /// is wrong by a factor of two.
    ///
    /// **Who actually reads it, and when** (resolved from the building Guard
    /// handler — the function at the BuildingClass vtable slot the mission
    /// dispatcher's Guard and Sticky cases call; slot identity proven by
    /// reading that vtable entry, not by a label. Ghidra leaves the function
    /// unnamed and its plate comment guesses a different mission; ignore it):
    ///
    /// The handler branches once, at the top, on the object's "has a weapon"
    /// query, and that single branch selects the field. It reaches the control
    /// table on three paths, and all three are accounted for:
    ///
    /// - **Armed** (a base defence) → the timer return reads `AARate` and
    ///   yields `ftol(AARate x 900) + RandomRanged(0, 2)`. This path is taken
    ///   whenever the defence did not commit Attack this dispatch.
    /// - **Unarmed**, repair-depot-flagged → reads `Rate`, same shape.
    /// - **Unarmed**, otherwise → reads `Rate` and returns **three times** it,
    ///   plus the same jitter.
    ///
    /// So the selector is *the building is weapon-equipped*, NOT "the current
    /// target is an aircraft" — which is what the key's name suggests and what
    /// an earlier note here assumed. An armed structure re-arms at `AARate`
    /// against ground and air alike.
    ///
    /// Whether any non-building handler reads `AARate` is UNCHECKED; a
    /// binary-wide scan found only building-side readers.
    ///
    /// No `sim/` caller reads this yet — VERA has no building mission-handler
    /// cadence (recorded gap GSI-07.02 G2).
    #[inline]
    pub fn aa_rate_frames(&self, mission: MissionType) -> u32 {
        self.entries.get(&mission).map_or(0, |e| e.aa_rate_frames)
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
        assert_eq!(rate_to_frames(0.016), 14); // 14.4 -> ftol 14 (stock, unchanged)
        assert_eq!(rate_to_frames(0.030), 27);
        assert_eq!(rate_to_frames(0.040), 36);
        // ftol truncates toward zero: a modded rate whose ×900 has a ≥.5
        // fraction lands one frame below what `.round()` would have produced.
        assert_eq!(rate_to_frames(0.0206), 18); // 18.54 -> ftol 18 (round would give 19)
    }

    #[test]
    fn aarate_absent_copies_rate_present_overrides() {
        let mc =
            MissionControl::from_ini(&ini("[Move]\nRate=.016\n[Guard]\nRate=.030\nAARate=.016\n"));
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
        // Guard sets AARate/Rate; a keyless mission must NOT inherit them —
        // it keeps its own constructed values, which for the rate pair is
        // 0.016 minutes = 14 frames, NOT zero.
        let mc = MissionControl::from_ini(&ini("[Guard]\nRate=.030\nAARate=.016\n"));
        let stop = mc.entry(MissionType::Stop).unwrap(); // no [Stop] section
        assert_eq!(stop.rate_frames, 14);
        assert_eq!(stop.aa_rate_frames, 14);
        assert!(stop.recruitable); // documented defaults, not Guard's values
        assert!(stop.retaliate);
        assert!(stop.scatter);
    }

    #[test]
    fn constructed_rate_is_fourteen_frames_not_zero() {
        // The constructor stores 0.016 minutes into Rate and AARate alike, and
        // neither the absent-section path nor the absent-key path overwrites
        // it. Both must therefore land on ftol(0.016 * 900) = 14.
        assert_eq!(rate_to_frames(CONSTRUCTED_RATE_MINUTES), 14);

        // Absent section (rows 26-31 in stock: the paradrop/spyplane legs,
        // Wait and Attack Move declare no `[<MissionName>]` section at all).
        let absent_section = MissionControl::from_ini(&ini("[Guard]\nRate=.030\nAARate=.016\n"));
        for mission in [
            MissionType::ParadropApproach,
            MissionType::ParadropOverfly,
            MissionType::Deliberate,
            MissionType::AttackMove,
            MissionType::SpyplaneApproach,
            MissionType::SpyplaneOverfly,
        ] {
            let entry = absent_section.entry(mission).unwrap();
            assert_eq!(entry.rate_frames, 14, "{mission:?} Rate");
            assert_eq!(entry.aa_rate_frames, 14, "{mission:?} AARate");
        }

        // Present-but-Rate-less section (rows 12/13/14/18/19 in stock:
        // Return, Stop, Ambush, Construction, Selling).
        let rate_less = MissionControl::from_ini(&ini("[Selling]\nNoThreat=yes\nRetaliate=no\n"));
        let selling = rate_less.entry(MissionType::Selling).unwrap();
        assert_eq!(selling.rate_frames, 14);
        assert_eq!(selling.aa_rate_frames, 14);
        assert!(selling.no_threat);
        assert!(!selling.retaliate);
    }

    #[test]
    fn guard_and_area_guard_keep_distinct_rate_and_aa_rate() {
        // The two stock missions whose AARate differs from Rate. A building
        // cadence must read AARate (14 / 28), not Rate (27 / 36).
        let mc = MissionControl::from_ini(&ini(
            "[Guard]\nRate=.030\nAARate=.016\n[Area Guard]\nRate=.040\nAARate=.032\n",
        ));
        assert_eq!(mc.rate_frames(MissionType::Guard), 27);
        assert_eq!(mc.aa_rate_frames(MissionType::Guard), 14);
        // .032 * 900 = 28.8; ftol truncates toward zero -> 28 (round gives 29).
        assert_eq!(mc.rate_frames(MissionType::AreaGuard), 36);
        assert_eq!(mc.aa_rate_frames(MissionType::AreaGuard), 28);
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
