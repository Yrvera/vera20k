//! Per-object AI dispatch scaffold (TechnoClass/FootClass spine, Slice S0).
//!
//! Walks the substrate's live-object order and dispatches each live object
//! through a per-`EntityCategory` shell. THIS SLICE the shell is a strict
//! no-op: it visits every live, present, non-dying object exactly once in live
//! order and changes nothing the lockstep hash observes. Later slices replace
//! the no-op arms with the absorbed per-leaf behavior (movement, turret,
//! combat, mission dispatch) one at a time.
//!
//! Depends on: `world::Simulation` (substrate live order + entity store).
//! Must NOT depend on render/ui/sidebar/audio/net (sim invariant #1).
//! Dispatch is `match category` only — no trait object / dyn / vtable
//! (invariant #2). No RNG, no hashed-state mutation, no phase reorder.

use super::Simulation;
use crate::map::entities::EntityCategory;
use crate::rules::particle_system_type::ParticleSystemBehavesLike;
use crate::rules::ruleset::RuleSet;
// `DispatchSlot` types the always-defined `UnitDispatchRecord`, so its import is non-gated.
use crate::sim::mission::dispatch::DispatchSlot;
// `unit_dispatch_family` is consumed only by the gated record pass + proof below.
#[cfg(any(test, debug_assertions))]
use crate::sim::mission::dispatch::unit_dispatch_family;

// Slice S1 (shadow) imports — used only by the `#[cfg(any(test, debug_assertions))]`
// dispatch-before-locomotor observation below; gated to avoid release dead-code.
#[cfg(any(test, debug_assertions))]
use crate::sim::game_entity::GameEntity;
#[cfg(any(test, debug_assertions))]
use crate::sim::mission::MissionType;
#[cfg(any(test, debug_assertions))]
use crate::sim::movement::{DriveProcessOutcome, process_drive_locomotion_shell};

// P3 oracle probe import — used only by the `#[cfg(test)]` factory_oracle_step_trace.
#[cfg(test)]
use crate::sim::production::StepOutcome;

/// One live Unit's host-time dispatch routing, recorded at `object_ai_stage` time (top of
/// tick, after commands) for the end-of-tick churn proof. Copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct UnitDispatchRecord {
    pub id: u64,
    /// `derived_mission().0` evaluated fresh at host time (NOT the stale `mission.current`).
    pub host_mission: crate::sim::mission::MissionType,
    pub family: DispatchSlot,
}

/// The per-tick host-time dispatch trace. Populated only in debug/test builds; release
/// returns an empty `Vec` (lazy `Vec::new()` → no allocation on the hot path).
pub(crate) type UnitDispatchTrace = Vec<UnitDispatchRecord>;

impl Simulation {
    /// Object-AI stage (S4a: AUTHORITATIVE per-object mission commit).
    ///
    /// Walks the live LogicVector order via `for_each_live_object` — the same
    /// re-read contract the native scheduler uses — and runs each live, present,
    /// non-dying object through `techno_ai_shell`. A live non-miner Unit's mission
    /// (`+0xC4` tick_counter + `derived_mission`) is committed HERE, at the
    /// gamemd-faithful per-object AI point (pre-movement); its id is inserted into
    /// `committed` so the Phase-9 tail projection skips it (no double-commit). The
    /// other arms are still no-ops (their slices land later).
    ///
    /// `record` is true only in debug builds, where the recorded visit trace is
    /// asserted to equal the live (present, non-dying) order — the first
    /// tripwire for any future arm that mutates live membership mid-pass.
    /// Release builds pass `false`, so the trace `Vec` is never pushed to and
    /// never allocates (no per-tick hot-path cost).
    pub(crate) fn object_ai_stage(
        &mut self,
        rules: Option<&RuleSet>,
        committed: &mut std::collections::BTreeSet<u64>,
    ) -> UnitDispatchTrace {
        let visited = self.object_ai_walk(cfg!(debug_assertions), rules, committed);

        #[cfg(debug_assertions)]
        debug_assert_eq!(
            visited,
            self.object_ai_live_order_filtered(),
            "object_ai_stage visit order diverged from live LogicVector order",
        );

        #[cfg(not(debug_assertions))]
        let _ = visited;

        // Unit dispatch shadow: record host-time routing (debug/test only; empty in
        // release). The `Unit => {}` arm of `techno_ai_shell` stays a no-op — the shadow
        // is a parallel pass, exactly like the S1 shadow.
        self.unit_dispatch_record_pass()
    }

    /// Host-time Unit dispatch shadow pass (debug/test only). Walks the live-object order
    /// (the gamemd dispatch set), and for each live, non-dying, NON-miner Unit records its
    /// fresh-at-host-time mission (`derived_mission().0` — NOT the stale `mission.current`,
    /// which excludes this tick's commands) and the family it routes to. Read-only: mutates
    /// no entity, no occupancy, no hash. Miners are skipped — the miner session's Harvest
    /// seam owns that path.
    #[cfg(any(test, debug_assertions))]
    fn unit_dispatch_record_pass(&self) -> UnitDispatchTrace {
        let mut trace: UnitDispatchTrace = Vec::new();
        for id in self.live_object_order_snapshot() {
            let Some(e) = self.substrate.entities.get(id) else {
                continue;
            };
            if e.dying || e.category != EntityCategory::Unit || e.miner.is_some() {
                continue;
            }
            let (host_mission, _substate) = e.derived_mission();
            trace.push(UnitDispatchRecord {
                id,
                host_mission,
                family: unit_dispatch_family(host_mission),
            });
        }
        trace
    }

    /// Release stub: the host-time trace is empty and never allocates.
    #[cfg(not(any(test, debug_assertions)))]
    fn unit_dispatch_record_pass(&self) -> UnitDispatchTrace {
        Vec::new()
    }

    /// End-of-tick Unit dispatch proof (debug/test only). Runs after `refresh_mission_shadow`,
    /// beside `debug_assert_s1_shadow`. For each host-time record it:
    ///   1. asserts the routed family is correct for the recorded mission (router determinism),
    ///   2. asserts a non-miner Unit never routes to `Skip`, and that `AttackMove` is never the
    ///      host mission of a Unit (unreachable — `derived_mission` cannot yield it),
    ///   3. re-derives the Unit's mission FRESH now (tail) and, if the family differs from the
    ///      host-time family, LOGS the churn with tick+id+both missions — it does NOT assert
    ///      equality (host-time and tail derivations legitimately differ when a Unit's machines
    ///      change mid-tick). Read-only; never hashed; never silently equalized.
    ///
    /// Returns the per-tick churn count (live non-miner Units whose host-time family differs
    /// from the tail re-derivation) — the S2 go/no-go measurement signal, surfaced to the
    /// caller via the (unhashed, unserialized) `TickResult`. Read-only.
    #[cfg(any(test, debug_assertions))]
    pub(crate) fn debug_assert_unit_dispatch_shadow(&self, trace: &UnitDispatchTrace) -> u32 {
        let mut churn = 0u32;
        for rec in trace {
            // (1) router determinism: the recorded family is exactly the router's output.
            debug_assert_eq!(
                rec.family,
                unit_dispatch_family(rec.host_mission),
                "dispatch: tick {} unit {}: recorded family must equal the router output",
                self.session.tick,
                rec.id,
            );
            // (2) a Unit is never on AttackMove (derived_mission cannot yield it).
            debug_assert_ne!(
                rec.host_mission,
                MissionType::AttackMove,
                "dispatch: tick {} unit {}: a Unit must never derive AttackMove",
                self.session.tick,
                rec.id,
            );
            debug_assert!(
                !matches!(rec.family, DispatchSlot::Skip),
                "dispatch: tick {} unit {}: a live Unit must never route to Skip",
                self.session.tick,
                rec.id,
            );
            // (3) churn metric: compare host-time family to a fresh tail re-derivation.
            if let Some(e) = self.substrate.entities.get(rec.id) {
                if !e.dying && e.miner.is_none() {
                    let (tail_mission, _) = e.derived_mission();
                    let tail_family = unit_dispatch_family(tail_mission);
                    if tail_family != rec.family {
                        // Surfaced, never equalized — the S2 go/no-go churn signal.
                        churn += 1;
                        log::debug!(
                            "dispatch churn: tick {} unit {}: host {:?} -> tail {:?}",
                            self.session.tick,
                            rec.id,
                            rec.host_mission,
                            tail_mission,
                        );
                    }
                }
            }
        }
        churn
    }

    /// Live-set coverage (T5): every Unit that a legacy dispatch phase would touch — i.e. it
    /// carries a dispatch machine AND passes that phase's own guards — must be in the host's
    /// live-object set. The legacy phases iterate `iter_sorted()` (all entities); the host
    /// iterates the LogicVector. With the legacy guards applied (mirroring `tick_attack_pursuit`:
    /// not dying, not Structure, no aircraft mission, not deployed, not a transport passenger)
    /// the residual set is expected-empty in normal play. A residual member is a real Rust drift
    /// to investigate before S2 — LOGGED with tick+id, never hard-asserted.
    #[cfg(any(test, debug_assertions))]
    pub(crate) fn debug_check_dispatch_live_set_coverage(&self) {
        use std::collections::BTreeSet;
        let live: BTreeSet<u64> = self.live_object_order_snapshot().into_iter().collect();
        // `iter_sorted()` yields `(u64, &GameEntity)` in ascending-id order (deterministic).
        for (id, e) in self.substrate.entities.iter_sorted() {
            if e.dying
                || e.category == EntityCategory::Structure
                || e.aircraft_mission.is_some()
                || e.is_deployed()
                || e.passenger_role.is_inside_transport()
            {
                continue;
            }
            // A Unit a legacy dispatch phase would act on: has a movement/attack/dock machine.
            let touched = e.movement_target.is_some()
                || e.attack_target.is_some()
                || e.dock_state.is_some()
                || e.order_intent.is_some();
            if touched && !live.contains(&id) {
                log::debug!(
                    "dispatch coverage drift: tick {} unit {} touched by a legacy phase but \
                     absent from live order",
                    self.session.tick,
                    id,
                );
            }
        }
    }

    /// Slice S4c — passive/opportunity-acquire eligibility SHADOW (read-only,
    /// hash-neutral). For each live Unit, counts whether it would reach the
    /// passive-acquire scanner this pass, per the verified gamemd gate
    /// `TechnoClass::PassiveAcquireGate` (decompiled 0x00709290) inside the
    /// mission-{Move(2),Guard(5),Harvest(10)} block: base can-acquire
    /// (`TechnoClass::CanAcquireTarget` 0x007091d0) AND (`OpportunityFire` OR
    /// (Guard AND weapon)). A Guard-mission unit auto-acquires regardless of
    /// `OpportunityFire`.
    ///
    /// VERA models the CONFIRMED core via `s4c_passive_acquire_eligible`: mission
    /// in {Move,Guard,Harvest}, the type carries a weapon, and (`opportunity_fire`
    /// OR mission==Guard). The base-can-acquire sub-conditions (not-disabled,
    /// capture-managed, player-gated, the `Type+0xd99` flag) are UNCHECKED
    /// refinements deferred to the S5 authoritative flip — which runs the actual
    /// scanner and sets the target. This pass mutates nothing and is never
    /// hashed; it returns the eligible count (the cadence/eligibility metric).
    #[cfg(any(test, debug_assertions))]
    pub(crate) fn debug_s4c_passive_acquire_shadow(
        &self,
        rules: Option<&crate::rules::ruleset::RuleSet>,
    ) -> u32 {
        let Some(rules) = rules else {
            return 0;
        };
        let mut eligible = 0u32;
        for id in self.live_object_order_snapshot() {
            let Some(e) = self.substrate.entities.get(id) else {
                continue;
            };
            if e.dying || e.category != EntityCategory::Unit {
                continue;
            }
            let mission = e.derived_mission().0;
            let Some(obj) = rules.object(self.interner.resolve(e.type_ref)) else {
                continue;
            };
            // CanAcquireTarget weapon-equipped proxy: the type has a Primary or
            // Secondary weapon (the runtime vtable+0x2ac equip check is UNCHECKED).
            let has_weapon = obj.primary.is_some() || obj.secondary.is_some();
            if s4c_passive_acquire_eligible(mission, has_weapon, obj.opportunity_fire) {
                eligible += 1;
                log::trace!(
                    "S4c passive-acquire eligible: tick {} unit {} mission {:?} (opp_fire {})",
                    self.session.tick,
                    id,
                    mission,
                    obj.opportunity_fire,
                );
            }
        }
        eligible
    }

    /// The walk: dispatch every live, present, non-dying object once, in live
    /// order, through the no-op shell. When `record`, return the dispatched ids
    /// in order (debug/test observation); otherwise the returned `Vec` is empty
    /// and unallocated. Reads only — touches no hashed state, consumes no RNG.
    fn object_ai_walk(
        &mut self,
        record: bool,
        rules: Option<&RuleSet>,
        committed: &mut std::collections::BTreeSet<u64>,
    ) -> Vec<u64> {
        let mut visited: Vec<u64> = Vec::new();
        self.for_each_live_object(|sim, id| {
            // Tolerate an absent id (the loop's documented contract). The stage
            // runs AFTER the end-of-tick flush_pending_delete drain, so the order
            // should not reference a freed slot — but inherit the guard.
            let Some(entity) = sim.substrate.entities.get(id) else {
                return;
            };
            // A dying object is mid death-teardown and is not dispatched (the
            // closest live `IsActive` analogue today). Dying units are off the
            // LogicVector anyway and fall to the tail projection.
            if entity.dying {
                return;
            }
            let category = entity.category;
            if record {
                visited.push(id);
            }
            // A non-miner live Unit commits its mission in the bracket; record it
            // so the tail projection skips it (no double-commit / double-count).
            if techno_ai_shell(sim, id, category, rules) {
                committed.insert(id);
            }
        });
        visited
    }

    /// The ids the walk dispatches, derived independently from the post-pass
    /// live order: present in the store and not dying, in live order. For the
    /// S0 no-op shell this always equals the recorded visit trace; a future arm
    /// that removes/reorders a live object mid-pass would break the equality.
    #[cfg(any(test, debug_assertions))]
    fn object_ai_live_order_filtered(&self) -> Vec<u64> {
        self.live_object_order_snapshot()
            .into_iter()
            .filter(|&id| self.substrate.entities.get(id).is_some_and(|e| !e.dying))
            .collect()
    }
}

/// Per-category dispatch shell. Slice S0: every arm is a strict no-op.
///
/// `match category` — NO trait / dyn / vtable (invariant #2). `sim`/`id` are
/// threaded so later slices can fill an arm with the absorbed behavior without
/// changing this signature. The match is exhaustive over the four real
/// variants (no `_` arm), so a future `EntityCategory` addition is a compile
/// error, intentionally.
/// Returns `true` iff this object authoritatively committed its mission this
/// pass (a non-miner live Unit) — the walk adds those ids to the tail skip-set.
fn techno_ai_shell(
    sim: &mut Simulation,
    id: u64,
    category: EntityCategory,
    rules: Option<&RuleSet>,
) -> bool {
    match category {
        // S4a: run the AUTHORITATIVE TechnoClass common bracket; a non-miner live
        // Unit commits its mission here (the host owns it; the tail skips it).
        EntityCategory::Unit => {
            matches!(unit_techno_bracket(sim, id, rules), BracketReach::Committed)
        }
        EntityCategory::Infantry => false, // S6: absorb fear / sequence / self-removal
        EntityCategory::Structure => false, // S8 absorb bracket; P3 oracle probe is factory_oracle_step_trace
        EntityCategory::Aircraft => false, // S7: absorb per-object aircraft dispatch
    }
}

// ===== Slice S4a — TechnoClass common-body bracket (shadow shell) =====
//
// Per live Unit, gamemd's `TechnoClass::AI_Update` body is one contiguous
// bracket: pre-mission block -> +0xC4 -> Mission_Dispatch -> post-mission block,
// with two IsAlive early-returns (after the pre-block, after dispatch). THIS
// SLICE the bracket is a SHADOW SHELL: `techno_common_pre`/`techno_common_post`
// are no-op stubs and the authoritative `+0xC4`/mission-commit stays in
// `movement_tick` (only a dispatch MARKER sits between the guards here). The
// shell runs every tick in the live-object walk and is hash-neutral (stubs do
// nothing; the guards are read-only). The authoritative flip — relocating the
// `+0xC4`/commit out of `movement_tick` into this dispatch point and filling the
// stubs — is the next step, gated on the body decode (U6) + a hash re-baseline.
// Design: docs/plans/2026-06-10-s4a-common-bracket-design.md.

/// S4a pre-mission common block (the `TechnoClass::AI_Update` head: one-shot
/// flag clear, turret-anim loop sound, cloak tick, health smoothing, target
/// validation, …). No-op stub this slice — the verified body lands at the
/// authoritative flip. Present so the bracket order is real code, not a comment.
#[allow(unused_variables)]
fn techno_common_pre(sim: &mut Simulation, id: u64) {}

/// `damage_particle_live_until` sentinel for a spawned spark system whose
/// `Lifetime <= 0`: gamemd's `ParticleSystemClass::AI` removal counter (set from
/// `Type+0x2b8`) only fires on `--counter == 0`, so a non-positive lifetime never
/// reaches 0 going down → the system (and thus `+0x308`) holds for the whole
/// match. Distinct from a real finite `spawn_tick + lifetime` (always `>= 1` for
/// `lifetime > 0`, and `tick` won't reach `u64::MAX` in any real match).
const DAMAGE_PARTICLE_LIVE_FOREVER: u64 = u64::MAX;

/// Largest `roll` value the gamemd damage-Spark prob-roll yields
/// (`RandomRanged(0, 0x7ffffffe)`).
const DAMAGE_SPARK_ROLL_MAX: u32 = 0x7fff_fffe;

/// S4a post-mission common block (the steps after `Mission_Dispatch`: passive
/// acquire (S4c), the damage-particle RNG (S4b), the timer accumulator, EMP
/// recovery).
///
/// S4b — the AI_Update damage-Spark `scenario_rng` consumption, modelled exactly
/// from the verified gamemd block. Per object, per tick:
///   - Outer gate: `emits_damage_spark` (TechnoTypeClass `+0xC8F` = `Cyborg`,
///     infantry-only) AND `HealthRatio < ConditionYellow` (STRICT) AND
///     not-in-special-damage-state (`vtable+0x1c8() > -10`, unmodelled → pass).
///   - Build the Spark sublist of `DamageParticleSystems` (`BehavesLike == Spark`).
///     No RNG.
///   - Inner gate: no live spark system (`+0x308`-equivalent empty) AND Spark
///     count > 0.
///   - Draw #1 (always, on inner-gate pass): the prob-roll on `scenario_rng`;
///     succeed iff `roll < threshold` (red band if `HealthRatio < ConditionRed`,
///     else yellow). On success → Draw #2: the list-pick `n(0, count-1)` (consumes
///     no draw when count == 1, matching gamemd `RandomRanged(min == max)`), and
///     arm the live-system hold to `tick + sparkType.Lifetime`.
///
/// Draw truth table (`scenario_rng`): 0 (outer gate fail / live system / no
/// Spark) — 1 (roll fails) — 2 (roll succeeds, count >= 2) — 1 (roll succeeds,
/// count == 1). The spawn/offset/ctor draw NOTHING. The visual is render-side;
/// this consumes the draws and tracks the gate only.
///
/// Dormant in stock YR: `emits_damage_spark` is false for every stock type (no
/// `Cyborg=yes` units, and `techno_common_post` runs only for the vehicle arm),
/// so the early-out fires before any allocation or draw — zero `scenario_rng`
/// movement. Modelled exactly so the stream stays aligned if a mod ever enables it.
fn techno_common_post(sim: &mut Simulation, id: u64, rules: Option<&RuleSet>) {
    let Some(rules) = rules else {
        return;
    };

    // Read the entity facts we need, then resolve the type. `obj` borrows `rules`
    // (external), not `sim`, so the later &mut sim draws/writes don't alias it.
    let Some(entity) = sim.substrate.entities.get(id) else {
        return;
    };
    let cur = entity.health.current as i64;
    let max = entity.health.max as i64;
    let type_ref = entity.type_ref;
    let live_until_in = entity.damage_particle_live_until;
    let Some(obj) = rules.object(sim.interner.resolve(type_ref)) else {
        return;
    };

    // Outer gate. Check the cheap, near-always-false `emits_damage_spark` first so
    // the common path (every stock vehicle) exits before building the Spark list.
    // `HealthRatio < ConditionYellow` reproduced as the project's integer
    // cross-multiply (`GetHealthRatio` is current/max; STRICT `<` per the binary).
    // The `vtable+0x1c8() > -10` special-state term is unmodelled here → pass.
    let below_yellow = cur * 1000 < max * rules.general.condition_yellow_x1000;
    if !(obj.emits_damage_spark() && below_yellow) {
        return;
    }

    // Spark sublist: `DamageParticleSystems` entries resolving to a
    // `BehavesLike == Spark` particle system, in list order. Collect each one's
    // Lifetime for the `+0x308` hold; the list-pick indexes into this sublist.
    let mut spark_lifetimes: Vec<i32> = Vec::new();
    for name in &obj.damage_particle_systems {
        if let Some(ps_id) = rules.ps_type_id_by_name(name) {
            let pst = rules.particle_system_type(ps_id);
            if pst.behaves_like == ParticleSystemBehavesLike::Spark {
                spark_lifetimes.push(pst.lifetime);
            }
        }
    }
    let spark_count = spark_lifetimes.len() as u32;
    // Band select needs ConditionRed; bind once (both gate and draw read it).
    let below_red = cur * 1000 < max * rules.general.condition_red_x1000;

    // `+0x308`-equivalent live-system gate. Resolve expiry lazily here (the only
    // observable effect of the hold is gating draws, which only happen under this
    // gate, so lazy expiry yields the same draw sequence as gamemd's eager null).
    let tick = sim.session.tick;
    let mut live_until = live_until_in;
    if live_until != 0 && live_until != DAMAGE_PARTICLE_LIVE_FOREVER && tick >= live_until {
        live_until = 0; // system expired → `+0x308` nulls; may roll again this tick
    }
    let system_live = live_until != 0;

    // Inner gate: no live system AND at least one Spark system.
    if !system_live && spark_count > 0 {
        // Draw #1 — prob-roll on Scen->Random (always, on inner-gate pass).
        let roll = sim
            .scenario_rng
            .next_range_u32_inclusive(0, DAMAGE_SPARK_ROLL_MAX);
        let threshold = if below_red {
            rules.general.condition_red_spark_threshold
        } else {
            rules.general.condition_yellow_spark_threshold
        };
        if roll < threshold {
            // Draw #2 — list-pick (no draw when spark_count == 1: n(0,0)).
            let idx = sim
                .scenario_rng
                .next_range_u32_inclusive(0, spark_count - 1) as usize;
            let lifetime = spark_lifetimes[idx];
            live_until = if lifetime > 0 {
                tick.saturating_add(lifetime as u64)
            } else {
                DAMAGE_PARTICLE_LIVE_FOREVER
            };
        }
    }

    // Commit the (possibly cleared or freshly-armed) live-system state.
    if live_until != live_until_in {
        if let Some(entity) = sim.substrate.entities.get_mut(id) {
            entity.damage_particle_live_until = live_until;
        }
    }
}

/// Outcome of the S4a bracket for one Unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BracketReach {
    /// Died after the pre-block (health 0); no mission commit.
    DiedInPre,
    /// Non-miner live Unit: committed (`+0xC4` tick_counter + `derived_mission`)
    /// authoritatively at the host.
    Committed,
    /// Miner Unit: the host runs the bracket but does NOT commit — the miner
    /// session / tail projection owns the miner mission (deferred, as before S4a).
    MinerDeferred,
}

/// S4a per-Unit TechnoClass common bracket (AUTHORITATIVE — Option B flip).
/// Runs the contiguous `pre -> [IsAlive B] -> +0xC4/Mission_Dispatch ->
/// [IsAlive E] -> post` structure and commits the Unit's mission HERE, at the
/// gamemd-faithful per-object AI point (pre-movement, LogicVector order), rather
/// than in `movement_tick` (scoped movers) or the Phase-9 tail (idle) as before.
/// Miners stay tail-owned. Returns the outcome; the walk collects `Committed`
/// ids as the tail's skip-set.
fn unit_techno_bracket(sim: &mut Simulation, id: u64, rules: Option<&RuleSet>) -> BracketReach {
    techno_common_pre(sim, id);
    // Guard B (post-pre IsAlive): a health-0 Unit makes no commit. No lethal
    // pre-block step exists yet, so this fires only for an already-dead Unit.
    if !sim.substrate.entities.get(id).is_some_and(|e| e.is_alive()) {
        return BracketReach::DiedInPre;
    }
    // Miners are deferred to the tail projection (the miner session owns that
    // path); the host runs the bracket but skips the authoritative commit, so a
    // miner is NOT added to the skip-set and the tail commits it.
    if sim.substrate.entities.get(id).is_some_and(|e| e.miner.is_some()) {
        techno_common_post(sim, id, rules);
        return BracketReach::MinerDeferred;
    }
    // `+0xC4` AI-tick counter + `Mission_Dispatch` (AUTHORITATIVE commit): the
    // mission is a deterministic projection of the unit's machines, committed at
    // this per-object AI point. `current`/`substate` mirror `derived_mission`;
    // the verbs own `queued`/`suspended`/`timer`.
    if let Some(e) = sim.substrate.entities.get_mut(id) {
        e.mission.tick_counter = e.mission.tick_counter.wrapping_add(1);
        let (current, substate) = e.derived_mission();
        e.mission.current = current;
        e.mission.substate = substate;
    }
    // Guard E (post-dispatch IsAlive): a mission commit cannot kill the Unit, so
    // this cannot fire yet; the structure is preserved for the S5 dispatch
    // handlers that can self-destruct. The Unit is committed regardless.
    techno_common_post(sim, id, rules);
    BracketReach::Committed
}

/// S4c passive-acquire gate predicate (pure; the testable core of
/// `debug_s4c_passive_acquire_shadow`). A Unit reaches the passive-acquire
/// scanner iff its mission is in {Move(2), Guard(5), Harvest(10)}, it carries a
/// weapon, AND (`OpportunityFire` OR mission == Guard). The Guard term is the
/// verified gamemd behavior: a Guard-mission unit auto-acquires regardless of
/// `OpportunityFire` (decompiled `TechnoClass::PassiveAcquireGate` 0x00709290).
#[cfg(any(test, debug_assertions))]
fn s4c_passive_acquire_eligible(
    mission: MissionType,
    has_weapon: bool,
    opportunity_fire: bool,
) -> bool {
    matches!(
        mission,
        MissionType::Move | MissionType::Guard | MissionType::Harvest
    ) && has_weapon
        && (opportunity_fire || mission == MissionType::Guard)
}

// ===== Slice S1 — first behavior-bearing ordering (shadow) =====
//
// For one bounded scenario — a moving drive `UnitClass` on a pure Move mission —
// observe the mission decision THEN the locomotor `Process` marker within a
// single object pass, proving `dispatch_seq < process_seq` (the verified gamemd
// ordering: FootClass::AI runs the locomotor AFTER mission dispatch). Read-only,
// debug-only, never hashed — the authority flip is a later slice.

/// The bounded S1 scenario: a moving, drive-locomotor `UnitClass` on a pure
/// Move mission, with no combat, miner, dock, or aircraft concern.
/// `derived_mission()` yields exactly `(MissionType::Move, 0)` for this set.
/// Requiring a drive locomotor narrows the scope to the units the dispatch→
/// process ordering proof targets and makes the `is_drive` marker exact —
/// avoiding a false agreement-assert on a non-drive mover (ship / hover).
/// Consumed only by the S1 dispatch-before-Process shadow in this module (the
/// in-loop movement_tick consumer was retired by the S4a host-commit flip).
#[cfg(any(test, debug_assertions))]
pub(crate) fn is_s1_scoped_move_unit(e: &GameEntity) -> bool {
    e.category == EntityCategory::Unit
        && e.movement_target.is_some()
        && e.drive_locomotion.is_some()
        && e.miner.is_none()
        && e.dock_state.is_none()
        && e.attack_target.is_none()
        && e.aircraft_mission.is_none()
}

/// Read-only observation of one in-scope object's pass: where the mission
/// decision was observed relative to the locomotor `Process` marker. Never
/// committed to live state or the hash.
#[cfg(any(test, debug_assertions))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ShellTrace {
    /// Ordinal at which the mission decision was observed this pass.
    dispatch_seq: u32,
    /// Ordinal at which the locomotor `Process` marker was observed.
    process_seq: u32,
    /// The observed mission (must be `Move` for an in-scope unit).
    mission: MissionType,
    /// Whether the locomotor `Process` marker reported a drive unit.
    is_drive: bool,
}

/// S1 shadow step: for one in-scope moving Unit, observe the mission decision
/// and THEN the locomotor `Process` marker within a single object pass, and
/// return the trace. READ-ONLY — `&Simulation`, mutates nothing (no entity, no
/// occupancy, no hash). `seq` is a shared monotonic counter across the pass;
/// `dispatch_seq < process_seq` by construction proves the decision is observed
/// before Process. Returns `None` for any object outside the bounded scope (the
/// over-claim guard) — and never rewrites the observed mission (a divergence is
/// surfaced, never silently equalized).
#[cfg(any(test, debug_assertions))]
fn unit_ai_shadow_step(sim: &Simulation, id: u64, seq: &mut u32) -> Option<ShellTrace> {
    let entity = sim.substrate.entities.get(id)?;
    if !is_s1_scoped_move_unit(entity) {
        return None;
    }
    // Mission dispatch (decision) FIRST — the fresh per-object decision marker.
    // S4a: read `derived_mission` (the decision), NOT the committed `mission.current`:
    // post-flip the latter is the host's Phase-0 commit, which legitimately goes
    // stale when a unit retasks mid-tick (e.g. Attack at the host, move-scoped by
    // the Phase-9 read). The S1 proof is dispatch-decision-before-Process; the
    // decision for an in-scope move unit is `Move` by the scope predicate.
    let mission = entity.derived_mission().0;
    let dispatch_seq = *seq;
    *seq += 1;
    // Locomotor Process SECOND — the read-only drive presence marker.
    let outcome = process_drive_locomotion_shell(entity);
    let process_seq = *seq;
    *seq += 1;
    Some(ShellTrace {
        dispatch_seq,
        process_seq,
        mission,
        is_drive: matches!(outcome, DriveProcessOutcome::Processed),
    })
}

impl Simulation {
    /// Debug-only S1 shadow pass: walk the live order and, for each in-scope
    /// moving Unit, assert the mission decision is observed before the locomotor
    /// Process within one object pass (the verified gamemd ordering) and that
    /// the observed mission is the in-scope `Move`. Read-only; never hashed,
    /// never serialized; a divergence is asserted with tick + id, never silently
    /// equalized. (The Slice-2 mission shadow-agreement assert it once mirrored
    /// was retired in Slice 8 when `mission` became hashed-authoritative.)
    #[cfg(any(test, debug_assertions))]
    pub(crate) fn debug_assert_s1_shadow(&self) {
        let mut seq = 0u32;
        for id in self.live_object_order_snapshot() {
            let Some(trace) = unit_ai_shadow_step(self, id, &mut seq) else {
                continue;
            };
            debug_assert!(
                trace.dispatch_seq < trace.process_seq,
                "S1: tick {} unit {}: dispatch_seq {} must precede process_seq {}",
                self.session.tick,
                id,
                trace.dispatch_seq,
                trace.process_seq,
            );
            debug_assert_eq!(
                trace.mission,
                MissionType::Move,
                "S1: tick {} unit {}: in-scope unit must derive Move, observed {:?}",
                self.session.tick,
                id,
                trace.mission,
            );
            debug_assert!(
                trace.is_drive,
                "S1: tick {} unit {}: in-scope unit must be a drive mover",
                self.session.tick,
                id,
            );
        }
    }
}

// ===== P2 (factory substrate) — Structure-arm read-only shadow trace (FIT a) =====
//
// FIT option (a): the per-(house, category) factory step is driven from the
// Structure arm of object_ai_stage() in LogicVector order; the FactoryRegistry is
// a LOOKUP, not a tick-loop owner. In P1+P2 there is no authoritative step, so the
// `EntityCategory::Structure` arm stays a no-op and this debug-only trace records
// each live Structure in LogicVector order — the same "proof lives beside, not
// inside, the no-op arm" shape as the S1 shadow. The order-follows-LogicVector
// property is proven by a test that injects a known non-sorted order
// (`factory_shadow_trace_order_matches_logic_vector`); the runtime debug_assert
// only checks the cheap intrinsic invariants (strictly-increasing visit ordinal;
// each traced id resolves to a live, non-dying Structure). Read-only, never hashed.

/// One Structure visited by the P2 factory shell trace, in LogicVector order.
#[cfg(any(test, debug_assertions))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FactoryShellTrace {
    structure_id: u64,
    visit_seq: u32,
}

impl Simulation {
    /// Build the P2 factory shell trace: each live, non-dying Structure in
    /// LogicVector order. Read-only; never hashed, never serialized. The order IS
    /// LogicVector order by construction (it walks `live_object_order_snapshot`) —
    /// the FIT-(a) ordering, exercised by the injected-order test.
    #[cfg(any(test, debug_assertions))]
    fn factory_shell_trace(&self) -> Vec<FactoryShellTrace> {
        let mut seq = 0u32;
        let mut traces: Vec<FactoryShellTrace> = Vec::new();
        for id in self.live_object_order_snapshot() {
            let is_live_structure = self
                .substrate
                .entities
                .get(id)
                .is_some_and(|e| !e.dying && e.category == EntityCategory::Structure);
            if !is_live_structure {
                continue;
            }
            traces.push(FactoryShellTrace {
                structure_id: id,
                visit_seq: seq,
            });
            seq += 1;
        }
        traces
    }

    /// Test-only accessor: the structure ids the P2 trace visits, in order. The
    /// test injects a non-sorted live order and asserts this equals it (so it
    /// would fail if the trace used BTreeMap/entity-id order instead).
    #[cfg(test)]
    pub(crate) fn factory_shell_trace_order(&self) -> Vec<u64> {
        self.factory_shell_trace()
            .iter()
            .map(|t| t.structure_id)
            .collect()
    }

    /// Debug-only P2 assert: the factory shell trace visits live, non-dying
    /// Structures with a strictly-increasing visit ordinal. INTRINSIC invariants
    /// only — not a self-comparison; the LogicVector-order property is proven by a
    /// dedicated injected-order test, never re-derived here.
    #[cfg(any(test, debug_assertions))]
    pub(crate) fn debug_assert_factory_shell_trace(&self) {
        let traces = self.factory_shell_trace();
        for w in traces.windows(2) {
            debug_assert!(
                w[0].visit_seq < w[1].visit_seq,
                "P2: tick {}: factory shell trace visit_seq must strictly increase",
                self.session.tick,
            );
        }
        for t in &traces {
            debug_assert!(
                self.substrate
                    .entities
                    .get(t.structure_id)
                    .is_some_and(|e| !e.dying && e.category == EntityCategory::Structure),
                "P2: tick {}: factory shell trace id {} must resolve to a live Structure",
                self.session.tick,
                t.structure_id,
            );
        }
    }

    /// Test-only P3 oracle probe: walk live Structures in LogicVector order and, for
    /// each, step a CLONE of its owner's factories against a CLONE of the owner's
    /// economy — exercising `set_rate` + `advance_one_step` on throwaways. READ-ONLY
    /// w.r.t. all hashed state: it writes only local clones, NEVER the registry, the
    /// wallet, or any entity. The `EntityCategory::Structure` arm stays a no-op; this
    /// is the "proof beside the no-op" shape (FIT option a) and the P5 precursor (the
    /// flip swaps the arm body, not the iteration source). The full per-building
    /// Primary_For* routing is a later slice — the probe uses a bounded per-owner
    /// scope (every factory the visited Structure's owner holds), hash-neutral
    /// regardless of routing precision.
    #[cfg(test)]
    pub(crate) fn factory_oracle_step_trace(&self) -> Vec<(u64, StepOutcome)> {
        use crate::sim::economy::Economy;
        let mut out: Vec<(u64, StepOutcome)> = Vec::new();
        for id in self.live_object_order_snapshot() {
            let Some(entity) = self.substrate.entities.get(id) else {
                continue;
            };
            if entity.dying || entity.category != EntityCategory::Structure {
                continue;
            }
            let owner = entity.owner;
            // Clone the owner's economy (the oracle wallet); default if no house.
            let mut oracle_econ = self
                .houses
                .get(&owner)
                .map(|h| h.economy.clone())
                .unwrap_or_default();
            // Bounded scope: step a CLONE of each of this owner's factories. The
            // registry is a LOOKUP (FIT a); we read it, never mutate it.
            for factory in self.production.factory_shadow.iter_insertion_ordered() {
                if factory.owner != owner || factory.object.is_none() {
                    continue;
                }
                let mut oracle_factory = factory.clone();
                // Exercise SetRate (build-step total is a placeholder until the
                // GetBuildStepTime pipeline lands; original_balance is a stand-in
                // input — the probe proves the step machine runs, not the rate value).
                oracle_factory.set_rate(oracle_factory.original_balance);
                let outcome = oracle_factory.advance_one_step(&mut oracle_econ);
                out.push((id, outcome));
                // local clones dropped here; nothing written back.
            }
        }
        out
    }

    /// Test-only dormant probe (P5a): prove the C7 delivery -> start_next_queued
    /// mechanics on a CLONE of the registry (NEVER the hashed shadow). Returns, per
    /// factory, (owner, category, popped-front-after-a-simulated-delivery). NO
    /// authoritative call site — a later slice binds start_next_queued to the real
    /// delivery commit; this only proves the post-delivery pop end-to-end.
    #[cfg(test)]
    pub(crate) fn factory_delivery_probe(
        &self,
    ) -> Vec<(
        crate::sim::intern::InternedId,
        crate::sim::production::ProductionCategory,
        Option<crate::sim::intern::InternedId>,
    )> {
        let mut out = Vec::new();
        for factory in self.production.factory_shadow.iter_insertion_ordered() {
            let mut d = factory.clone();
            d.object = None; // simulate the delivery commit
            d.suspended = false;
            let popped = d.start_next_queued(0, 0);
            out.push((factory.owner, factory.category, popped));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::aircraft::AircraftMission;
    use crate::sim::combat::{AttackTarget, TargetKind};
    use crate::sim::components::{DriveLocomotionRuntime, MovementTarget};
    use crate::sim::docking::building_dock::{DockPhase, DockState};
    use crate::sim::game_entity::GameEntity;
    use crate::sim::miner::{Miner, MinerConfig, MinerKind};
    use crate::sim::mission::MissionType;

    /// Build a test entity of a specific category (`test_default` makes a Unit).
    fn entity_of(id: u64, category: EntityCategory) -> GameEntity {
        let mut e = GameEntity::test_default(id, "TEST", "Americans", 5, 5);
        e.category = category;
        e
    }

    #[test]
    fn object_ai_stage_commits_live_unit_mission() {
        // S4a (Option B): the stage AUTHORITATIVELY commits each live non-miner
        // Unit's mission (+0xC4 tick_counter + derived_mission); non-Units are
        // untouched here (the Phase-9 tail projects them).
        let mut sim = Simulation::new();
        sim.substrate
            .entities
            .insert(entity_of(1, EntityCategory::Unit));
        sim.substrate
            .entities
            .insert(entity_of(2, EntityCategory::Infantry));
        sim.substrate
            .entities
            .insert(entity_of(3, EntityCategory::Structure));
        sim.substrate
            .entities
            .insert(entity_of(4, EntityCategory::Aircraft));
        sim.set_logic_order_for_test(vec![1, 2, 3, 4]);

        let mut committed = std::collections::BTreeSet::new();
        sim.object_ai_stage(None, &mut committed);

        // The idle non-miner Unit committed Guard and ticked its counter once.
        assert_eq!(committed.iter().copied().collect::<Vec<_>>(), vec![1]);
        let u = sim.substrate.entities.get(1).unwrap();
        assert_eq!(u.mission.current, MissionType::Guard);
        assert_eq!(u.mission.tick_counter, 1);
        // Non-Units are not committed by the host (tail owns them): counter at 0.
        for id in [2u64, 3, 4] {
            assert_eq!(
                sim.substrate.entities.get(id).unwrap().mission.tick_counter,
                0,
                "non-Unit {id} must not be committed by the host"
            );
        }
    }

    #[test]
    fn techno_ai_shell_membership_matches_phase_snapshot() {
        let mut sim = Simulation::new();
        sim.substrate
            .entities
            .insert(entity_of(1, EntityCategory::Unit));
        sim.substrate
            .entities
            .insert(entity_of(2, EntityCategory::Structure));
        sim.substrate
            .entities
            .insert(entity_of(3, EntityCategory::Aircraft));
        // Deliberately NON-sorted order to prove the walk preserves live order
        // verbatim (no sort).
        sim.set_logic_order_for_test(vec![3, 1, 2]);

        let visited = sim.object_ai_walk(true, None, &mut std::collections::BTreeSet::new());
        assert_eq!(
            visited,
            sim.live_object_order_snapshot(),
            "every live object visited exactly once, in live order"
        );
        assert_eq!(visited, vec![3, 1, 2], "live order preserved verbatim (no sort)");
    }

    #[test]
    fn techno_ai_shell_preserves_advance_tick_phase_order() {
        // The stage is wired into advance_tick (called every tick, before
        // refresh_mission_shadow). Identical fixtures must produce identical
        // per-tick state_hash sequences — the stage introduces no nondeterminism
        // and no panic. Together with the commit proof
        // (object_ai_stage_commits_live_unit_mission, which exercises the entity
        // walk directly) this shows the stage perturbs no phase and no surrounding
        // ordering beyond its own mission commit. The fixture is intentionally entity-free:
        // raw test_default entities carry interned ids that advance_tick's
        // entity systems would resolve against an empty interner (a fixture
        // concern unrelated to the stage); the stage still runs each tick over
        // the empty live order.
        fn run() -> Vec<u64> {
            let mut sim = Simulation::new();
            let heights = std::collections::BTreeMap::new();
            (0..5)
                .map(|_| {
                    sim.advance_tick(&[], None, &heights, None, None, 67);
                    sim.state_hash()
                })
                .collect()
        }
        assert_eq!(
            run(),
            run(),
            "advance_tick with the object-AI stage stays deterministic"
        );
    }

    #[test]
    fn object_ai_stage_skips_dying_object() {
        let mut sim = Simulation::new();
        sim.substrate
            .entities
            .insert(entity_of(1, EntityCategory::Unit));
        sim.substrate
            .entities
            .insert(entity_of(2, EntityCategory::Unit));
        sim.set_logic_order_for_test(vec![1, 2]);
        // Mark id 2 dying AFTER set_logic_order_for_test — that helper resets
        // presence / in_logic_vector but does NOT touch `dying`, and id 2 stays
        // in the live order.
        sim.substrate.entities.get_mut(2).unwrap().dying = true;

        let visited = sim.object_ai_walk(true, None, &mut std::collections::BTreeSet::new());
        assert_eq!(
            visited,
            vec![1],
            "dying object skipped; the live object is still visited"
        );
        // The internal order-proof assert filters dying members, so the stage
        // must not panic even with a dying member in the live order.
        sim.object_ai_stage(None, &mut std::collections::BTreeSet::new());
    }

    #[test]
    fn object_ai_stage_tolerates_absent_id_in_order() {
        let mut sim = Simulation::new();
        let live_id = 1u64;
        let absent_id = 999u64;
        sim.substrate
            .entities
            .insert(entity_of(live_id, EntityCategory::Unit));
        // Force the live order to include an id with no entity in the store
        // (set_logic_order_for_test only flips flags on existing ids, so set the
        // order directly to keep the absent id a non-member with no entity).
        sim.substrate
            .logic
            .set_order_for_test(vec![absent_id, live_id]);

        let visited = sim.object_ai_walk(true, None, &mut std::collections::BTreeSet::new());
        assert_eq!(
            visited,
            vec![live_id],
            "absent id skipped without panic; live id still visited"
        );
        // Stage must not panic on the absent member either.
        sim.object_ai_stage(None, &mut std::collections::BTreeSet::new());
    }

    // ===== Slice S4a — TechnoClass common bracket (authoritative, Option B) =====

    #[test]
    fn s4a_bracket_commits_live_non_miner_unit() {
        let mut sim = Simulation::new();
        sim.substrate
            .entities
            .insert(entity_of(1, EntityCategory::Unit));
        // A live non-miner Unit reaches the dispatch point and commits: +0xC4
        // tick_counter + derived_mission (idle -> Guard).
        assert_eq!(unit_techno_bracket(&mut sim, 1, None), BracketReach::Committed);
        let u = sim.substrate.entities.get(1).unwrap();
        assert_eq!(u.mission.tick_counter, 1);
        assert_eq!(u.mission.current, MissionType::Guard);
    }

    #[test]
    fn s4a_bracket_pre_guard_short_circuits_dead_unit() {
        let mut sim = Simulation::new();
        let mut e = entity_of(1, EntityCategory::Unit);
        e.health.current = 0; // not alive
        sim.substrate.entities.insert(e);
        // Guard B fires after the (empty) pre-block: a health-0 Unit makes no
        // commit (counter stays 0) and never reaches the dispatch point.
        assert_eq!(unit_techno_bracket(&mut sim, 1, None), BracketReach::DiedInPre);
        assert_eq!(sim.substrate.entities.get(1).unwrap().mission.tick_counter, 0);
    }

    #[test]
    fn s4a_bracket_defers_miner_to_tail() {
        let mut sim = Simulation::new();
        let mut miner = entity_of(1, EntityCategory::Unit);
        miner.miner = Some(Miner::new(MinerKind::War, &MinerConfig::default(), 0));
        sim.substrate.entities.insert(miner);
        // A miner runs the bracket but the host does NOT commit it — the miner
        // session / tail projection owns the miner mission (counter stays 0).
        assert_eq!(unit_techno_bracket(&mut sim, 1, None), BracketReach::MinerDeferred);
        assert_eq!(sim.substrate.entities.get(1).unwrap().mission.tick_counter, 0);
    }

    // ===== Slice S4c — passive-acquire eligibility gate (shadow) =====

    #[test]
    fn s4c_gate_move_with_opportunity_fire_and_weapon_eligible() {
        assert!(s4c_passive_acquire_eligible(MissionType::Move, true, true));
    }

    #[test]
    fn s4c_gate_guard_with_weapon_eligible_without_opportunity_fire() {
        // Guard units auto-acquire regardless of OpportunityFire (verified gate).
        assert!(s4c_passive_acquire_eligible(MissionType::Guard, true, false));
    }

    #[test]
    fn s4c_gate_harvest_with_opportunity_fire_eligible() {
        assert!(s4c_passive_acquire_eligible(MissionType::Harvest, true, true));
    }

    #[test]
    fn s4c_gate_move_without_opportunity_fire_not_eligible() {
        assert!(!s4c_passive_acquire_eligible(MissionType::Move, true, false));
    }

    #[test]
    fn s4c_gate_no_weapon_not_eligible_even_on_guard() {
        // The weapon (CanAcquireTarget equip) gate applies to ALL paths, incl Guard.
        assert!(!s4c_passive_acquire_eligible(MissionType::Guard, false, true));
        assert!(!s4c_passive_acquire_eligible(MissionType::Move, false, true));
    }

    #[test]
    fn s4c_gate_off_mission_not_eligible() {
        // Missions outside {Move,Guard,Harvest} never reach the passive-acquire block.
        assert!(!s4c_passive_acquire_eligible(MissionType::Attack, true, true));
        assert!(!s4c_passive_acquire_eligible(MissionType::Sleep, true, true));
    }

    #[test]
    fn s4c_shadow_is_hash_neutral() {
        // The shadow is read-only; calling it must not move the lockstep hash.
        let mut sim = Simulation::new();
        sim.substrate
            .entities
            .insert(entity_of(1, EntityCategory::Unit));
        sim.set_logic_order_for_test(vec![1]);
        let before = sim.state_hash();
        let _ = sim.debug_s4c_passive_acquire_shadow(None);
        let after = sim.state_hash();
        assert_eq!(before, after, "S4c shadow must not perturb the state hash");
    }

    // ===== Slice S1 — dispatch-before-locomotor shadow =====

    /// A bounded-S1-scoped unit: a moving drive `UnitClass` with no combat,
    /// miner, dock, or aircraft concern. `derived_mission()` yields `(Move, 0)`.
    fn scoped_move_unit(id: u64) -> GameEntity {
        let mut e = GameEntity::test_default(id, "TEST", "Americans", 5, 5); // category Unit
        e.movement_target = Some(MovementTarget::default());
        e.drive_locomotion = Some(DriveLocomotionRuntime::default());
        e
    }

    #[test]
    fn unit_ai_mission_dispatch_precedes_locomotor_process() {
        let mut sim = Simulation::new();
        sim.substrate.entities.insert(scoped_move_unit(1));
        sim.set_logic_order_for_test(vec![1]);
        sim.refresh_mission_shadow(); // mission.current = derived_mission() = Move

        let mut seq = 0u32;
        let trace =
            unit_ai_shadow_step(&sim, 1, &mut seq).expect("a scoped move unit yields a trace");
        assert!(
            trace.dispatch_seq < trace.process_seq,
            "mission dispatch must be observed before the locomotor Process"
        );
        assert_eq!(trace.mission, MissionType::Move);
        assert!(trace.is_drive);
    }

    #[test]
    fn unit_move_dispatch_then_process_shadow_reads_fresh_decision() {
        let mut sim = Simulation::new();
        sim.substrate.entities.insert(scoped_move_unit(1));
        sim.substrate.entities.insert(scoped_move_unit(2));
        sim.set_logic_order_for_test(vec![1, 2]);
        sim.refresh_mission_shadow();

        // Agreement: every in-scope unit derives Move / is_drive / dispatch <
        // process, so the shadow pass asserts cleanly (no divergence, no panic).
        sim.debug_assert_s1_shadow();

        // S4a: the shadow reads the FRESH per-object decision (`derived_mission`),
        // NOT the committed `mission.current`. Post-flip the committed value is the
        // host's Phase-0 commit and can go stale (a unit retasked mid-tick), so a
        // stale committed value must NOT change what the shadow observes: an
        // in-scope move unit's decision is `Move` regardless of the stale commit.
        sim.substrate.entities.get_mut(1).unwrap().mission.current = MissionType::Guard;
        let mut seq = 0u32;
        let trace = unit_ai_shadow_step(&sim, 1, &mut seq).expect("still in scope");
        assert_eq!(
            trace.mission,
            MissionType::Move,
            "shadow reads the fresh decision (Move), not the stale committed mission"
        );
    }

    #[test]
    fn s1_no_hash_change_shadow() {
        let mut sim = Simulation::new();
        sim.substrate.entities.insert(scoped_move_unit(1));
        sim.set_logic_order_for_test(vec![1]);
        sim.refresh_mission_shadow();

        let before = sim.state_hash();
        sim.debug_assert_s1_shadow(); // read-only shadow pass
        let after = sim.state_hash();
        assert_eq!(before, after, "the S1 shadow pass must not perturb the state hash");
    }

    #[test]
    fn s1_shadow_skips_non_scoped_units() {
        let mut sim = Simulation::new();

        // Miner (highest derived-mission priority) — disqualified.
        let mut miner = scoped_move_unit(1);
        miner.miner = Some(Miner::new(MinerKind::War, &MinerConfig::default(), 0));
        sim.substrate.entities.insert(miner);

        // Docking unit — disqualified.
        let mut docking = scoped_move_unit(2);
        docking.dock_state = Some(DockState {
            dock_building_id: 99,
            phase: DockPhase::Approach,
            service_timer: 0,
            no_funds_ticks: 0,
        });
        sim.substrate.entities.insert(docking);

        // Attacking unit — disqualified.
        let mut attacking = scoped_move_unit(3);
        attacking.attack_target = Some(AttackTarget {
            target: TargetKind::Entity(99),
            cooldown_ticks: 0,
            burst_remaining: 1,
            burst_delay_ticks: 0,
            pending_infantry_fire: None,
        });
        sim.substrate.entities.insert(attacking);

        // Aircraft — disqualified by the category gate.
        let mut aircraft = scoped_move_unit(4);
        aircraft.category = EntityCategory::Aircraft;
        aircraft.aircraft_mission = Some(AircraftMission::Guard);
        sim.substrate.entities.insert(aircraft);

        let mut seq = 0u32;
        for id in [1u64, 2, 3, 4] {
            assert!(
                unit_ai_shadow_step(&sim, id, &mut seq).is_none(),
                "non-scoped object {id} must be skipped by the S1 shadow"
            );
        }
        assert_eq!(seq, 0, "skipped objects never advance the ordinal counter");
    }

    #[test]
    fn s1_shadow_preserves_advance_tick_phase_order() {
        // The shadow runs (debug builds) inside advance_tick, between
        // refresh_mission_shadow and state_hash. Identical fixtures must produce
        // identical per-tick hash sequences — the read-only shadow perturbs no
        // phase. Entity-free for the same interner reason as
        // techno_ai_shell_preserves_advance_tick_phase_order.
        fn run() -> Vec<u64> {
            let mut sim = Simulation::new();
            let heights = std::collections::BTreeMap::new();
            (0..5)
                .map(|_| {
                    sim.advance_tick(&[], None, &heights, None, None, 67);
                    sim.state_hash()
                })
                .collect()
        }
        assert_eq!(run(), run());
    }

    // ===== Slice S2a — host-time Unit dispatch shadow =====

    #[test]
    fn unit_dispatch_record_pass_skips_miner_and_nonunit() {
        let mut sim = Simulation::new();
        // A plain moving Unit — recorded.
        sim.substrate.entities.insert(scoped_move_unit(1));
        // A miner Unit — skipped (the miner session owns Harvest).
        let mut miner = scoped_move_unit(2);
        miner.miner = Some(Miner::new(MinerKind::War, &MinerConfig::default(), 0));
        sim.substrate.entities.insert(miner);
        // A non-Unit — skipped by category.
        sim.substrate
            .entities
            .insert(entity_of(3, EntityCategory::Structure));
        sim.set_logic_order_for_test(vec![1, 2, 3]);

        let trace = sim.unit_dispatch_record_pass();
        assert_eq!(trace.len(), 1, "only the non-miner Unit is recorded");
        assert_eq!(trace[0].id, 1);
        assert_eq!(trace[0].host_mission, MissionType::Move);
        assert_eq!(trace[0].family, DispatchSlot::Move);
    }

    #[test]
    fn unit_dispatch_proof_passes_on_scoped_units() {
        let mut sim = Simulation::new();
        sim.substrate.entities.insert(scoped_move_unit(1)); // Move
        let mut idle = scoped_move_unit(2);
        idle.movement_target = None; // S3: idle Unit derives Guard -> Guard family
        sim.substrate.entities.insert(idle);
        sim.set_logic_order_for_test(vec![1, 2]);

        let trace = sim.unit_dispatch_record_pass();
        sim.debug_assert_unit_dispatch_shadow(&trace); // no panic
        assert_eq!(trace[0].family, DispatchSlot::Move);
        assert_eq!(trace[1].family, DispatchSlot::Guard);
    }

    #[test]
    fn unit_dispatch_attackmove_unreachable_for_units() {
        // derived_mission never yields AttackMove for any machine combination.
        let mut e = GameEntity::test_default(1, "TEST", "Americans", 5, 5);
        e.movement_target = Some(MovementTarget::default());
        e.attack_target = Some(AttackTarget {
            target: TargetKind::Entity(99),
            cooldown_ticks: 0,
            burst_remaining: 1,
            burst_delay_ticks: 0,
            pending_infantry_fire: None,
        });
        assert_ne!(e.derived_mission().0, MissionType::AttackMove);
    }

    #[test]
    fn dispatch_live_set_covers_moving_units() {
        let mut sim = Simulation::new();
        sim.substrate.entities.insert(scoped_move_unit(1)); // movement_target set, in live order
        sim.set_logic_order_for_test(vec![1]);
        // Must not log/panic: the moving Unit is in the live set.
        sim.debug_check_dispatch_live_set_coverage();
    }

    #[test]
    fn unit_dispatch_host_commits_scoped_mover() {
        // S4a: the host commits the scoped mover's mission (Move) authoritatively
        // and reports it in the skip-set; the read-only proofs must not panic.
        let mut sim = Simulation::new();
        sim.substrate.entities.insert(scoped_move_unit(1));
        sim.set_logic_order_for_test(vec![1]);

        let mut committed = std::collections::BTreeSet::new();
        let trace = sim.object_ai_stage(None, &mut committed); // host pass (commits id 1)
        sim.debug_assert_unit_dispatch_shadow(&trace); // read-only proof (no panic)
        sim.debug_check_dispatch_live_set_coverage(); // read-only coverage

        assert!(committed.contains(&1), "the scoped mover is committed by the host");
        let u = sim.substrate.entities.get(1).unwrap();
        assert_eq!(u.mission.current, MissionType::Move);
        assert_eq!(u.mission.tick_counter, 1);
    }

    #[test]
    fn unit_dispatch_preserves_advance_tick_phase_order() {
        fn run() -> Vec<u64> {
            let mut sim = Simulation::new();
            let heights = std::collections::BTreeMap::new();
            (0..5)
                .map(|_| {
                    sim.advance_tick(&[], None, &heights, None, None, 67);
                    sim.state_hash()
                })
                .collect()
        }
        assert_eq!(
            run(),
            run(),
            "advance_tick with the dispatch host stays deterministic"
        );
    }

    #[test]
    fn unit_dispatch_shadow_counts_churn() {
        // Guards the churn counter against a stuck-at-zero bug: a unit recorded as
        // host-time Move whose machine changes before the tail re-derivation (here we
        // clear movement_target → tail derives None → Sleep) must count as one churn.
        let mut sim = Simulation::new();
        sim.substrate.entities.insert(scoped_move_unit(1)); // host: Move
        sim.set_logic_order_for_test(vec![1]);
        let trace = sim.unit_dispatch_record_pass(); // captures host = Move
        assert_eq!(trace[0].family, DispatchSlot::Move);
        // Simulate the unit's machines changing between host-time and tail.
        sim.substrate.entities.get_mut(1).unwrap().movement_target = None;
        let churn = sim.debug_assert_unit_dispatch_shadow(&trace);
        assert_eq!(churn, 1, "a host Move that became tail Sleep must count as one churn");
    }

    // ===== Slice S2 — in-loop dispatch authority =====

    /// Like `scoped_move_unit`, but interned through the SIM's interner so the
    /// unit survives a real `advance_tick` (test_intern ids don't exist in
    /// `sim.interner`, and tick-path resolves would panic).
    fn insert_s2_scoped_move_unit(sim: &mut Simulation, id: u64, rx: u16, ry: u16) {
        let owner = sim.interner.intern("Americans");
        let type_ref = sim.interner.intern("TEST");
        let mut e = GameEntity::new(
            id,
            rx,
            ry,
            0, // z = ground level
            0, // facing = north
            owner,
            crate::sim::components::Health { current: 100, max: 100 },
            type_ref,
            EntityCategory::Unit,
            0, // veterancy = rookie
            5, // vision_range = 5 cells
            true,
        );
        e.movement_target = Some(MovementTarget::default());
        e.drive_locomotion = Some(DriveLocomotionRuntime::default());
        sim.substrate.entities.insert(e);
    }

    /// S2: the arrival tick hashes the dispatch-time mission (`Move`); the
    /// transition away happens on the NEXT tick (gamemd-faithful). S3: the
    /// post-arrival idle mission is `Guard` — the gamemd Move→Guard sequence.
    #[test]
    fn arrival_tick_mission_is_move_not_sleep() {
        let mut sim = Simulation::new();
        insert_s2_scoped_move_unit(&mut sim, 1, 5, 5); // default target: arrives tick 1
        sim.set_logic_order_for_test(vec![1]);
        let heights = std::collections::BTreeMap::new();

        let _ = sim.advance_tick(&[], None, &heights, None, None, 67);
        let e = sim.substrate.entities.get(1).unwrap();
        assert!(e.movement_target.is_none(), "fixture must arrive on tick 1");
        assert_eq!(e.mission.current, MissionType::Move, "arrival tick keeps Move");

        let _ = sim.advance_tick(&[], None, &heights, None, None, 67);
        let e = sim.substrate.entities.get(1).unwrap();
        // S3 idle→Guard: a machine-less idle Unit derives Guard (gamemd's
        // post-arrival idle mission), not the legacy None placeholder.
        assert_eq!(e.mission.current, MissionType::Guard, "post-arrival tick → Guard");
    }

    /// S3 (G5 pin): the tail projection treats dying Units uniformly — a dying
    /// machine-less Unit also projects Guard. Corpse-mission freeze (gamemd
    /// does not re-derive a corpse's mission) is deferred to the
    /// deferred-delete substrate. Scope of the divergence window (review-
    /// corrected): voxel-death corpses are freed by flush_pending_delete
    /// BEFORE the tail projection and hash, so they never hit this path; only
    /// SHP-art Units with death animations linger, for the duration of the
    /// death anim (app-driven despawn). Pre-S3 the same unfiltered projection
    /// rewrote those corpses to None each tick — S3 changes the value, not
    /// the window. Pinned here so the choice is intentional, not accidental.
    #[test]
    fn dying_unit_projection_uniform() {
        let mut sim = Simulation::new();
        insert_s2_scoped_move_unit(&mut sim, 1, 5, 5);
        sim.substrate.entities.get_mut(1).unwrap().movement_target = None; // idle
        sim.substrate.entities.get_mut(1).unwrap().dying = true;
        sim.refresh_mission_shadow();
        assert_eq!(
            sim.substrate.entities.get(1).unwrap().mission.current,
            MissionType::Guard,
            "dying machine-less Unit projects Guard (uniform tail projection)"
        );
    }

    /// S2: exactly one tick_counter increment per unit-tick — in-loop for a
    /// dispatched mover, tail for an idle (never-collected) unit. Double or
    /// zero count is permanent lockstep drift.
    #[test]
    fn s2_tick_counter_increments_exactly_once() {
        let mut sim = Simulation::new();
        insert_s2_scoped_move_unit(&mut sim, 1, 5, 5); // dispatched on tick 1
        insert_s2_scoped_move_unit(&mut sim, 2, 8, 8);
        // never collected; never scoped
        sim.substrate.entities.get_mut(2).unwrap().movement_target = None;
        sim.set_logic_order_for_test(vec![1, 2]);
        let heights = std::collections::BTreeMap::new();

        let _ = sim.advance_tick(&[], None, &heights, None, None, 67);
        assert_eq!(sim.substrate.entities.get(1).unwrap().mission.tick_counter, 1);
        assert_eq!(sim.substrate.entities.get(2).unwrap().mission.tick_counter, 1);
        let _ = sim.advance_tick(&[], None, &heights, None, None, 67);
        assert_eq!(sim.substrate.entities.get(1).unwrap().mission.tick_counter, 2);
        assert_eq!(sim.substrate.entities.get(2).unwrap().mission.tick_counter, 2);
    }

    /// S2 P1 guard: a save taken on the arrival tick (current=Move while a fresh
    /// derivation says None) must restore an IDENTICAL state hash. Guards the
    /// deleted post-load re-derive against reintroduction.
    #[test]
    fn save_load_round_trip_on_arrival_tick() {
        use crate::sim::snapshot::GameSnapshot;
        let mut sim = Simulation::new();
        insert_s2_scoped_move_unit(&mut sim, 1, 5, 5);
        sim.set_logic_order_for_test(vec![1]);
        let heights = std::collections::BTreeMap::new();
        let _ = sim.advance_tick(&[], None, &heights, None, None, 67); // arrival tick

        let e = sim.substrate.entities.get(1).unwrap();
        assert_eq!(e.mission.current, MissionType::Move, "precondition: divergent window");
        assert!(e.movement_target.is_none());
        let hash_before = sim.state_hash();

        let bytes = GameSnapshot::save(&sim, 0, 0, "test_map", 0);
        let mut restored = GameSnapshot::load(&bytes).expect("load").sim;
        restored.rebuild_logic_membership(); // the real post-deserialize step
        assert_eq!(
            restored.state_hash(),
            hash_before,
            "load must trust serialized MissionCom"
        );
        assert_eq!(
            restored.substrate.entities.get(1).unwrap().mission.current,
            MissionType::Move,
        );
    }

    // ===== Slice S4b — AI_Update damage-Spark scenario_rng consumption =====

    /// Two Spark systems, each `Lifetime=5`, so the list-pick (count==2) consumes
    /// a draw and the armed hold is `tick+5` regardless of the picked index.
    const TWO_SPARK_SYSTEMS: &str = "[ParticleSystems]\n\
1=SparkA\n2=SparkB\n\n[SparkA]\nBehavesLike=Spark\nLifetime=5\n\n[SparkB]\nBehavesLike=Spark\nLifetime=5\n";

    /// One Spark (`Lifetime=5`) plus one Smoke — exercises the Spark filter and the
    /// single-Spark list-pick (count==1 → no draw, matching `n(0,0)`).
    const ONE_SPARK_ONE_SMOKE_SYSTEMS: &str = "[ParticleSystems]\n\
1=SparkA\n2=SmokeA\n\n[SparkA]\nBehavesLike=Spark\nLifetime=5\n\n[SmokeA]\nBehavesLike=Smoke\n";

    /// A Smoke-only damage particle system — no Spark, so the inner gate never
    /// passes (zero draws even below ConditionRed).
    const SMOKE_ONLY_SYSTEMS: &str = "[ParticleSystems]\n1=SmokeA\n\n[SmokeA]\nBehavesLike=Smoke\n";

    /// Minimal RuleSet with one `Cyborg=yes` infantry "CYB" (so `emits_damage_spark`
    /// is true), `DamageParticleSystems=dps`, the named particle `systems`, and the
    /// two damage-Spark probabilities. prob "1.0" → always-succeed threshold; "0.0"
    /// → always-fail — so the draw outcome is deterministic regardless of the seed's
    /// actual roll value.
    fn cyborg_rules(red_prob: &str, yellow_prob: &str, dps: &str, systems: &str) -> RuleSet {
        use crate::rules::ini_parser::IniFile;
        let text = format!(
            "[General]\n\
BuildSpeed=0.75\nMultipleFactory=0.7\nLowPowerPenaltyModifier=1.25\n\
MinLowPowerProductionSpeed=0.4\nMaxLowPowerProductionSpeed=0.85\n\
ConditionRedSparkingProbability={red_prob}\nConditionYellowSparkingProbability={yellow_prob}\n\n\
[InfantryTypes]\n1=CYB\n[VehicleTypes]\n[AircraftTypes]\n[BuildingTypes]\n\n\
[CYB]\nCyborg=yes\nDamageParticleSystems={dps}\n\n{systems}\n"
        );
        RuleSet::from_ini(&IniFile::from_str(&text)).expect("cyborg test rules parse")
    }

    /// Insert a unit whose type resolves to the Cyborg infantry "CYB". The entity
    /// category is `Unit` (the only arm hosting `techno_common_post` today); the
    /// gate keys off the TYPE's `emits_damage_spark`, so this exercises the draw
    /// path. `current`/`max` set the health band.
    fn insert_cyborg_unit(sim: &mut Simulation, id: u64, current: u16, max: u16) {
        let owner = sim.interner.intern("Americans");
        let type_ref = sim.interner.intern("CYB");
        let e = GameEntity::new(
            id,
            5,
            5,
            0,
            0,
            owner,
            crate::sim::components::Health { current, max },
            type_ref,
            EntityCategory::Unit,
            0,
            5,
            true,
        );
        sim.substrate.entities.insert(e);
    }

    fn live_until(sim: &Simulation, id: u64) -> u64 {
        sim.substrate
            .entities
            .get(id)
            .unwrap()
            .damage_particle_live_until
    }

    #[test]
    fn s4b_no_draw_above_condition_yellow() {
        // 60/100 = above ConditionYellow (0.5): the outer gate fails → zero draws.
        let rules = cyborg_rules("1.0", "1.0", "SparkA,SparkB", TWO_SPARK_SYSTEMS);
        let mut sim = Simulation::new();
        insert_cyborg_unit(&mut sim, 1, 60, 100);
        let scen = sim.scenario_rng.state();
        let main = sim.main_rng.state();
        techno_common_post(&mut sim, 1, Some(&rules));
        assert_eq!(sim.scenario_rng.state(), scen, "no scenario draw above ConditionYellow");
        assert_eq!(sim.main_rng.state(), main);
        assert_eq!(live_until(&sim, 1), 0);
    }

    #[test]
    fn s4b_one_draw_when_roll_fails() {
        // Below ConditionRed, prob 0.0 → threshold 0 → roll always fails: exactly
        // one draw (the prob-roll), no list-pick, no live system armed.
        let rules = cyborg_rules("0.0", "0.0", "SparkA,SparkB", TWO_SPARK_SYSTEMS);
        let mut sim = Simulation::new();
        insert_cyborg_unit(&mut sim, 1, 20, 100); // below red
        let mut expect = sim.scenario_rng.clone();
        let main = sim.main_rng.state();
        techno_common_post(&mut sim, 1, Some(&rules));
        expect.next_range_u32_inclusive(0, DAMAGE_SPARK_ROLL_MAX);
        assert_eq!(sim.scenario_rng.state(), expect.state(), "exactly one prob-roll draw");
        assert_eq!(sim.main_rng.state(), main, "scenario stream only, never main");
        assert_eq!(live_until(&sim, 1), 0, "roll failed → no live system");
    }

    #[test]
    fn s4b_two_draws_when_roll_succeeds() {
        // prob 1.0 → threshold MAX → roll always succeeds; 2 Spark systems → the
        // list-pick (n(0,1)) consumes a second draw, and the hold arms to tick+5.
        let rules = cyborg_rules("1.0", "1.0", "SparkA,SparkB", TWO_SPARK_SYSTEMS);
        let mut sim = Simulation::new();
        insert_cyborg_unit(&mut sim, 1, 20, 100);
        let tick = sim.session.tick;
        let mut expect = sim.scenario_rng.clone();
        let main = sim.main_rng.state();
        techno_common_post(&mut sim, 1, Some(&rules));
        expect.next_range_u32_inclusive(0, DAMAGE_SPARK_ROLL_MAX); // roll
        expect.next_range_u32_inclusive(0, 1); // list-pick over 2 sparks
        assert_eq!(sim.scenario_rng.state(), expect.state(), "roll + list-pick = two draws");
        assert_eq!(sim.main_rng.state(), main);
        assert_eq!(live_until(&sim, 1), tick + 5, "armed to spawn_tick + Lifetime");
    }

    #[test]
    fn s4b_one_draw_when_single_spark_succeeds() {
        // Single Spark system: on success the list-pick is n(0,0) → consumes NO
        // draw (gamemd RandomRanged min==max), so a successful roll is ONE draw.
        let rules = cyborg_rules("1.0", "1.0", "SparkA,SmokeA", ONE_SPARK_ONE_SMOKE_SYSTEMS);
        let mut sim = Simulation::new();
        insert_cyborg_unit(&mut sim, 1, 20, 100);
        let tick = sim.session.tick;
        let mut expect = sim.scenario_rng.clone();
        techno_common_post(&mut sim, 1, Some(&rules));
        expect.next_range_u32_inclusive(0, DAMAGE_SPARK_ROLL_MAX); // roll only
        assert_eq!(sim.scenario_rng.state(), expect.state(), "single-spark success = one draw");
        assert_eq!(live_until(&sim, 1), tick + 5, "armed despite the no-draw list-pick");
    }

    #[test]
    fn s4b_no_draw_while_system_live() {
        // After a successful spawn (live_until = 5 at tick 0), a same-tick re-entry
        // sees the live system (+0x308 != 0) and makes zero draws; advancing past
        // live_until expires it and rolling resumes.
        let rules = cyborg_rules("1.0", "1.0", "SparkA,SparkB", TWO_SPARK_SYSTEMS);
        let mut sim = Simulation::new();
        insert_cyborg_unit(&mut sim, 1, 20, 100);
        techno_common_post(&mut sim, 1, Some(&rules)); // spawn → live_until = 5
        assert_eq!(live_until(&sim, 1), 5);

        let frozen = sim.scenario_rng.state();
        techno_common_post(&mut sim, 1, Some(&rules)); // still tick 0 < 5 → no draw
        assert_eq!(sim.scenario_rng.state(), frozen, "live system blocks the draw");
        assert_eq!(live_until(&sim, 1), 5, "hold unchanged while live");

        // At tick 5 the system has expired: clears and re-rolls (2 draws, re-armed).
        sim.session.tick = 5;
        let mut expect = sim.scenario_rng.clone();
        techno_common_post(&mut sim, 1, Some(&rules));
        expect.next_range_u32_inclusive(0, DAMAGE_SPARK_ROLL_MAX);
        expect.next_range_u32_inclusive(0, 1);
        assert_eq!(sim.scenario_rng.state(), expect.state(), "expiry resumes rolling");
        assert_eq!(live_until(&sim, 1), 10, "re-armed to 5 + Lifetime");
    }

    #[test]
    fn s4b_zero_draw_without_spark_systems() {
        // Below ConditionRed but DamageParticleSystems has no Spark entry: the
        // inner gate (Spark count > 0) fails → zero draws, even at prob 1.0.
        let rules = cyborg_rules("1.0", "1.0", "SmokeA", SMOKE_ONLY_SYSTEMS);
        let mut sim = Simulation::new();
        insert_cyborg_unit(&mut sim, 1, 20, 100);
        let scen = sim.scenario_rng.state();
        techno_common_post(&mut sim, 1, Some(&rules));
        assert_eq!(sim.scenario_rng.state(), scen, "no Spark system → no draw");
        assert_eq!(live_until(&sim, 1), 0);
    }

    #[test]
    fn s4b_dormant_for_non_cyborg_type() {
        // The slice's faithfulness claim: a non-Cyborg type makes zero draws even
        // below ConditionRed with Spark systems, because emits_damage_spark
        // (Type+0xC8F) is false. Here a VEHICLE with `Cyborg=yes` (nonsensical, but
        // it proves the category gate) — gamemd only honours Cyborg on infantry, so
        // its +0xC8F stays 0 and it never sparks. Build rules inline registering the
        // type under [VehicleTypes].
        use crate::rules::ini_parser::IniFile;
        let text = format!(
            "[General]\n\
BuildSpeed=0.75\nMultipleFactory=0.7\nLowPowerPenaltyModifier=1.25\n\
MinLowPowerProductionSpeed=0.4\nMaxLowPowerProductionSpeed=0.85\n\
ConditionRedSparkingProbability=1.0\nConditionYellowSparkingProbability=1.0\n\n\
[InfantryTypes]\n[VehicleTypes]\n1=VEHCYB\n[AircraftTypes]\n[BuildingTypes]\n\n\
[VEHCYB]\nCyborg=yes\nDamageParticleSystems=SparkA,SparkB\n\n{TWO_SPARK_SYSTEMS}\n"
        );
        let rules = RuleSet::from_ini(&IniFile::from_str(&text)).expect("veh rules parse");
        // Sanity: the type parsed as a Cyborg vehicle that nonetheless does NOT emit.
        let obj = rules.object("VEHCYB").expect("VEHCYB present");
        assert!(obj.cyborg, "Cyborg= parsed");
        assert!(!obj.emits_damage_spark(), "a vehicle never emits AI_Update sparks");

        let mut sim = Simulation::new();
        let owner = sim.interner.intern("Americans");
        let type_ref = sim.interner.intern("VEHCYB");
        let e = GameEntity::new(
            1,
            5,
            5,
            0,
            0,
            owner,
            crate::sim::components::Health { current: 20, max: 100 },
            type_ref,
            EntityCategory::Unit,
            0,
            5,
            true,
        );
        sim.substrate.entities.insert(e);
        let scen = sim.scenario_rng.state();
        techno_common_post(&mut sim, 1, Some(&rules));
        assert_eq!(sim.scenario_rng.state(), scen, "non-Cyborg-infantry type makes zero draws");
        assert_eq!(live_until(&sim, 1), 0);
    }

    #[test]
    fn s4b_permanent_hold_blocks_draw() {
        // A spawned spark whose Lifetime <= 0 holds +0x308 indefinitely: live_until
        // = u64::MAX never expires, so the object never rolls again.
        let rules = cyborg_rules(
            "1.0",
            "1.0",
            "SparkA",
            "[ParticleSystems]\n1=SparkA\n\n[SparkA]\nBehavesLike=Spark\nLifetime=-1\n",
        );
        let mut sim = Simulation::new();
        insert_cyborg_unit(&mut sim, 1, 20, 100);
        techno_common_post(&mut sim, 1, Some(&rules)); // success → permanent hold
        assert_eq!(live_until(&sim, 1), u64::MAX, "Lifetime<=0 → indefinite hold");
        sim.session.tick = 1_000_000;
        let frozen = sim.scenario_rng.state();
        techno_common_post(&mut sim, 1, Some(&rules));
        assert_eq!(sim.scenario_rng.state(), frozen, "permanent hold never re-rolls");
    }
}
