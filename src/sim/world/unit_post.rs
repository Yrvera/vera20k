//! Per-object UnitClass turret facing step (the Facing_Update half of gamemd's
//! `UnitClass::AI`).
//!
//! AUTHORITATIVE for Unit barrel facing: `tick_unit_facing` drives every Unit's barrel
//! toward its target (or back to body facing when idle), replacing the Unit arm of the
//! global `tick_turret_rotation` sweep (which now skips Units). It iterates the same
//! `keys_sorted()` id-order over the same entity set the sweep used, restricted to
//! Units, so the processed set is bit-identical. Unit FIRE stays in the shared,
//! category-agnostic `combat::resolve_attacker_fire` path — there is no Unit-specific
//! fire behavior to own yet; the fire seam moves here when the S4 per-object walk
//! introduces it.
//!
//! The flip is hash-neutral: `desired_turret_facing` is per-entity, and the set + order
//! match the legacy sweep, so each Unit's barrel destination is unchanged. No
//! `SNAPSHOT_VERSION` bump.
//!
//! Depends on `sim/movement/turret` (facing helper). Never depends on
//! render/ui/sidebar/audio/net (sim invariant #1). Dispatch is a `category == Unit`
//! filter — no trait object / dyn (invariant #2).

use crate::map::entities::EntityCategory;
use crate::rules::ruleset::RuleSet;
use crate::sim::entity_store::EntityStore;
use crate::sim::intern::StringInterner;
use crate::sim::movement::turret;

/// When true, `tick_unit_facing` is authoritative for Unit barrel facing and
/// `tick_turret_rotation` skips Units. The flip is hash-neutral (per-entity facing,
/// shadow-proven); no `SNAPSHOT_VERSION` bump.
pub(crate) const L2_UNIT_POST_AUTHORITATIVE: bool = true;

/// Drive every Unit's barrel toward its desired facing. Mirrors `tick_turret_rotation`
/// exactly (same `keys_sorted()` id-order, same two-phase read-then-apply, same
/// idempotent `FacingClass::set`) but restricted to Units — the global sweep, now
/// Unit-skipping, handles every other category. Iterating `keys_sorted()` (not the
/// LogicVector) keeps the processed set bit-identical to the legacy sweep, so the flip
/// is output-neutral. Called once per tick from Phase 5, after combat (reads
/// post-combat `attack_target` and target positions, exactly as the legacy sweep did).
pub(crate) fn tick_unit_facing(
    entities: &mut EntityStore,
    rules: &RuleSet,
    interner: &StringInterner,
    binary_frame: u32,
) {
    // Phase 1: read each Unit's desired barrel facing (id-ascending, like the sweep).
    let keys: Vec<u64> = entities.keys_sorted();
    let mut updates: Vec<(u64, u16)> = Vec::new();
    for &id in &keys {
        let Some(entity) = entities.get(id) else {
            continue;
        };
        if entity.category != EntityCategory::Unit {
            continue;
        }
        // None when not turreted; otherwise faces the target (or body when idle).
        let Some(desired) = turret::desired_turret_facing(entity, entities) else {
            continue;
        };
        updates.push((id, desired));
    }

    // Phase 2: apply via the shared write half.
    apply_unit_facing(entities, &updates, rules, interner, binary_frame);
}

/// Apply precomputed Unit barrel destinations (the write half of the post-Foot
/// Facing slot). `FacingClass::set` is pure in `(state, binary_frame)` and no
/// system writes Unit barrels between the combat Phase-2 read window and this
/// site, so the apply point within Phase 5 does not affect the resulting
/// state. Idempotent — `set` is a no-op when the destination already matches.
/// ROT byte refreshed from rules each apply, same as the legacy sweep.
pub(crate) fn apply_unit_facing(
    entities: &mut EntityStore,
    updates: &[(u64, u16)],
    rules: &RuleSet,
    interner: &StringInterner,
    binary_frame: u32,
) {
    for &(id, desired) in updates {
        let rot_byte: u8 = rules
            .object(
                interner.resolve(entities.get(id).map(|e| e.type_ref).unwrap_or_default()),
            )
            .map(|obj| obj.turret_rot.clamp(0, 0xFF) as u8)
            .unwrap_or(5);
        if let Some(entity) = entities.get_mut(id) {
            if let Some(ref mut barrel) = entity.barrel_facing {
                barrel.set_rot(rot_byte);
                barrel.set(desired, binary_frame);
            }
        }
    }
}
