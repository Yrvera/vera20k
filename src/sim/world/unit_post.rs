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
//!   6. (no SpawnManager slot here — see below)
//!
//! Correction (verified 2026-08-03): the SpawnManager is NOT a post-Foot
//! `UnitClass::AI` slot. `decompile_function 0x006F9E50` shows
//! `TechnoClass::AI_Update` dispatching it directly — `if (this+0x2D0)
//! (*(vtable+0x5C))()` — after the self-heal/power block and before the cloak
//! block, for every techno rather than for units only. VERA runs it as its own
//! pass right after the combat phase (`sim::spawn_manager::tick_spawn_managers`,
//! called from `World::advance_tick` Phase 5), which preserves the native
//! "this object fired and set its spawn target, then its manager reads it"
//! ordering within the tick.
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

/// Apply the precomputed Unit Facing slot (the write half of the post-Foot
/// Facing slot). `FacingClass::set` is pure in `(state, binary_frame)` and no
/// system writes Unit facings between the combat Phase-2 read window and this
/// site, so the apply point within Phase 5 does not affect the resulting
/// state. Idempotent — `set` is a no-op when the destination already matches.
/// ROT byte refreshed from rules each apply, same as the legacy sweep.
///
/// Three writes land here, in native order:
/// 1. the turret destination from `UnitClass::Facing_Update @ 0x00736990`
///    (`None` = native calls no `Set`, so the turret holds its aim);
/// 2. the hull destination from `UnitClass::Fire_At_Target @ 0x00736DF0` case 2
///    (`FacingClass::Set(+0x388)` at `0x00737004`, then the hull's raw
///    destination copied into the turret slot at `0x0073701C` — `FUN_004C9470`
///    returns the DESTINATION dword, not the animated value);
/// 3. the `+0x6AF` rotation latch (`0x00736AD5`/`0x00736B16`), read next tick by
///    `UnitClass::GetFireError @ 0x00741233`.
///
/// It then mirrors the animated hull back into `entity.facing`, which is VERA's
/// authoritative 8-bit heading. gamemd has no such byte — `+0x388` IS the
/// heading — so the mirror is what keeps rendering, movement and the fire gate
/// on the same value. Units the movement tick owns (`movement_target` set) are
/// skipped: that path already mirrors, and clearing its interpolator here would
/// fight it.
pub(crate) fn apply_unit_facing(
    entities: &mut EntityStore,
    updates: &[crate::sim::combat::UnitFacingUpdate],
    rules: &RuleSet,
    interner: &StringInterner,
    binary_frame: u32,
) {
    for update in updates {
        let id = update.entity_id;
        let rot_byte: u8 = rules
            .object(interner.resolve(entities.get(id).map(|e| e.type_ref).unwrap_or_default()))
            .map(|obj| obj.turret_rot.clamp(0, 0xFF) as u8)
            .unwrap_or(5);
        let Some(entity) = entities.get_mut(id) else {
            continue;
        };
        entity.turret_rotation_latch = update.latch;
        if let Some(desired) = update.hull_destination {
            let hull = entity.body_facing.get_or_insert_with(|| {
                crate::sim::movement::FacingClass::new(u16::from(entity.facing) << 8, rot_byte)
            });
            hull.set_rot(rot_byte);
            hull.set(desired, binary_frame);
            let raw_destination = hull.destination();
            if let Some(ref mut barrel) = entity.barrel_facing {
                barrel.set_rot(rot_byte);
                barrel.set(raw_destination, binary_frame);
            }
        }
        if let Some(desired) = update.turret_destination
            && let Some(ref mut barrel) = entity.barrel_facing
        {
            barrel.set_rot(rot_byte);
            barrel.set(desired, binary_frame);
        }
        // Mirror the animated hull into the 8-bit heading. The movement tick
        // owns that mirror while a path is live.
        if entity.movement_target.is_none()
            && let Some(ref hull) = entity.body_facing
        {
            entity.facing = (hull.current(binary_frame) >> 8) as u8;
        }
    }
}
