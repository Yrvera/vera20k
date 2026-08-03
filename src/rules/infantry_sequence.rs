//! Infantry animation sequence definitions parsed from art.ini.
//!
//! RA2 art.ini assigns each infantry type a `Sequence=` key (e.g., `Sequence=ConSequence`)
//! pointing to a `[ConSequence]` section that defines the SHP frame layout for every
//! animation: stand, walk, fire, idle, die, crawl, prone, etc.
//!
//! Format per key: `Walk=8,6,6` means start_frame=8, frames_per_facing=6, facings=6.
//! An optional 4th field is a facing direction hint: `Idle1=56,15,0,S` (face South).
//!
//! There are 41 unique sequence definitions in artmd.ini. Each infantry type can have
//! different frame counts, facing counts, and animation ranges. Without parsing these,
//! all infantry share a single hardcoded layout which breaks for non-standard units
//! (Brute has 10 facings, Rocketeer has Fly/Hover, Tanya has Swim, etc.).
//!
//! ## Dependency rules
//! - Part of rules/ — depends only on rules/ini_parser.
//! - Does NOT depend on sim/, render/, ui/, or any game module.

use std::collections::HashMap;

use crate::rules::ini_parser::IniFile;
use crate::sim::animation::{FacingSlots, LoopMode, SequenceDef, SequenceKind, SequenceSet};

/// Native action-record delay byte for all 42 infantry actions.
const ACTION_FRAME_DELAYS: [u8; 42] = [
    0, 0, 6, 3, 1, 1, 1, 1, 1, 3, 3, 1, 1, 1, 1, 1, 3, 1, 3, 3, 1, 1, 1, 2, 1, 1, 1, 1, 1, 1, 1, 1,
    3, 1, 3, 1, 3, 4, 6, 3, 1, 1,
];

const NORMALIZED_ACTIONS: [u8; 6] = [0x09, 0x0A, 0x12, 0x13, 0x17, 0x20];

/// Compass direction hint for non-directional animations.
///
/// When `facings=0` in the INI, the animation is non-directional (plays the same
/// frames regardless of facing). The optional direction suffix tells the engine
/// which facing to display during playback (e.g., idle fidgets face a fixed direction).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FacingHint {
    N,
    NE,
    E,
    SE,
    S,
    SW,
    W,
    NW,
}

/// One animation entry parsed from an INI value like `"8,6,6"` or `"56,15,0,S"`.
#[derive(Debug, Clone)]
pub struct InfantrySequenceEntry {
    /// First SHP frame index for this animation.
    pub start_frame: u16,
    /// Number of animation frames per facing direction.
    pub frames_per_facing: u16,
    /// Number of facing directions in the INI (0 = non-directional, 6 = standard, 8, 10...).
    pub facings: u8,
    /// Optional facing direction hint for non-directional animations.
    pub facing_hint: Option<FacingHint>,
}

/// All animation entries from one `[*Sequence]` section, keyed by INI key name (uppercase).
#[derive(Debug, Clone)]
pub struct InfantrySequenceDef {
    /// Animation entries keyed by uppercase INI key (e.g., "WALK", "FIREUP", "IDLE1").
    pub entries: HashMap<String, InfantrySequenceEntry>,
}

/// Registry of all parsed sequence sections, keyed by uppercase section name.
pub type InfantrySequenceRegistry = HashMap<String, InfantrySequenceDef>;

/// Parse a single sequence value string like `"8,6,6"` or `"56,15,0,S"`.
///
/// Returns `None` if the format is invalid or cannot be parsed.
pub fn parse_sequence_value(value: &str) -> Option<InfantrySequenceEntry> {
    let parts: Vec<&str> = value.split(',').map(|s| s.trim()).collect();
    if parts.len() < 3 {
        return None;
    }

    let start_frame: u16 = parts[0].parse::<u16>().ok()?;
    let frames_per_facing: u16 = parts[1].parse::<u16>().ok()?;
    let facings: u8 = parts[2].parse::<u8>().ok()?;

    let facing_hint: Option<FacingHint> = if parts.len() >= 4 {
        parse_facing_hint(parts[3])
    } else {
        None
    };

    Some(InfantrySequenceEntry {
        start_frame,
        frames_per_facing,
        facings,
        facing_hint,
    })
}

/// Parse a facing direction hint string (e.g., "S", "NE", "W").
fn parse_facing_hint(s: &str) -> Option<FacingHint> {
    match s.to_uppercase().as_str() {
        "N" => Some(FacingHint::N),
        "NE" => Some(FacingHint::NE),
        "E" => Some(FacingHint::E),
        "SE" => Some(FacingHint::SE),
        "S" => Some(FacingHint::S),
        "SW" => Some(FacingHint::SW),
        "W" => Some(FacingHint::W),
        "NW" => Some(FacingHint::NW),
        _ => None,
    }
}

/// Suffix used to identify sequence definition sections in art.ini.
const SEQUENCE_SUFFIX: &str = "sequence";

/// Parse all infantry sequence definition sections from art.ini.
///
/// Scans for any section whose name ends with "Sequence" (case-insensitive)
/// and parses each key-value pair as an animation entry.
pub fn parse_infantry_sequence_registry(ini: &IniFile) -> InfantrySequenceRegistry {
    let mut registry: InfantrySequenceRegistry = HashMap::new();

    for section_name in ini.section_names() {
        if !section_name.to_ascii_lowercase().ends_with(SEQUENCE_SUFFIX) {
            continue;
        }

        let section = match ini.section(section_name) {
            Some(s) => s,
            None => continue,
        };

        let mut entries: HashMap<String, InfantrySequenceEntry> = HashMap::new();

        for key in section.keys() {
            let value = match section.get(key) {
                Some(v) => v,
                None => continue,
            };

            // Skip non-sequence keys (e.g., sound-related keys that don't follow the format).
            if let Some(entry) = parse_sequence_value(value) {
                entries.insert(key.to_uppercase(), entry);
            }
        }

        if !entries.is_empty() {
            registry.insert(section_name.to_uppercase(), InfantrySequenceDef { entries });
        }
    }

    log::info!(
        "InfantrySequenceRegistry: {} sequence definitions loaded from art.ini",
        registry.len()
    );
    registry
}

/// Map an INI key name (e.g., "Ready", "Walk", "FireUp") to the engine's SequenceKind.
///
/// Returns None for keys the engine doesn't yet support (Fly, Swim, Deploy, etc.).
pub fn sequence_kind_from_ini_key(key: &str) -> Option<SequenceKind> {
    match key.to_uppercase().as_str() {
        "READY" | "GUARD" => Some(SequenceKind::Stand),
        "WALK" => Some(SequenceKind::Walk),
        "PRONE" => Some(SequenceKind::Prone),
        "CRAWL" => Some(SequenceKind::Crawl),
        "FIREUP" => Some(SequenceKind::Attack),
        "FIREPRONE" => Some(SequenceKind::FireProne),
        "DOWN" => Some(SequenceKind::Down),
        "UP" => Some(SequenceKind::Up),
        "IDLE1" => Some(SequenceKind::Idle1),
        "IDLE2" => Some(SequenceKind::Idle2),
        "DIE1" => Some(SequenceKind::Die1),
        "DIE2" => Some(SequenceKind::Die2),
        "DIE3" => Some(SequenceKind::Die3),
        "DIE4" => Some(SequenceKind::Die4),
        "DIE5" => Some(SequenceKind::Die5),
        "CHEER" => Some(SequenceKind::Cheer),
        "PARADROP" => Some(SequenceKind::Paradrop),
        "PANIC" => Some(SequenceKind::Panic),
        "DEPLOY" => Some(SequenceKind::Deploy),
        "UNDEPLOY" => Some(SequenceKind::Undeploy),
        "DEPLOYED" => Some(SequenceKind::Deployed),
        "DEPLOYEDFIRE" => Some(SequenceKind::DeployedFire),
        "DEPLOYEDIDLE" => Some(SequenceKind::DeployedIdle),
        "SECONDARYFIRE" => Some(SequenceKind::SecondaryFire),
        "SECONDARYPRONE" => Some(SequenceKind::SecondaryProne),
        "SWIM" => Some(SequenceKind::Swim),
        "FLY" => Some(SequenceKind::Fly),
        "FIREFLY" => Some(SequenceKind::FireFly),
        "HOVER" => Some(SequenceKind::Hover),
        "TREAD" => Some(SequenceKind::Tread),
        "WETATTACK" => Some(SequenceKind::WetAttack),
        "WETIDLE1" => Some(SequenceKind::WetIdle1),
        "WETIDLE2" => Some(SequenceKind::WetIdle2),
        _ => None,
    }
}

/// Native action id for a sequence, i.e. its slot in the 42-entry action table.
///
/// The ids are the index order of the engine's own sequence-name array, not the
/// alphabetical or INI order. They are only meaningful as indices into
/// `ACTION_FRAME_DELAYS` / `NORMALIZED_ACTIONS`, so an id that is off by even one
/// hands the sequence another action's playback speed. That is exactly what the
/// water, flight, deploy-adjacent and secondary-weapon rows did before this
/// table was walked out of the binary: `SecondaryFire` in particular ran three
/// times slower than retail, and because an infantryman discharges his weapon on
/// a specific frame of the fire sequence rather than at its start, that slowed
/// the Brute's building-smash rate to a third.
///
/// Ids 20/21 (WetDie1/WetDie2), 25 (Tumble), 34–36 (AirDeath*) and 38/39
/// (Shovel/Carry) have no `SequenceKind` yet and so are absent below.
fn action_id(kind: SequenceKind) -> u8 {
    match kind {
        SequenceKind::Stand => 0,
        SequenceKind::Prone => 2,
        SequenceKind::Walk => 3,
        SequenceKind::Attack => 4,
        SequenceKind::Down => 5,
        SequenceKind::Crawl => 6,
        SequenceKind::Up => 7,
        SequenceKind::FireProne => 8,
        SequenceKind::Idle1 => 9,
        SequenceKind::Idle2 => 10,
        SequenceKind::Die1 => 11,
        SequenceKind::Die2 => 12,
        SequenceKind::Die3 => 13,
        SequenceKind::Die4 => 14,
        SequenceKind::Die5 => 15,
        SequenceKind::Tread => 16,
        SequenceKind::Swim => 17,
        SequenceKind::WetIdle1 => 18,
        SequenceKind::WetIdle2 => 19,
        SequenceKind::WetAttack => 22,
        SequenceKind::Hover => 23,
        SequenceKind::Fly => 24,
        SequenceKind::FireFly => 26,
        SequenceKind::Deploy => 27,
        SequenceKind::Deployed => 28,
        SequenceKind::DeployedFire => 29,
        SequenceKind::DeployedIdle => 30,
        SequenceKind::Undeploy => 31,
        SequenceKind::Cheer => 32,
        SequenceKind::Paradrop => 33,
        SequenceKind::Panic => 37,
        SequenceKind::SecondaryFire => 40,
        SequenceKind::SecondaryProne => 41,
    }
}

fn action_timing(kind: SequenceKind) -> (u16, bool) {
    let id = action_id(kind);
    (
        u16::from(ACTION_FRAME_DELAYS[id as usize]),
        NORMALIZED_ACTIONS.contains(&id),
    )
}

/// Get the default LoopMode for a given SequenceKind.
fn default_loop_mode(kind: SequenceKind) -> LoopMode {
    match kind {
        SequenceKind::Stand
        | SequenceKind::Walk
        | SequenceKind::Crawl
        | SequenceKind::Prone
        | SequenceKind::Deployed
        | SequenceKind::Panic
        | SequenceKind::Swim
        | SequenceKind::Fly
        | SequenceKind::Hover
        | SequenceKind::Tread => LoopMode::Loop,
        SequenceKind::Die1
        | SequenceKind::Die2
        | SequenceKind::Die3
        | SequenceKind::Die4
        | SequenceKind::Die5
        | SequenceKind::Paradrop => LoopMode::HoldLast,
        SequenceKind::Attack
        | SequenceKind::FireProne
        | SequenceKind::SecondaryFire
        | SequenceKind::SecondaryProne => LoopMode::TransitionTo(SequenceKind::Stand),
        SequenceKind::DeployedFire => LoopMode::TransitionTo(SequenceKind::Deployed),
        SequenceKind::FireFly => LoopMode::TransitionTo(SequenceKind::Fly),
        SequenceKind::WetAttack => LoopMode::TransitionTo(SequenceKind::Swim),
        SequenceKind::Idle1 | SequenceKind::Idle2 | SequenceKind::Cheer => {
            LoopMode::TransitionTo(SequenceKind::Stand)
        }
        SequenceKind::DeployedIdle => LoopMode::TransitionTo(SequenceKind::Deployed),
        SequenceKind::WetIdle1 | SequenceKind::WetIdle2 => {
            LoopMode::TransitionTo(SequenceKind::Swim)
        }
        SequenceKind::Down => LoopMode::TransitionTo(SequenceKind::Prone),
        SequenceKind::Up | SequenceKind::Undeploy => LoopMode::TransitionTo(SequenceKind::Stand),
        SequenceKind::Deploy => LoopMode::TransitionTo(SequenceKind::Deployed),
    }
}

/// Standard number of facing directions for infantry.
const INFANTRY_FACINGS: u8 = 8;

/// Convert a parsed INI sequence definition into the engine's SequenceSet.
///
/// Maps known INI keys (Ready, Walk, FireUp, etc.) to SequenceKind variants.
/// Unknown keys are silently skipped — they can be added to SequenceKind later
/// when the engine supports those gameplay systems.
///
/// The INI 3rd field is the FacingMultiplier (stride between facings), NOT the
/// facing count. For directional sequences the actual facing count is always 8
/// (standard infantry). For non-directional sequences (multiplier=0), facings=1.
pub fn build_sequence_set(def: &InfantrySequenceDef) -> SequenceSet {
    let mut set: SequenceSet = SequenceSet::new();

    for (key, entry) in &def.entries {
        let kind: SequenceKind = match sequence_kind_from_ini_key(key) {
            Some(k) => k,
            None => continue,
        };

        // If this SequenceKind was already inserted (e.g., Ready and Guard both map
        // to Stand), skip duplicates — the first one wins.
        if set.get(&kind).is_some() {
            continue;
        }

        // INI 3rd field is FacingMultiplier (stride), not facing count.
        // Multiplier=0 → non-directional (facings=1).
        // Multiplier>0 → directional with 8 infantry facings.
        let (facings, facing_multiplier): (u8, u16) = if entry.facings == 0 {
            (1, 0)
        } else {
            (INFANTRY_FACINGS, entry.facings as u16)
        };

        let (frame_delay, normalized) = action_timing(kind);
        set.insert(
            kind,
            SequenceDef {
                start_frame: entry.start_frame,
                frame_count: entry.frames_per_facing,
                facings,
                facing_multiplier,
                frame_delay,
                normalized,
                loop_mode: default_loop_mode(kind),
                facing_slots: FacingSlots::InfantryTable,
            },
        );
    }

    set
}

#[cfg(test)]
#[path = "infantry_sequence_tests.rs"]
mod tests;
