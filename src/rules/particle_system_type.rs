//! ParticleSystemType — container that owns particles, manages spawning, and dispatches AI.
//!
//! Each `[ParticleSystemName]` section in rulesmd.ini defines one ParticleSystemType.
//! A `ParticleSystem` (runtime instance) is created via `Simulation::spawn_particle_system`
//! by combat, damage events, refinery dumps, area damage, gap generators, and triggers.
//!
//! ## Dependency rules
//! - Part of rules/ — no dependencies on sim/, render/, ui/, etc.
//! - References `ParticleTypeId` from `crate::rules::particle_type`.

use glam::IVec3;
use serde::{Deserialize, Serialize};

use crate::rules::ini_parser::IniSection;
use crate::rules::particle_type::ParticleTypeId;
use crate::util::fixed_math::{SIM_ZERO, SimFixed, sim_from_f32};

/// Default `ParticlesPerCoord` from the constructor (Railgun field, parsed
/// for every system).
const DEFAULT_PARTICLES_PER_COORD: SimFixed = SimFixed::lit("0.1");
/// Default `SpiralDeltaPerCoord` from the constructor.
const DEFAULT_SPIRAL_DELTA_PER_COORD: SimFixed = SimFixed::lit("0.025");
/// Default `SpiralRadius` from the constructor.
const DEFAULT_SPIRAL_RADIUS: SimFixed = SimFixed::lit("25");

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

/// A particle system definition parsed from a `[ParticleSystemName]` section.
///
/// Tier 3 fields (Railgun spiral params, Spark percentages, laser color) are
/// parsed but unused at Tier 2 — the binary parses them unconditionally and
/// we mirror that so Tier 3 lights up cleanly later.
#[derive(Debug, Clone)]
pub struct ParticleSystemType {
    // ── Identity ─────────────────────────────────────────────────────
    /// Section name in rulesmd.ini (e.g., "BigGreySmokeSys").
    pub name: String,
    /// System-level behavior dispatch — selects the per-tick spawn / AI branch.
    pub behaves_like: ParticleSystemBehavesLike,

    // ── Core spawn / lifetime ────────────────────────────────────────
    /// Resolved `HoldsWhat` reference (the particle this system spawns).
    /// Always `None` after A3; the 2-pass resolver in Task A4 fills it in.
    pub holds_what: Option<ParticleTypeId>,
    /// Whether this system spawns new particles over time (vs. a one-shot).
    pub spawns: bool,
    /// Frame interval between spawns (default 1).
    pub spawn_frames: u32,
    /// Rate at which smoke particles decelerate (default 0.0).
    pub slowdown: SimFixed,
    /// Maximum particles this system can hold simultaneously (default 50).
    pub particle_cap: u32,
    /// Random radius for spawn position offset.
    pub spawn_radius: i32,
    /// Distance threshold to stop spawning new particles.
    pub spawn_cutoff: SimFixed,
    /// Distance threshold to start fading new particles.
    pub spawn_translucency_cutoff: SimFixed,
    /// System lifetime in frames. -1 = infinite (system stays alive until all
    /// particles die). Default -1 per constructor.
    pub lifetime: i32,
    /// Direction vector for spawning (CoordStruct).
    pub spawn_direction: IVec3,

    // ── Railgun-only (Tier 3 — parsed but unused) ────────────────────
    /// Particles spawned per coordinate unit along a railgun beam.
    pub particles_per_coord: SimFixed,
    /// Railgun spiral angle increment per coord.
    pub spiral_delta_per_coord: SimFixed,
    /// Radius of the railgun spiral pattern.
    pub spiral_radius: SimFixed,
    /// Random position offset scale.
    pub position_perturbation_coefficient: SimFixed,
    /// Random movement offset scale.
    pub movement_perturbation_coefficient: SimFixed,
    /// Random velocity perturbation scale.
    pub velocity_perturbation_coefficient: SimFixed,

    // ── Spark-only (Tier 3 — parsed but unused) ──────────────────────
    /// Probability of spawning a spark each tick.
    pub spawn_spark_percentage: SimFixed,
    /// Spark spawn frame counter.
    pub spark_spawn_frames: u32,
    /// Light radius for spark systems.
    pub light_size: i32,
    /// Spark light only lasts one frame.
    pub one_frame_light: bool,
    /// Whether to draw a railgun laser line.
    pub laser: bool,
    /// Railgun laser beam color.
    pub laser_color: [u8; 3],
}

impl ParticleSystemType {
    /// Parse a ParticleSystemType from an INI section.
    ///
    /// `holds_what` is always `None` here — the referenced ParticleType may
    /// not be parsed yet. Task A4 introduces a `Pending` parse-state struct
    /// that captures the name string for resolution in a second pass.
    pub fn from_ini_section(name: &str, section: &IniSection) -> Self {
        let behaves_like = section
            .get("BehavesLike")
            .and_then(ParticleSystemBehavesLike::parse)
            // Binary's string-table loop falls through to index 0 (Smoke) when
            // the INI string doesn't match any entry.
            .unwrap_or(ParticleSystemBehavesLike::Smoke);

        Self {
            name: name.to_string(),
            behaves_like,
            holds_what: None,
            spawns: section.get_bool("Spawns").unwrap_or(false),
            spawn_frames: section
                .get_i32("SpawnFrames")
                .map(|n| n.max(0) as u32)
                .unwrap_or(1),
            slowdown: section
                .get_f32("Slowdown")
                .map(sim_from_f32)
                .unwrap_or(SIM_ZERO),
            particle_cap: section
                .get_i32("ParticleCap")
                .map(|n| n.max(0) as u32)
                .unwrap_or(50),
            spawn_radius: section.get_i32("SpawnRadius").unwrap_or(0),
            spawn_cutoff: section
                .get_f32("SpawnCutoff")
                .map(sim_from_f32)
                .unwrap_or(SIM_ZERO),
            spawn_translucency_cutoff: section
                .get_f32("SpawnTranslucencyCutoff")
                .map(sim_from_f32)
                .unwrap_or(SIM_ZERO),
            lifetime: section.get_i32("Lifetime").unwrap_or(-1),
            spawn_direction: section
                .get("SpawnDirection")
                .map(parse_coord_offset)
                .unwrap_or(IVec3::ZERO),

            particles_per_coord: section
                .get_f32("ParticlesPerCoord")
                .map(sim_from_f32)
                .unwrap_or(DEFAULT_PARTICLES_PER_COORD),
            spiral_delta_per_coord: section
                .get_f32("SpiralDeltaPerCoord")
                .map(sim_from_f32)
                .unwrap_or(DEFAULT_SPIRAL_DELTA_PER_COORD),
            spiral_radius: section
                .get_f32("SpiralRadius")
                .map(sim_from_f32)
                .unwrap_or(DEFAULT_SPIRAL_RADIUS),
            position_perturbation_coefficient: section
                .get_f32("PositionPerturbationCoefficient")
                .map(sim_from_f32)
                .unwrap_or(SIM_ZERO),
            movement_perturbation_coefficient: section
                .get_f32("MovementPerturbationCoefficient")
                .map(sim_from_f32)
                .unwrap_or(SIM_ZERO),
            velocity_perturbation_coefficient: section
                .get_f32("VelocityPerturbationCoefficient")
                .map(sim_from_f32)
                .unwrap_or(SIM_ZERO),

            spawn_spark_percentage: section
                .get_f32("SpawnSparkPercentage")
                .map(sim_from_f32)
                .unwrap_or(SIM_ZERO),
            spark_spawn_frames: section
                .get_i32("SparkSpawnFrames")
                .map(|n| n.max(0) as u32)
                .unwrap_or(0),
            light_size: section.get_i32("LightSize").unwrap_or(0),
            one_frame_light: section.get_bool("OneFrameLight").unwrap_or(false),
            laser: section.get_bool("Laser").unwrap_or(false),
            laser_color: section
                .get("LaserColor")
                .map(parse_rgb_color)
                .unwrap_or([0, 0, 0]),
        }
    }
}

/// Parse an `R,G,B` color string into `[u8; 3]`. Components clamp to 0–255;
/// returns `[0, 0, 0]` if fewer than 3 numbers parse.
fn parse_rgb_color(raw: &str) -> [u8; 3] {
    let parts: Vec<&str> = raw.split(',').map(|s| s.trim()).collect();
    if parts.len() >= 3 {
        let r = parts[0].parse::<u8>().unwrap_or(0);
        let g = parts[1].parse::<u8>().unwrap_or(0);
        let b = parts[2].parse::<u8>().unwrap_or(0);
        [r, g, b]
    } else {
        [0, 0, 0]
    }
}

/// Parse an `X,Y,Z` coordinate offset (CoordStruct) into an `IVec3`.
/// Missing or unparseable components default to 0.
fn parse_coord_offset(raw: &str) -> IVec3 {
    let mut parts = raw.split(',').map(|s| s.trim());
    let x = parts.next().and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
    let y = parts.next().and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
    let z = parts.next().and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
    IVec3::new(x, y, z)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::ini_parser::IniFile;

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

    #[test]
    fn defaults_match_binary_constructor() {
        let ini = IniFile::from_str("[Foo]\n");
        let section = ini.section("Foo").unwrap();
        let pst = ParticleSystemType::from_ini_section("Foo", section);

        assert_eq!(pst.lifetime, -1);
        assert_eq!(pst.particle_cap, 50);
        assert_eq!(pst.spawn_frames, 1);
        assert_eq!(pst.particles_per_coord, SimFixed::lit("0.1"));
        assert_eq!(pst.spiral_delta_per_coord, SimFixed::lit("0.025"));
        assert_eq!(pst.spiral_radius, SimFixed::lit("25"));
        assert_eq!(pst.slowdown, SIM_ZERO);
        assert_eq!(pst.spawn_radius, 0);
        assert_eq!(pst.spawn_direction, IVec3::ZERO);
        assert!(!pst.spawns);
        assert!(!pst.laser);
        assert!(!pst.one_frame_light);
        assert_eq!(pst.laser_color, [0, 0, 0]);
        // Unknown / absent BehavesLike falls through to the index-0 entry (Smoke).
        assert_eq!(pst.behaves_like, ParticleSystemBehavesLike::Smoke);
    }

    #[test]
    fn from_ini_parses_smoke_system_like_section() {
        let ini = IniFile::from_str(
            "[BigGreySSys]\n\
             BehavesLike=Smoke\n\
             HoldsWhat=GreySmoke\n\
             Spawns=yes\n\
             SpawnFrames=18\n\
             ParticleCap=15\n\
             Slowdown=0.4\n\
             Lifetime=200\n",
        );
        let section = ini.section("BigGreySSys").unwrap();
        let pst = ParticleSystemType::from_ini_section("BigGreySSys", section);

        assert_eq!(pst.name, "BigGreySSys");
        assert_eq!(pst.behaves_like, ParticleSystemBehavesLike::Smoke);
        // HoldsWhat is unresolved at A3 — the string is captured and resolved in A4.
        assert_eq!(pst.holds_what, None);
        assert!(pst.spawns);
        assert_eq!(pst.spawn_frames, 18);
        assert_eq!(pst.particle_cap, 15);
        assert_eq!(pst.slowdown, sim_from_f32(0.4));
        assert_eq!(pst.lifetime, 200);
    }

    #[test]
    fn from_ini_parses_railgun_system() {
        let ini = IniFile::from_str(
            "[RGSys]\n\
             BehavesLike=Railgun\n\
             ParticlesPerCoord=0.5\n\
             SpiralDeltaPerCoord=0.05\n\
             SpiralRadius=12.0\n\
             Laser=yes\n\
             LaserColor=255,128,64\n",
        );
        let section = ini.section("RGSys").unwrap();
        let pst = ParticleSystemType::from_ini_section("RGSys", section);

        assert_eq!(pst.behaves_like, ParticleSystemBehavesLike::Railgun);
        assert_eq!(pst.particles_per_coord, sim_from_f32(0.5));
        assert_eq!(pst.spiral_delta_per_coord, sim_from_f32(0.05));
        assert_eq!(pst.spiral_radius, sim_from_f32(12.0));
        assert!(pst.laser);
        assert_eq!(pst.laser_color, [255, 128, 64]);
    }

    #[test]
    fn from_ini_parses_spawn_direction() {
        let ini = IniFile::from_str("[Foo]\nSpawnDirection=10,-5,42\n");
        let section = ini.section("Foo").unwrap();
        let pst = ParticleSystemType::from_ini_section("Foo", section);
        assert_eq!(pst.spawn_direction, IVec3::new(10, -5, 42));
    }

    #[test]
    fn from_ini_unknown_behaves_like_falls_through_to_smoke() {
        let ini = IniFile::from_str("[Foo]\nBehavesLike=Bogus\n");
        let section = ini.section("Foo").unwrap();
        let pst = ParticleSystemType::from_ini_section("Foo", section);
        assert_eq!(pst.behaves_like, ParticleSystemBehavesLike::Smoke);
    }
}
