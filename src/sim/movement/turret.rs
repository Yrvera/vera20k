//! Turret rotation system — rotates turrets toward attack targets or back to body facing.
//!
//! Units with a barrel `FacingClass` have an independently rotating turret.
//! When attacking, the turret rotates toward the target at the unit's ROT
//! speed. When idle, it returns to body facing. The weapon-fire alignment
//! check is performed in combat.rs against the FacingClass animated value.
//!
//! ## Dependency rules
//! - Part of sim/ — depends on sim/components, sim/combat, rules/.
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

use crate::rules::ruleset::RuleSet;
use crate::sim::entity_store::EntityStore;
use crate::sim::game_entity::GameEntity;
use crate::util::fixed_math::{SimFixed, facing_from_delta_int_u16};

/// Compute the signed shortest-path rotation from `current` to `target` in 8-bit facing space.
/// Returns a value in -128..=127 (positive = clockwise, negative = counter-clockwise).
pub fn shortest_rotation(current: u8, target: u8) -> i16 {
    let diff: i16 = target as i16 - current as i16;
    // Wrap into -128..127 range for shortest path.
    if diff > 128 {
        diff - 256
    } else if diff < -128 {
        diff + 256
    } else {
        diff
    }
}

/// Convert ROT (degrees/frame at 15fps) + tick_ms into an 8-bit facing delta per tick.
/// Returns the maximum facing units the entity can rotate this tick.
pub fn rot_to_facing_delta(rot: i32, tick_ms: u32) -> u8 {
    if rot <= 0 || tick_ms == 0 {
        return 0;
    }
    // ROT degrees/frame * 15 frames/sec = degrees/sec
    // degrees/sec * tick_ms/1000 = degrees this tick
    // degrees * 256/360 = facing units this tick
    let numerator: u64 = rot as u64 * 256 * 15 * tick_ms as u64;
    let denominator: u64 = 360 * 1000;
    let delta: u64 = numerator.div_ceil(denominator);
    delta.clamp(1, 128) as u8
}

/// Compute 16-bit turret facing from source to target using lepton-precise
/// positions, providing sub-cell accuracy for targeting.
pub fn facing_toward_lepton(
    from_rx: u16,
    from_ry: u16,
    from_sub_x: SimFixed,
    from_sub_y: SimFixed,
    to_rx: u16,
    to_ry: u16,
    to_sub_x: SimFixed,
    to_sub_y: SimFixed,
) -> u16 {
    let from_lep_x: i32 = from_rx as i32 * 256 + from_sub_x.to_num::<i32>();
    let from_lep_y: i32 = from_ry as i32 * 256 + from_sub_y.to_num::<i32>();
    let to_lep_x: i32 = to_rx as i32 * 256 + to_sub_x.to_num::<i32>();
    let to_lep_y: i32 = to_ry as i32 * 256 + to_sub_y.to_num::<i32>();
    let dx: i32 = to_lep_x - from_lep_x;
    let dy: i32 = to_lep_y - from_lep_y;
    facing_from_delta_int_u16(dx, dy)
}

/// Convert 8-bit body facing to 16-bit turret facing.
/// Maps 0..255 → 0..65280 (shifts into the upper byte).
#[inline]
pub fn body_facing_to_turret(body: u8) -> u16 {
    (body as u16) << 8
}

/// The barrel facing `tick_turret_rotation` drives this entity toward this tick:
/// toward its attack target (lepton-precise) when it has one, else back to body
/// facing. `None` when the entity has no turret. PURE READ — mutates neither the
/// entity nor the store. Shared by the global turret sweep and the per-object
/// Fire→Facing host so both compute identical barrel destinations (per-entity, so
/// id-order and live-order walks produce the same result).
pub(crate) fn desired_turret_facing(entity: &GameEntity, entities: &EntityStore) -> Option<u16> {
    entity.barrel_facing.as_ref()?;
    let desired: u16 = if let Some(ref attack) = entity.attack_target {
        // Look up target position. Entity targets via stable ID, Cell targets via
        // cell-center leptons (force-fire on ground).
        let target_pos = match attack.target {
            crate::sim::combat::TargetKind::Entity(target_id) => entities.get(target_id).map(|t| {
                (
                    t.position.rx,
                    t.position.ry,
                    t.position.sub_x,
                    t.position.sub_y,
                )
            }),
            crate::sim::combat::TargetKind::Cell(rx, ry) => {
                Some((rx, ry, SimFixed::from_num(128), SimFixed::from_num(128)))
            }
        };
        match target_pos {
            Some((trx, try_, tsx, tsy)) => facing_toward_lepton(
                entity.position.rx,
                entity.position.ry,
                entity.position.sub_x,
                entity.position.sub_y,
                trx,
                try_,
                tsx,
                tsy,
            ),
            // Target gone — idle-return to body facing.
            None => body_facing_to_turret(entity.facing),
        }
    } else {
        // No target — return to body facing (research doc §5.1).
        body_facing_to_turret(entity.facing)
    };
    Some(desired)
}

/// Per-binary-frame turret rotation — drives barrel_facing toward each
/// entity's desired facing.
///
/// - If entity has AttackTarget: rotate barrel toward target (lepton-precise).
/// - Otherwise: rotate barrel back to body facing (idle return — research
///   doc §5.1, ledger #20).
///
/// Calls FacingClass::set, which is a no-op when the desired facing equals
/// the current destination — so this function is idempotent.
pub fn tick_turret_rotation(
    entities: &mut EntityStore,
    rules: &RuleSet,
    binary_frame: u32,
    interner: &crate::sim::intern::StringInterner,
) {
    struct TurretUpdate {
        id: u64,
        target_facing: u16,
    }
    let mut updates: Vec<TurretUpdate> = Vec::new();

    // Phase 1: read each turreted entity's desired facing.
    let keys: Vec<u64> = entities.keys_sorted();
    for &id in &keys {
        let entity = match entities.get(id) {
            Some(e) => e,
            None => continue,
        };
        // Unit turrets are driven per-object by unit_post once authoritative; leave
        // Aircraft/Building turrets on this sweep.
        if crate::sim::world::unit_post::L2_UNIT_POST_AUTHORITATIVE
            && entity.category == crate::map::entities::EntityCategory::Unit
        {
            continue;
        }
        // Skip non-turreted entities; otherwise take the per-entity desired facing
        // from the shared helper (single source for sweep + per-object host).
        let Some(desired_facing) = desired_turret_facing(entity, entities) else {
            continue;
        };

        updates.push(TurretUpdate {
            id,
            target_facing: desired_facing,
        });
    }

    // Phase 2: apply rotation via FacingClass::set. Idempotent — no-op when
    // target already equals current destination.
    for update in &updates {
        let rot_byte: u8 = rules
            .object(
                interner.resolve(
                    entities
                        .get(update.id)
                        .map(|e| e.type_ref)
                        .unwrap_or_default(),
                ),
            )
            .map(|obj| obj.turret_rot.clamp(0, 0xFF) as u8)
            .unwrap_or(5);
        if let Some(entity) = entities.get_mut(update.id) {
            if let Some(ref mut barrel) = entity.barrel_facing {
                // Refresh ROT in case rules changed (cheap; idempotent).
                barrel.set_rot(rot_byte);
                barrel.set(update.target_facing, binary_frame);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shortest_rotation_clockwise() {
        assert_eq!(shortest_rotation(0, 10), 10);
        assert_eq!(shortest_rotation(200, 210), 10);
    }

    #[test]
    fn test_shortest_rotation_counter_clockwise() {
        assert_eq!(shortest_rotation(10, 0), -10);
        assert_eq!(shortest_rotation(10, 250), -16); // 250 - 10 = 240 > 128, so 240-256=-16
    }

    #[test]
    fn test_shortest_rotation_wrap_around() {
        // From 250 to 10: clockwise is +16, counter-clockwise is -240. Should pick +16.
        assert_eq!(shortest_rotation(250, 10), 16);
        // From 10 to 250: clockwise is +240, counter-clockwise is -16. Should pick -16.
        assert_eq!(shortest_rotation(10, 250), -16);
    }

    #[test]
    fn test_rot_to_facing_delta() {
        // ROT=5, tick_ms=33 (30Hz): 5 * 256 * 15 * 33 / (360 * 1000) = ~1.76 -> 2
        let delta: u8 = rot_to_facing_delta(5, 33);
        assert!(delta >= 1 && delta <= 3, "delta={}", delta);

        // ROT=0 -> 0
        assert_eq!(rot_to_facing_delta(0, 33), 0);

        // ROT=7 (Grizzly), tick_ms=33: 7*256*15*33 / 360000 = ~2.46 -> 3
        let delta: u8 = rot_to_facing_delta(7, 33);
        assert!(delta >= 2 && delta <= 4, "delta={}", delta);
    }

    #[test]
    fn test_facing_toward_lepton_cardinal() {
        use crate::util::fixed_math::SimFixed;
        let center = SimFixed::from_num(128);
        // Target 5 cells east: should be ~16384 (E).
        let f = facing_toward_lepton(10, 10, center, center, 15, 10, center, center);
        assert!((f as i32 - 16384).abs() < 2, "east facing={f}");
        // Target 5 cells south: should be ~32768 (S).
        let f = facing_toward_lepton(10, 10, center, center, 10, 15, center, center);
        assert!((f as i32 - 32768).abs() < 2, "south facing={f}");
    }

    #[test]
    fn test_facing_toward_lepton_subcell_precision() {
        use crate::util::fixed_math::SimFixed;
        // Same cell, but target is at sub_x=200, sub_y=128 vs source at sub_x=50, sub_y=128.
        // Delta: dx_lep = +150, dy_lep = 0 → pure east → ~16384.
        let f = facing_toward_lepton(
            10,
            10,
            SimFixed::from_num(50),
            SimFixed::from_num(128),
            10,
            10,
            SimFixed::from_num(200),
            SimFixed::from_num(128),
        );
        assert!((f as i32 - 16384).abs() < 2, "sub-cell east facing={f}");
    }

    #[test]
    fn test_body_facing_to_turret() {
        assert_eq!(body_facing_to_turret(0), 0);
        assert_eq!(body_facing_to_turret(64), 16384);
        assert_eq!(body_facing_to_turret(128), 32768);
        assert_eq!(body_facing_to_turret(255), 65280);
    }
}
