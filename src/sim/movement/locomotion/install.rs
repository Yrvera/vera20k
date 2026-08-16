//! Resolving a type's installed locomotor from its `Locomotor=` key.
//!
//! ## The native mechanism, and why there is only one fallback
//!
//! It is tempting to model this as two rules — "no key means X" and "a bad
//! CLSID means Y". The original engine has neither. It has one field and one
//! default:
//!
//! 1. The type constructor seeds the type's locomotor-CLSID field with the
//!    **Teleport** GUID, before any INI is read. That is a plain 16-byte copy
//!    from a static GUID, not a category lookup.
//! 2. The INI read then passes **the field's current value as the default
//!    argument** to the CLSID reader, and copies the reader's result back.
//!
//! So an absent key and an unparseable value take the same path: the reader
//! returns the default it was handed, which is still the constructor's Teleport
//! seed. One mechanism, no branch, and no dependence on the unit's category.
//!
//! This is why the previous per-category fallback — infantry to Walk, vehicles
//! to Drive, aircraft to Fly — was wrong. It was invented VERA-side and has no
//! counterpart in the original. In stock Yuri's Revenge the divergence is nearly
//! unreachable: of the 157 units on the `InfantryTypes`/`VehicleTypes`/
//! `AircraftTypes` rosters, exactly one — `DeathDummy`, an internal type — omits
//! `Locomotor=`. Every unit a player builds names its CLSID explicitly,
//! including the six Teleport ones. So this corrects the rule, not the game.

use crate::rules::locomotor_type::LocomotorKind;
use crate::sim::substrate::locomotion::{LocomotorClass, class_from_clsid};

use super::slot::LocomotorSlot;

/// The class a type installs when its `Locomotor=` value is absent or does not
/// parse: the constructor's seed.
pub const DEFAULT_INSTALLED_CLASS: LocomotorClass = LocomotorClass::Teleport;

/// Resolve the locomotor class a type installs at spawn.
///
/// `value` is the raw `Locomotor=` text, or `None` when the key is absent. Both
/// absence and an unrecognised CLSID yield [`DEFAULT_INSTALLED_CLASS`], matching
/// the native read-with-current-value-as-default described in the module docs.
pub fn resolve_installed_class(value: Option<&str>) -> LocomotorClass {
    value
        .and_then(class_from_clsid)
        .unwrap_or(DEFAULT_INSTALLED_CLASS)
}

/// [`resolve_installed_class`], as the runtime discriminant the movement
/// systems consume.
pub fn resolve_installed_kind(value: Option<&str>) -> LocomotorKind {
    LocomotorSlot::new(resolve_installed_class(value)).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The retail CLSID every Teleport unit names, in the exact spelling the
    /// INI uses (lower-case `11d1`, braces present).
    const TELEPORT_CLSID: &str = "{4A582747-9839-11d1-B709-00A024DDAFD1}";
    const DRIVE_CLSID: &str = "{4A582741-9839-11D1-B709-00A024DDAFD1}";

    #[test]
    fn missing_locomotor_key_defaults_to_teleport() {
        assert_eq!(resolve_installed_class(None), LocomotorClass::Teleport);
    }

    /// The old per-category fallback would have answered Walk for infantry,
    /// Drive for vehicles and Fly for aircraft. The native default has no
    /// category input at all, so there is nothing to vary here — one answer.
    #[test]
    fn default_does_not_depend_on_any_category_input() {
        assert_eq!(
            resolve_installed_class(None),
            DEFAULT_INSTALLED_CLASS,
            "the default is a constructor seed, not a per-category lookup"
        );
        assert_ne!(DEFAULT_INSTALLED_CLASS, LocomotorClass::Walk);
        assert_ne!(DEFAULT_INSTALLED_CLASS, LocomotorClass::Drive);
        assert_ne!(DEFAULT_INSTALLED_CLASS, LocomotorClass::Fly);
    }

    #[test]
    fn unparseable_clsid_falls_back_silently() {
        for bad in [
            "",
            "   ",
            "not-a-guid",
            "{00000000-0000-0000-0000-000000000000}",
            "{4A582741-9839-11D1-B709-00A024DDAF}", // truncated
            "4A582741",
        ] {
            assert_eq!(
                resolve_installed_class(Some(bad)),
                LocomotorClass::Teleport,
                "unparseable CLSID {bad:?} must fall back to the constructor seed"
            );
        }
    }

    #[test]
    fn recognised_clsid_wins_over_the_default() {
        assert_eq!(
            resolve_installed_class(Some(DRIVE_CLSID)),
            LocomotorClass::Drive
        );
        assert_eq!(
            resolve_installed_class(Some(TELEPORT_CLSID)),
            LocomotorClass::Teleport
        );
    }

    /// The six stock sections that run a Teleport locomotor. Four name the CLSID
    /// with a trailing comment, and two of those spell `11d1` in lower case —
    /// both are exercised here because an earlier reading of these sections got
    /// them wrong by matching section headers too strictly.
    #[test]
    fn six_stock_sections_resolve_to_teleport() {
        for section_value in [
            TELEPORT_CLSID,                           // CLEG
            "{4A582747-9839-11d1-B709-00A024DDAFD1}", // CCOMAND
            "{4A582747-9839-11d1-B709-00A024DDAFD1}", // CIVAN
            "{4A582747-9839-11d1-B709-00A024DDAFD1}", // CMIN
            "{4A582747-9839-11d1-B709-00A024DDAFD1}", // CMON
            "{4A582747-9839-11D1-B709-00A024DDAFD1}", // SMON
        ] {
            assert_eq!(
                resolve_installed_class(Some(section_value)),
                LocomotorClass::Teleport
            );
        }
    }

    #[test]
    fn resolved_kind_matches_resolved_class() {
        assert_eq!(resolve_installed_kind(None), LocomotorKind::Teleport);
        assert_eq!(
            resolve_installed_kind(Some(DRIVE_CLSID)),
            LocomotorKind::Drive
        );
    }

    /// Lock the rules-owned install table (the production parsing path used by
    /// `ObjectType`) to the substrate's class table so the two cannot drift:
    /// same row count, same CLSID -> discriminant on both paths, same seed
    /// fallback, and the dormant Mech CLSID resolving nowhere on either.
    #[test]
    fn install_tables_agree_with_rules_kind_table() {
        use crate::rules::locomotor_type::{
            INSTALLED_CLSID_KIND_TABLE, kind_from_clsid,
            resolve_installed_kind as rules_resolve_installed_kind,
        };
        use crate::sim::substrate::locomotion::CLSID_CLASS_TABLE;

        assert_eq!(CLSID_CLASS_TABLE.len(), INSTALLED_CLSID_KIND_TABLE.len());
        for &(clsid, class) in &CLSID_CLASS_TABLE {
            let kind: LocomotorKind = LocomotorSlot::new(class).into();
            assert_eq!(kind_from_clsid(clsid), Some(kind), "CLSID: {clsid}");
            assert_eq!(
                rules_resolve_installed_kind(Some(clsid)),
                resolve_installed_kind(Some(clsid)),
                "CLSID: {clsid}"
            );
        }
        for &(clsid, kind) in &INSTALLED_CLSID_KIND_TABLE {
            assert_eq!(
                class_from_clsid(clsid).map(|class| LocomotorSlot::new(class).into()),
                Some(kind),
                "CLSID: {clsid}"
            );
        }
        assert_eq!(rules_resolve_installed_kind(None), resolve_installed_kind(None));

        let mech = "{55D141B8-DB94-11D1-AC98-006008055BB5}";
        assert_eq!(kind_from_clsid(mech), None);
        assert_eq!(class_from_clsid(mech), None);
    }
}
