//! Mission scheduler substrate — vocabulary + components.
//!
//! Models the original engine's mission *contract* as a Rust-native service (a
//! single current-mission selector + frame-anchored dispatch timer + verb API),
//! not its C++ class tree. `timer` owns the deferral primitive; `control` owns
//! the per-mission INI table; the verbs and dispatch land in later slices.
//! Depends on `rules/` (the control table parses from an `IniFile`); `sim/`
//! only — never render/ui/sidebar/audio/net.

pub mod control;
pub mod dispatch;
pub mod retask;
pub mod timer;
pub mod verb;
pub use control::{MissionControl, MissionControlEntry};
pub use retask::DockTeardown;
pub use timer::MissionTimer;

/// Number of dispatched mission ids (0..=31). The `None` sentinel is outside
/// this range and is never iterated by [`MissionType::all`].
pub const MISSION_COUNT: usize = 32;

/// The canonical mission selector. Discriminants 0..=31 equal the wire mission
/// id; `None = 0xFF` is the idle sentinel. `repr(u16)` so the discriminant folds
/// stably into the state hash in later slices.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default,
    serde::Serialize, serde::Deserialize,
)]
#[repr(u16)]
pub enum MissionType {
    /// No committed mission (idle). Sentinel discriminant `0xFF`; the `Default`.
    #[default]
    None = 0xFF,
    Sleep = 0,
    Attack = 1,
    Move = 2,
    QMove = 3,
    Retreat = 4,
    Guard = 5,
    Sticky = 6,
    Enter = 7,
    Capture = 8,
    /// TS-legacy; occupies index 9 and shifts every later index by one.
    Eaten = 9,
    Harvest = 10,
    AreaGuard = 11,
    Return = 12,
    Stop = 13,
    /// Dead stub in YR (no live assigner). Round-trips for map-INI name
    /// fidelity; executes as a Sleep-equivalent no-op.
    Ambush = 14,
    Hunt = 15,
    Unload = 16,
    Sabotage = 17,
    Construction = 18,
    Selling = 19,
    Repair = 20,
    /// Live AI-only behavior: idle teammates are tasked to converge on an
    /// attacker. A real handler is required; it is never player-assigned.
    Rescue = 21,
    Missile = 22,
    Harmless = 23,
    Open = 24,
    Patrol = 25,
    ParadropApproach = 26,
    ParadropOverfly = 27,
    /// Index 28. Named "Wait" in the INI mission-name table and "Deliberate"
    /// in unit reports — one mission. The guard-protected interrupt mission.
    Deliberate = 28,
    /// No dispatch case — resolved upstream as a queued command, never executed
    /// as a committed mission. Present so the selector can represent the
    /// command; the dispatcher MUST skip it (parity requirement).
    AttackMove = 29,
    SpyplaneApproach = 30,
    SpyplaneOverfly = 31,
}

impl MissionType {
    /// The wire mission id (0..=31; `0xFF` for `None`).
    #[inline]
    pub fn id(self) -> u8 {
        self as u8
    }

    /// Alias of [`MissionType::id`] for dispatch call sites.
    #[inline]
    pub fn dispatch_id(self) -> u8 {
        self as u8
    }

    /// Map a wire id to its mission. Explicit match (no transmute) so a
    /// malformed map byte `>= 32` yields `None` rather than UB — lockstep-safe.
    pub fn from_id(id: u8) -> Option<Self> {
        Some(match id {
            0 => Self::Sleep,
            1 => Self::Attack,
            2 => Self::Move,
            3 => Self::QMove,
            4 => Self::Retreat,
            5 => Self::Guard,
            6 => Self::Sticky,
            7 => Self::Enter,
            8 => Self::Capture,
            9 => Self::Eaten,
            10 => Self::Harvest,
            11 => Self::AreaGuard,
            12 => Self::Return,
            13 => Self::Stop,
            14 => Self::Ambush,
            15 => Self::Hunt,
            16 => Self::Unload,
            17 => Self::Sabotage,
            18 => Self::Construction,
            19 => Self::Selling,
            20 => Self::Repair,
            21 => Self::Rescue,
            22 => Self::Missile,
            23 => Self::Harmless,
            24 => Self::Open,
            25 => Self::Patrol,
            26 => Self::ParadropApproach,
            27 => Self::ParadropOverfly,
            28 => Self::Deliberate,
            29 => Self::AttackMove,
            30 => Self::SpyplaneApproach,
            31 => Self::SpyplaneOverfly,
            _ => return None,
        })
    }

    /// The `[<MissionName>]` INI section header for this mission's control entry.
    pub fn ini_section(self) -> &'static str {
        match self {
            Self::Sleep => "Sleep",
            Self::Attack => "Attack",
            Self::Move => "Move",
            Self::QMove => "QMove",
            Self::Retreat => "Retreat",
            Self::Guard => "Guard",
            Self::Sticky => "Sticky",
            Self::Enter => "Enter",
            Self::Capture => "Capture",
            Self::Eaten => "Eaten",
            Self::Harvest => "Harvest",
            Self::AreaGuard => "Area Guard",
            Self::Return => "Return",
            Self::Stop => "Stop",
            Self::Ambush => "Ambush",
            Self::Hunt => "Hunt",
            Self::Unload => "Unload",
            Self::Sabotage => "Sabotage",
            Self::Construction => "Construction",
            Self::Selling => "Selling",
            Self::Repair => "Repair",
            Self::Rescue => "Rescue",
            Self::Missile => "Missile",
            Self::Harmless => "Harmless",
            Self::Open => "Open",
            Self::Patrol => "Patrol",
            Self::ParadropApproach => "ParadropApproach",
            Self::ParadropOverfly => "ParadropOverfly",
            Self::Deliberate => "Wait",
            Self::AttackMove => "AttackMove",
            Self::SpyplaneApproach => "SpyplaneApproach",
            Self::SpyplaneOverfly => "SpyplaneOverfly",
            Self::None => "None",
        }
    }

    /// Iterate all 32 dispatched missions in id order (table builds, round-trip).
    pub fn all() -> impl Iterator<Item = MissionType> {
        (0u8..MISSION_COUNT as u8).filter_map(MissionType::from_id)
    }
}

/// Shadow mission component: the single current-mission selector plus the
/// queued/suspended interrupt stack, a sub-phase byte, the dispatch deferral
/// timer, and a per-entity refresh counter.
///
/// Canonical hashed lockstep state (Slice 8): folded into `world_hash` and fully
/// serde round-tripped — load trusts it verbatim (no post-load re-derivation).
/// `current`/`substate` are written by the tail projection for most units and,
/// as of S2, at host/dispatch time for scoped move units (where the dispatch-time
/// value is authoritative — e.g. an arrival tick hashes `Move`). The verb API
/// owns `queued`/`suspended`/`timer`.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize,
)]
pub struct MissionCom {
    /// The committed current mission (`MissionType::None` = idle).
    pub current: MissionType,
    /// A queued follow-up to commence after the current mission, if any.
    pub queued: Option<MissionType>,
    /// A suspended mission to restore after an interrupt (override/restore stack).
    pub suspended: Option<MissionType>,
    /// Sub-phase byte within the current mission (attack sub-state, dock phase, …).
    pub substate: u8,
    /// Frame-anchored dispatch deferral.
    pub timer: MissionTimer,
    /// Monotonic per-entity refresh counter (wrapping).
    pub tick_counter: u32,
}

impl MissionCom {
    /// The idle component: no current mission, empty stack, default timer.
    #[inline]
    pub fn idle() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_ids_round_trip() {
        for id in 0u8..MISSION_COUNT as u8 {
            let m = MissionType::from_id(id).expect("ids < 32 map to a mission");
            assert_eq!(m.id(), id);
            assert_eq!(m.dispatch_id(), id);
        }
    }

    #[test]
    fn out_of_range_ids_are_none() {
        assert_eq!(MissionType::from_id(32), None);
        // 0xFF is the idle sentinel's discriminant, not a dispatched id.
        assert_eq!(MissionType::from_id(0xFF), None);
    }

    #[test]
    fn verified_spot_indices() {
        assert_eq!(MissionType::Sleep.id(), 0);
        assert_eq!(MissionType::Guard.id(), 5);
        assert_eq!(MissionType::Enter.id(), 7);
        assert_eq!(MissionType::Eaten.id(), 9);
        assert_eq!(MissionType::Harvest.id(), 10);
        assert_eq!(MissionType::AreaGuard.id(), 11);
        assert_eq!(MissionType::Selling.id(), 19);
        assert_eq!(MissionType::Rescue.id(), 21);
        assert_eq!(MissionType::AttackMove.id(), 29);
        assert_eq!(MissionType::SpyplaneOverfly.id(), 31);
    }

    #[test]
    fn all_iterates_thirty_two() {
        assert_eq!(MissionType::all().count(), MISSION_COUNT);
    }

    #[test]
    fn default_is_none_sentinel() {
        assert_eq!(MissionType::default(), MissionType::None);
        assert_eq!(MissionType::None as u16, 0xFF);
    }

    #[test]
    fn ini_section_names_match_table() {
        assert_eq!(MissionType::AreaGuard.ini_section(), "Area Guard");
        assert_eq!(MissionType::Deliberate.ini_section(), "Wait");
        assert_eq!(MissionType::Sleep.ini_section(), "Sleep");
    }
}
