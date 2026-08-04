//! Mission scheduler substrate — vocabulary, native-width state, and authority.
//!
//! Models the original engine's mission contract as Rust-native state and
//! functions rather than reproducing its C++ class tree. `state` owns the
//! lossless selectors and private common fields, `timer` owns both legacy and
//! signed dispatch timing, and `authority` owns the LIVE exact verb surface:
//! player commands queue through it (the event-execute shape) and the
//! per-object AI host promotes queued missions (Ready→Commence). The mission
//! handler bodies remain the legacy per-system state machines — dispatch-time
//! handler execution (timer/handler-state writes) is the recorded residual.
//! Depends on `rules/` for the INI control table and otherwise remains in
//! `sim/` — never render/ui/sidebar/audio/net.

pub(crate) mod authority;
pub(crate) mod concrete_effects;
pub mod control;
pub(crate) mod leaf;
pub(crate) mod readiness;
pub mod retask;
pub mod state;
pub mod timer;
pub mod verb;
pub use control::{MissionControl, MissionControlEntry};
pub(crate) use leaf::MissionLeafState;
pub use retask::DockTeardown;
pub use state::{MissionCom, MissionId};
pub use timer::{MissionDispatchTimer, MissionTimer};

/// Number of dispatched mission ids (0..=31). The `None` sentinel is outside
/// this range and is never iterated by [`MissionType::all`].
pub const MISSION_COUNT: usize = 32;

/// The canonical mission selector. Discriminants 0..=31 equal the wire mission
/// id; `None = 0xFF` is the idle sentinel. `repr(u16)` so the discriminant folds
/// stably into the state hash in later slices.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Default,
    serde::Serialize,
    serde::Deserialize,
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
    /// fidelity. Its dispatch slot is a *distinct* base stub from Sleep's —
    /// the return value coincides (450 frames, do nothing) but a subclass
    /// could override one without the other.
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
    /// Index 29. The dispatcher has no *case* for it: like `QMove(3)` and
    /// every out-of-range id, it falls through the jump table's `default` arm
    /// to the same base handler `Sleep(0)` uses, which re-arms the dispatch
    /// timer with a flat 450 frames. VERA resolves attack-move upstream as a
    /// standing order and never commits this selector, so the arm is dead
    /// either way — but the dispatcher does NOT skip it.
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
    ///
    /// These are the literal strings from the original's mission-name pointer
    /// table, which is what its `MissionControl` reader hands to the section
    /// lookup. Six of the 32 contain a space — `Area Guard`, `Paradrop
    /// Approach`, `Paradrop Overfly`, `Attack Move`, `Spyplane Approach`,
    /// `Spyplane Overfly` — and must be spelled with it or a mod/map INI
    /// declaring one is silently ignored.
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
            Self::ParadropApproach => "Paradrop Approach",
            Self::ParadropOverfly => "Paradrop Overfly",
            Self::Deliberate => "Wait",
            Self::AttackMove => "Attack Move",
            Self::SpyplaneApproach => "Spyplane Approach",
            Self::SpyplaneOverfly => "Spyplane Overfly",
            Self::None => "None",
        }
    }

    /// Iterate all 32 dispatched missions in id order (table builds, round-trip).
    pub fn all() -> impl Iterator<Item = MissionType> {
        (0u8..MISSION_COUNT as u8).filter_map(MissionType::from_id)
    }

    /// Resolve a map-INI `MISSION=` name, the way the original's
    /// `Mission_From_Name` does: a linear, **case-insensitive**, ASCII-only
    /// scan of the same 32-entry name table in id order, first match wins,
    /// and no match (or an absent field) yields the `-1` idle sentinel —
    /// which is a *distinct* selector from `Sleep(0)`, even though both run
    /// the same base handler.
    ///
    /// The table is [`MissionType::ini_section`]: the original hands the very
    /// same pointer table to both its map-name lookup and its
    /// `[<MissionName>]` control-section lookup. Five of those names carry a
    /// space (`Area Guard`, `Paradrop Approach`, `Paradrop Overfly`,
    /// `Attack Move`, `Spyplane Approach`/`Spyplane Overfly`) and index 28 is
    /// spelled `Wait`; a name table that loses those silently resolves the
    /// affected map lines to the sentinel.
    pub fn from_map_name(name: &str) -> Option<MissionType> {
        let name = name.trim();
        if name.is_empty() {
            return None;
        }
        MissionType::all().find(|mission| mission.ini_section().eq_ignore_ascii_case(name))
    }

    /// Missions with **no completion transition**: the object holds them until
    /// something else retasks it, so a "the committed mission's work is over"
    /// reading must never override them.
    ///
    /// `Sleep` and `Harmless` dispatch to base handlers that do nothing and
    /// re-arm themselves forever; `Sticky` shares the Guard handler but is
    /// excluded from passive acquisition and refuses to promote a queued
    /// mission at all. All three mean "stand still and do nothing", which is
    /// the opposite of the idle-Unit `Guard` reading VERA derives for an
    /// object with no live machine.
    ///
    /// These are exactly the three the stock map `MISSION=` column authors for
    /// neutral scenery objects, so the distinction is load-bearing from the
    /// first frame of any map that carries them.
    pub fn holds_until_retasked(self) -> bool {
        matches!(
            self,
            MissionType::Sleep | MissionType::Sticky | MissionType::Harmless
        )
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

    /// Every mission name the original's table spells with a space must be
    /// spelled with it here, or the `[<MissionName>]` lookup misses.
    #[test]
    fn spaced_section_names_keep_their_space() {
        assert_eq!(MissionType::AreaGuard.ini_section(), "Area Guard");
        assert_eq!(
            MissionType::ParadropApproach.ini_section(),
            "Paradrop Approach"
        );
        assert_eq!(
            MissionType::ParadropOverfly.ini_section(),
            "Paradrop Overfly"
        );
        assert_eq!(MissionType::AttackMove.ini_section(), "Attack Move");
        assert_eq!(
            MissionType::SpyplaneApproach.ini_section(),
            "Spyplane Approach"
        );
        assert_eq!(
            MissionType::SpyplaneOverfly.ini_section(),
            "Spyplane Overfly"
        );
    }

    /// `Mission_From_Name` returns the table index for an exact-but-
    /// case-insensitive name, so every one of the 32 names round-trips.
    #[test]
    fn map_name_round_trips_every_mission() {
        for mission in MissionType::all() {
            assert_eq!(
                MissionType::from_map_name(mission.ini_section()),
                Some(mission),
                "{mission:?} did not round-trip through its map name"
            );
            assert_eq!(
                MissionType::from_map_name(&mission.ini_section().to_ascii_lowercase()),
                Some(mission),
                "{mission:?} map name is not case-insensitive"
            );
        }
    }

    /// The retail comparator is `stricmp`, and the five spaced names plus
    /// `Wait` are the ones a naive `{:?}` spelling would lose.
    #[test]
    fn map_name_resolves_the_spaced_and_renamed_entries() {
        assert_eq!(
            MissionType::from_map_name("Area Guard"),
            Some(MissionType::AreaGuard)
        );
        assert_eq!(
            MissionType::from_map_name("attack move"),
            Some(MissionType::AttackMove)
        );
        assert_eq!(
            MissionType::from_map_name("Wait"),
            Some(MissionType::Deliberate)
        );
        // The enum's own Rust name is NOT the table name.
        assert_eq!(MissionType::from_map_name("AreaGuard"), None);
        assert_eq!(MissionType::from_map_name("Deliberate"), None);
    }

    /// An unknown or absent name is the `-1` sentinel, NOT `Sleep(0)`.
    #[test]
    fn unknown_map_name_is_the_idle_sentinel() {
        assert_eq!(MissionType::from_map_name("Wibble"), None);
        assert_eq!(MissionType::from_map_name(""), None);
        assert_eq!(MissionType::from_map_name("   "), None);
        // "None" is this project's spelling of the sentinel, not a table name.
        assert_eq!(MissionType::from_map_name("None"), None);
        assert_eq!(MissionType::from_map_name("Sleep"), Some(MissionType::Sleep));
    }

    #[test]
    fn only_the_stand_still_missions_hold_until_retasked() {
        for mission in MissionType::all() {
            let expected = matches!(
                mission,
                MissionType::Sleep | MissionType::Sticky | MissionType::Harmless
            );
            assert_eq!(mission.holds_until_retasked(), expected, "{mission:?}");
        }
        assert!(!MissionType::None.holds_until_retasked());
    }

    /// The 32 section names are distinct — a duplicate would make two missions
    /// share one control slot.
    #[test]
    fn section_names_are_unique() {
        let mut seen = std::collections::BTreeSet::new();
        for mission in MissionType::all() {
            assert!(
                seen.insert(mission.ini_section()),
                "duplicate section name for {mission:?}"
            );
        }
        assert_eq!(seen.len(), MISSION_COUNT);
    }
}
