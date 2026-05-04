//! ParticleType — runtime parameters for a single particle (gas, smoke, fire, spark, railgun).
//!
//! Each `[ParticleName]` section in rulesmd.ini defines one ParticleType. Particles are
//! the leaf entity inside a `ParticleSystem`: they carry position, lifetime, animation
//! state, and (for Gas/Fire) deal damage to objects in their cell.
//!
//! ## Dependency rules
//! - Part of rules/ — no dependencies on sim/, render/, ui/, etc.

use glam::IVec3;
use serde::{Deserialize, Serialize};

use crate::rules::ini_parser::IniSection;
use crate::util::fixed_math::{SIM_ZERO, SimFixed, sim_from_f32};

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

/// A particle definition parsed from a `[ParticleName]` section in rulesmd.ini.
///
/// Tier 3 fields (Spark/Railgun: `XVelocity`, `ColorList`, etc.) are parsed but
/// unused at Tier 2 — the binary parses them unconditionally and we mirror that
/// so Tier 3 lights up cleanly later without re-parsing.
#[derive(Debug, Clone)]
pub struct ParticleType {
    // ── Identity ─────────────────────────────────────────────────────
    /// Section name in rulesmd.ini (e.g., "GasCloud1", "Smoke").
    pub name: String,
    /// Per-particle behavior dispatch — selects the AI branch each tick.
    pub behaves_like: ParticleBehavesLike,

    // ── Object-base (from ObjectTypeClass) ───────────────────────────
    /// SHP image for animated particles. Resolved later via the SHP cache.
    pub image: Option<String>,

    // ── [Particles] direct keys (§2.2) ───────────────────────────────
    /// Damage countdown reset value: frames between damage ticks (Gas/Fire).
    pub max_dc: u16,
    /// Lifetime in frames (countdown decremented each tick).
    pub max_ec: u16,
    /// Damage amount per damage tick.
    pub damage: i32,
    /// Warhead reference name, resolved later against the warhead registry.
    pub warhead: Option<String>,
    /// Starting animation frame.
    pub start_frame: u16,
    /// Frames per animation loop.
    pub num_loop_frames: u16,
    /// Base translucency level: 0, 25, or 50.
    pub translucency: u8,
    /// Wind sensitivity: 0..=5.
    pub wind_effect: u8,
    /// Movement speed (per-tick).
    pub velocity: SimFixed,
    /// Per-frame deceleration applied to velocity.
    pub deacc: SimFixed,
    /// Interaction radius (cells).
    pub radius: i32,
    /// Delete the particle when its animation state hits the limit.
    pub delete_on_state_limit: bool,
    /// Final animation state — particle is marked for deletion at this state.
    pub end_state_ai: u8,
    /// Initial animation state (start of state machine).
    pub start_state_ai: u8,
    /// State advance rate divisor (default 4).
    pub state_ai_advance: u8,
    /// State at which damage stops being applied. Defaults to `end_state_ai`
    /// when the INI key is absent (binary reads with `+0x309` as default).
    pub final_damage_state: u8,
    /// State at which 25% translucency begins. 0xFF = never.
    pub translucent_25_state: u8,
    /// State at which 50% translucency begins. 0xFF = never.
    pub translucent_50_state: u8,
    /// Normalize the direction vector based on distance to target.
    pub normalized: bool,
    /// Resolved next-particle reference. Always `None` after A2; the
    /// 2-pass resolver introduced in Task A4 fills this in from the
    /// captured name string.
    pub next_particle: Option<ParticleTypeId>,
    /// Position offset applied when transitioning to next particle in a chain.
    pub next_particle_offset: IVec3,

    // ── ColorList runtime (§11.1) ────────────────────────────────────
    /// Packed RGB triplets parsed from `ColorList=`. Stride is 3 bytes per entry,
    /// no padding; empty/missing key yields an empty Vec.
    pub color_list: Vec<[u8; 3]>,
    /// Rate of color interpolation across the ColorList.
    pub color_speed: SimFixed,
    /// Spark: starting color 1 (RGB).
    pub start_color_1: [u8; 3],
    /// Spark: starting color 2 (RGB).
    pub start_color_2: [u8; 3],

    // ── Spark-only (parsed but unused at Tier 2) ─────────────────────
    /// Spark: max random X velocity.
    pub x_velocity: i32,
    /// Spark: max random Y velocity.
    pub y_velocity: i32,
    /// Spark: minimum upward Z velocity.
    pub min_z_velocity: i32,
    /// Spark: random range added to MinZVelocity.
    pub z_velocity_range: i32,
}

impl ParticleType {
    /// Parse a ParticleType from an INI section.
    ///
    /// `next_particle` is always `None` here — the referenced ParticleType may
    /// not be parsed yet. Task A4 introduces a `Pending` parse-state struct
    /// that captures the name string for resolution in a second pass.
    pub fn from_ini_section(name: &str, section: &IniSection) -> Self {
        let behaves_like = section
            .get("BehavesLike")
            .and_then(ParticleBehavesLike::parse)
            // Binary's string-table loop falls through to index 0 (Gas) when
            // the INI string doesn't match any entry.
            .unwrap_or(ParticleBehavesLike::Gas);

        let end_state_ai = section.get_i32("EndStateAI").unwrap_or(0) as u8;
        let final_damage_state = section
            .get_i32("FinalDamageState")
            .map(|n| n as u8)
            .unwrap_or(end_state_ai);

        Self {
            name: name.to_string(),
            behaves_like,
            image: section.get("Image").map(|s| s.to_string()),

            max_dc: section.get_i32("MaxDC").unwrap_or(0).clamp(0, u16::MAX as i32) as u16,
            max_ec: section.get_i32("MaxEC").unwrap_or(0).clamp(0, u16::MAX as i32) as u16,
            damage: section.get_i32("Damage").unwrap_or(0),
            warhead: section.get("Warhead").map(|s| s.to_string()),
            start_frame: section
                .get_i32("StartFrame")
                .unwrap_or(0)
                .clamp(0, u16::MAX as i32) as u16,
            num_loop_frames: section
                .get_i32("NumLoopFrames")
                .unwrap_or(0)
                .clamp(0, u16::MAX as i32) as u16,
            translucency: section.get_i32("Translucency").unwrap_or(0) as u8,
            wind_effect: section.get_i32("WindEffect").unwrap_or(0) as u8,
            velocity: section
                .get_f32("Velocity")
                .map(sim_from_f32)
                .unwrap_or(SIM_ZERO),
            deacc: section
                .get_f32("Deacc")
                .map(sim_from_f32)
                .unwrap_or(SIM_ZERO),
            radius: section.get_i32("Radius").unwrap_or(0),
            delete_on_state_limit: section.get_bool("DeleteOnStateLimit").unwrap_or(false),
            end_state_ai,
            start_state_ai: section.get_i32("StartStateAI").unwrap_or(0) as u8,
            state_ai_advance: section
                .get_i32("StateAIAdvance")
                .map(|n| n as u8)
                .unwrap_or(4),
            final_damage_state,
            translucent_25_state: section
                .get_i32("Translucent25State")
                .map(|n| n as u8)
                .unwrap_or(0xFF),
            translucent_50_state: section
                .get_i32("Translucent50State")
                .map(|n| n as u8)
                .unwrap_or(0xFF),
            normalized: section.get_bool("Normalized").unwrap_or(false),
            next_particle: None,
            next_particle_offset: section
                .get("NextParticleOffset")
                .map(parse_coord_offset)
                .unwrap_or(IVec3::ZERO),

            color_list: parse_color_list(section.get("ColorList")),
            color_speed: section
                .get_f32("ColorSpeed")
                .map(sim_from_f32)
                .unwrap_or(SIM_ZERO),
            start_color_1: section
                .get("StartColor1")
                .map(parse_rgb_color)
                .unwrap_or([0, 0, 0]),
            start_color_2: section
                .get("StartColor2")
                .map(parse_rgb_color)
                .unwrap_or([0, 0, 0]),

            x_velocity: section.get_i32("XVelocity").unwrap_or(0),
            y_velocity: section.get_i32("YVelocity").unwrap_or(0),
            min_z_velocity: section.get_i32("MinZVelocity").unwrap_or(0),
            z_velocity_range: section.get_i32("ZVelocityRange").unwrap_or(0),
        }
    }
}

/// Parse `ColorList=R,G,B,R,G,B,...` into a Vec of packed RGB triplets.
///
/// Stride is 3 bytes per entry, no padding; the binary's strtok loop
/// reads triples and discards any trailing partial entry. Empty or missing
/// values return an empty Vec.
fn parse_color_list(value: Option<&str>) -> Vec<[u8; 3]> {
    let Some(raw) = value else {
        return Vec::new();
    };
    let mut nums = raw
        .split(',')
        .filter_map(|s| s.trim().parse::<i32>().ok())
        .map(|n| n.clamp(0, 255) as u8);
    let mut out = Vec::new();
    while let (Some(r), Some(g), Some(b)) = (nums.next(), nums.next(), nums.next()) {
        out.push([r, g, b]);
    }
    out
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

    #[test]
    fn color_list_packs_triplets() {
        let v = parse_color_list(Some("255,255,255,200,200,80,200,10,10,0,0,0"));
        assert_eq!(
            v,
            vec![[255, 255, 255], [200, 200, 80], [200, 10, 10], [0, 0, 0]]
        );
    }

    #[test]
    fn color_list_handles_partial_trailing() {
        // 5 numbers — only one full triplet
        let v = parse_color_list(Some("1,2,3,4,5"));
        assert_eq!(v, vec![[1, 2, 3]]);
    }

    #[test]
    fn color_list_empty_or_missing() {
        assert_eq!(parse_color_list(None), Vec::<[u8; 3]>::new());
        assert_eq!(parse_color_list(Some("")), Vec::<[u8; 3]>::new());
    }

    #[test]
    fn from_ini_uses_documented_defaults() {
        let ini = IniFile::from_str("[Foo]\n");
        let section = ini.section("Foo").unwrap();
        let pt = ParticleType::from_ini_section("Foo", section);
        assert_eq!(pt.state_ai_advance, 4);
        assert_eq!(pt.translucent_25_state, 0xFF);
        assert_eq!(pt.translucent_50_state, 0xFF);
        assert_eq!(pt.color_list, Vec::<[u8; 3]>::new());
        // BehavesLike defaults to Gas (binary's string-table fallthrough).
        assert_eq!(pt.behaves_like, ParticleBehavesLike::Gas);
        // FinalDamageState mirrors EndStateAI when both absent — both 0 here.
        assert_eq!(pt.final_damage_state, pt.end_state_ai);
        assert_eq!(pt.next_particle, None);
        assert_eq!(pt.next_particle_offset, IVec3::ZERO);
        assert_eq!(pt.velocity, SIM_ZERO);
    }

    #[test]
    fn from_ini_parses_gas_cloud_like_section() {
        // Mirrors GasCloudM1 from rulesmd.ini, with NextParticle deferred to A4.
        let ini = IniFile::from_str(
            "[GasCloudM1]\n\
             BehavesLike=Gas\n\
             Image=GASLRGMK\n\
             MaxDC=60\n\
             MaxEC=448\n\
             Damage=0\n\
             Warhead=Gas\n\
             EndStateAI=11\n\
             StateAIAdvance=3\n\
             Translucency=50\n\
             DeleteOnStateLimit=yes\n\
             NextParticleOffset=0,0,150\n",
        );
        let section = ini.section("GasCloudM1").unwrap();
        let pt = ParticleType::from_ini_section("GasCloudM1", section);

        assert_eq!(pt.name, "GasCloudM1");
        assert_eq!(pt.behaves_like, ParticleBehavesLike::Gas);
        assert_eq!(pt.image.as_deref(), Some("GASLRGMK"));
        assert_eq!(pt.max_dc, 60);
        assert_eq!(pt.max_ec, 448);
        assert_eq!(pt.damage, 0);
        assert_eq!(pt.warhead.as_deref(), Some("Gas"));
        assert_eq!(pt.end_state_ai, 11);
        assert_eq!(pt.state_ai_advance, 3);
        assert_eq!(pt.translucency, 50);
        assert!(pt.delete_on_state_limit);
        assert_eq!(pt.next_particle_offset, IVec3::new(0, 0, 150));
        // FinalDamageState absent → mirrors EndStateAI.
        assert_eq!(pt.final_damage_state, 11);
    }

    #[test]
    fn from_ini_final_damage_state_overrides_default_when_present() {
        let ini = IniFile::from_str(
            "[Foo]\n\
             EndStateAI=10\n\
             FinalDamageState=5\n",
        );
        let section = ini.section("Foo").unwrap();
        let pt = ParticleType::from_ini_section("Foo", section);
        assert_eq!(pt.end_state_ai, 10);
        assert_eq!(pt.final_damage_state, 5);
    }

    #[test]
    fn from_ini_parses_color_list_and_start_colors() {
        let ini = IniFile::from_str(
            "[Spark]\n\
             BehavesLike=Spark\n\
             ColorList=255,255,255,200,200,80\n\
             StartColor1=255,128,0\n\
             StartColor2=128,64,32\n",
        );
        let section = ini.section("Spark").unwrap();
        let pt = ParticleType::from_ini_section("Spark", section);
        assert_eq!(pt.color_list, vec![[255, 255, 255], [200, 200, 80]]);
        assert_eq!(pt.start_color_1, [255, 128, 0]);
        assert_eq!(pt.start_color_2, [128, 64, 32]);
    }

    #[test]
    fn from_ini_unknown_behaves_like_falls_through_to_gas() {
        let ini = IniFile::from_str("[Foo]\nBehavesLike=Bogus\n");
        let section = ini.section("Foo").unwrap();
        let pt = ParticleType::from_ini_section("Foo", section);
        assert_eq!(pt.behaves_like, ParticleBehavesLike::Gas);
    }

    #[test]
    fn parse_coord_offset_handles_short_input() {
        assert_eq!(parse_coord_offset("1,2,3"), IVec3::new(1, 2, 3));
        assert_eq!(parse_coord_offset("1,2"), IVec3::new(1, 2, 0));
        assert_eq!(parse_coord_offset(""), IVec3::ZERO);
        assert_eq!(parse_coord_offset("garbage"), IVec3::ZERO);
    }
}
