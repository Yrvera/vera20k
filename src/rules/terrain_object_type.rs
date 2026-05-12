//! Parsing for `[TIBTRE*]`-style terrain object types (rules.ini sections).
//!
//! Distinct from `terrain_rules` (which parses LAND types like Clear/Rough/Water).
//! These are per-object-type definitions for terrain decorations — currently
//! only TIBTRE (Tiberium Tree) is consumed by sim. Other terrain objects
//! (TREE01, ROCK01, etc.) parse to the same struct but have all-default flags
//! and are ignored by the spawner system.

use crate::rules::ini_parser::IniSection;

/// Type-class data for a terrain object (e.g. `[TIBTRE01]`).
///
/// Only the fields the sim needs; render-only fields (LightVisibility, tints,
/// IsFlammable) are intentionally not parsed.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TerrainObjectType {
    /// Section name, e.g. "TIBTRE01".
    pub name: String,
    /// `SpawnsTiberium=yes` — periodically spawns ore in adjacent cells.
    pub spawns_tiberium: bool,
    /// `IsAnimated=yes` — required gate for SpawnsTiberium logic.
    pub is_animated: bool,
    /// `AnimationRate=` (frames per anim step). Currently parsed but unused
    /// in sim — animation timing is collapsed to single-phase. Kept for
    /// future render-side use and to surface mod-tuning differences.
    pub animation_rate: u8,
    /// `AnimationProbability=` × 1_000_000, stored as integer micros.
    /// Used directly in the sim tick: `rng.next_range_u32(1_000_000) < this`.
    /// Avoids f32 in the hot path.
    pub animation_probability_micros: u32,
}

impl TerrainObjectType {
    pub fn from_ini_section(name: &str, section: &IniSection) -> Self {
        let probability_f = section
            .get("AnimationProbability")
            .and_then(|s| s.trim().parse::<f32>().ok())
            .unwrap_or(0.0);
        let animation_probability_micros: u32 =
            (probability_f.clamp(0.0, 1.0) * 1_000_000.0).round() as u32;

        Self {
            name: name.to_string(),
            spawns_tiberium: section.get_bool("SpawnsTiberium").unwrap_or(false),
            is_animated: section.get_bool("IsAnimated").unwrap_or(false),
            animation_rate: section.get_i32("AnimationRate").unwrap_or(0).clamp(0, 255) as u8,
            animation_probability_micros,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::ini_parser::IniFile;

    #[test]
    fn parse_tibtre_section_defaults() {
        let ini = IniFile::from_str(
            "[TIBTRE01]\nSpawnsTiberium=yes\nIsAnimated=yes\n\
             AnimationRate=3\nAnimationProbability=.003\n",
        );
        let section = ini.section("TIBTRE01").expect("section");
        let t = TerrainObjectType::from_ini_section("TIBTRE01", section);
        assert_eq!(t.name, "TIBTRE01");
        assert!(t.spawns_tiberium);
        assert!(t.is_animated);
        assert_eq!(t.animation_rate, 3);
        assert_eq!(t.animation_probability_micros, 3000);
    }

    #[test]
    fn parse_non_spawning_terrain_section() {
        let ini = IniFile::from_str("[TREE01]\nIsAnimated=no\n");
        let section = ini.section("TREE01").expect("section");
        let t = TerrainObjectType::from_ini_section("TREE01", section);
        assert!(!t.spawns_tiberium);
        assert!(!t.is_animated);
        assert_eq!(t.animation_probability_micros, 0);
    }

    #[test]
    fn animation_probability_clamps_above_one() {
        let ini = IniFile::from_str("[X]\nAnimationProbability=2.5\n");
        let section = ini.section("X").expect("section");
        let t = TerrainObjectType::from_ini_section("X", section);
        assert_eq!(t.animation_probability_micros, 1_000_000);
    }
}
