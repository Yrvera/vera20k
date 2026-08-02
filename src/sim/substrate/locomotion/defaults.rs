//! Pure forms of the base locomotor bodies reached by active YR classes.
//!
//! Render-only `Apparent_Speed` is intentionally outside this substrate. Slot
//! 7 is represented because its inheritance pattern is binary-derived, though
//! the Rust mechanism will use the constant on the host-side entry model.

use super::class::LocomotorClass;

/// Base-vtable slots whose inherit/override pattern is relevant to live YR.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BaseDefaultSlot {
    HeadToCoord,
    CanEnterCell,
    DoTurn,
    Unlimbo,
    ForceTrack,
    ForceImmediateDestination,
    ForceNewSlope,
    IsMovingNow,
    MarkAllOccupationBits,
}

impl BaseDefaultSlot {
    pub const fn number(self) -> u8 {
        match self {
            Self::HeadToCoord => 6,
            Self::CanEnterCell => 7,
            Self::DoTurn => 19,
            Self::Unlimbo => 20,
            Self::ForceTrack => 28,
            Self::ForceImmediateDestination => 30,
            Self::ForceNewSlope => 31,
            Self::IsMovingNow => 32,
            Self::MarkAllOccupationBits => 39,
        }
    }

    const fn table_index(self) -> usize {
        match self {
            Self::HeadToCoord => 0,
            Self::CanEnterCell => 1,
            Self::DoTurn => 2,
            Self::Unlimbo => 3,
            Self::ForceTrack => 4,
            Self::ForceImmediateDestination => 5,
            Self::ForceNewSlope => 6,
            Self::IsMovingNow => 7,
            Self::MarkAllOccupationBits => 8,
        }
    }
}

pub const BASE_DEFAULT_SLOTS: [BaseDefaultSlot; 9] = [
    BaseDefaultSlot::HeadToCoord,
    BaseDefaultSlot::CanEnterCell,
    BaseDefaultSlot::DoTurn,
    BaseDefaultSlot::Unlimbo,
    BaseDefaultSlot::ForceTrack,
    BaseDefaultSlot::ForceImmediateDestination,
    BaseDefaultSlot::ForceNewSlope,
    BaseDefaultSlot::IsMovingNow,
    BaseDefaultSlot::MarkAllOccupationBits,
];

// Rows follow `LocomotorClass::ALL`; columns follow `BASE_DEFAULT_SLOTS`.
// `true` means the class installs the base body.
const INHERITS_BASE_DEFAULT: [[bool; 9]; 8] = [
    // Head  Enter Turn   Unlimbo Track  Immediate Slope  MovingNow MarkBits
    [false, true, false, false, false, true, false, false, false], // Drive
    [false, true, false, true, true, true, true, false, false],    // Hover
    [false, true, false, true, true, false, true, false, false],   // Walk
    [true, true, false, true, true, true, true, false, true],      // Fly
    [true, true, false, true, true, true, true, true, false],      // Teleport
    [false, true, false, false, false, true, false, false, false], // Ship
    [false, true, false, true, true, true, true, false, false],    // Jumpjet
    [true, true, true, true, true, true, true, false, true],       // Rocket
];

/// Whether `class` installs the native base body for `slot`.
pub const fn inherits_base_default(class: LocomotorClass, slot: BaseDefaultSlot) -> bool {
    INHERITS_BASE_DEFAULT[class.table_index()][slot.table_index()]
}

/// Whether `class` replaces the native base body for `slot`.
pub const fn overrides_base_default(class: LocomotorClass, slot: BaseDefaultSlot) -> bool {
    !inherits_base_default(class, slot)
}

/// Slot 6: the linked host's current coordinate is also the base head-to answer.
pub const fn head_to_coord(linked_object_coordinate: (i32, i32, i32)) -> (i32, i32, i32) {
    linked_object_coordinate
}

/// Slot 7: no live class supplies a different locomotor-level answer.
pub const fn can_enter_cell() -> i32 {
    0
}

/// Slot 19 base body.
pub const fn do_turn() {}

/// Slot 20 base body.
pub const fn unlimbo() {}

/// Slot 28 base body.
pub const fn force_track() {}

/// Slot 30 base body.
pub const fn force_immediate_destination() {}

/// Slot 31 base body.
pub const fn force_new_slope() {}

/// Slot 32 forwards to the installed class's own `Is_Moving` result.
pub const fn is_moving_now(own_is_moving: bool) -> bool {
    own_is_moving
}

/// Slot 39 base body.
pub const fn mark_all_occupation_bits() {}

#[cfg(test)]
mod tests {
    use super::*;

    /// Which classes inherit `slot`, in `LocomotorClass::ALL` order.
    fn inheritors(slot: BaseDefaultSlot) -> Vec<LocomotorClass> {
        LocomotorClass::ALL
            .into_iter()
            .filter(|class| inherits_base_default(*class, slot))
            .collect()
    }

    /// The inherit/override matrix, asserted as per-slot inheritor sets.
    ///
    /// RATCHET, not a parity check. Every expectation below was decoded from
    /// live vtable reads — all 8 classes × 9 slots, re-verified independently on
    /// 2026-07-29 (see the S1 review note in
    /// `docs/plans/2026-07-29-locomotion-substrate-design.md`) — but nothing here
    /// re-reads the binary, so this pins a transcription rather than proving it.
    /// Only a gamemd-derived executable check could raise that to VERIFIED.
    ///
    /// Deliberately NOT a copy of `INHERITS_BASE_DEFAULT`: the previous version of
    /// this test duplicated that constant verbatim and compared it to itself, so
    /// it could only catch someone editing one copy and not the other. Stating
    /// the expectation in a different shape — per-slot sets rather than per-class
    /// rows — means any single-cell change fails at least one assertion.
    #[test]
    fn base_default_map_matches_vtables() {
        use BaseDefaultSlot::*;
        use LocomotorClass::*;

        // Slot 7 is an always-OK stub in the base and no live class replaces it.
        assert_eq!(inheritors(CanEnterCell), LocomotorClass::ALL.to_vec());

        // Rocket alone keeps the base turn body — it is a ballistic projectile
        // with no steering of its own.
        assert_eq!(inheritors(DoTurn), vec![Rocket]);

        // Teleport alone keeps the base "am I moving right now" body, which
        // forwards to its own Is_Moving answer.
        assert_eq!(inheritors(IsMovingNow), vec![Teleport]);

        // Walk alone replaces the immediate-destination snap, for sub-cell
        // infantry placement.
        assert_eq!(
            inheritors(ForceImmediateDestination),
            vec![Drive, Hover, Fly, Teleport, Ship, Jumpjet, Rocket]
        );

        // The three classes that never carry the host themselves.
        assert_eq!(inheritors(HeadToCoord), vec![Fly, Teleport, Rocket]);
        assert_eq!(inheritors(MarkAllOccupationBits), vec![Fly, Rocket]);

        // Drive and Ship own their unlimbo, track and slope bodies; the rest
        // inherit all three.
        for slot in [Unlimbo, ForceTrack, ForceNewSlope] {
            assert_eq!(
                inheritors(slot),
                vec![Hover, Walk, Fly, Teleport, Jumpjet, Rocket],
                "slot {}",
                slot.number()
            );
        }

        // Ship is a near-copy of Drive, and their inherit rows are identical.
        for slot in BASE_DEFAULT_SLOTS {
            assert_eq!(
                inherits_base_default(Drive, slot),
                inherits_base_default(Ship, slot),
                "Drive and Ship must agree on slot {}",
                slot.number()
            );
        }

        // overrides_base_default is the exact complement, on every cell.
        for class in LocomotorClass::ALL {
            for slot in BASE_DEFAULT_SLOTS {
                assert_ne!(
                    inherits_base_default(class, slot),
                    overrides_base_default(class, slot),
                    "class={class:?}, slot={}",
                    slot.number()
                );
            }
        }

        // Whole-matrix total, so a compensating pair of edits cannot pass.
        let inherited: usize = LocomotorClass::ALL
            .into_iter()
            .map(|class| {
                BASE_DEFAULT_SLOTS
                    .into_iter()
                    .filter(|slot| inherits_base_default(class, *slot))
                    .count()
            })
            .sum();
        assert_eq!(inherited, 40, "40 of the 72 cells inherit the base body");
    }

    #[test]
    fn nontrivial_base_bodies_preserve_inputs() {
        let coordinate = (-256, 640, 17);
        assert_eq!(head_to_coord(coordinate), coordinate);
        assert!(is_moving_now(true));
        assert!(!is_moving_now(false));
        assert_eq!(can_enter_cell(), 0);
    }
}
