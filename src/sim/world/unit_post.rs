//! Per-object UnitClass post-Foot host (the write half of the Facing slot).
//!
//! Post-Foot UnitClass slot order (gamemd `UnitClass::AI` steps 3m–3r; see
//! `docs/plans/2026-06-10-s3-unit-postfoot-ordering-design.md`):
//!   1. Fire         — per-attacker in combat Phase 2, live order        [LANDED L2/S3]
//!   2. Facing       — destinations read per-object in the combat Phase-2
//!                     window (pre-death state; kill-tick aim hold),
//!                     applied here post-batch                           [LANDED S3]
//!   3. GuardTerrain — Guard + invalid terrain + sight → self-destroy    [SLOT — UNCHECKED, needs RE]
//!   4. HarvestBrain — idle Harvester/Weeder → Harvest decision          [SLOT — miner substrate owns]
//!   5. Anim/Ammo    — the per-unit anim/ammo wrapper                    [SLOT — target unresolved, needs RE]
//!   6. SpawnManager — Carrier/Dreadnought spawn dispatch                [SLOT — feature absent, named gap]
//! Pre-fire idle turret scan + AI auto-hunt / stuck-harvester rescue are
//! S4 / AI-deferred respectively.
//!
//! AUTHORITATIVE for Unit barrel facing: destinations are computed per-object
//! in `combat::tick_combat_with_fog` Phase 2 (immediately after each Unit
//! attacker's own fire resolution; a residual pass covers target-less and
//! in-transport Units over the same `keys_sorted()` coverage the legacy sweep
//! had) and applied here, after the damage/death batch, at the unchanged
//! write point. Reading pre-death state is the S3 fidelity fix: a unit whose
//! target dies this tick keeps aiming at it this tick; idle-return begins the
//! next tick. `FacingClass::set` is pure in `(state, binary_frame)`, so the
//! apply point within Phase 5 does not change the resulting facing state.
//!
//! Depends on `sim/movement/turret` (facing math). Never depends on
//! render/ui/sidebar/audio/net (sim invariant #1). Dispatch is a
//! `category == Unit` filter — no trait object / dyn (invariant #2).

use crate::rules::ruleset::RuleSet;
use crate::sim::entity_store::EntityStore;
use crate::sim::intern::StringInterner;

/// When true, Unit barrel facing is owned by the per-object path (combat
/// Phase-2 read window + `apply_unit_facing`) and `tick_turret_rotation`
/// skips Units.
pub(crate) const L2_UNIT_POST_AUTHORITATIVE: bool = true;

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
