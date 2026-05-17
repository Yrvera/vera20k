//! Target acquisition and retaliation for the combat system.
//!
//! Handles two responsibilities:
//! 1. **Target acquisition** — finding the best hostile target for an idle or
//!    attack-moving unit within its guard/weapon range.
//! 2. **Retaliation** — idle units automatically attack the entity that hit them.
//!
//! ## Target priority
//! When multiple valid targets exist, the nearest is preferred. Ties are broken
//! by threat class (armed units > unarmed > buildings) and stable entity ID
//! (for deterministic replay).
//!
//! ## Auto-deploy on target acquisition
//! Targeting NEVER initiates a deploy transition. A walking GGI that acquires
//! an air target uses its Secondary weapon in place — it does not auto-deploy.
//! This matches the original's behavior: deploy is a player-driven command,
//! never triggered by AI target acquisition. Verified by grepping every writer
//! of `deploy_state` — only the player command handler and the deploy tick
//! advance set it.
//!
//! ## Dependency rules
//! - Part of sim/ — depends on rules/ (RuleSet) and sim/components.
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

use super::combat_weapon::{VersesGate, select_weapon_with_override, verses_gate};
use super::{is_within_range_leptons, lepton_distance_sq_raw};
use crate::map::entities::EntityCategory;
use crate::map::resolved_terrain::ResolvedTerrainGrid;
use crate::rules::object_type::ObjectCategory;
use crate::rules::ruleset::RuleSet;
use crate::sim::entity_store::EntityStore;
use crate::sim::game_entity::GameEntity;
use crate::sim::intern::{InternedId, StringInterner};
use crate::sim::vision::FogState;
use crate::util::fixed_math::SimFixed;

/// Snapshot of garrison state for a garrisoned building attacker.
/// Extracted during Phase 1 to avoid borrow conflicts in Phase 2.
pub(crate) struct GarrisonSnapshot {
    /// Type ID of the occupant that will fire this tick.
    pub occupant_type_id: InternedId,
    /// Veterancy of the firing occupant (for elite weapon selection).
    pub occupant_veterancy: u16,
    /// Current round-robin fire index.
    pub fire_index: u8,
    /// Total occupant count (for ROF division).
    pub occupant_count: u8,
    /// Half foundation size: `min(width, height) / 2` (for range formula).
    pub half_foundation: u16,
}

/// Snapshot of an attacker's state for target scanning.
/// Extracted to avoid borrow conflicts during entity iteration.
pub(crate) struct AttackerSnapshot {
    pub stable_id: u64,
    pub owner: InternedId,
    pub category: EntityCategory,
    /// What the attacker is firing at — entity ID or cell coord.
    /// Cell targets skip auto-retarget and friendly-fire checks (the player
    /// explicitly chose this cell).
    pub target: super::TargetKind,
    pub pos_rx: u16,
    pub pos_ry: u16,
    pub sub_x: SimFixed,
    pub sub_y: SimFixed,
    pub type_id: InternedId,
    pub facing: u8,
    pub veterancy: u16,
    pub cooldown_ticks: u16,
    pub animation_sequence: Option<crate::sim::animation::SequenceKind>,
    pub animation_frame: Option<u16>,
    pub is_prone: bool,
    pub is_fully_deployed: bool,
    pub has_movement: bool,
    pub pending_infantry_fire: Option<super::PendingInfantryFire>,
    pub barrel_facing: Option<crate::sim::movement::FacingClass>,
    pub burst_remaining: u8,
    pub burst_delay_ticks: u8,
    /// Weapon-selection override (Gunner-IFV slot OR open-topped passenger weapon).
    pub weapon_override: Option<super::combat_weapon::WeaponOverride>,
    /// Garrison state — present only for garrisoned buildings (IsOccupied).
    pub garrison: Option<GarrisonSnapshot>,
}

/// Acquire the best currently valid target for one attacker entity.
/// Returns the target's stable entity ID.
///
/// `terrain` is threaded through for the 3D InRange check; when `None`
/// (headless tests, no map loaded), the range check falls back to the
/// existing 2D behavior.
pub fn acquire_best_target_for_entity(
    entities: &EntityStore,
    rules: &RuleSet,
    interner: &StringInterner,
    attacker_id: u64,
    fog: Option<&FogState>,
    terrain: Option<&ResolvedTerrainGrid>,
) -> Option<u64> {
    let entity = entities.get(attacker_id)?;
    // Aircraft with 0 ammo should not acquire new targets — need to reload.
    if let Some(ref ammo) = entity.aircraft_ammo {
        if ammo.current <= 0 {
            return None;
        }
    }
    let obj = rules.object(interner.resolve(entity.type_ref))?;
    // Need at least one weapon to acquire targets.
    if obj.primary.is_none() && obj.secondary.is_none() {
        return None;
    }

    let snapshot = AttackerSnapshot {
        stable_id: entity.stable_id,
        owner: entity.owner,
        category: entity.category,
        target: super::TargetKind::Entity(0), // Dummy — no current target when acquiring fresh
        pos_rx: entity.position.rx,
        pos_ry: entity.position.ry,
        sub_x: entity.position.sub_x,
        sub_y: entity.position.sub_y,
        type_id: entity.type_ref,
        facing: entity.facing,
        veterancy: entity.veterancy,
        cooldown_ticks: 0,
        animation_sequence: entity.animation.as_ref().map(|a| a.sequence),
        animation_frame: entity.animation.as_ref().map(|a| a.frame_index),
        is_prone: entity
            .infantry
            .as_ref()
            .is_some_and(|infantry| infantry.is_prone),
        is_fully_deployed: entity.is_fully_deployed(),
        has_movement: entity.movement_target.is_some(),
        pending_infantry_fire: None,
        barrel_facing: entity.barrel_facing,
        burst_remaining: 0,
        burst_delay_ticks: 0,
        weapon_override: entity.weapon_override,
        garrison: None,
    };
    acquire_best_target(
        entities, rules, interner, &snapshot, obj, fog, None, terrain,
    )
}

fn threat_class(rules: &RuleSet, interner: &StringInterner, type_id: InternedId) -> u8 {
    match rules.object(interner.resolve(type_id)) {
        Some(obj) if obj.primary.is_some() => 0,
        Some(obj) => match obj.category {
            ObjectCategory::Vehicle | ObjectCategory::Aircraft | ObjectCategory::Infantry => 1,
            ObjectCategory::Building => 2,
        },
        None => 3,
    }
}

/// Find the best hostile target within scan range for a given attacker snapshot.
///
/// Filters by: alive, hostile, visible (fog), weapon compatibility (projectile
/// flags + Verses > 0%), and range. Ranks by distance, threat class, stable ID.
/// Returns the target's stable entity ID.
///
/// `scan_range_override`: when `Some`, replaces the computed guard_range/weapon_range
/// for the distance check. Used by garrisoned buildings whose scan range is
/// derived from foundation size + OccupyWeaponRange.
pub(crate) fn acquire_best_target(
    entities: &EntityStore,
    rules: &RuleSet,
    interner: &StringInterner,
    attacker: &AttackerSnapshot,
    attacker_obj: &crate::rules::object_type::ObjectType,
    fog: Option<&FogState>,
    scan_range_override: Option<SimFixed>,
    terrain: Option<&ResolvedTerrainGrid>,
) -> Option<u64> {
    let mut best: Option<(i64, u8, u64)> = None;

    for candidate in entities.values() {
        if candidate.stable_id == attacker.stable_id {
            continue;
        }
        if candidate.health.current == 0 || candidate.dying {
            continue;
        }
        // Skip entities inside a transport — they are hidden from the battlefield.
        if candidate.passenger_role.is_inside_transport() {
            continue;
        }
        let attacker_owner_str = interner.resolve(attacker.owner);
        let candidate_owner_str = interner.resolve(candidate.owner);
        if fog
            .is_some_and(|fog_state| fog_state.is_friendly(attacker_owner_str, candidate_owner_str))
            || candidate.owner == attacker.owner
        {
            continue;
        }
        if let Some(fog_state) = fog {
            if !fog_state.is_cell_visible(
                attacker.owner,
                candidate.position.rx,
                candidate.position.ry,
            ) {
                continue;
            }
        }

        // Check if any weapon can engage this target (projectile flags + Verses > 0%).
        let target_cat: EntityCategory = candidate.category;
        let target_armor: &str = rules
            .object(interner.resolve(candidate.type_ref))
            .map(|o| o.armor.as_str())
            .unwrap_or("none");
        let selected = match select_weapon_with_override(
            rules,
            attacker_obj,
            target_cat,
            target_armor,
            attacker.veterancy,
            attacker.weapon_override,
        ) {
            Some(s) => s,
            None => continue, // No weapon can engage this target.
        };

        // For passive acquisition, skip targets where Verses is Suppressed (1%).
        if verses_gate(selected.verses_pct) == VersesGate::Suppressed {
            continue;
        }

        // Use override (garrison), guard_range, or weapon range for the distance check.
        let scan_range = scan_range_override
            .unwrap_or_else(|| attacker_obj.guard_range.unwrap_or(selected.weapon.range));
        // 2D dist_sq still feeds the ranking key below; the in-range boolean
        // is computed separately via 3D when possible.
        let dist_sq = lepton_distance_sq_raw(
            attacker.pos_rx,
            attacker.pos_ry,
            attacker.sub_x,
            attacker.sub_y,
            candidate.position.rx,
            candidate.position.ry,
            candidate.position.sub_x,
            candidate.position.sub_y,
        );
        let in_range = if scan_range == selected.weapon.range {
            // Standard scan range — 3D check when terrain + attacker entity available.
            match (terrain, entities.get(attacker.stable_id)) {
                (Some(t), Some(attacker_entity)) => {
                    let src = (
                        attacker.pos_rx as i64 * 256 + attacker.sub_x.to_num::<i64>(),
                        attacker.pos_ry as i64 * 256 + attacker.sub_y.to_num::<i64>(),
                        super::in_range::effective_z_leptons(attacker_entity),
                    );
                    super::in_range::compute_in_range(
                        attacker_entity,
                        src,
                        &super::TargetKind::Entity(candidate.stable_id),
                        selected.weapon,
                        rules,
                        interner,
                        entities,
                        t,
                    )
                }
                _ => is_within_range_leptons(dist_sq, scan_range),
            }
        } else {
            // Override path (guard_range / garrison) — keep 2D until later stages
            // refine override threading through compute_in_range.
            is_within_range_leptons(dist_sq, scan_range)
        };
        if !in_range {
            continue;
        }

        let class = threat_class(rules, interner, candidate.type_ref);
        let rank = (dist_sq, class, candidate.stable_id);
        match best {
            Some(current) if rank >= current => {}
            _ => best = Some(rank),
        }
    }

    best.map(|(_, _, sid)| sid)
}

/// Check if an entity can retaliate against an attacker (weapon + Verses gate).
fn can_retaliate(
    entity: &GameEntity,
    attacker: &GameEntity,
    rules: &RuleSet,
    interner: &StringInterner,
) -> bool {
    let obj = match rules.object(interner.resolve(entity.type_ref)) {
        Some(o) => o,
        None => return false,
    };
    let target_cat: EntityCategory = attacker.category;
    let target_armor: &str = rules
        .object(interner.resolve(attacker.type_ref))
        .map(|o| o.armor.as_str())
        .unwrap_or("none");
    let selected = match select_weapon_with_override(
        rules,
        obj,
        target_cat,
        target_armor,
        entity.veterancy,
        entity.weapon_override,
    ) {
        Some(s) => s,
        None => return false,
    };
    // 0% is already filtered by select_weapon_with_override (returns None).
    // 1% (Suppressed) also blocks retaliation.
    verses_gate(selected.verses_pct) != VersesGate::Suppressed
}

/// Retaliation system: idle units that were recently hit auto-attack their attacker.
///
/// Called after `tick_combat_with_fog()` in the game loop. Iterates entities
/// that have a `last_attacker_id` but no `attack_target` and no `order_intent`.
/// Skips retaliation if the weapon has 0% or 1% Verses against the attacker's armor.
pub fn tick_retaliation(entities: &mut EntityStore, rules: &RuleSet, interner: &StringInterner) {
    // Collect retaliation candidates: (retaliator_id, attacker_id).
    let mut retaliators: Vec<(u64, u64)> = Vec::new();
    let keys: Vec<u64> = entities.keys_sorted();
    for &id in &keys {
        let entity = match entities.get(id) {
            Some(e) => e,
            None => continue,
        };
        // Must have last_attacker, no current attack target, no order intent.
        let attacker_sid = match entity.last_attacker_id {
            Some(sid) => sid,
            None => continue,
        };
        if entity.attack_target.is_some() || entity.order_intent.is_some() {
            continue;
        }
        // Verify attacker is still alive.
        let attacker_alive = entities
            .get(attacker_sid)
            .is_some_and(|a| a.health.current > 0);
        if !attacker_alive {
            continue;
        }
        retaliators.push((id, attacker_sid));
    }

    // Process retaliation — issue attack commands.
    for (entity_id, attacker_sid) in retaliators {
        let retaliate = {
            let entity = match entities.get(entity_id) {
                Some(e) => e,
                None => continue,
            };
            let attacker = match entities.get(attacker_sid) {
                Some(a) => a,
                None => {
                    // Attacker gone — clear last_attacker.
                    if let Some(e) = entities.get_mut(entity_id) {
                        e.last_attacker_id = None;
                    }
                    continue;
                }
            };
            can_retaliate(entity, attacker, rules, interner)
        };

        if retaliate {
            // Read attacker rx/ry (only needed for body-only retaliators).
            let attacker_pos = match entities.get(attacker_sid) {
                Some(a) => (a.position.rx, a.position.ry),
                None => continue,
            };
            if let Some(entity) = entities.get_mut(entity_id) {
                if entity.barrel_facing.is_none() {
                    // Body-only retaliator — instantly face the attacker. Turreted
                    // retaliators get their turret rotation driven by
                    // tick_turret_rotation in subsequent ticks (matches gamemd).
                    let dx: i32 = attacker_pos.0 as i32 - entity.position.rx as i32;
                    let dy: i32 = attacker_pos.1 as i32 - entity.position.ry as i32;
                    entity.facing = crate::sim::movement::facing_from_delta(dx, dy);
                }
                entity.movement_target = None;
                entity.attack_target = Some(crate::sim::combat::AttackTarget::new(attacker_sid));
            }
        }
        // Clear last_attacker regardless (prevent repeated attempts).
        if let Some(entity) = entities.get_mut(entity_id) {
            entity.last_attacker_id = None;
        }
    }
}
