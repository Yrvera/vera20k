//! Missile-spawn globals — the three hardcoded rocket families the spawn
//! manager treats as "missile-style" children.
//!
//! The per-slot `IsMissileSpawn` flag does **not** come from the child's
//! `MissileSpawn=` key. `SpawnManagerClass` compares the resolved `Spawns=`
//! TechnoType pointer against three fixed `RulesClass` slots —
//! `[General] V3RocketType=`, `DMislType=`, `CMislType=` — and sets the flag
//! only on a match. That is what this module models.
//!
//! The child's own `MissileSpawn=` key is a *separate* test the manager also
//! makes, on a different decision: in the Launching arm it reads
//! `childType+0xD68` to pick retreat (fire-and-forget) versus return-to-dock,
//! and `FUN_0054e3b0` reads it again to pick the retreat-list path versus an
//! outright kill. So a modded child with `MissileSpawn=yes` that is not one of
//! the three named types takes the fire-and-forget branch but never gets the
//! slot flag, and so never gets the pause+tilt kamikaze window. That asymmetry
//! is a real YR quirk. The two sets coincide in stock YR.
//!
//! The same three families carry the launch-pause / tilt frame counts and the
//! detonation damage + warhead used when the missile impacts.
//!
//! Sources: `docs/research/SPAWN_MANAGER_CLASS_GHIDRA_REPORT.md` §6 (Rules
//! slot offsets, hardcoded pointer test), `docs/research/
//! ROCKET_LOCOMOTION_CLASS_GHIDRA_REPORT.md` §3–§4 (RocketStruct fields,
//! detonation warhead selection), plus `decompile_function 0x006B7230`
//! (`SpawnManagerClass::AI`) read this session for the state-1 timer sources.
//!
//! ## Dependency rules
//! - Part of rules/ — no dependencies on sim/, render/, ui/, etc.

use crate::rules::ini_parser::IniSection;

/// Retail `[General]`/`[CombatDamage]` values, used when a key is absent.
///
/// These mirror stock `rulesmd.ini`, not the `RulesClass` constructor: the
/// binary's own constructor defaults for these slots are **UNCHECKED**. Every
/// retail INI ships all of them, so the fallback never fires in stock play.
mod retail_defaults {
    pub const V3_TYPE: &str = "V3ROCKET";
    pub const V3_PAUSE_FRAMES: u32 = 0;
    pub const V3_TILT_FRAMES: u32 = 60;
    pub const V3_DAMAGE: i32 = 200;
    pub const V3_ELITE_DAMAGE: i32 = 400;
    pub const V3_WARHEAD: &str = "V3WH";
    pub const V3_ELITE_WARHEAD: &str = "V3EWH";

    pub const DMISL_TYPE: &str = "DMISL";
    pub const DMISL_PAUSE_FRAMES: u32 = 20;
    pub const DMISL_TILT_FRAMES: u32 = 60;
    pub const DMISL_DAMAGE: i32 = 300;
    pub const DMISL_ELITE_DAMAGE: i32 = 600;
    pub const DMISL_WARHEAD: &str = "DMISLWH";
    pub const DMISL_ELITE_WARHEAD: &str = "DMISLEWH";

    pub const CMISL_TYPE: &str = "CMISL";
    pub const CMISL_DAMAGE: i32 = 200;
    pub const CMISL_ELITE_DAMAGE: i32 = 250;
    pub const CMISL_WARHEAD: &str = "CMISLWH";
    pub const CMISL_ELITE_WARHEAD: &str = "CMISLEWH";
}

/// Which of the three hardcoded rocket families a spawn child belongs to.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum MissileFamily {
    /// `[General] V3RocketType=` — V3 Launcher.
    V3Rocket,
    /// `[General] DMislType=` — Dreadnought.
    DMisl,
    /// `[General] CMislType=` — Boomer.
    CMisl,
}

/// Per-family launch timing, detonation damage and warhead.
#[derive(Debug, Clone)]
pub struct MissileSpawnParams {
    /// Resolved child TechnoType section name (`V3ROCKET` / `DMISL` / `CMISL`).
    pub type_name: String,
    /// `*PauseFrames` — frames the missile rests on the launcher before tilting.
    pub pause_frames: u32,
    /// `*TiltFrames` — frames the tilt-to-firing-position takes.
    pub tilt_frames: u32,
    /// `[CombatDamage] *Damage` — damage applied at impact (rookie/veteran).
    pub damage: i32,
    /// `[CombatDamage] *EliteDamage` — damage applied when the launcher is elite.
    pub elite_damage: i32,
    /// `[CombatDamage] *Warhead` — impact warhead (rookie/veteran).
    pub warhead: String,
    /// `[CombatDamage] *EliteWarhead` — impact warhead when the launcher is elite.
    pub elite_warhead: String,
}

impl MissileSpawnParams {
    /// Damage for a launcher at the supplied veterancy (0 rookie / 100 veteran
    /// / 200 elite). Only the elite band swaps to the elite value; gamemd's
    /// `RocketLocomotion::Detonate` selects on the elite flag alone.
    pub fn damage_for(&self, veterancy: u16) -> i32 {
        if veterancy >= ELITE_VETERANCY {
            self.elite_damage
        } else {
            self.damage
        }
    }

    /// Warhead for a launcher at the supplied veterancy. Same elite-only split
    /// as `damage_for`.
    pub fn warhead_for(&self, veterancy: u16) -> &str {
        if veterancy >= ELITE_VETERANCY {
            &self.elite_warhead
        } else {
            &self.warhead
        }
    }
}

/// Veterancy value at which a unit counts as elite (0/100/200 scale).
const ELITE_VETERANCY: u16 = 200;

/// The three hardcoded missile-spawn families.
#[derive(Debug, Clone)]
pub struct MissileSpawnRules {
    pub v3: MissileSpawnParams,
    pub dmisl: MissileSpawnParams,
    pub cmisl: MissileSpawnParams,
}

impl Default for MissileSpawnRules {
    fn default() -> Self {
        use retail_defaults as d;
        Self {
            v3: MissileSpawnParams {
                type_name: d::V3_TYPE.to_string(),
                pause_frames: d::V3_PAUSE_FRAMES,
                tilt_frames: d::V3_TILT_FRAMES,
                damage: d::V3_DAMAGE,
                elite_damage: d::V3_ELITE_DAMAGE,
                warhead: d::V3_WARHEAD.to_string(),
                elite_warhead: d::V3_ELITE_WARHEAD.to_string(),
            },
            dmisl: MissileSpawnParams {
                type_name: d::DMISL_TYPE.to_string(),
                pause_frames: d::DMISL_PAUSE_FRAMES,
                tilt_frames: d::DMISL_TILT_FRAMES,
                damage: d::DMISL_DAMAGE,
                elite_damage: d::DMISL_ELITE_DAMAGE,
                warhead: d::DMISL_WARHEAD.to_string(),
                elite_warhead: d::DMISL_ELITE_WARHEAD.to_string(),
            },
            cmisl: MissileSpawnParams {
                type_name: d::CMISL_TYPE.to_string(),
                // The manager's state-1 timer reads the DMisl pause/tilt slots
                // for every non-V3 family, so CMisl's own `CMisl*Frames` keys
                // are not consulted there. They are left out rather than
                // parsed-and-ignored.
                pause_frames: d::DMISL_PAUSE_FRAMES,
                tilt_frames: d::DMISL_TILT_FRAMES,
                damage: d::CMISL_DAMAGE,
                elite_damage: d::CMISL_ELITE_DAMAGE,
                warhead: d::CMISL_WARHEAD.to_string(),
                elite_warhead: d::CMISL_ELITE_WARHEAD.to_string(),
            },
        }
    }
}

impl MissileSpawnRules {
    /// Parse from `[General]` (type names + pause/tilt frames) and
    /// `[CombatDamage]` (damage + warheads). Missing keys keep the retail
    /// fallback.
    pub fn from_ini_sections(
        general: Option<&IniSection>,
        combat_damage: Option<&IniSection>,
    ) -> Self {
        let mut out = Self::default();

        if let Some(g) = general {
            if let Some(name) = read_name(g, "V3RocketType") {
                out.v3.type_name = name;
            }
            if let Some(name) = read_name(g, "DMislType") {
                out.dmisl.type_name = name;
            }
            if let Some(name) = read_name(g, "CMislType") {
                out.cmisl.type_name = name;
            }
            if let Some(v) = g.get_i32("V3RocketPauseFrames") {
                out.v3.pause_frames = v.max(0) as u32;
            }
            if let Some(v) = g.get_i32("V3RocketTiltFrames") {
                out.v3.tilt_frames = v.max(0) as u32;
            }
            if let Some(v) = g.get_i32("DMislPauseFrames") {
                out.dmisl.pause_frames = v.max(0) as u32;
                // Every non-V3 family reads the DMisl slots in the manager's
                // state-1 timer; keep CMisl's copy in lockstep so the timer is
                // sourced from one place.
                out.cmisl.pause_frames = v.max(0) as u32;
            }
            if let Some(v) = g.get_i32("DMislTiltFrames") {
                out.dmisl.tilt_frames = v.max(0) as u32;
                out.cmisl.tilt_frames = v.max(0) as u32;
            }
        }

        if let Some(c) = combat_damage {
            read_damage(c, "V3RocketDamage", &mut out.v3.damage);
            read_damage(c, "V3RocketEliteDamage", &mut out.v3.elite_damage);
            read_damage(c, "DMislDamage", &mut out.dmisl.damage);
            read_damage(c, "DMislEliteDamage", &mut out.dmisl.elite_damage);
            read_damage(c, "CMislDamage", &mut out.cmisl.damage);
            read_damage(c, "CMislEliteDamage", &mut out.cmisl.elite_damage);
            read_into(c, "V3Warhead", &mut out.v3.warhead);
            read_into(c, "V3EliteWarhead", &mut out.v3.elite_warhead);
            read_into(c, "DMislWarhead", &mut out.dmisl.warhead);
            read_into(c, "DMislEliteWarhead", &mut out.dmisl.elite_warhead);
            read_into(c, "CMislWarhead", &mut out.cmisl.warhead);
            read_into(c, "CMislEliteWarhead", &mut out.cmisl.elite_warhead);
        }

        // The damage keys live in `[General]` in stock rulesmd.ini even though
        // the warheads live in `[CombatDamage]`. Read them from `[General]`
        // too so either placement resolves.
        if let Some(g) = general {
            read_damage(g, "V3RocketDamage", &mut out.v3.damage);
            read_damage(g, "V3RocketEliteDamage", &mut out.v3.elite_damage);
            read_damage(g, "DMislDamage", &mut out.dmisl.damage);
            read_damage(g, "DMislEliteDamage", &mut out.dmisl.elite_damage);
            read_damage(g, "CMislDamage", &mut out.cmisl.damage);
            read_damage(g, "CMislEliteDamage", &mut out.cmisl.elite_damage);
        }

        out
    }

    /// Which hardcoded family (if any) this child TechnoType belongs to.
    ///
    /// This is the Rust stand-in for gamemd's three pointer-equality tests
    /// against `Rules+0x4E0 / +0x514 / +0x548`.
    pub fn family_of(&self, type_name: &str) -> Option<MissileFamily> {
        if type_name.eq_ignore_ascii_case(&self.v3.type_name) {
            Some(MissileFamily::V3Rocket)
        } else if type_name.eq_ignore_ascii_case(&self.dmisl.type_name) {
            Some(MissileFamily::DMisl)
        } else if type_name.eq_ignore_ascii_case(&self.cmisl.type_name) {
            Some(MissileFamily::CMisl)
        } else {
            None
        }
    }

    /// Launch/impact parameters for a family.
    pub fn params(&self, family: MissileFamily) -> &MissileSpawnParams {
        match family {
            MissileFamily::V3Rocket => &self.v3,
            MissileFamily::DMisl => &self.dmisl,
            MissileFamily::CMisl => &self.cmisl,
        }
    }

    /// Frames a missile slot waits in the post-launch `KamikazeWait` state.
    ///
    /// `SpawnManagerClass::AI` reads the V3 pause+tilt slots only when the
    /// manager's spawn type is the V3 rocket type; every other family reads
    /// the DMisl slots. `MissileSpawnRules` keeps CMisl's copy of those two
    /// fields equal to DMisl's, so this is a straight lookup.
    pub fn kamikaze_wait_frames(&self, family: MissileFamily) -> u32 {
        let p = self.params(family);
        p.pause_frames.saturating_add(p.tilt_frames)
    }
}

fn read_name(section: &IniSection, key: &str) -> Option<String> {
    section
        .get(key)
        .map(|s| s.trim().to_ascii_uppercase())
        .filter(|s| !s.is_empty())
}

fn read_into(section: &IniSection, key: &str, dest: &mut String) {
    if let Some(v) = section.get(key).map(|s| s.trim()).filter(|s| !s.is_empty()) {
        *dest = v.to_string();
    }
}

fn read_damage(section: &IniSection, key: &str, dest: &mut i32) {
    if let Some(v) = section.get_i32(key) {
        *dest = v.max(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::ini_parser::IniFile;

    fn parse(text: &str) -> MissileSpawnRules {
        let ini = IniFile::from_str(text);
        MissileSpawnRules::from_ini_sections(ini.section("General"), ini.section("CombatDamage"))
    }

    #[test]
    fn defaults_match_retail_families() {
        let rules = MissileSpawnRules::default();
        assert_eq!(rules.family_of("V3ROCKET"), Some(MissileFamily::V3Rocket));
        assert_eq!(rules.family_of("DMISL"), Some(MissileFamily::DMisl));
        assert_eq!(rules.family_of("CMISL"), Some(MissileFamily::CMisl));
        assert_eq!(rules.family_of("HORNET"), None);
    }

    #[test]
    fn parses_general_and_combat_damage() {
        let rules = parse(
            "[General]\n\
             V3RocketType=V3ROCKET\n\
             V3RocketPauseFrames=0\n\
             V3RocketTiltFrames=60\n\
             V3RocketDamage=200\n\
             V3RocketEliteDamage=400\n\
             DMislType=DMISL\n\
             DMislPauseFrames=20\n\
             DMislTiltFrames=60\n\
             DMislDamage=300\n\
             CMislType=CMISL\n\
             \n\
             [CombatDamage]\n\
             V3Warhead=V3WH\n\
             V3EliteWarhead=V3EWH\n\
             DMislWarhead=DMISLWH\n",
        );
        assert_eq!(rules.v3.damage, 200);
        assert_eq!(rules.v3.elite_damage, 400);
        assert_eq!(rules.v3.warhead, "V3WH");
        assert_eq!(rules.v3.warhead_for(200), "V3EWH");
        assert_eq!(rules.v3.damage_for(100), 200);
        assert_eq!(rules.v3.damage_for(200), 400);
        assert_eq!(rules.dmisl.damage, 300);
        assert_eq!(rules.dmisl.warhead, "DMISLWH");
    }

    #[test]
    fn kamikaze_wait_uses_dmisl_frames_for_cmisl() {
        // gamemd's state-1 timer reads the DMisl pause/tilt slots for every
        // family except V3 — including CMisl, whose own CMisl*Frames keys are
        // never consulted there.
        let rules = parse(
            "[General]\n\
             V3RocketPauseFrames=0\n\
             V3RocketTiltFrames=60\n\
             DMislPauseFrames=20\n\
             DMislTiltFrames=60\n\
             CMislPauseFrames=20\n\
             CMislTiltFrames=100\n",
        );
        assert_eq!(rules.kamikaze_wait_frames(MissileFamily::V3Rocket), 60);
        assert_eq!(rules.kamikaze_wait_frames(MissileFamily::DMisl), 80);
        assert_eq!(rules.kamikaze_wait_frames(MissileFamily::CMisl), 80);
    }

    #[test]
    fn custom_type_names_reroute_the_family_test() {
        let rules = parse("[General]\nV3RocketType=MYROCKET\n");
        assert_eq!(rules.family_of("MYROCKET"), Some(MissileFamily::V3Rocket));
        assert_eq!(rules.family_of("V3ROCKET"), None);
    }
}
