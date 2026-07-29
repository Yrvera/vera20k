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

    #[test]
    fn base_default_map_matches_vtables() {
        // PARITY: expected cells are byte-decoded from the base/live-class
        // vtables via `read_memory 0x007EADF4 len 160` and the eleven class
        // vtable reads recorded in design section 2.4. The source matrix was
        // independently re-decoded across all 440 cells.
        let expected = [
            // slot:  6      7      19     20     28     30     31     32     39
            [false, true, false, false, false, true, false, false, false], // Drive
            [false, true, false, true, true, true, true, false, false],    // Hover
            [false, true, false, true, true, false, true, false, false],   // Walk
            [true, true, false, true, true, true, true, false, true],      // Fly
            [true, true, false, true, true, true, true, true, false],      // Teleport
            [false, true, false, false, false, true, false, false, false], // Ship
            [false, true, false, true, true, true, true, false, false],    // Jumpjet
            [true, true, true, true, true, true, true, false, true],       // Rocket
        ];

        for (class_index, class) in LocomotorClass::ALL.into_iter().enumerate() {
            for (slot_index, slot) in BASE_DEFAULT_SLOTS.into_iter().enumerate() {
                assert_eq!(
                    inherits_base_default(class, slot),
                    expected[class_index][slot_index],
                    "class={class:?}, slot={}",
                    slot.number()
                );
                assert_eq!(
                    overrides_base_default(class, slot),
                    !expected[class_index][slot_index],
                    "class={class:?}, slot={}",
                    slot.number()
                );
            }
        }
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
