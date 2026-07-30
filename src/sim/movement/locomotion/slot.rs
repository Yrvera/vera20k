//! The installed-locomotor slot: the single authority for which locomotor class
//! a unit runs.
//!
//! Natively a unit holds exactly one locomotor interface pointer, created once
//! in its class constructor from the type's `Locomotor=` CLSID and linked to the
//! owner. There is no second slot and no re-selection: no stock Yuri's Revenge
//! unit is constructed with one locomotor and later permanently swapped to
//! another. The Rust equivalent is therefore a resolved class, not a pointer —
//! the per-class runtime state lives in [`super::super::locomotor`], and a
//! temporary override (the Magnetron lift) is a *stash*, not a replacement, so
//! it belongs to the piggyback mechanism rather than here.

use crate::rules::locomotor_type::LocomotorKind;
use crate::sim::substrate::locomotion::LocomotorClass;

/// The locomotor class installed on a unit at spawn.
///
/// A newtype rather than a bare [`LocomotorClass`] so that "the class this unit
/// was built with" cannot be silently confused with "the class currently
/// driving it" — those differ while a piggyback stash is active, and conflating
/// them is what made the previous `kind`/`primary_kind` pair ambiguous.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct LocomotorSlot {
    installed: LocomotorClass,
}

impl LocomotorSlot {
    /// Install `class` as this unit's locomotor.
    pub const fn new(installed: LocomotorClass) -> Self {
        Self { installed }
    }

    /// The installed class.
    pub const fn installed(self) -> LocomotorClass {
        self.installed
    }
}

impl From<LocomotorSlot> for LocomotorClass {
    fn from(slot: LocomotorSlot) -> Self {
        slot.installed()
    }
}

impl From<LocomotorClass> for LocomotorSlot {
    fn from(class: LocomotorClass) -> Self {
        Self::new(class)
    }
}

impl From<LocomotorSlot> for LocomotorKind {
    /// Bridge to the runtime discriminant the movement systems still key off.
    ///
    /// Total by construction: every [`LocomotorClass`] is a class some stock
    /// unit selects, and each has a `LocomotorKind` counterpart. The dormant
    /// Tiberian Sun kinds have no `LocomotorClass` at all, so they are
    /// unreachable from an installed slot — which is the point.
    fn from(slot: LocomotorSlot) -> Self {
        match slot.installed() {
            LocomotorClass::Drive => Self::Drive,
            LocomotorClass::Hover => Self::Hover,
            LocomotorClass::Walk => Self::Walk,
            LocomotorClass::Fly => Self::Fly,
            LocomotorClass::Teleport => Self::Teleport,
            LocomotorClass::Ship => Self::Ship,
            LocomotorClass::Jumpjet => Self::Jumpjet,
            LocomotorClass::Rocket => Self::Rocket,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_round_trips_through_class() {
        for class in LocomotorClass::ALL {
            let slot = LocomotorSlot::new(class);
            assert_eq!(slot.installed(), class);
            assert_eq!(LocomotorClass::from(slot), class);
        }
    }

    #[test]
    fn every_installable_class_maps_to_a_distinct_runtime_kind() {
        let kinds: Vec<LocomotorKind> = LocomotorClass::ALL
            .into_iter()
            .map(|class| LocomotorKind::from(LocomotorSlot::new(class)))
            .collect();
        let mut unique = kinds.clone();
        unique.sort_by_key(|kind| *kind as u8);
        unique.dedup();
        assert_eq!(
            unique.len(),
            kinds.len(),
            "two installable classes collapsed onto one runtime kind"
        );
    }
}
