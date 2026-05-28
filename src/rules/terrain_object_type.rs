//! Parsing for `[TIBTRE*]`-style terrain object types (rules.ini sections).
//!
//! Distinct from `terrain_rules` (which parses LAND types like Clear/Rough/Water).
//! These are per-object-type definitions for terrain decorations — currently
//! only TIBTRE (Tiberium Tree) is consumed by sim. Other terrain objects
//! (TREE01, ROCK01, etc.) parse to the same struct but have all-default flags
//! and are ignored by the spawner system.

use crate::rules::foundation;
use crate::rules::ini_parser::IniSection;

const DEFAULT_TREE_STRENGTH: i32 = 200;

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
    /// Inherited `Armor=`. TerrainTypeClass constructor defaults to Wood.
    pub armor: String,
    /// Inherited `Strength=`. Missing value resolves through `[General] TreeStrength`.
    pub strength: i32,
    /// Inherited `Immune=`. Stock TIBTRE sets this, blocking normal terrain damage.
    pub immune: bool,
    /// Inherited `LegalTarget=`. `IsVeinhole=yes` force-enables it in gamemd.
    pub legal_target: bool,
    /// Inherited `Insignificant=`. TerrainTypeClass constructor defaults true.
    pub insignificant: bool,
    /// Inherited `RadarInvisible=`. TerrainTypeClass constructor defaults true.
    pub radar_invisible: bool,
    /// `WaterBound=`.
    pub water_bound: bool,
    /// `IsVeinhole=`; when true, gamemd also forces `LegalTarget=true`.
    pub is_veinhole: bool,
    /// `TemperateOccupationBits=`.
    pub temperate_occupation_bits: u8,
    /// `SnowOccupationBits=`.
    pub snow_occupation_bits: u8,
    /// Art `Foundation=`. Defaults to 1x1 until art.ini merge patches it.
    pub foundation: String,
}

impl TerrainObjectType {
    pub fn from_ini_section(name: &str, section: &IniSection) -> Self {
        Self::from_ini_section_with_tree_strength(name, section, DEFAULT_TREE_STRENGTH)
    }

    pub fn from_ini_section_with_tree_strength(
        name: &str,
        section: &IniSection,
        tree_strength: i32,
    ) -> Self {
        let probability_f = section
            .get("AnimationProbability")
            .and_then(|s| s.trim().parse::<f32>().ok())
            .unwrap_or(0.0);
        let animation_probability_micros: u32 =
            (probability_f.clamp(0.0, 1.0) * 1_000_000.0).round() as u32;
        let is_veinhole = section.get_bool("IsVeinhole").unwrap_or(false);

        Self {
            name: name.to_string(),
            spawns_tiberium: section.get_bool("SpawnsTiberium").unwrap_or(false),
            is_animated: section.get_bool("IsAnimated").unwrap_or(false),
            animation_rate: section.get_i32("AnimationRate").unwrap_or(0).clamp(0, 255) as u8,
            animation_probability_micros,
            armor: section.get("Armor").unwrap_or("wood").to_ascii_lowercase(),
            strength: section.get_i32("Strength").unwrap_or(tree_strength).max(1),
            immune: section.get_bool("Immune").unwrap_or(false),
            legal_target: section.get_bool("LegalTarget").unwrap_or(false) || is_veinhole,
            insignificant: section.get_bool("Insignificant").unwrap_or(true),
            radar_invisible: section.get_bool("RadarInvisible").unwrap_or(true),
            water_bound: section.get_bool("WaterBound").unwrap_or(false),
            is_veinhole,
            temperate_occupation_bits: section
                .get_i32("TemperateOccupationBits")
                .unwrap_or(7)
                .clamp(0, 7) as u8,
            snow_occupation_bits: section
                .get_i32("SnowOccupationBits")
                .unwrap_or(7)
                .clamp(0, 7) as u8,
            foundation: foundation::foundation_name("1x1").to_string(),
        }
    }

    pub fn merge_art_foundation(&mut self, foundation_name: &str) {
        self.foundation = foundation::foundation_name(foundation_name).to_string();
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
        assert_eq!(t.armor, "wood");
        assert_eq!(t.strength, 200);
        assert!(!t.immune);
        assert!(!t.legal_target);
        assert!(t.insignificant);
        assert!(t.radar_invisible);
        assert!(!t.water_bound);
        assert!(!t.is_veinhole);
        assert_eq!(t.temperate_occupation_bits, 7);
        assert_eq!(t.snow_occupation_bits, 7);
        assert_eq!(t.foundation, "1x1");
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

    #[test]
    fn parse_tibtre_lifecycle_fields_and_tree_strength_fallback() {
        let ini = IniFile::from_str(
            "[TIBTRE03]\nSpawnsTiberium=yes\nIsAnimated=yes\nImmune=yes\n\
             Armor=None\nIsVeinhole=true\nTemperateOccupationBits=4\nSnowOccupationBits=7\n",
        );
        let section = ini.section("TIBTRE03").expect("section");
        let t = TerrainObjectType::from_ini_section_with_tree_strength("TIBTRE03", section, 375);

        assert_eq!(t.strength, 375);
        assert_eq!(t.armor, "none");
        assert!(t.immune);
        assert!(t.is_veinhole);
        assert!(t.legal_target);
        assert_eq!(t.temperate_occupation_bits, 4);
        assert_eq!(t.snow_occupation_bits, 7);
    }

    #[test]
    fn art_foundation_is_normalized() {
        let ini = IniFile::from_str("[TREE01]\n");
        let section = ini.section("TREE01").expect("section");
        let mut t = TerrainObjectType::from_ini_section("TREE01", section);
        t.merge_art_foundation("2x2");
        assert_eq!(t.foundation, "2x2");
    }
}
