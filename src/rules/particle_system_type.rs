//! ParticleSystemType — container that owns particles, manages spawning, and dispatches AI.
//!
//! Each `[ParticleSystemName]` section in rulesmd.ini defines one ParticleSystemType.
//! A `ParticleSystem` (runtime instance) is created via `Simulation::spawn_particle_system`
//! by combat, damage events, refinery dumps, area damage, gap generators, and triggers.
//!
//! ## Dependency rules
//! - Part of rules/ — no dependencies on sim/, render/, ui/, etc.
//! - References `ParticleTypeId` from `crate::rules::particle_type`.

use serde::{Deserialize, Serialize};

/// Interned identifier for a `ParticleSystemType`. Resolved at INI parse time;
/// consumers (TechnoType, WeaponType, RulesClass) store the ID, not the name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub struct ParticleSystemTypeId(pub u32);

/// System-level behavior dispatch enum.
///
/// Variant ordering matches the binary's string table:
/// `Smoke=0, Gas=1, Fire=2, Spark=3, Railgun=4`. This is **different** from
/// `ParticleBehavesLike` — Smoke and Gas are swapped at the particle level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum ParticleSystemBehavesLike {
    Smoke = 0,
    Gas = 1,
    Fire = 2,
    Spark = 3,
    Railgun = 4,
}

impl ParticleSystemBehavesLike {
    /// Parse a `BehavesLike=` value from INI. Returns `None` for unknown strings.
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim() {
            "Smoke" => Some(Self::Smoke),
            "Gas" => Some(Self::Gas),
            "Fire" => Some(Self::Fire),
            "Spark" => Some(Self::Spark),
            "Railgun" => Some(Self::Railgun),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn behaves_like_string_to_enum() {
        assert_eq!(ParticleSystemBehavesLike::parse("Smoke"), Some(ParticleSystemBehavesLike::Smoke));
        assert_eq!(ParticleSystemBehavesLike::parse("Gas"), Some(ParticleSystemBehavesLike::Gas));
        assert_eq!(ParticleSystemBehavesLike::parse("Fire"), Some(ParticleSystemBehavesLike::Fire));
        assert_eq!(ParticleSystemBehavesLike::parse("Spark"), Some(ParticleSystemBehavesLike::Spark));
        assert_eq!(ParticleSystemBehavesLike::parse("Railgun"), Some(ParticleSystemBehavesLike::Railgun));
        assert_eq!(ParticleSystemBehavesLike::parse("nope"), None);
    }

    #[test]
    fn behaves_like_parse_trims_whitespace() {
        assert_eq!(ParticleSystemBehavesLike::parse("  Smoke  "), Some(ParticleSystemBehavesLike::Smoke));
    }

    #[test]
    fn behaves_like_discriminants_match_binary() {
        // System-level enum: Smoke=0, Gas=1 (NOT Gas=0 like the particle-level enum).
        assert_eq!(ParticleSystemBehavesLike::Smoke as u8, 0);
        assert_eq!(ParticleSystemBehavesLike::Gas as u8, 1);
        assert_eq!(ParticleSystemBehavesLike::Fire as u8, 2);
        assert_eq!(ParticleSystemBehavesLike::Spark as u8, 3);
        assert_eq!(ParticleSystemBehavesLike::Railgun as u8, 4);
    }

    #[test]
    fn system_and_particle_enum_have_swapped_smoke_gas() {
        // Critical asymmetry: at the SYSTEM level Smoke=0/Gas=1, but at the PARTICLE
        // level Gas=0/Smoke=1. Mismatching them is the kind of bug that produces
        // "smoke deals damage / gas drifts silently" symptoms.
        use crate::rules::particle_type::ParticleBehavesLike;
        assert_ne!(
            ParticleSystemBehavesLike::Smoke as u8,
            ParticleBehavesLike::Smoke as u8
        );
        assert_ne!(
            ParticleSystemBehavesLike::Gas as u8,
            ParticleBehavesLike::Gas as u8
        );
    }

    #[test]
    fn particle_system_type_id_is_copy_eq_hash() {
        let a = ParticleSystemTypeId(13);
        let b = a;
        assert_eq!(a, b);
        let mut set = std::collections::HashSet::new();
        set.insert(a);
        assert!(set.contains(&b));
    }
}
