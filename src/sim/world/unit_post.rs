//! Per-object UnitClass post-Foot step: Fire_At_Target → Facing_Update.
//!
//! Native order (gamemd `UnitClass::AI`): FIRE reads the PREVIOUS-tick barrel
//! facing, then FACING_UPDATE rotates the barrel toward the target for next tick
//! — so a freshly-acquired target cannot rotate-and-fire the same tick (1-tick
//! acquisition latency).
//!
//! L2 scope: SHADOW only. The legacy `tick_combat_with_fog` + `tick_turret_rotation`
//! sweeps stay authoritative; this host runs read-only in debug to prove agreement
//! before a later slice flips authority (and bumps `SNAPSHOT_VERSION`). The shadow
//! computes FIRE on pre-combat state (with the cooldown decremented locally, since
//! the legacy decrement runs inside combat AFTER this shadow) and compares FACING
//! against the post-combat turret sweep.
//!
//! Depends on `sim/combat` (fire body + snapshot builder) and `sim/movement/turret`
//! (facing helper). Never depends on render/ui/sidebar/audio/net (sim invariant #1).
//! Dispatch is a `category == Unit` filter — no trait object / dyn (invariant #2).

use super::Simulation;
use crate::rules::ruleset::RuleSet;
use crate::sim::combat;

#[cfg(debug_assertions)]
use crate::map::entities::EntityCategory;
#[cfg(debug_assertions)]
use crate::map::overlay_types::OverlayTypeRegistry;
#[cfg(debug_assertions)]
use crate::map::resolved_terrain::ResolvedTerrainGrid;
#[cfg(debug_assertions)]
use crate::sim::entity_store::EntityStore;
#[cfg(debug_assertions)]
use crate::sim::intern::{InternedId, StringInterner};
#[cfg(debug_assertions)]
use crate::sim::movement::turret;
#[cfg(debug_assertions)]
use crate::sim::occupancy::OccupancyGrid;
#[cfg(debug_assertions)]
use crate::sim::overlay_grid::OverlayGrid;
#[cfg(debug_assertions)]
use crate::sim::vision::FogState;
#[cfg(debug_assertions)]
use std::collections::BTreeSet;

/// When true, `unit_post` is authoritative for Unit fire+facing (flips the legacy
/// combat + turret sweeps off for Units). L2 leaves this FALSE — the flip, the
/// `SNAPSHOT_VERSION` bump, and the golden re-baseline are a later slice.
#[allow(dead_code)]
pub(crate) const L2_UNIT_POST_AUTHORITATIVE: bool = false;

/// Authoritative per-object Unit Fire→Facing step (the future home of the flipped
/// behavior). UNUSED while `L2_UNIT_POST_AUTHORITATIVE == false`; the later flip
/// slice fills the body (per-object cooldown decrement → fire into the P4/P6 batch
/// → `barrel.set` toward `desired_turret_facing`). Defined now to fix the seam so
/// the flip is a small diff.
#[allow(dead_code)]
pub(crate) fn unit_post(
    _sim: &mut Simulation,
    _id: u64,
    _rules: &RuleSet,
    _binary_frame: u32,
    _tick_ms: u32,
    _out: &mut combat::CombatEmit,
) {
}

/// Read-only shadow of one Unit's FIRE for the current tick. Emits fire events
/// into `out`; mutates no entity/occupancy/barrel state and does not decrement the
/// stored cooldown. The fire-inclusion predicate mirrors legacy combat Phase-1
/// exactly: has `attack_target`, not inside a transport, not fire-blocked. Caller
/// invokes once per Unit in live-LOGIC order. Debug builds only.
#[cfg(debug_assertions)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn unit_post_shadow_fire_step(
    entities: &EntityStore,
    occupancy: &OccupancyGrid,
    fog: &FogState,
    overlay_grid: Option<&OverlayGrid>,
    overlay_registry: Option<&OverlayTypeRegistry>,
    terrain: Option<&ResolvedTerrainGrid>,
    interner: &mut StringInterner,
    id: u64,
    rules: &RuleSet,
    binary_frame: u32,
    tick_ms: u32,
    fire_blocked: &BTreeSet<u64>,
    out: &mut combat::CombatEmit,
) {
    let Some(entity) = entities.get(id) else {
        return;
    };
    if entity.passenger_role.is_inside_transport() {
        return;
    }
    let Some(attack) = entity.attack_target.as_ref() else {
        return;
    };
    if fire_blocked.contains(&id) {
        return;
    }
    // Legacy combat Phase-1 decrements cooldown/burst-delay this tick BEFORE the
    // fire decision, but that runs inside tick_combat_with_fog — AFTER this
    // pre-combat shadow. Replicate the decrement locally (read-only; the entity is
    // not mutated) so the shadow's fire gate matches legacy. saturating_sub is
    // per-entity, so applying it here is order-independent (Task 3 test pins this).
    let snap = combat::build_attacker_snapshot(
        entity,
        attack.target,
        attack.cooldown_ticks.saturating_sub(1),
        attack.burst_remaining,
        attack.burst_delay_ticks.saturating_sub(1),
        attack.pending_infantry_fire,
        None, // Units are never garrison occupants
    );
    combat::resolve_attacker_fire(
        &snap,
        entities,
        rules,
        interner,
        Some(fog),
        occupancy,
        overlay_grid,
        overlay_registry,
        terrain,
        binary_frame,
        tick_ms,
        out,
    );
}

impl Simulation {
    /// Pre-combat Unit FIRE shadow: walk the live LogicVector order, run each
    /// Unit's fire body into a scratch `CombatEmit`, and collect the live Unit id
    /// set (for the post-sweep agreement check). Mutates nothing the lockstep hash
    /// observes — it reads entity/fog/occupancy state and interns combat strings,
    /// but warhead/weapon/report/anim ids are never part of the state hash and the
    /// interner is append-only, so the type/owner ids that ARE hashed are untouched.
    /// Debug builds only; called at the TOP of Phase 5, before the legacy sweeps.
    #[cfg(debug_assertions)]
    pub(crate) fn l2_unit_post_fire_shadow(
        &mut self,
        logic_order: &[u64],
        rules: &RuleSet,
        overlay_registry: Option<&OverlayTypeRegistry>,
        tick_ms: u32,
    ) -> (combat::CombatEmit, BTreeSet<u64>) {
        let mut scratch = combat::CombatEmit::default();
        let mut unit_ids: BTreeSet<u64> = BTreeSet::new();
        // tick_ms == 0: legacy combat returns early (no fire, no decrement); the
        // shadow must emit nothing too. Still collect unit_ids for the facing check.
        if tick_ms == 0 {
            for &id in logic_order {
                if self
                    .substrate
                    .entities
                    .get(id)
                    .is_some_and(|e| e.category == EntityCategory::Unit)
                {
                    unit_ids.insert(id);
                }
            }
            return (scratch, unit_ids);
        }
        // Same fire-block set legacy combat computes at its start (entities and
        // power_states are unchanged between here and the combat call — the shadow
        // is read-only).
        let fire_blocked = combat::combat_fire_gate::collect_fire_blocked_entities(
            &self.substrate.entities,
            &self.power_states,
            Some(rules),
            &self.interner,
        );
        // Fire in live-LOGIC order — matches legacy emission order (the downstream
        // scenario_rng smudge cursor depends on it).
        for &id in logic_order {
            if !self
                .substrate
                .entities
                .get(id)
                .is_some_and(|e| e.category == EntityCategory::Unit)
            {
                continue;
            }
            unit_ids.insert(id);
            unit_post_shadow_fire_step(
                &self.substrate.entities,
                &self.substrate.occupancy,
                &self.fog,
                self.overlay_grid.as_ref(),
                overlay_registry,
                self.resolved_terrain.as_ref(),
                &mut self.interner,
                id,
                rules,
                self.binary_frame,
                tick_ms,
                &fire_blocked,
                &mut scratch,
            );
        }
        (scratch, unit_ids)
    }

    /// Assert the per-object Unit Fire→Facing host agrees with the legacy combat +
    /// turret sweeps this tick. FIRE: the Unit subset of the legacy fire stream (in
    /// emission order) must equal the shadow's. FACING: each surviving live Unit's
    /// desired barrel facing — computed on POST-combat state, the same state the
    /// turret sweep used — must equal the destination the sweep set. Runs AFTER
    /// tick_turret_rotation. Debug builds only; reads only.
    #[cfg(debug_assertions)]
    pub(crate) fn l2_unit_post_assert(
        &self,
        shadow: (combat::CombatEmit, BTreeSet<u64>),
        combat_result: &combat::CombatTickResult,
    ) {
        let (scratch, unit_ids) = shadow;
        // FIRE agreement — projected to (attacker_id, weapon_id). SimFireEvent is
        // not PartialEq; this projection catches fire-or-not, emission order, and
        // weapon selection, which is what the future per-object flip must preserve.
        let shadow_fire: Vec<(u64, InternedId)> = scratch
            .fire_events
            .iter()
            .map(|e| (e.attacker_id, e.weapon_id))
            .collect();
        let legacy_unit_fire: Vec<(u64, InternedId)> = combat_result
            .fire_events
            .iter()
            .filter(|e| unit_ids.contains(&e.attacker_id))
            .map(|e| (e.attacker_id, e.weapon_id))
            .collect();
        debug_assert_eq!(
            shadow_fire, legacy_unit_fire,
            "L2 unit_post shadow Unit fire stream diverged from the legacy combat sweep"
        );
        // FACING agreement — desired_turret_facing reads attack_target/positions,
        // which the turret sweep does NOT mutate (it writes only barrel_facing), so
        // computing it now (post-sweep) reads the same post-combat state the sweep
        // used. A surviving live Unit's desired facing must equal what the sweep set.
        for &id in &unit_ids {
            let Some(entity) = self.substrate.entities.get(id) else {
                continue; // Unit despawned in combat — nothing to compare.
            };
            let Some(desired) = turret::desired_turret_facing(entity, &self.substrate.entities)
            else {
                continue; // not turreted
            };
            if let Some(dest) = entity.barrel_facing.as_ref().map(|b| b.destination()) {
                debug_assert_eq!(
                    dest, desired,
                    "L2 unit_post shadow Unit {id} barrel destination diverged from turret sweep"
                );
            }
        }
    }
}
