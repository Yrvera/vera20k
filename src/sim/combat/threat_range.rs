//! Scan radius for passive target acquisition.
//!
//! Retail computes the acquisition radius *inside* the threat scan, from a
//! small mask the calling mission supplies — no caller passes a range. This
//! module is that computation plus the mask selection, kept as a mechanism
//! rather than folded into the candidate loop because the two missions that
//! reach the scanner produce genuinely different filters.
//!
//! ## The two filters
//!
//! - **Guard** (and the other plain passive-acquire missions) asks for the
//!   object's own `GuardRange=`. When the type has none, the computed radius is
//!   zero, and a zero radius means *no distance cutoff of its own*: acceptance
//!   falls through to the attacker's own can-fire-at-this-target query, i.e.
//!   the range of the weapon picked against that specific candidate. That is
//!   [`ScanRange::CanFireAt`].
//! - **Area Guard** asks for `GuardRange=` **or** the wider of the type's two
//!   weapon ranges, **doubled**, capped at [`AREA_GUARD_MAX_SCAN_CELLS`]. The
//!   result is always non-zero, and a non-zero radius is a hard Euclidean
//!   cutoff — the can-fire-at query is not consulted at all. This is why a unit
//!   parked on Area Guard reaches out roughly twice as far as the same unit
//!   sitting on plain Guard.
//!
//! The doubling applies to `GuardRange=` too, not just to the weapon-range
//! fallback.
//!
//! ## The third mask: Hunt asks for no filter and no ring walk
//!
//! `FootClass::Mission_Hunt @ 0x004D5373` pushes the literal `0`, and mask 0
//! does not select a third radius formula — it skips the radius block, the
//! airborne pre-pass and the ring walk outright and enumerates the global
//! object array instead, passing a literal `-1` where the ring path passes its
//! computed radius. That is a scan *topology*, so it lives in
//! [`super::greatest_threat`]; [`ScanRange::NoCutoff`] is only the
//! per-candidate half of it. See [`ScanMission::Hunt`].
//!
//! ## Why plain Guard really is `CanFireAt` — the mask literals
//!
//! This has now been challenged twice, so the chain is written down here.
//!
//! The threat scan takes a bitmask, and the radius formula is selected from it:
//! **bit0 set → the narrow formula; bit0 clear and bit1 set → the doubled one**
//! (with a Patrol-only third variant). Nobody chooses that mask at the scan —
//! the *caller* supplies it as a literal, and there are exactly two callers:
//!
//! - The common Techno AI body, which is the ONLY route to the scan for
//!   missions {Move, Harvest, Guard} — those three ids and no others — pushes
//!   the literal **1** together with the object's own coordinates.
//! - `FootClass::Mission_AreaGuard`, which pushes the literal **2** together
//!   with the guard post's coordinates.
//!
//! So Guard's mask cannot carry bit1: it is a hardcoded `1` in the caller.
//! Guard therefore takes the narrow formula, which for a type with no
//! `GuardRange=` computes zero — and radius zero is what defers acceptance to
//! the attacker's own can-fire-at query. The doubling belongs to Area Guard
//! (and Patrol) alone.
//!
//! The counter-argument — "the FootClass override that rewrites the mask only
//! makes sense if Guard's mask carried bit1" — does not hold: that override
//! reads `mask & ~bit1 | bit0`, i.e. it can only ever *downgrade* a bit1 caller
//! to the narrow formula, never add bit1 to a bit0 one. Its purpose is the
//! freshly-moved latch below, whose only bit1 callers are Area Guard and
//! Patrol. The same override clears the latch when a scan comes back empty.
//!
//! ## What is deliberately NOT modelled
//!
//! **The freshly-moved latch.** Mobile objects (not buildings) carry a flag the
//! locomotors raise whenever they process movement. While it is raised, the
//! scan's mask is forced down to the plain-Guard one, so a unit that has just
//! moved takes the narrow radius even on Area Guard; the first scan that finds
//! nothing lowers the flag, and the wide radius applies from the next scan on.
//! Its observable effect is to delay the widening until the first scan that
//! finds nothing — usually one cadence after each movement, so a parked guard
//! reaches its doubled radius on its second scan; but the clear is conditional
//! on the *result*, not on the scan happening (`TEST EAX,EAX ; JNZ 0x004D995B`
//! at `0x004D9951` skips the store), so a unit that keeps finding targets at
//! the narrow radius keeps the narrow radius. VERA has no such field, and
//! adding one
//! means writes from the locomotors plus a snapshot field, so it is recorded
//! here rather than modelled: units acquire at the wide radius at least one
//! cadence sooner than retail after they stop moving.
//!
//! **The unarmed-Guard override.** When the scanning object has no usable
//! weapon at all *and* its mission is exactly Guard, retail forces the radius
//! to a flat 2 cells instead of computing one. VERA never reaches this: the
//! base can-acquire predicate already requires a weapon slot, so an unarmed
//! object never scans. Recorded because it is the only other place the Guard
//! mission id is read inside the scan.
//!
//! Retail has a third radius formula, reached only when the scanning object is
//! on **Patrol**: the same doubled-and-capped value as Area Guard but with a
//! 7-cell *floor* underneath it. Nothing in VERA assigns the Patrol mission —
//! the enum variant exists with no writer — so that branch is unreachable and
//! is not represented here. It becomes real the day a Patrol handler lands.
//!
//! Retail walks outward cell by cell and bounds that walk at
//! `wider weapon range + 1 + AirRangeBonus` cells. That number is a **search**
//! bound, not an acceptance radius: the walk is a Chebyshev square strictly
//! wider than the reach acceptance would allow, and it exists only on the
//! radius-zero branch, so it never clips a candidate acceptance would have
//! taken. Reading that bound as the acquisition radius is a recurring wrong
//! turn — it is the reason this file exists rather than a widened per-candidate
//! range. [`super::greatest_threat`] owns the walk and applies that bound;
//! this file owns the acceptance radius the walk carries into each candidate.
//!
//! ## Dependency rules
//! - Part of sim/ — depends on rules/ (RuleSet, ObjectType) and sim/ only.
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

use super::combat_weapon::{primary_for_tier, secondary_for_tier};
use crate::rules::object_type::ObjectType;
use crate::rules::ruleset::RuleSet;
use crate::sim::components::OrderIntent;
use crate::sim::game_entity::GameEntity;
use crate::sim::mission::MissionType;
use crate::util::fixed_math::SimFixed;

/// Retail doubles the Area Guard radius before clamping it.
const AREA_GUARD_RANGE_MULTIPLIER: i32 = 2;

/// Ceiling on the doubled Area Guard radius, in cells. Retail clamps the
/// lepton value to 0x1000; at 256 leptons per cell that is 16 cells.
const AREA_GUARD_MAX_SCAN_CELLS: i32 = 16;

/// The threat mask a callsite pushes into the scan — retail's `Greatest_Threat`
/// argument 2, a literal at every callsite, named here after the mission that
/// pushes it. It selects the radius formula and, for [`ScanMission::Hunt`], the
/// scan topology. It is NOT a property of the object being scanned for, nor
/// read off the scanner's mission field: two callers can hand the same object
/// different masks, and `FUN_0051F330` does exactly that when it re-acquires in
/// place for a deployed infantryman that is sitting on Area Guard.
///
/// **The literal is not always what `TechnoClass::Greatest_Threat` receives.**
/// A `FootClass` dispatch goes through a `+0x3C4` override first, and two of
/// them rewrite it on the way:
/// - `UnitClass @ 0x00743190` and the Infantry override (`CALL @ 0x0051E39F`)
///   OR the attacker's own projectile class bits in (`FUN_00772A90`, AA → `4`,
///   AG → `0xB8`) when `mask & 0x1B978 == 0`, which mask 0 satisfies. The
///   topology survives — neither value carries `TEST AL,0x3` — but the derived
///   flags word changes.
/// - `FootClass::Greatest_Threat @ 0x004D9931` coerces the mask to
///   `(mask & ~2) | 1` while `FootClass+0x688` is set, turning mask 0 into mask
///   1 and so the flat walk into the ring walk, and clears that byte at
///   `0x004D9955` when the scan returns nothing.
///
/// Neither rewrite is modelled; both are recorded as residuals on
/// [`super::greatest_threat::greatest_threat`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanMission {
    /// Guard, Move, Harvest — the plain passive-acquire missions. All of them
    /// take the same mask, so they share one variant.
    Guard,
    /// Area Guard — "hold this spot and cover it".
    AreaGuard,
    /// Hunt — mask **0**, which is not a third radius formula but the absence
    /// of one.
    ///
    /// `FootClass::Mission_Hunt @ 0x004D5373` pushes the literal `0` as the
    /// threat mask (`disassemble_function 0x004D5350`), and
    /// `TechnoClass::Greatest_Threat @ 0x006F8FE0` opens its radius block with
    /// `TEST AL,0x3 ; JZ 0x006F9B6E` — with neither bit set it jumps **past**
    /// both the `Threat_Range` selection (`vt+0x31C` at `0x006F900A` /
    /// `0x006F9018`, result stored into the range slot `[ESP+0x2C]` at
    /// `0x006F9020`) and the cell walk that consumes it.
    ///
    /// Where mask 0 actually lands: at `0x006F9B6E` the **first** global walk
    /// (the `0x00A8E394` list) is gated on `TEST AL,0x4` against the derived
    /// flags word `[ESP+0x14]` — zeroed at entry (`XOR EDI,EDI @ 0x006F8DFD`,
    /// stored at `0x006F8F2C`) and thereafter built from the mask alone
    /// (`0x006F8F29`-`0x006F8F72`). For mask 0 it is 0, the bit is clear, and
    /// that walk is **skipped** (`JZ 0x006F9C56`). Execution falls through
    /// `0x006F9C56` (`TEST byte ptr [ESP+0x70],0x10 ; JZ 0x006F9C67`) into the
    /// **unconditional** second walk over the `0x00A8EC7C` list at `0x006F9C67`.
    ///
    /// What removes the cutoff is that walk's range argument, not the absence
    /// of a distance test: it passes the literal `PUSH -0x1` (`0x006F9D70`) in
    /// the argument slot where the radius path passes its computed range
    /// (`MOV ECX,[ESP+0x2C] @ 0x006F9292`, `PUSH ECX @ 0x006F92A7`).
    /// `TechnoClass::Evaluate_Candidate @ 0x006F7CA0` rejects on distance only
    /// when that argument is `> 0`, and falls back to the
    /// `TechnoType+0x5B8` / `vt+0x3A8` (Sight / can-fire-at) test only when it
    /// is `== 0`; `-1` trips neither. So a hunting object has no *distance*
    /// cutoff at all — that is the mechanism that sends a berserked unit across
    /// the map.
    ///
    /// It is not unfiltered, though. The same walk that removes the distance
    /// cutoff switches a **movement-zone** gate on: it passes the scanner's own
    /// zone id (`MapClass::GetZoneID @ 0x006F8EBF`, stored at `0x006F8EC4`) in
    /// `Evaluate_Candidate`'s arg6 (`PUSH ECX @ 0x006F9D69`) where every ring
    /// callsite passes `-1`, and a non-`-1` arg6 rejects any candidate whose
    /// cell is in a different component under the attacker's own
    /// `MovementZone=` (`0x006F7E7E`-`0x006F7E9C`). A hunter reaches the far
    /// side of the map but not the far side of a river.
    ///
    /// **Mask 0 is a scan TOPOLOGY, not a radius.** The jump at `0x006F8FE2`
    /// lands past the airborne pre-pass *and* past the expanding-ring cell walk
    /// (`0x006F9169` and `0x006F94D0` both sit below `0x006F9B6E`), so a
    /// hunting object never walks a single cell ring — it walks the global
    /// object array. [`super::greatest_threat`] owns that branch; this variant
    /// only names which mask the caller pushed.
    Hunt,
}

/// The distance filter the candidate acceptance test applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScanRange {
    /// Radius zero: no cutoff of the scan's own. Acceptance defers to whether
    /// the attacker can actually fire at that candidate, which is a per-
    /// candidate question because weapon choice depends on the target.
    CanFireAt,
    /// A hard Euclidean cutoff, in cells. The can-fire-at query is skipped.
    Hard(SimFixed),
    /// Native's literal `-1` range argument (`PUSH -0x1 @ 0x006F9D70`), which
    /// `Evaluate_Candidate` treats as neither `> 0` (no distance reject) nor
    /// `== 0` (no Sight / can-fire-at fallback). Weapon legality still decides;
    /// distance does not. Only mask 0 produces it, and it selects the flat
    /// global-list topology in [`super::greatest_threat`] as well as the
    /// per-candidate predicate.
    NoCutoff,
}

/// The mask the *passive* acquisition callsites push.
///
/// In retail no callsite derives the mask from the object it is scanning for —
/// it is a literal at the caller (what the `+0x3C4` overrides then do to that
/// literal is documented on [`ScanMission`]), and the
/// three literals are `1` (the common Techno AI body, the only route to the
/// scan for missions Move/Guard/Harvest, and `FUN_0051F330`'s in-place
/// re-acquire), `2` (`FootClass::Mission_AreaGuard`) and `0`
/// (`FootClass::Mission_Hunt @ 0x004D5373`). This function is therefore not a
/// general "what mask is this object on" reader: it is the *passive* block's
/// own choice between its literal `1` and the Area Guard handler's literal `2`,
/// and it is called BY those callsites. Hunt and the deploy shim pass their own
/// literals and never come through here — reading [`ScanMission::Hunt`] off an
/// entity's mission field would make the mask a property of the mission, which
/// it is not.
///
/// Area Guard has two representations here and both mean the same retail
/// mission: the committed mission substrate value, and the `OrderIntent::Guard`
/// anchor a player "guard this spot" order installs. `OrderIntent` still owns
/// the anchor coordinates because the mission substrate has no goal field yet,
/// so an entity on a player guard order carries the intent and *not* the
/// mission id — reading only the substrate would miss every player-issued
/// guard.
pub(crate) fn scan_mission_for(entity: &GameEntity) -> ScanMission {
    let committed_area_guard = entity.mission.current().known() == Some(MissionType::AreaGuard);
    let ordered_area_guard = matches!(entity.order_intent, Some(OrderIntent::Guard { .. }));
    if committed_area_guard || ordered_area_guard {
        ScanMission::AreaGuard
    } else {
        ScanMission::Guard
    }
}

/// Acquisition radius for one scanning object.
pub(crate) fn scan_range(
    rules: &RuleSet,
    obj: &ObjectType,
    veterancy: u16,
    mission: ScanMission,
) -> ScanRange {
    let guard_range = obj.guard_range.filter(|gr| *gr != SimFixed::ZERO);
    match mission {
        // Radius = GuardRange when set, else zero. Retail also consults a
        // per-class predicate that would suppress GuardRange here, but it is a
        // constant `false` on every class whose vtable carries this scan except
        // infantry, where it forwards an infantry-type flag; VERA does not
        // model that flag, so the suppression is not represented.
        ScanMission::Guard => match guard_range {
            Some(gr) => ScanRange::Hard(gr),
            None => ScanRange::CanFireAt,
        },
        // Mask 0 never reaches the radius block at all: `TEST AL,0x3 ; JZ
        // 0x006F9B6E` at `0x006F8FE0` jumps past `Threat_Range` *and* past the
        // ring walk, so no radius is computed for Hunt and `GuardRange=` is not
        // consulted — a `GuardRange=9` V3 on Hunt is no more limited than a
        // Grizzly is. `greatest_threat` branches on the mask before it asks for
        // a radius, so this arm is not on the live path; it carries native's
        // literal `PUSH -0x1` (`0x006F9D70`) so that a caller which does ask
        // gets the same answer the flat walk hardcodes.
        ScanMission::Hunt => ScanRange::NoCutoff,
        ScanMission::AreaGuard => {
            let base = guard_range.unwrap_or_else(|| max_weapon_range(rules, obj, veterancy));
            let doubled = base.saturating_mul(SimFixed::from_num(AREA_GUARD_RANGE_MULTIPLIER));
            ScanRange::Hard(doubled.min(SimFixed::from_num(AREA_GUARD_MAX_SCAN_CELLS)))
        }
    }
}

/// The wider of the type's two weapon slots, elite-swapped at elite veterancy.
/// A slot with no weapon contributes nothing; a type with neither contributes
/// zero, which is harmless because such a type never selects a weapon against
/// any candidate and so never accepts one.
pub(crate) fn max_weapon_range(rules: &RuleSet, obj: &ObjectType, veterancy: u16) -> SimFixed {
    let slot_range = |weapon_id: Option<&str>| -> Option<SimFixed> {
        weapon_id
            .and_then(|id| rules.weapon(id))
            .map(|weapon| weapon.range)
    };
    let primary = slot_range(primary_for_tier(obj, veterancy));
    let secondary = slot_range(secondary_for_tier(obj, veterancy));
    match (primary, secondary) {
        (Some(p), Some(s)) => p.max(s),
        (Some(p), None) => p,
        (None, Some(s)) => s,
        (None, None) => SimFixed::ZERO,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::ini_parser::IniFile;
    use crate::rules::ruleset::RuleSet;

    /// Three armed types: one with no `GuardRange=`, one with `GuardRange=9`
    /// (the retail V3 Launcher value), one with a single weapon slot. Weapon
    /// ranges are deliberately asymmetric so `max(primary, secondary)` is
    /// observable and slot order cannot be mistaken for the answer.
    fn test_rules() -> RuleSet {
        let ini_str: &str = "\
[VehicleTypes]\n0=NOGUARD\n1=WITHGUARD\n2=ONLYPRIMARY\n\n\
[NOGUARD]\nStrength=100\nArmor=heavy\nSpeed=6\nPrimary=ShortGun\nSecondary=LongGun\n\n\
[WITHGUARD]\nStrength=100\nArmor=heavy\nSpeed=6\nPrimary=ShortGun\nSecondary=LongGun\n\
GuardRange=9\n\n\
[ONLYPRIMARY]\nStrength=100\nArmor=heavy\nSpeed=6\nPrimary=ShortGun\n\n\
[ShortGun]\nDamage=50\nROF=20\nRange=4\nWarhead=AP\n\n\
[LongGun]\nDamage=50\nROF=20\nRange=6\nWarhead=AP\n\n\
[AP]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n";
        RuleSet::from_ini(&IniFile::from_str(ini_str))
            .expect("threat-range test rules should parse")
    }

    fn obj<'a>(rules: &'a RuleSet, id: &str) -> &'a ObjectType {
        rules.object(id).expect("test type present")
    }

    #[test]
    fn guard_without_guard_range_defers_to_can_fire_at() {
        // Retail radius 0 — the acceptance test falls through to the
        // attacker's own can-fire-at query rather than applying a cutoff.
        let rules = test_rules();
        assert_eq!(
            scan_range(&rules, obj(&rules, "NOGUARD"), 0, ScanMission::Guard),
            ScanRange::CanFireAt
        );
    }

    /// The mask literals are the whole argument, so pin the consequence: for
    /// one and the same type, Guard and Area Guard must NOT resolve to the same
    /// filter. Guard's caller pushes mask 1 (narrow formula → radius 0 for a
    /// type with no `GuardRange=` → defer to can-fire-at); Area Guard's pushes
    /// mask 2 (doubled formula → a hard cutoff). A regression that made plain
    /// Guard take the doubled radius would collapse these two into one value.
    #[test]
    fn guard_and_area_guard_do_not_share_a_filter() {
        let rules = test_rules();
        for type_id in ["NOGUARD", "WITHGUARD", "ONLYPRIMARY"] {
            let guard = scan_range(&rules, obj(&rules, type_id), 0, ScanMission::Guard);
            let area = scan_range(&rules, obj(&rules, type_id), 0, ScanMission::AreaGuard);
            assert_ne!(
                guard, area,
                "{type_id}: Guard (mask 1) and Area Guard (mask 2) select different formulas"
            );
            assert!(
                matches!(area, ScanRange::Hard(_)),
                "{type_id}: Area Guard is always a hard cutoff"
            );
        }
        // And the doubling is Area Guard's alone: a type WITH GuardRange keeps
        // it undoubled on Guard.
        assert_eq!(
            scan_range(&rules, obj(&rules, "WITHGUARD"), 0, ScanMission::Guard),
            ScanRange::Hard(SimFixed::from_num(9))
        );
    }

    #[test]
    fn guard_with_guard_range_is_a_hard_undoubled_cutoff() {
        // GuardRange=9 on plain Guard is used as-is. The doubling belongs to
        // the Area Guard branch only — a V3 on Guard scans 9 cells, not 18.
        let rules = test_rules();
        assert_eq!(
            scan_range(&rules, obj(&rules, "WITHGUARD"), 0, ScanMission::Guard),
            ScanRange::Hard(SimFixed::from_num(9))
        );
    }

    #[test]
    fn area_guard_without_guard_range_doubles_the_wider_weapon() {
        // max(Primary 4, Secondary 6) = 6, doubled = 12. Note it is the wider
        // of the two SLOTS, not the weapon that would be picked against any
        // particular candidate.
        let rules = test_rules();
        assert_eq!(
            scan_range(&rules, obj(&rules, "NOGUARD"), 0, ScanMission::AreaGuard),
            ScanRange::Hard(SimFixed::from_num(12))
        );
    }

    #[test]
    fn area_guard_prefers_guard_range_over_the_weapon_and_clamps_the_double() {
        // GuardRange=9 wins over max weapon range 6 (proved by the result
        // exceeding the 2 * 6 = 12 the weapon path would give), is itself
        // doubled to 18, and is then clamped to the 16-cell ceiling.
        let rules = test_rules();
        let clamped = scan_range(&rules, obj(&rules, "WITHGUARD"), 0, ScanMission::AreaGuard);
        let ScanRange::Hard(cells) = clamped else {
            panic!("Area Guard always produces a hard cutoff");
        };
        assert!(
            cells > SimFixed::from_num(12),
            "GuardRange must win: {cells}"
        );
        assert_eq!(clamped, ScanRange::Hard(SimFixed::from_num(16)));
    }

    #[test]
    fn area_guard_uses_the_only_slot_when_the_type_has_one_weapon() {
        // Primary 4 only -> 8. A missing Secondary must not drag the max to 0.
        let rules = test_rules();
        assert_eq!(
            scan_range(
                &rules,
                obj(&rules, "ONLYPRIMARY"),
                0,
                ScanMission::AreaGuard
            ),
            ScanRange::Hard(SimFixed::from_num(8))
        );
    }
}
