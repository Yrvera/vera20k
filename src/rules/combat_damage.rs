//! `[CombatDamage]` section parser — global default particle systems used
//! by various combat effects (smoke plumes, sparks, fire streams, debris).
//!
//! Fields hold unresolved particle-system section names; ID resolution
//! against the particle-system registry is deferred (matches the same
//! pattern used by ParticleType.warhead, ParticleSystemType.holds_what,
//! ObjectType.damage_particle_systems, and GeneralRules.barrel_particle).
//!
//! The 9 fields below mirror the fixed RulesClass slots at +0x1018..+0x1038;
//! retail rulesmd.ini ships a 10th key (`DefaultFirestormExplosionSystem=`)
//! that is not present in the verified RulesClass::ReadCombatDamage layout,
//! so we don't parse it.
//!
//! ## Dependency rules
//! - Part of rules/ — no dependencies on sim/, render/, ui/, etc.

use crate::rules::ini_parser::IniSection;

/// Default particle-system fallbacks read from `[CombatDamage]`.
///
/// Each field is the section name of a `ParticleSystemType` (resolved later
/// against `RuleSet::ps_type_id_by_name`). `None` means the key was absent
/// or empty — consumers must supply their own fallback in that case.
#[derive(Debug, Clone, Default)]
pub struct CombatDamageDefaults {
    /// Large grey smoke plume — buildings under heavy damage.
    pub default_large_grey_smoke_system: Option<String>,
    /// Small grey smoke plume.
    pub default_small_grey_smoke_system: Option<String>,
    /// Spark shower — used by capture / warp-attach / electric bolt impact.
    pub default_spark_system: Option<String>,
    /// Large red smoke plume.
    pub default_large_red_smoke_system: Option<String>,
    /// Small red smoke plume.
    pub default_small_red_smoke_system: Option<String>,
    /// Debris dust kicked up when wreckage hits the ground.
    pub default_debris_smoke_system: Option<String>,
    /// Flamethrower fire stream particle system.
    pub default_fire_stream_system: Option<String>,
    /// Hidden test particle system — never used in retail YR.
    pub default_test_particle_system: Option<String>,
    /// Sparks emitted when a unit gets repaired by a service depot.
    pub default_repair_particle_system: Option<String>,
}

impl CombatDamageDefaults {
    /// Parse from a `[CombatDamage]` `IniSection`. Missing keys become `None`.
    pub fn from_ini_section(section: &IniSection) -> Self {
        Self {
            default_large_grey_smoke_system: read_psname(section, "DefaultLargeGreySmokeSystem"),
            default_small_grey_smoke_system: read_psname(section, "DefaultSmallGreySmokeSystem"),
            default_spark_system: read_psname(section, "DefaultSparkSystem"),
            default_large_red_smoke_system: read_psname(section, "DefaultLargeRedSmokeSystem"),
            default_small_red_smoke_system: read_psname(section, "DefaultSmallRedSmokeSystem"),
            default_debris_smoke_system: read_psname(section, "DefaultDebrisSmokeSystem"),
            default_fire_stream_system: read_psname(section, "DefaultFireStreamSystem"),
            default_test_particle_system: read_psname(section, "DefaultTestParticleSystem"),
            default_repair_particle_system: read_psname(section, "DefaultRepairParticleSystem"),
        }
    }
}

fn read_psname(section: &IniSection, key: &str) -> Option<String> {
    section
        .get(key)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::ini_parser::IniFile;

    #[test]
    fn parses_full_combat_damage_section() {
        let ini = IniFile::from_str(
            "[CombatDamage]\n\
             DefaultLargeGreySmokeSystem=BigGreySmokeSys\n\
             DefaultSmallGreySmokeSystem=SmallGreySSys\n\
             DefaultSparkSystem=SparkSys\n\
             DefaultLargeRedSmokeSystem=BigGreySmokeSys\n\
             DefaultSmallRedSmokeSystem=SmallGreySSys\n\
             DefaultDebrisSmokeSystem=SmallGreySSys\n\
             DefaultFireStreamSystem=FireStreamSys\n\
             DefaultTestParticleSystem=TestSmokeSys\n\
             DefaultRepairParticleSystem=WeldingSys\n",
        );
        let section = ini.section("CombatDamage").unwrap();
        let cd = CombatDamageDefaults::from_ini_section(section);

        assert_eq!(
            cd.default_large_grey_smoke_system.as_deref(),
            Some("BigGreySmokeSys")
        );
        assert_eq!(
            cd.default_small_grey_smoke_system.as_deref(),
            Some("SmallGreySSys")
        );
        assert_eq!(cd.default_spark_system.as_deref(), Some("SparkSys"));
        assert_eq!(
            cd.default_large_red_smoke_system.as_deref(),
            Some("BigGreySmokeSys")
        );
        assert_eq!(
            cd.default_small_red_smoke_system.as_deref(),
            Some("SmallGreySSys")
        );
        assert_eq!(
            cd.default_debris_smoke_system.as_deref(),
            Some("SmallGreySSys")
        );
        assert_eq!(
            cd.default_fire_stream_system.as_deref(),
            Some("FireStreamSys")
        );
        assert_eq!(
            cd.default_test_particle_system.as_deref(),
            Some("TestSmokeSys")
        );
        assert_eq!(
            cd.default_repair_particle_system.as_deref(),
            Some("WeldingSys")
        );
    }

    #[test]
    fn empty_section_yields_all_none() {
        let ini = IniFile::from_str("[CombatDamage]\n");
        let section = ini.section("CombatDamage").unwrap();
        let cd = CombatDamageDefaults::from_ini_section(section);
        assert!(cd.default_large_grey_smoke_system.is_none());
        assert!(cd.default_spark_system.is_none());
        assert!(cd.default_fire_stream_system.is_none());
        assert!(cd.default_repair_particle_system.is_none());
    }

    #[test]
    fn whitespace_only_value_treated_as_none() {
        let ini = IniFile::from_str("[CombatDamage]\nDefaultSparkSystem=   \n");
        let section = ini.section("CombatDamage").unwrap();
        let cd = CombatDamageDefaults::from_ini_section(section);
        assert!(cd.default_spark_system.is_none());
    }

    #[test]
    fn trims_whitespace_around_value() {
        let ini = IniFile::from_str("[CombatDamage]\nDefaultSparkSystem=  SparkSys  \n");
        let section = ini.section("CombatDamage").unwrap();
        let cd = CombatDamageDefaults::from_ini_section(section);
        assert_eq!(cd.default_spark_system.as_deref(), Some("SparkSys"));
    }
}
