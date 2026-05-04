//! ParticleType — runtime parameters for a single particle (gas, smoke, fire, spark, railgun).
//!
//! Each `[ParticleName]` section in rulesmd.ini defines one ParticleType. Particles are
//! the leaf entity inside a `ParticleSystem`: they carry position, lifetime, animation
//! state, and (for Gas/Fire) deal damage to objects in their cell.
//!
//! ## Dependency rules
//! - Part of rules/ — no dependencies on sim/, render/, ui/, etc.

use serde::{Deserialize, Serialize};

/// Interned identifier for a `ParticleType`. Resolved at INI parse time;
/// cross-references between types (e.g., `NextParticle=`) store the ID, not the name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub struct ParticleTypeId(pub u32);

/// Per-particle behavior dispatch enum.
///
/// Variant ordering matches the binary's string table:
/// `Gas=0, Smoke=1, Fire=2, Spark=3, Railgun=4`. This is **different** from
/// `ParticleSystemBehavesLike` — Gas and Smoke are swapped at the system level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum ParticleBehavesLike {
    Gas = 0,
    Smoke = 1,
    Fire = 2,
    Spark = 3,
    Railgun = 4,
}

impl ParticleBehavesLike {
    /// Parse a `BehavesLike=` value from INI. Returns `None` for unknown strings.
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim() {
            "Gas" => Some(Self::Gas),
            "Smoke" => Some(Self::Smoke),
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
        assert_eq!(ParticleBehavesLike::parse("Gas"), Some(ParticleBehavesLike::Gas));
        assert_eq!(ParticleBehavesLike::parse("Smoke"), Some(ParticleBehavesLike::Smoke));
        assert_eq!(ParticleBehavesLike::parse("Fire"), Some(ParticleBehavesLike::Fire));
        assert_eq!(ParticleBehavesLike::parse("Spark"), Some(ParticleBehavesLike::Spark));
        assert_eq!(ParticleBehavesLike::parse("Railgun"), Some(ParticleBehavesLike::Railgun));
        assert_eq!(ParticleBehavesLike::parse("nope"), None);
    }

    #[test]
    fn behaves_like_parse_trims_whitespace() {
        assert_eq!(ParticleBehavesLike::parse("  Gas  "), Some(ParticleBehavesLike::Gas));
    }

    #[test]
    fn behaves_like_discriminants_match_binary() {
        // Binary indexes string-table by enum value; preserve the exact ordering.
        assert_eq!(ParticleBehavesLike::Gas as u8, 0);
        assert_eq!(ParticleBehavesLike::Smoke as u8, 1);
        assert_eq!(ParticleBehavesLike::Fire as u8, 2);
        assert_eq!(ParticleBehavesLike::Spark as u8, 3);
        assert_eq!(ParticleBehavesLike::Railgun as u8, 4);
    }

    #[test]
    fn particle_type_id_is_copy_eq_hash() {
        let a = ParticleTypeId(7);
        let b = a;
        assert_eq!(a, b);
        let mut set = std::collections::HashSet::new();
        set.insert(a);
        assert!(set.contains(&b));
    }
}
