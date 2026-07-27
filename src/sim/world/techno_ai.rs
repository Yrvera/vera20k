//! Per-object AI dispatch host (TechnoClass/FootClass spine).
//!
//! Walks the substrate's live-object order and dispatches each live object
//! through a per-`EntityCategory` shell. Since the Mission authority flip the
//! shell owns the native per-object Mission work: the `+0xC4` AI counter and
//! the owner-local queued-mission promotion (Ready→Commence) that every
//! per-category AI update performs. The mission handler *bodies* remain the
//! legacy per-system state machines (movement, combat, miner, aircraft, …)
//! running in their existing phases — absorbing them into this walk is the
//! remaining per-arm work.
//!
//! Depends on: `world::Simulation` (substrate live order + entity store).
//! Must NOT depend on render/ui/sidebar/audio/net (sim invariant #1).
//! Dispatch is `match category` only — no trait object / dyn / vtable
//! (invariant #2).

use super::Simulation;
use crate::map::entities::EntityCategory;
use crate::map::overlay_types::OverlayTypeRegistry;
use crate::rules::particle_system_type::ParticleSystemBehavesLike;
use crate::rules::ruleset::RuleSet;
use crate::sim::miner::MinerConfig;
#[cfg(any(test, debug_assertions))]
use crate::sim::mission::MissionType;
use crate::sim::pathfinding::PathGrid;

/// Non-rules world context the mission handler bodies dispatched from the
/// host need (grids and per-tick config the spine already owns). Empty in
/// barebones fixtures — handlers that need an absent piece degrade the same
/// way the legacy global phases did with `None` arguments.
#[derive(Default, Clone, Copy)]
pub(crate) struct ObjectAiCtx<'a> {
    pub(crate) path_grid: Option<&'a PathGrid>,
    pub(crate) overlay_registry: Option<&'a OverlayTypeRegistry>,
    pub(crate) miner_config: Option<&'a MinerConfig>,
}

// P3 oracle probe import — used only by the `#[cfg(test)]` factory_oracle_step_trace.
#[cfg(test)]
use crate::sim::production::StepOutcome;

impl Simulation {
    /// Object-AI stage: the authoritative per-object Mission host.
    ///
    /// Walks the live LogicVector order via `for_each_live_object` — the same
    /// re-read contract the native scheduler uses — and runs each live,
    /// present, non-dying object through `techno_ai_shell`: `+0xC4` AI-counter
    /// increment plus the owner-local queued-mission promotion at the verified
    /// per-category AI position (see `Simulation::mission_host_promote`).
    pub(crate) fn object_ai_stage(&mut self, rules: Option<&RuleSet>) {
        self.object_ai_stage_with(rules, ObjectAiCtx::default());
    }

    /// [`Simulation::object_ai_stage`] with the world context the dispatched
    /// mission handler bodies need (the production spine entry).
    pub(crate) fn object_ai_stage_with(&mut self, rules: Option<&RuleSet>, ctx: ObjectAiCtx<'_>) {
        let visited = self.object_ai_walk(cfg!(debug_assertions), rules, ctx);

        #[cfg(debug_assertions)]
        debug_assert_eq!(
            visited
                .iter()
                .copied()
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            visited.len(),
            "object_ai_stage visited one live object more than once",
        );

        #[cfg(not(debug_assertions))]
        let _ = visited;
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
    /// order, through the per-category shell. When `record`, return the
    /// dispatched ids in order (debug/test observation); otherwise the
    /// returned `Vec` is empty and unallocated.
    fn object_ai_walk(
        &mut self,
        record: bool,
        rules: Option<&RuleSet>,
        ctx: ObjectAiCtx<'_>,
    ) -> Vec<u64> {
        let mut visited: Vec<u64> = Vec::new();
        self.for_each_live_object(|sim, id| {
            // Tolerate an absent id (the loop's documented contract). The stage
            // runs AFTER the end-of-tick flush_pending_delete drain, so the order
            // should not reference a freed slot — but inherit the guard.
            if sim.substrate.anims.contains_key(id) {
                if record {
                    visited.push(id);
                }
                if let Some(rules) = rules {
                    sim.visit_anim(id, rules);
                }
                return;
            }
            let Some(entity) = sim.substrate.entities.get(id) else {
                return;
            };
            // A dying object is mid death-teardown and is not dispatched (the
            // closest live `IsActive` analogue today).
            if entity.dying {
                return;
            }
            let category = entity.category;
            if record {
                visited.push(id);
            }
            techno_ai_shell(sim, id, category, rules, ctx);
        });
        visited
    }
}

/// Per-category dispatch shell.
///
/// `match category` — NO trait / dyn / vtable (invariant #2). Every arm runs
/// the common per-object Mission work (`+0xC4` counter + owner-local queued
/// promotion); the Unit arm additionally runs the TechnoClass common bracket
/// (pre/post blocks). Absorbing the remaining per-leaf behavior (movement,
/// turret, combat, fear/sequence, aircraft dispatch) is later per-arm work.
/// The match is exhaustive over the four real variants (no `_` arm), so a
/// future `EntityCategory` addition is a compile error, intentionally.
fn techno_ai_shell(
    sim: &mut Simulation,
    id: u64,
    category: EntityCategory,
    rules: Option<&RuleSet>,
    ctx: ObjectAiCtx<'_>,
) {
    match category {
        EntityCategory::Unit => {
            unit_techno_bracket(sim, id, rules, ctx);
        }
        // InfantryClass::AI promotes queued missions via Ready→Commence
        // (`0x0051BC51`/`0x0051BF03`); the fear/sequence absorption is later work.
        EntityCategory::Infantry => {
            mission_common_step(sim, id, rules);
        }
        EntityCategory::Structure => {
            if let Some(rules) = rules {
                sim.update_building_damage_fire(id, rules);
            }
            // BuildingClass::Update consumes its ready latch via Ready→Commence
            // (`0x0043FE43`/`0x0043FFA3`); with no latch writers live the
            // promotion evaluates to not-ready (recorded residual).
            mission_common_step(sim, id, rules);
        }
        // AircraftClass::AI promotes via Ready→Commence (`0x00415058`).
        EntityCategory::Aircraft => {
            mission_common_step(sim, id, rules);
        }
    }
}

/// The common per-object Mission step every category's AI update performs:
/// the `+0xC4` per-mission AI counter tick and the owner-local queued-mission
/// promotion (Ready→Commence). Promotion needs parsed rules for the Unit
/// world lookups; a rules-less call (barebones fixtures) ticks the counter and
/// leaves the queue for a later rules-bearing pass.
fn mission_common_step(sim: &mut Simulation, id: u64, rules: Option<&RuleSet>) {
    if let Some(entity) = sim.substrate.entities.get_mut(id) {
        entity.mission.increment_ai_counter();
    }
    if let Some(rules) = rules {
        let now = sim.session.binary_frame;
        sim.mission_host_promote(id, now, rules);
    }
}

// ===== TechnoClass common-body bracket =====
//
// Per live Unit, gamemd's `TechnoClass::AI_Update` body is one contiguous
// bracket: pre-mission block -> +0xC4/Mission work -> post-mission block, with
// two IsAlive early-returns (after the pre-block, after dispatch). The mission
// work at the dispatch point is the flip's counter + owner-local promotion;
// the handler-body execution (dispatch-timer gate + per-mission handlers)
// remains with the legacy per-system phases (recorded residual).

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

/// Outcome of the common bracket for one Unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BracketReach {
    /// Died after the pre-block (health 0); no mission work ran.
    DiedInPre,
    /// Live Unit: ran the `+0xC4` counter + owner-local promotion at the
    /// dispatch point.
    Dispatched,
}

/// Per-Unit TechnoClass common bracket. Runs the contiguous
/// `pre -> [IsAlive B] -> +0xC4/promotion -> [IsAlive E] -> post` structure at
/// the gamemd-faithful per-object AI point (pre-movement, LogicVector order).
/// The promotion is UnitClass::AI's Ready→Commence (`0x00736473`, the call
/// before FootClass::AI; the second in-update Ready→Commence at `0x007366FD`
/// is a recorded residual).
fn unit_techno_bracket(
    sim: &mut Simulation,
    id: u64,
    rules: Option<&RuleSet>,
    ctx: ObjectAiCtx<'_>,
) -> BracketReach {
    techno_common_pre(sim, id);
    // Guard B (post-pre IsAlive): a health-0 Unit runs no mission work. No
    // lethal pre-block step exists yet, so this fires only for an already-dead
    // Unit.
    if !sim.substrate.entities.get(id).is_some_and(|e| e.is_alive()) {
        return BracketReach::DiedInPre;
    }
    mission_common_step(sim, id, rules);
    // Mission_Dispatch position: the absorbed handler bodies run here,
    // timer-gated, ending with the verified post-handler epilogue write
    // (start = current frame, delay = handler return). Harvest (the miner
    // FSM) is the first absorbed handler; Move/Guard are Track A2.
    if let (Some(rules), Some(config)) = (rules, ctx.miner_config) {
        crate::sim::miner::dispatch_harvest_for_object(
            sim,
            rules,
            config,
            ctx.path_grid,
            ctx.overlay_registry,
            id,
        );
    }
    // Guard E (post-dispatch IsAlive): the dispatched handler may have
    // destroyed the Unit; a dead Unit runs no post-mission block.
    if !sim.substrate.entities.get(id).is_some_and(|e| e.is_alive()) {
        return BracketReach::Dispatched;
    }
    techno_common_post(sim, id, rules);
    BracketReach::Dispatched
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
    use crate::map::tube_facts::TubeId;
    use crate::rules::ini_parser::IniFile;
    use crate::rules::locomotor_type::LocomotorKind;
    use crate::sim::aircraft::AircraftMission;
    use crate::sim::combat::{AttackTarget, TargetKind};
    use crate::sim::components::{DriveLocomotionRuntime, MovementTarget, NavTargetRef};
    use crate::sim::docking::building_dock::{DockPhase, DockState};
    use crate::sim::game_entity::GameEntity;
    use crate::sim::miner::{Miner, MinerConfig, MinerKind};
    use crate::sim::mission::state::MissionTestFixture;
    use crate::sim::mission::{
        MissionCom, MissionControl, MissionDispatchTimer, MissionId, MissionType,
    };
    use crate::sim::movement::drive_track::begin_forced_turn_track;
    use crate::sim::movement::locomotor::{
        LocomotorState, MovementLayer, OverrideKind, OverrideLocomotor, PiggybackLocomotor,
    };
    use crate::sim::movement::tube_movement::{LowBridgeTubeMovementState, LowBridgeTubePhase};
    use crate::sim::movement::{DriveProcessOutcome, process_drive_locomotion_shell};
    use crate::sim::rng::SimRngLogicalState;
    use crate::sim::snapshot::GameSnapshot;
    use crate::sim::world::SimulationRngState;
    use crate::util::fixed_math::SimFixed;

    /// Build a test entity of a specific category (`test_default` makes a Unit).
    fn entity_of(id: u64, category: EntityCategory) -> GameEntity {
        let mut e = GameEntity::test_default(id, "TEST", "Americans", 5, 5);
        e.category = category;
        e
    }

    fn mission_test_fixture(mission: &MissionCom) -> MissionTestFixture {
        MissionTestFixture {
            current: mission.current(),
            suspended: mission.suspended(),
            queued: mission.queued(),
            movement_bypass_latch: mission.movement_bypass_latch(),
            handler_state: mission.handler_state(),
            mission_start_frame: mission.mission_start_frame(),
            ai_counter: mission.ai_counter(),
            dispatch_timer: mission.dispatch_timer(),
        }
    }

    fn update_mission_test_fixture(
        mission: &mut MissionCom,
        update: impl FnOnce(&mut MissionTestFixture),
    ) {
        let mut fixture = mission_test_fixture(mission);
        update(&mut fixture);
        mission.apply_test_fixture(fixture);
    }

    #[test]
    fn object_ai_stage_ticks_every_live_object_counter() {
        // Post-flip: every live, non-dying object gets its `+0xC4` AI-counter
        // tick at the host; `current` is verb-owned and stays untouched (no
        // command queued anything here).
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

        sim.object_ai_stage(None);

        for id in [1u64, 2, 3, 4] {
            let e = sim.substrate.entities.get(id).unwrap();
            assert_eq!(
                e.mission.ai_counter(),
                1,
                "live object {id} gets exactly one counter tick per stage pass"
            );
            assert_eq!(
                e.mission.current(),
                MissionId::NONE,
                "no verb ran, so object {id}'s current selector stays none"
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

        let visited = sim.object_ai_walk(true, None, ObjectAiCtx::default());
        assert_eq!(
            visited,
            sim.live_object_order_snapshot(),
            "every live object visited exactly once, in live order"
        );
        assert_eq!(
            visited,
            vec![3, 1, 2],
            "live order preserved verbatim (no sort)"
        );
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

        let visited = sim.object_ai_walk(true, None, ObjectAiCtx::default());
        assert_eq!(
            visited,
            vec![1],
            "dying object skipped; the live object is still visited"
        );
        // The internal order-proof assert filters dying members, so the stage
        // must not panic even with a dying member in the live order.
        sim.object_ai_stage(None);
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

        let visited = sim.object_ai_walk(true, None, ObjectAiCtx::default());
        assert_eq!(
            visited,
            vec![live_id],
            "absent id skipped without panic; live id still visited"
        );
        // Stage must not panic on the absent member either.
        sim.object_ai_stage(None);
    }

    // ===== TechnoClass common bracket =====

    #[test]
    fn bracket_ticks_live_unit_counter_without_touching_current() {
        let mut sim = Simulation::new();
        sim.substrate
            .entities
            .insert(entity_of(1, EntityCategory::Unit));
        // A live Unit reaches the dispatch point: +0xC4 counter tick; the
        // verb-owned current selector is untouched.
        assert_eq!(
            unit_techno_bracket(&mut sim, 1, None, ObjectAiCtx::default()),
            BracketReach::Dispatched
        );
        let u = sim.substrate.entities.get(1).unwrap();
        assert_eq!(u.mission.ai_counter(), 1);
        assert_eq!(u.mission.current(), MissionId::NONE);
    }

    #[test]
    fn bracket_pre_guard_short_circuits_dead_unit() {
        let mut sim = Simulation::new();
        let mut e = entity_of(1, EntityCategory::Unit);
        e.health.current = 0; // not alive
        sim.substrate.entities.insert(e);
        // Guard B fires after the (empty) pre-block: a health-0 Unit runs no
        // mission work (counter stays 0).
        assert_eq!(
            unit_techno_bracket(&mut sim, 1, None, ObjectAiCtx::default()),
            BracketReach::DiedInPre
        );
        assert_eq!(
            sim.substrate.entities.get(1).unwrap().mission.ai_counter(),
            0
        );
    }

    #[test]
    fn bracket_ticks_miner_counter_like_any_unit() {
        let mut sim = Simulation::new();
        let mut miner = entity_of(1, EntityCategory::Unit);
        miner.miner = Some(Miner::new(MinerKind::War, &MinerConfig::default(), 0));
        sim.substrate.entities.insert(miner);
        // Post-flip there is no miner deferral: the bracket runs the same
        // common mission step for every live Unit (the miner FSM keeps driving
        // behavior; its mission commits arrive through the departure verbs).
        assert_eq!(
            unit_techno_bracket(&mut sim, 1, None, ObjectAiCtx::default()),
            BracketReach::Dispatched
        );
        assert_eq!(
            sim.substrate.entities.get(1).unwrap().mission.ai_counter(),
            1
        );
    }

    // ===== Slice S4c — passive-acquire eligibility gate (shadow) =====

    #[test]
    fn s4c_gate_move_with_opportunity_fire_and_weapon_eligible() {
        assert!(s4c_passive_acquire_eligible(MissionType::Move, true, true));
    }

    #[test]
    fn s4c_gate_guard_with_weapon_eligible_without_opportunity_fire() {
        // Guard units auto-acquire regardless of OpportunityFire (verified gate).
        assert!(s4c_passive_acquire_eligible(
            MissionType::Guard,
            true,
            false
        ));
    }

    #[test]
    fn s4c_gate_harvest_with_opportunity_fire_eligible() {
        assert!(s4c_passive_acquire_eligible(
            MissionType::Harvest,
            true,
            true
        ));
    }

    #[test]
    fn s4c_gate_move_without_opportunity_fire_not_eligible() {
        assert!(!s4c_passive_acquire_eligible(
            MissionType::Move,
            true,
            false
        ));
    }

    #[test]
    fn s4c_gate_no_weapon_not_eligible_even_on_guard() {
        // The weapon (CanAcquireTarget equip) gate applies to ALL paths, incl Guard.
        assert!(!s4c_passive_acquire_eligible(
            MissionType::Guard,
            false,
            true
        ));
        assert!(!s4c_passive_acquire_eligible(
            MissionType::Move,
            false,
            true
        ));
    }

    #[test]
    fn s4c_gate_off_mission_not_eligible() {
        // Missions outside {Move,Guard,Harvest} never reach the passive-acquire block.
        assert!(!s4c_passive_acquire_eligible(
            MissionType::Attack,
            true,
            true
        ));
        assert!(!s4c_passive_acquire_eligible(
            MissionType::Sleep,
            true,
            true
        ));
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

    /// A moving drive `UnitClass` with no combat, miner, dock, or aircraft
    /// concern (shared fixture for host/verb tests).
    fn scoped_move_unit(id: u64) -> GameEntity {
        let mut e = GameEntity::test_default(id, "TEST", "Americans", 5, 5); // category Unit
        e.movement_target = Some(MovementTarget::default());
        e.drive_locomotion = Some(DriveLocomotionRuntime::default());
        e
    }

    // ===== Checkpoint A — cloned ordinary-Drive host trace =====

    const ORDINARY_DRIVE_HOST_ID: u64 = 41;
    const STOCK_MOVE_RATE_FRAMES: u32 = 14;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum ActiveGate {
        GuardB,
        Dispatch,
        GuardE,
        FootPostTechno,
        FootPostProcess,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum UnitMoveByte {
        Byte6e1,
        Byte6e2,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum HostTraceEvent {
        TechnoPreThroughRocking,
        ActiveGate {
            gate: ActiveGate,
            pass: bool,
        },
        TechnoRemainingPre,
        MissionAiCounter {
            before: u32,
            after: u32,
        },
        MissionDispatchEnter,
        ObjectAiMarker,
        DispatchTimerGate {
            due: bool,
        },
        DispatchHealthGate {
            pass: bool,
        },
        UnitMoveRead6e0 {
            nonzero: bool,
        },
        UnitMoveClear6d2,
        UnitMoveCheckSaved6e0 {
            nonzero: bool,
        },
        UnitMoveCheck {
            byte: UnitMoveByte,
            nonzero: bool,
        },
        QueueMissionMarker {
            mission_id: MissionId,
            arg: u32,
        },
        UnitTrackerCheckMarker,
        UnitTrackerRestartMarker,
        FootMissionMove,
        NavComCheck {
            present: bool,
        },
        IsMovingCall {
            moving: bool,
        },
        NullLocomotorInvariant,
        OnArrivalMarker {
            arg0: u32,
            arg1: u32,
        },
        RateLookup {
            mission: MissionType,
            frames: u32,
        },
        ScenarioRandomRangedApi {
            low: u32,
            high: u32,
            value: u32,
            raw_advances: usize,
        },
        DispatchWriteStart {
            frame: i32,
        },
        DispatchWriteScratchMarker,
        DispatchWriteDelay {
            delay: i32,
        },
        PassiveAcquireMarker,
        BombMarker,
        SlaveManagerMarker,
        CaptureManagerMarker,
        TechnoLatePostMarker,
        FootPreProcessMarker,
        FootProcessGate {
            ordinal: u8,
            pass: bool,
        },
        DriveProcessMarker,
        FootLaterWorkMarker,
        FootReturnMarker,
    }

    #[derive(Debug, Clone, Copy)]
    struct HostTraceGates {
        guard_b_active: bool,
        dispatch_active: bool,
        guard_e_active: bool,
        foot_post_techno_active: bool,
        foot_post_process_active: bool,
        unit_move_bytes: [bool; 3],
        tracker_needs_restart: bool,
        is_moving: bool,
        foot_process_gates: [bool; 5],
        class_special_pre_foot_path: bool,
        lifecycle_countdown_exit: bool,
    }

    impl HostTraceGates {
        fn ordinary() -> Self {
            Self {
                guard_b_active: true,
                dispatch_active: true,
                guard_e_active: true,
                foot_post_techno_active: true,
                foot_post_process_active: true,
                unit_move_bytes: [false; 3],
                tracker_needs_restart: false,
                is_moving: true,
                foot_process_gates: [true; 5],
                class_special_pre_foot_path: false,
                lifecycle_countdown_exit: false,
            }
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct ClonedHostTrace {
        events: Vec<HostTraceEvent>,
        mission_after: MissionCom,
        scenario_rng_after: SimRngLogicalState,
        is_moving_calls: u8,
        move_random_ranged_calls: u8,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum HostTraceError {
        MissingEntity,
        NonUnit,
        NonMoveStoredMission,
        MinerPath,
        DockPath,
        AircraftPath,
        SpecialLocomotorPath,
        ActiveTube,
        ForcedTrack,
        ClassSpecialPath,
        LifecyclePath,
        MissingDriveRuntime,
        StockMoveRate { actual: u32 },
    }

    #[derive(Debug, PartialEq, Eq)]
    struct LiveHostWitness {
        state_hash: u64,
        rng_state: SimulationRngState,
        mission: Option<MissionCom>,
        snapshot: Vec<u8>,
        occupancy_debug: String,
        occupancy_generation: u64,
        occupied_cell_count: usize,
        event_lengths: [usize; 6],
    }

    fn stock_move_control() -> MissionControl {
        MissionControl::from_ini(&IniFile::from_str("[Move]\nRate=.016\n"))
    }

    fn ordinary_drive_host_sim(seed: u64) -> Simulation {
        let mut sim = Simulation::with_seed(seed);
        let mut entity = entity_of(ORDINARY_DRIVE_HOST_ID, EntityCategory::Unit);
        entity.locomotor = Some(LocomotorState::for_test_kind(LocomotorKind::Drive));
        entity.drive_locomotion = Some(DriveLocomotionRuntime::default());
        entity.navigation.nav_com = Some(NavTargetRef::cell(8, 8));
        update_mission_test_fixture(&mut entity.mission, |fixture| {
            fixture.current = MissionId::from_known(MissionType::Move);
            fixture.dispatch_timer = MissionDispatchTimer::at_frame(0);
        });
        sim.substrate.entities.insert(entity);
        sim.set_logic_order_for_test(vec![ORDINARY_DRIVE_HOST_ID]);
        sim
    }

    fn capture_live_host_witness(sim: &Simulation, id: u64) -> LiveHostWitness {
        LiveHostWitness {
            state_hash: sim.state_hash(),
            rng_state: sim.rng_state(),
            mission: sim.substrate.entities.get(id).map(|entity| entity.mission),
            snapshot: GameSnapshot::save(sim, 0, 0, "checkpoint_a_host_trace", 0),
            occupancy_debug: format!("{:?}", sim.occupancy()),
            occupancy_generation: sim.occupancy().generation(),
            occupied_cell_count: sim.occupancy().occupied_cell_count(),
            event_lengths: [
                sim.sound_events.len(),
                sim.fire_events.len(),
                sim.pending_smudge_requests.len(),
                sim.bale_events.len(),
                sim.bunker_wall_events.len(),
                sim.world_effects.len(),
            ],
        }
    }

    fn validate_ordinary_drive_host_entity(
        entity: &GameEntity,
        gates: HostTraceGates,
    ) -> Result<(), HostTraceError> {
        if entity.category == EntityCategory::Aircraft || entity.aircraft_mission.is_some() {
            return Err(HostTraceError::AircraftPath);
        }
        if entity.category != EntityCategory::Unit {
            return Err(HostTraceError::NonUnit);
        }
        if entity.mission.current() != MissionId::from_known(MissionType::Move) {
            return Err(HostTraceError::NonMoveStoredMission);
        }
        if entity.miner.is_some() {
            return Err(HostTraceError::MinerPath);
        }
        if entity.dock_state.is_some() {
            return Err(HostTraceError::DockPath);
        }
        if entity.low_bridge_tube_state.is_some()
            || entity
                .drive_locomotion
                .as_ref()
                .is_some_and(|drive| drive.active_tube.is_some())
        {
            return Err(HostTraceError::ActiveTube);
        }
        if entity.forced_drive_track.is_some() {
            return Err(HostTraceError::ForcedTrack);
        }
        if gates.class_special_pre_foot_path {
            return Err(HostTraceError::ClassSpecialPath);
        }
        if gates.lifecycle_countdown_exit {
            return Err(HostTraceError::LifecyclePath);
        }
        if entity.teleport_state.is_some()
            || entity.tunnel_state.is_some()
            || entity.rocket_state.is_some()
            || entity.homing_state.is_some()
            || entity.droppod_state.is_some()
            || entity.parachute_state.is_some()
        {
            return Err(HostTraceError::SpecialLocomotorPath);
        }

        let locomotor = entity
            .locomotor
            .as_ref()
            .ok_or(HostTraceError::SpecialLocomotorPath)?;
        if locomotor.active_kind() != LocomotorKind::Drive
            || locomotor.primary_kind() != LocomotorKind::Drive
            || locomotor.piggyback.is_some()
            || locomotor.is_overridden()
        {
            return Err(HostTraceError::SpecialLocomotorPath);
        }
        if entity.drive_locomotion.is_none() && entity.navigation.nav_com.is_some() {
            return Err(HostTraceError::MissingDriveRuntime);
        }
        Ok(())
    }

    fn finish_cloned_host_trace(
        events: Vec<HostTraceEvent>,
        entity: &GameEntity,
        scenario_rng: &crate::sim::rng::SimRng,
        is_moving_calls: u8,
        move_random_ranged_calls: u8,
    ) -> ClonedHostTrace {
        ClonedHostTrace {
            events,
            mission_after: entity.mission,
            scenario_rng_after: scenario_rng.logical_state(),
            is_moving_calls,
            move_random_ranged_calls,
        }
    }

    fn draw_cloned_move_jitter(rng: &mut crate::sim::rng::SimRng) -> (u32, usize) {
        let mut probe = rng.clone();
        let value = rng.next_range_u32_inclusive(0, 2);
        let mut raw_advances = 0usize;
        let probe_value = loop {
            raw_advances += 1;
            let candidate = probe.next_u32() & 3;
            if candidate <= 2 {
                break candidate;
            }
        };
        assert_eq!(
            probe_value, value,
            "the raw probe must reproduce the ranged result"
        );
        assert_eq!(
            probe.logical_state(),
            rng.logical_state(),
            "the raw probe must reproduce the complete ranged-call RNG state"
        );
        (value, raw_advances)
    }

    fn trace_cloned_ordinary_drive_host(
        sim: &Simulation,
        id: u64,
        mission_control: &MissionControl,
        native_frame: u32,
        gates: HostTraceGates,
    ) -> Result<ClonedHostTrace, HostTraceError> {
        let source = sim
            .substrate
            .entities
            .get(id)
            .ok_or(HostTraceError::MissingEntity)?;
        validate_ordinary_drive_host_entity(source, gates)?;
        let move_rate = mission_control.rate_frames(MissionType::Move);
        if move_rate != STOCK_MOVE_RATE_FRAMES {
            return Err(HostTraceError::StockMoveRate { actual: move_rate });
        }

        let mut entity = source.clone();
        let mut scenario_rng = sim.clone_scenario_rng();
        let mut events = Vec::new();
        let mut is_moving_calls = 0u8;
        let mut move_random_ranged_calls = 0u8;

        events.push(HostTraceEvent::TechnoPreThroughRocking);
        events.push(HostTraceEvent::ActiveGate {
            gate: ActiveGate::GuardB,
            pass: gates.guard_b_active,
        });
        if !gates.guard_b_active {
            events.push(HostTraceEvent::ActiveGate {
                gate: ActiveGate::FootPostTechno,
                pass: false,
            });
            events.push(HostTraceEvent::FootReturnMarker);
            return Ok(finish_cloned_host_trace(
                events,
                &entity,
                &scenario_rng,
                is_moving_calls,
                move_random_ranged_calls,
            ));
        }

        events.push(HostTraceEvent::TechnoRemainingPre);
        let ai_counter_before = entity.mission.ai_counter();
        update_mission_test_fixture(&mut entity.mission, |fixture| {
            fixture.ai_counter = ai_counter_before.wrapping_add(1);
        });
        events.push(HostTraceEvent::MissionAiCounter {
            before: ai_counter_before,
            after: entity.mission.ai_counter(),
        });
        events.push(HostTraceEvent::MissionDispatchEnter);
        events.push(HostTraceEvent::ObjectAiMarker);
        events.push(HostTraceEvent::ActiveGate {
            gate: ActiveGate::Dispatch,
            pass: gates.dispatch_active,
        });

        let dispatch_inactive = !gates.dispatch_active;
        let mut handler_delay: Option<i32> = None;
        if gates.dispatch_active {
            let due = entity.mission.dispatch_timer().due(native_frame);
            events.push(HostTraceEvent::DispatchTimerGate { due });
            if due {
                let health_pass = entity.health.current > 0;
                events.push(HostTraceEvent::DispatchHealthGate { pass: health_pass });
                if health_pass {
                    let byte6e0 = gates.unit_move_bytes[0];
                    events.push(HostTraceEvent::UnitMoveRead6e0 { nonzero: byte6e0 });
                    events.push(HostTraceEvent::UnitMoveClear6d2);
                    events.push(HostTraceEvent::UnitMoveCheckSaved6e0 { nonzero: byte6e0 });

                    let queue_guard = if byte6e0 {
                        true
                    } else {
                        let byte6e1 = gates.unit_move_bytes[1];
                        events.push(HostTraceEvent::UnitMoveCheck {
                            byte: UnitMoveByte::Byte6e1,
                            nonzero: byte6e1,
                        });
                        if byte6e1 {
                            true
                        } else {
                            let byte6e2 = gates.unit_move_bytes[2];
                            events.push(HostTraceEvent::UnitMoveCheck {
                                byte: UnitMoveByte::Byte6e2,
                                nonzero: byte6e2,
                            });
                            byte6e2
                        }
                    };

                    if queue_guard {
                        events.push(HostTraceEvent::QueueMissionMarker {
                            mission_id: MissionId::from_known(MissionType::Guard),
                            arg: 0,
                        });
                        handler_delay = Some(1);
                    } else {
                        events.push(HostTraceEvent::UnitTrackerCheckMarker);
                        if gates.tracker_needs_restart {
                            events.push(HostTraceEvent::UnitTrackerRestartMarker);
                        }
                        events.push(HostTraceEvent::FootMissionMove);
                        let nav_com_present = entity.navigation.nav_com.is_some();
                        events.push(HostTraceEvent::NavComCheck {
                            present: nav_com_present,
                        });

                        if !nav_com_present && entity.drive_locomotion.is_none() {
                            events.push(HostTraceEvent::NullLocomotorInvariant);
                            return Ok(finish_cloned_host_trace(
                                events,
                                &entity,
                                &scenario_rng,
                                is_moving_calls,
                                move_random_ranged_calls,
                            ));
                        }

                        let stopped_arrival = if nav_com_present {
                            false
                        } else {
                            is_moving_calls = is_moving_calls.wrapping_add(1);
                            events.push(HostTraceEvent::IsMovingCall {
                                moving: gates.is_moving,
                            });
                            !gates.is_moving && entity.mission.queued() == MissionId::NONE
                        };

                        if stopped_arrival {
                            events.push(HostTraceEvent::OnArrivalMarker { arg0: 0, arg1: 1 });
                            handler_delay = Some(1);
                        } else {
                            events.push(HostTraceEvent::RateLookup {
                                mission: MissionType::Move,
                                frames: move_rate,
                            });
                            let (jitter, raw_advances) = draw_cloned_move_jitter(&mut scenario_rng);
                            move_random_ranged_calls = move_random_ranged_calls.wrapping_add(1);
                            events.push(HostTraceEvent::ScenarioRandomRangedApi {
                                low: 0,
                                high: 2,
                                value: jitter,
                                raw_advances,
                            });
                            handler_delay = Some((move_rate + jitter) as i32);
                        }
                    }
                }
            }
        }

        if let Some(delay) = handler_delay {
            events.push(HostTraceEvent::DispatchWriteStart {
                frame: native_frame as i32,
            });
            events.push(HostTraceEvent::DispatchWriteScratchMarker);
            events.push(HostTraceEvent::DispatchWriteDelay { delay });
            update_mission_test_fixture(&mut entity.mission, |fixture| {
                fixture.dispatch_timer = MissionDispatchTimer::from_raw(native_frame as i32, delay);
            });
        }

        events.push(HostTraceEvent::PassiveAcquireMarker);
        events.push(HostTraceEvent::BombMarker);
        events.push(HostTraceEvent::SlaveManagerMarker);
        events.push(HostTraceEvent::CaptureManagerMarker);
        let guard_e_active = !dispatch_inactive && gates.guard_e_active;
        events.push(HostTraceEvent::ActiveGate {
            gate: ActiveGate::GuardE,
            pass: guard_e_active,
        });
        if !guard_e_active {
            events.push(HostTraceEvent::ActiveGate {
                gate: ActiveGate::FootPostTechno,
                pass: false,
            });
            events.push(HostTraceEvent::FootReturnMarker);
            return Ok(finish_cloned_host_trace(
                events,
                &entity,
                &scenario_rng,
                is_moving_calls,
                move_random_ranged_calls,
            ));
        }

        events.push(HostTraceEvent::TechnoLatePostMarker);
        events.push(HostTraceEvent::ActiveGate {
            gate: ActiveGate::FootPostTechno,
            pass: gates.foot_post_techno_active,
        });
        if !gates.foot_post_techno_active {
            events.push(HostTraceEvent::FootReturnMarker);
            return Ok(finish_cloned_host_trace(
                events,
                &entity,
                &scenario_rng,
                is_moving_calls,
                move_random_ranged_calls,
            ));
        }

        events.push(HostTraceEvent::FootPreProcessMarker);
        for (index, pass) in gates.foot_process_gates.into_iter().enumerate() {
            events.push(HostTraceEvent::FootProcessGate {
                ordinal: index as u8 + 1,
                pass,
            });
            if !pass {
                events.push(HostTraceEvent::FootLaterWorkMarker);
                events.push(HostTraceEvent::FootReturnMarker);
                return Ok(finish_cloned_host_trace(
                    events,
                    &entity,
                    &scenario_rng,
                    is_moving_calls,
                    move_random_ranged_calls,
                ));
            }
        }

        if matches!(
            process_drive_locomotion_shell(&entity),
            DriveProcessOutcome::Processed
        ) {
            events.push(HostTraceEvent::DriveProcessMarker);
        } else {
            events.push(HostTraceEvent::NullLocomotorInvariant);
            return Ok(finish_cloned_host_trace(
                events,
                &entity,
                &scenario_rng,
                is_moving_calls,
                move_random_ranged_calls,
            ));
        }

        events.push(HostTraceEvent::ActiveGate {
            gate: ActiveGate::FootPostProcess,
            pass: gates.foot_post_process_active,
        });
        if gates.foot_post_process_active {
            events.push(HostTraceEvent::FootLaterWorkMarker);
        }
        events.push(HostTraceEvent::FootReturnMarker);
        Ok(finish_cloned_host_trace(
            events,
            &entity,
            &scenario_rng,
            is_moving_calls,
            move_random_ranged_calls,
        ))
    }

    fn run_inert_ordinary_drive_host_trace(
        sim: &Simulation,
        id: u64,
        mission_control: &MissionControl,
        native_frame: u32,
        gates: HostTraceGates,
    ) -> Result<ClonedHostTrace, HostTraceError> {
        let before = capture_live_host_witness(sim, id);
        let result =
            trace_cloned_ordinary_drive_host(sim, id, mission_control, native_frame, gates);
        let after = capture_live_host_witness(sim, id);
        assert_eq!(
            before, after,
            "the cloned host trace must leave live state inert"
        );
        result
    }

    #[track_caller]
    fn ordinary_drive_host_trace_ok(
        sim: &Simulation,
        native_frame: u32,
        gates: HostTraceGates,
    ) -> ClonedHostTrace {
        run_inert_ordinary_drive_host_trace(
            sim,
            ORDINARY_DRIVE_HOST_ID,
            &stock_move_control(),
            native_frame,
            gates,
        )
        .expect("ordinary Drive host fixture should trace")
    }

    #[track_caller]
    fn assert_ordinary_drive_host_error(
        sim: &Simulation,
        mission_control: &MissionControl,
        native_frame: u32,
        gates: HostTraceGates,
        expected: HostTraceError,
    ) {
        assert_eq!(
            run_inert_ordinary_drive_host_trace(
                sim,
                ORDINARY_DRIVE_HOST_ID,
                mission_control,
                native_frame,
                gates,
            )
            .expect_err("out-of-scope fixture must be rejected"),
            expected
        );
    }

    #[test]
    fn checkpoint_a_ordinary_drive_host_due_move_full_order_is_inert() {
        let sim = ordinary_drive_host_sim(1);
        let native_frame = 100;
        let trace = ordinary_drive_host_trace_ok(&sim, native_frame, HostTraceGates::ordinary());
        let (jitter, raw_advances) = trace
            .events
            .iter()
            .find_map(|event| match event {
                HostTraceEvent::ScenarioRandomRangedApi {
                    low: 0,
                    high: 2,
                    value,
                    raw_advances,
                } => Some((*value, *raw_advances)),
                _ => None,
            })
            .expect("the due Move path makes one ranged call");
        let delay = (STOCK_MOVE_RATE_FRAMES + jitter) as i32;
        assert_eq!(
            trace.events,
            vec![
                HostTraceEvent::TechnoPreThroughRocking,
                HostTraceEvent::ActiveGate {
                    gate: ActiveGate::GuardB,
                    pass: true,
                },
                HostTraceEvent::TechnoRemainingPre,
                HostTraceEvent::MissionAiCounter {
                    before: 0,
                    after: 1,
                },
                HostTraceEvent::MissionDispatchEnter,
                HostTraceEvent::ObjectAiMarker,
                HostTraceEvent::ActiveGate {
                    gate: ActiveGate::Dispatch,
                    pass: true,
                },
                HostTraceEvent::DispatchTimerGate { due: true },
                HostTraceEvent::DispatchHealthGate { pass: true },
                HostTraceEvent::UnitMoveRead6e0 { nonzero: false },
                HostTraceEvent::UnitMoveClear6d2,
                HostTraceEvent::UnitMoveCheckSaved6e0 { nonzero: false },
                HostTraceEvent::UnitMoveCheck {
                    byte: UnitMoveByte::Byte6e1,
                    nonzero: false,
                },
                HostTraceEvent::UnitMoveCheck {
                    byte: UnitMoveByte::Byte6e2,
                    nonzero: false,
                },
                HostTraceEvent::UnitTrackerCheckMarker,
                HostTraceEvent::FootMissionMove,
                HostTraceEvent::NavComCheck { present: true },
                HostTraceEvent::RateLookup {
                    mission: MissionType::Move,
                    frames: STOCK_MOVE_RATE_FRAMES,
                },
                HostTraceEvent::ScenarioRandomRangedApi {
                    low: 0,
                    high: 2,
                    value: jitter,
                    raw_advances,
                },
                HostTraceEvent::DispatchWriteStart {
                    frame: native_frame as i32,
                },
                HostTraceEvent::DispatchWriteScratchMarker,
                HostTraceEvent::DispatchWriteDelay { delay },
                HostTraceEvent::PassiveAcquireMarker,
                HostTraceEvent::BombMarker,
                HostTraceEvent::SlaveManagerMarker,
                HostTraceEvent::CaptureManagerMarker,
                HostTraceEvent::ActiveGate {
                    gate: ActiveGate::GuardE,
                    pass: true,
                },
                HostTraceEvent::TechnoLatePostMarker,
                HostTraceEvent::ActiveGate {
                    gate: ActiveGate::FootPostTechno,
                    pass: true,
                },
                HostTraceEvent::FootPreProcessMarker,
                HostTraceEvent::FootProcessGate {
                    ordinal: 1,
                    pass: true,
                },
                HostTraceEvent::FootProcessGate {
                    ordinal: 2,
                    pass: true,
                },
                HostTraceEvent::FootProcessGate {
                    ordinal: 3,
                    pass: true,
                },
                HostTraceEvent::FootProcessGate {
                    ordinal: 4,
                    pass: true,
                },
                HostTraceEvent::FootProcessGate {
                    ordinal: 5,
                    pass: true,
                },
                HostTraceEvent::DriveProcessMarker,
                HostTraceEvent::ActiveGate {
                    gate: ActiveGate::FootPostProcess,
                    pass: true,
                },
                HostTraceEvent::FootLaterWorkMarker,
                HostTraceEvent::FootReturnMarker,
            ]
        );
        assert_eq!(trace.is_moving_calls, 0);
        assert_eq!(trace.move_random_ranged_calls, 1);
        assert!((14..=16).contains(&delay));
        assert_eq!(trace.mission_after.ai_counter(), 1);
        assert_eq!(
            trace.mission_after.current().known(),
            Some(MissionType::Move)
        );
        assert_eq!(
            trace.mission_after.dispatch_timer(),
            MissionDispatchTimer::from_raw(native_frame as i32, delay)
        );
    }

    #[test]
    fn checkpoint_a_ordinary_drive_host_timer_not_due_still_marks_process() {
        let mut sim = ordinary_drive_host_sim(2);
        update_mission_test_fixture(
            &mut sim
                .substrate
                .entities
                .get_mut(ORDINARY_DRIVE_HOST_ID)
                .unwrap()
                .mission,
            |fixture| fixture.dispatch_timer = MissionDispatchTimer::from_raw(10, 5),
        );
        let trace = ordinary_drive_host_trace_ok(&sim, 14, HostTraceGates::ordinary());

        assert!(
            trace
                .events
                .contains(&HostTraceEvent::DispatchTimerGate { due: false })
        );
        assert!(!trace.events.iter().any(|event| matches!(
            event,
            HostTraceEvent::DispatchHealthGate { .. }
                | HostTraceEvent::UnitMoveRead6e0 { .. }
                | HostTraceEvent::FootMissionMove
                | HostTraceEvent::ScenarioRandomRangedApi { .. }
                | HostTraceEvent::DispatchWriteStart { .. }
                | HostTraceEvent::DispatchWriteScratchMarker
                | HostTraceEvent::DispatchWriteDelay { .. }
        )));
        assert!(trace.events.contains(&HostTraceEvent::DriveProcessMarker));
        assert_eq!(trace.events.last(), Some(&HostTraceEvent::FootReturnMarker));
        assert_eq!(
            trace.mission_after.dispatch_timer(),
            MissionDispatchTimer::from_raw(10, 5)
        );
    }

    #[test]
    fn checkpoint_a_ordinary_drive_host_due_health_failure_skips_handler_and_write() {
        let mut sim = ordinary_drive_host_sim(3);
        sim.substrate
            .entities
            .get_mut(ORDINARY_DRIVE_HOST_ID)
            .unwrap()
            .health
            .current = 0;
        let trace = ordinary_drive_host_trace_ok(&sim, 20, HostTraceGates::ordinary());

        assert!(
            trace
                .events
                .contains(&HostTraceEvent::DispatchHealthGate { pass: false })
        );
        assert!(!trace.events.iter().any(|event| matches!(
            event,
            HostTraceEvent::UnitMoveRead6e0 { .. }
                | HostTraceEvent::FootMissionMove
                | HostTraceEvent::ScenarioRandomRangedApi { .. }
                | HostTraceEvent::DispatchWriteStart { .. }
                | HostTraceEvent::DispatchWriteDelay { .. }
        )));
        assert!(trace.events.contains(&HostTraceEvent::PassiveAcquireMarker));
        assert!(trace.events.contains(&HostTraceEvent::DriveProcessMarker));
        assert_eq!(
            trace.mission_after.dispatch_timer(),
            MissionDispatchTimer::at_frame(0)
        );
    }

    #[test]
    fn checkpoint_a_ordinary_drive_host_dispatch_inactive_propagates_to_foot_return() {
        let sim = ordinary_drive_host_sim(4);
        let mut gates = HostTraceGates::ordinary();
        gates.dispatch_active = false;
        let trace = ordinary_drive_host_trace_ok(&sim, 30, gates);

        assert!(!trace.events.iter().any(|event| matches!(
            event,
            HostTraceEvent::DispatchTimerGate { .. }
                | HostTraceEvent::DispatchHealthGate { .. }
                | HostTraceEvent::UnitMoveRead6e0 { .. }
                | HostTraceEvent::DriveProcessMarker
        )));
        assert!(trace.events.ends_with(&[
            HostTraceEvent::PassiveAcquireMarker,
            HostTraceEvent::BombMarker,
            HostTraceEvent::SlaveManagerMarker,
            HostTraceEvent::CaptureManagerMarker,
            HostTraceEvent::ActiveGate {
                gate: ActiveGate::GuardE,
                pass: false,
            },
            HostTraceEvent::ActiveGate {
                gate: ActiveGate::FootPostTechno,
                pass: false,
            },
            HostTraceEvent::FootReturnMarker,
        ]));
        assert!(!trace.events.contains(&HostTraceEvent::TechnoLatePostMarker));
    }

    #[test]
    fn checkpoint_a_ordinary_drive_host_each_unit_wrapper_byte_queues_guard() {
        let sim = ordinary_drive_host_sim(5);
        for (bytes, expected_checks) in [
            (
                [true, false, false],
                vec![
                    HostTraceEvent::UnitMoveRead6e0 { nonzero: true },
                    HostTraceEvent::UnitMoveClear6d2,
                    HostTraceEvent::UnitMoveCheckSaved6e0 { nonzero: true },
                ],
            ),
            (
                [false, true, false],
                vec![
                    HostTraceEvent::UnitMoveRead6e0 { nonzero: false },
                    HostTraceEvent::UnitMoveClear6d2,
                    HostTraceEvent::UnitMoveCheckSaved6e0 { nonzero: false },
                    HostTraceEvent::UnitMoveCheck {
                        byte: UnitMoveByte::Byte6e1,
                        nonzero: true,
                    },
                ],
            ),
            (
                [false, false, true],
                vec![
                    HostTraceEvent::UnitMoveRead6e0 { nonzero: false },
                    HostTraceEvent::UnitMoveClear6d2,
                    HostTraceEvent::UnitMoveCheckSaved6e0 { nonzero: false },
                    HostTraceEvent::UnitMoveCheck {
                        byte: UnitMoveByte::Byte6e1,
                        nonzero: false,
                    },
                    HostTraceEvent::UnitMoveCheck {
                        byte: UnitMoveByte::Byte6e2,
                        nonzero: true,
                    },
                ],
            ),
        ] {
            let mut gates = HostTraceGates::ordinary();
            gates.unit_move_bytes = bytes;
            let trace = ordinary_drive_host_trace_ok(&sim, 40, gates);
            let observed_checks: Vec<HostTraceEvent> = trace
                .events
                .iter()
                .filter(|event| {
                    matches!(
                        event,
                        HostTraceEvent::UnitMoveRead6e0 { .. }
                            | HostTraceEvent::UnitMoveClear6d2
                            | HostTraceEvent::UnitMoveCheckSaved6e0 { .. }
                            | HostTraceEvent::UnitMoveCheck { .. }
                    )
                })
                .cloned()
                .collect();
            assert_eq!(observed_checks, expected_checks);
            assert!(trace.events.contains(&HostTraceEvent::QueueMissionMarker {
                mission_id: MissionId::from_known(MissionType::Guard),
                arg: 0,
            }));
            assert!(
                trace
                    .events
                    .contains(&HostTraceEvent::DispatchWriteDelay { delay: 1 })
            );
            assert!(!trace.events.contains(&HostTraceEvent::FootMissionMove));
            assert!(
                !trace
                    .events
                    .iter()
                    .any(|event| matches!(event, HostTraceEvent::ScenarioRandomRangedApi { .. }))
            );
            assert!(trace.events.contains(&HostTraceEvent::DriveProcessMarker));
        }
    }

    #[test]
    fn checkpoint_a_ordinary_drive_host_foot_move_branch_matrix_uses_is_moving() {
        let live_nav = ordinary_drive_host_sim(6);
        let live_trace = ordinary_drive_host_trace_ok(&live_nav, 50, HostTraceGates::ordinary());
        assert_eq!(live_trace.is_moving_calls, 0);
        assert_eq!(live_trace.move_random_ranged_calls, 1);

        let mut null_moving = ordinary_drive_host_sim(6);
        null_moving
            .substrate
            .entities
            .get_mut(ORDINARY_DRIVE_HOST_ID)
            .unwrap()
            .navigation
            .nav_com = None;
        let moving_trace =
            ordinary_drive_host_trace_ok(&null_moving, 50, HostTraceGates::ordinary());
        assert_eq!(moving_trace.is_moving_calls, 1);
        assert_eq!(moving_trace.move_random_ranged_calls, 1);
        assert!(
            !moving_trace
                .events
                .iter()
                .any(|event| matches!(event, HostTraceEvent::OnArrivalMarker { .. }))
        );

        let mut null_stopped_queued = ordinary_drive_host_sim(6);
        {
            let entity = null_stopped_queued
                .substrate
                .entities
                .get_mut(ORDINARY_DRIVE_HOST_ID)
                .unwrap();
            entity.navigation.nav_com = None;
            update_mission_test_fixture(&mut entity.mission, |fixture| {
                fixture.queued = MissionId::from_known(MissionType::Guard);
            });
        }
        let mut stopped = HostTraceGates::ordinary();
        stopped.is_moving = false;
        let queued_trace = ordinary_drive_host_trace_ok(&null_stopped_queued, 50, stopped);
        assert_eq!(queued_trace.is_moving_calls, 1);
        assert_eq!(queued_trace.move_random_ranged_calls, 1);
        assert!(
            !queued_trace
                .events
                .iter()
                .any(|event| matches!(event, HostTraceEvent::OnArrivalMarker { .. }))
        );

        let mut null_stopped = ordinary_drive_host_sim(6);
        null_stopped
            .substrate
            .entities
            .get_mut(ORDINARY_DRIVE_HOST_ID)
            .unwrap()
            .navigation
            .nav_com = None;
        let arrived_trace = ordinary_drive_host_trace_ok(&null_stopped, 50, stopped);
        assert_eq!(arrived_trace.is_moving_calls, 1);
        assert_eq!(arrived_trace.move_random_ranged_calls, 0);
        assert!(
            arrived_trace
                .events
                .contains(&HostTraceEvent::OnArrivalMarker { arg0: 0, arg1: 1 })
        );
        assert!(
            arrived_trace
                .events
                .contains(&HostTraceEvent::DispatchWriteDelay { delay: 1 })
        );
    }

    #[test]
    fn checkpoint_a_ordinary_drive_host_null_locomotor_is_invariant() {
        let mut sim = ordinary_drive_host_sim(7);
        {
            let entity = sim
                .substrate
                .entities
                .get_mut(ORDINARY_DRIVE_HOST_ID)
                .unwrap();
            entity.navigation.nav_com = None;
            entity.drive_locomotion = None;
        }
        let trace = ordinary_drive_host_trace_ok(&sim, 60, HostTraceGates::ordinary());
        assert_eq!(
            trace.events.last(),
            Some(&HostTraceEvent::NullLocomotorInvariant)
        );
        assert!(!trace.events.iter().any(|event| matches!(
            event,
            HostTraceEvent::OnArrivalMarker { .. }
                | HostTraceEvent::DispatchWriteStart { .. }
                | HostTraceEvent::PassiveAcquireMarker
                | HostTraceEvent::DriveProcessMarker
                | HostTraceEvent::FootReturnMarker
        )));
        assert_eq!(trace.move_random_ranged_calls, 0);
        assert_eq!(
            trace.mission_after.dispatch_timer(),
            MissionDispatchTimer::at_frame(0)
        );
    }

    #[test]
    fn checkpoint_a_ordinary_drive_host_guard_exits_truncate_exact_segments() {
        let sim = ordinary_drive_host_sim(8);

        let mut guard_b = HostTraceGates::ordinary();
        guard_b.guard_b_active = false;
        let guard_b_trace = ordinary_drive_host_trace_ok(&sim, 70, guard_b);
        assert_eq!(
            guard_b_trace.events,
            vec![
                HostTraceEvent::TechnoPreThroughRocking,
                HostTraceEvent::ActiveGate {
                    gate: ActiveGate::GuardB,
                    pass: false,
                },
                HostTraceEvent::ActiveGate {
                    gate: ActiveGate::FootPostTechno,
                    pass: false,
                },
                HostTraceEvent::FootReturnMarker,
            ]
        );

        let mut guard_e = HostTraceGates::ordinary();
        guard_e.guard_e_active = false;
        guard_e.foot_post_techno_active = true;
        let guard_e_trace = ordinary_drive_host_trace_ok(&sim, 70, guard_e);
        assert!(guard_e_trace.events.ends_with(&[
            HostTraceEvent::ActiveGate {
                gate: ActiveGate::GuardE,
                pass: false,
            },
            HostTraceEvent::ActiveGate {
                gate: ActiveGate::FootPostTechno,
                pass: false,
            },
            HostTraceEvent::FootReturnMarker,
        ]));
        assert!(
            !guard_e_trace
                .events
                .contains(&HostTraceEvent::TechnoLatePostMarker)
        );
        assert!(
            !guard_e_trace
                .events
                .contains(&HostTraceEvent::FootPreProcessMarker)
        );

        let mut foot_guard = HostTraceGates::ordinary();
        foot_guard.foot_post_techno_active = false;
        let foot_trace = ordinary_drive_host_trace_ok(&sim, 70, foot_guard);
        assert!(foot_trace.events.ends_with(&[
            HostTraceEvent::TechnoLatePostMarker,
            HostTraceEvent::ActiveGate {
                gate: ActiveGate::FootPostTechno,
                pass: false,
            },
            HostTraceEvent::FootReturnMarker,
        ]));
        assert!(
            !foot_trace
                .events
                .contains(&HostTraceEvent::FootPreProcessMarker)
        );
    }

    #[test]
    fn checkpoint_a_ordinary_drive_host_each_foot_process_gate_short_circuits() {
        let sim = ordinary_drive_host_sim(10);
        for failed_index in 0..5 {
            let mut gates = HostTraceGates::ordinary();
            gates.foot_process_gates[failed_index] = false;
            let trace = ordinary_drive_host_trace_ok(&sim, 80, gates);
            let observed: Vec<(u8, bool)> = trace
                .events
                .iter()
                .filter_map(|event| match event {
                    HostTraceEvent::FootProcessGate { ordinal, pass } => Some((*ordinal, *pass)),
                    _ => None,
                })
                .collect();
            let expected: Vec<(u8, bool)> = (0..=failed_index)
                .map(|index| (index as u8 + 1, index != failed_index))
                .collect();
            assert_eq!(observed, expected);
            assert!(!trace.events.contains(&HostTraceEvent::DriveProcessMarker));
            assert!(trace.events.ends_with(&[
                HostTraceEvent::FootLaterWorkMarker,
                HostTraceEvent::FootReturnMarker,
            ]));
        }
    }

    #[test]
    fn checkpoint_a_ordinary_drive_host_post_process_guard_uses_epilogue_exit() {
        let sim = ordinary_drive_host_sim(11);
        let mut gates = HostTraceGates::ordinary();
        gates.foot_post_process_active = false;
        let trace = ordinary_drive_host_trace_ok(&sim, 90, gates);
        assert!(trace.events.ends_with(&[
            HostTraceEvent::DriveProcessMarker,
            HostTraceEvent::ActiveGate {
                gate: ActiveGate::FootPostProcess,
                pass: false,
            },
            HostTraceEvent::FootReturnMarker,
        ]));
        assert!(!trace.events.contains(&HostTraceEvent::FootLaterWorkMarker));
    }

    #[test]
    fn checkpoint_a_ordinary_drive_host_rng_rejection_advances_clone_twice() {
        let sim = ordinary_drive_host_sim(9);
        let mut reference = sim.clone_scenario_rng();
        assert_eq!(reference.next_u32() & 3, 3);
        assert_eq!(reference.next_u32() & 3, 0);

        let trace = ordinary_drive_host_trace_ok(&sim, 100, HostTraceGates::ordinary());
        let ranged_events: Vec<(u32, usize)> = trace
            .events
            .iter()
            .filter_map(|event| match event {
                HostTraceEvent::ScenarioRandomRangedApi {
                    value,
                    raw_advances,
                    ..
                } => Some((*value, *raw_advances)),
                _ => None,
            })
            .collect();
        assert_eq!(ranged_events, vec![(0, 2)]);
        assert_eq!(trace.move_random_ranged_calls, 1);
        assert_eq!(trace.scenario_rng_after, reference.logical_state());
        assert_eq!(
            sim.rng_state().scenario,
            sim.clone_scenario_rng().logical_state()
        );
    }

    #[test]
    fn checkpoint_a_ordinary_drive_host_reads_stored_move_not_derived_projection() {
        let sim = ordinary_drive_host_sim(12);
        let entity = sim.substrate.entities.get(ORDINARY_DRIVE_HOST_ID).unwrap();
        assert_eq!(entity.mission.current().known(), Some(MissionType::Move));
        assert!(entity.movement_target.is_none());
        assert!(entity.attack_target.is_none());
        assert!(entity.dock_state.is_none());
        assert!(entity.miner.is_none());

        let trace = ordinary_drive_host_trace_ok(&sim, 110, HostTraceGates::ordinary());
        assert!(trace.events.contains(&HostTraceEvent::FootMissionMove));
        assert_eq!(
            trace.mission_after.current().known(),
            Some(MissionType::Move)
        );
    }

    #[test]
    fn checkpoint_a_ordinary_drive_host_rejects_out_of_scope_fixtures() {
        let control = stock_move_control();
        let ordinary = HostTraceGates::ordinary();

        let missing = Simulation::with_seed(13);
        assert_ordinary_drive_host_error(
            &missing,
            &control,
            120,
            ordinary,
            HostTraceError::MissingEntity,
        );

        let mut non_unit = ordinary_drive_host_sim(13);
        non_unit
            .substrate
            .entities
            .get_mut(ORDINARY_DRIVE_HOST_ID)
            .unwrap()
            .category = EntityCategory::Infantry;
        assert_ordinary_drive_host_error(
            &non_unit,
            &control,
            120,
            ordinary,
            HostTraceError::NonUnit,
        );

        let mut non_move = ordinary_drive_host_sim(13);
        update_mission_test_fixture(
            &mut non_move
                .substrate
                .entities
                .get_mut(ORDINARY_DRIVE_HOST_ID)
                .unwrap()
                .mission,
            |fixture| fixture.current = MissionId::from_known(MissionType::Guard),
        );
        assert_ordinary_drive_host_error(
            &non_move,
            &control,
            120,
            ordinary,
            HostTraceError::NonMoveStoredMission,
        );

        let mut low_bridge_tube = ordinary_drive_host_sim(13);
        low_bridge_tube
            .substrate
            .entities
            .get_mut(ORDINARY_DRIVE_HOST_ID)
            .unwrap()
            .low_bridge_tube_state = Some(LowBridgeTubeMovementState {
            tube_id: TubeId(0),
            cursor: 0,
            entry: (5, 5),
            exit: (6, 5),
            phase: LowBridgeTubePhase::Traversing,
        });
        assert_ordinary_drive_host_error(
            &low_bridge_tube,
            &control,
            120,
            ordinary,
            HostTraceError::ActiveTube,
        );

        let mut drive_tube = ordinary_drive_host_sim(13);
        drive_tube
            .substrate
            .entities
            .get_mut(ORDINARY_DRIVE_HOST_ID)
            .unwrap()
            .drive_locomotion
            .as_mut()
            .unwrap()
            .active_tube = Some(Default::default());
        assert_ordinary_drive_host_error(
            &drive_tube,
            &control,
            120,
            ordinary,
            HostTraceError::ActiveTube,
        );

        let mut forced_track = ordinary_drive_host_sim(13);
        forced_track
            .substrate
            .entities
            .get_mut(ORDINARY_DRIVE_HOST_ID)
            .unwrap()
            .forced_drive_track = begin_forced_turn_track(0, 0, 0, SimFixed::from_num(1), false);
        assert!(
            forced_track
                .substrate
                .entities
                .get(ORDINARY_DRIVE_HOST_ID)
                .unwrap()
                .forced_drive_track
                .is_some()
        );
        assert_ordinary_drive_host_error(
            &forced_track,
            &control,
            120,
            ordinary,
            HostTraceError::ForcedTrack,
        );

        let mut miner = ordinary_drive_host_sim(13);
        miner
            .substrate
            .entities
            .get_mut(ORDINARY_DRIVE_HOST_ID)
            .unwrap()
            .miner = Some(Miner::new(MinerKind::War, &MinerConfig::default(), 0));
        assert_ordinary_drive_host_error(
            &miner,
            &control,
            120,
            ordinary,
            HostTraceError::MinerPath,
        );

        let mut dock = ordinary_drive_host_sim(13);
        dock.substrate
            .entities
            .get_mut(ORDINARY_DRIVE_HOST_ID)
            .unwrap()
            .dock_state = Some(DockState {
            dock_building_id: 99,
            phase: DockPhase::Approach,
            service_timer: 0,
            no_funds_ticks: 0,
        });
        assert_ordinary_drive_host_error(&dock, &control, 120, ordinary, HostTraceError::DockPath);

        let mut aircraft = ordinary_drive_host_sim(13);
        aircraft
            .substrate
            .entities
            .get_mut(ORDINARY_DRIVE_HOST_ID)
            .unwrap()
            .aircraft_mission = Some(AircraftMission::Guard);
        assert_ordinary_drive_host_error(
            &aircraft,
            &control,
            120,
            ordinary,
            HostTraceError::AircraftPath,
        );

        let mut primary_mismatch = ordinary_drive_host_sim(13);
        primary_mismatch
            .substrate
            .entities
            .get_mut(ORDINARY_DRIVE_HOST_ID)
            .unwrap()
            .locomotor
            .as_mut()
            .unwrap()
            .primary_kind = Some(LocomotorKind::Teleport);
        assert_ordinary_drive_host_error(
            &primary_mismatch,
            &control,
            120,
            ordinary,
            HostTraceError::SpecialLocomotorPath,
        );

        let mut piggyback = ordinary_drive_host_sim(13);
        piggyback
            .substrate
            .entities
            .get_mut(ORDINARY_DRIVE_HOST_ID)
            .unwrap()
            .locomotor
            .as_mut()
            .unwrap()
            .piggyback = Some(PiggybackLocomotor {
            kind: LocomotorKind::Teleport,
            layer: MovementLayer::Ground,
        });
        assert_ordinary_drive_host_error(
            &piggyback,
            &control,
            120,
            ordinary,
            HostTraceError::SpecialLocomotorPath,
        );

        let mut overridden = ordinary_drive_host_sim(13);
        overridden
            .substrate
            .entities
            .get_mut(ORDINARY_DRIVE_HOST_ID)
            .unwrap()
            .locomotor
            .as_mut()
            .unwrap()
            .override_state = Some(OverrideLocomotor {
            saved: Box::new(LocomotorState::for_test_kind(LocomotorKind::Drive)),
            override_kind: OverrideKind::Teleport,
        });
        assert_ordinary_drive_host_error(
            &overridden,
            &control,
            120,
            ordinary,
            HostTraceError::SpecialLocomotorPath,
        );

        let mut class_special = HostTraceGates::ordinary();
        class_special.class_special_pre_foot_path = true;
        assert_ordinary_drive_host_error(
            &ordinary_drive_host_sim(13),
            &control,
            120,
            class_special,
            HostTraceError::ClassSpecialPath,
        );

        let mut lifecycle = HostTraceGates::ordinary();
        lifecycle.lifecycle_countdown_exit = true;
        assert_ordinary_drive_host_error(
            &ordinary_drive_host_sim(13),
            &control,
            120,
            lifecycle,
            HostTraceError::LifecyclePath,
        );

        let mut missing_runtime = ordinary_drive_host_sim(13);
        missing_runtime
            .substrate
            .entities
            .get_mut(ORDINARY_DRIVE_HOST_ID)
            .unwrap()
            .drive_locomotion = None;
        assert_ordinary_drive_host_error(
            &missing_runtime,
            &control,
            120,
            ordinary,
            HostTraceError::MissingDriveRuntime,
        );

        let bad_rate = MissionControl::from_ini(&IniFile::from_str("[Move]\nRate=.017\n"));
        assert_ordinary_drive_host_error(
            &ordinary_drive_host_sim(13),
            &bad_rate,
            120,
            ordinary,
            HostTraceError::StockMoveRate { actual: 15 },
        );
    }

    #[test]
    fn checkpoint_a_ordinary_drive_host_uses_exact_signed_dispatch_timer_domain() {
        for (timer, native_frame, expected_due) in [
            (MissionDispatchTimer::from_raw(-1, 1), 120, true),
            (MissionDispatchTimer::from_raw(-1, 0), i32::MIN as u32, true),
            (MissionDispatchTimer::from_raw(10, 5), 9, false),
            (
                MissionDispatchTimer::from_raw(i32::MIN, 0),
                i32::MIN as u32,
                true,
            ),
            (MissionDispatchTimer::from_raw(0, 0), i32::MIN as u32, false),
            (MissionDispatchTimer::from_raw(0, i32::MIN), 120, true),
            (MissionDispatchTimer::from_raw(-3, 5), 2, true),
        ] {
            let mut sim = ordinary_drive_host_sim(13);
            update_mission_test_fixture(
                &mut sim
                    .substrate
                    .entities
                    .get_mut(ORDINARY_DRIVE_HOST_ID)
                    .unwrap()
                    .mission,
                |fixture| fixture.dispatch_timer = timer,
            );
            let trace =
                ordinary_drive_host_trace_ok(&sim, native_frame, HostTraceGates::ordinary());
            assert!(
                trace
                    .events
                    .contains(&HostTraceEvent::DispatchTimerGate { due: expected_due }),
                "signed timer {timer:?} at frame {native_frame:#010x}"
            );
            assert_eq!(trace.mission_after.ai_counter(), 1);
            if expected_due {
                assert!(trace.events.contains(&HostTraceEvent::FootMissionMove));
                assert!(trace.events.iter().any(|event| matches!(
                    event,
                    HostTraceEvent::DispatchWriteStart { frame }
                        if *frame == native_frame as i32
                )));
                assert_eq!(
                    trace.mission_after.dispatch_timer().start_frame(),
                    native_frame as i32
                );
                assert!((14..=16).contains(&trace.mission_after.dispatch_timer().delay()));
            } else {
                assert!(!trace.events.iter().any(|event| matches!(
                    event,
                    HostTraceEvent::DispatchHealthGate { .. }
                        | HostTraceEvent::FootMissionMove
                        | HostTraceEvent::DispatchWriteStart { .. }
                )));
                assert_eq!(trace.mission_after.dispatch_timer(), timer);
            }
        }
    }

    // ===== Host promotion (Ready→Commence at the per-object AI position) =====

    /// Minimal rules for host-promotion tests: one vehicle type "TEST" plus a
    /// weapons-factory building type "FACT".
    fn promotion_rules() -> RuleSet {
        let text = "[General]\n\
BuildSpeed=0.75\nMultipleFactory=0.7\nLowPowerPenaltyModifier=1.25\n\
MinLowPowerProductionSpeed=0.4\nMaxLowPowerProductionSpeed=0.85\n\n\
[InfantryTypes]\n[VehicleTypes]\n1=TEST\n[AircraftTypes]\n[BuildingTypes]\n1=FACT\n\n\
[TEST]\n\n[FACT]\nWeaponsFactory=yes\n";
        RuleSet::from_ini(&IniFile::from_str(text)).expect("promotion test rules parse")
    }

    /// A unit interned through the SIM's interner (survives verb + promotion
    /// paths that resolve the type name).
    fn insert_interned_unit(sim: &mut Simulation, id: u64, rx: u16, ry: u16) {
        let owner = sim.interner.intern("Americans");
        let type_ref = sim.interner.intern("TEST");
        let e = GameEntity::new_at_frame_zero_for_test(
            id,
            rx,
            ry,
            0,
            0,
            owner,
            crate::sim::components::Health {
                current: 100,
                max: 100,
            },
            type_ref,
            EntityCategory::Unit,
            0,
            5,
            true,
        );
        sim.substrate.entities.insert(e);
    }

    #[test]
    fn host_promotes_queued_mission() {
        // Queue(Move, 0) at command time, then the host's Ready→Commence
        // promotes it: current=Move, queue cleared, Commence reset applied.
        let rules = promotion_rules();
        let mut sim = Simulation::new();
        insert_interned_unit(&mut sim, 1, 5, 5);
        sim.mission_queue_exact(
            1,
            MissionId::from_known(MissionType::Move),
            0,
            0,
            &crate::sim::mission::authority::EntityReadyInputProvider,
        )
        .unwrap();
        let e = sim.substrate.entities.get(1).unwrap();
        assert_eq!(e.mission.current(), MissionId::NONE);
        assert_eq!(e.mission.queued().known(), Some(MissionType::Move));

        sim.mission_host_promote(1, 7, &rules);

        let e = sim.substrate.entities.get(1).unwrap();
        assert_eq!(e.mission.current().known(), Some(MissionType::Move));
        assert_eq!(e.mission.queued(), MissionId::NONE);
        assert_eq!(e.mission.mission_start_frame(), 7);
    }

    #[test]
    fn host_promotion_empty_queue_is_noop() {
        let rules = promotion_rules();
        let mut sim = Simulation::new();
        insert_interned_unit(&mut sim, 1, 5, 5);
        let before = sim.substrate.entities.get(1).unwrap().mission;
        sim.mission_host_promote(1, 7, &rules);
        assert_eq!(sim.substrate.entities.get(1).unwrap().mission, before);
    }

    #[test]
    fn host_promotion_holds_on_weapons_factory_contact() {
        // A Unit whose Radio slot 0 is a weapons-factory Building is NOT ready
        // for a non-Move/Enter queued mission (the exact slot-0 hold).
        let rules = promotion_rules();
        let mut sim = Simulation::new();
        insert_interned_unit(&mut sim, 1, 5, 5);
        let owner = sim.interner.intern("Americans");
        let fact_type = sim.interner.intern("FACT");
        let fact = GameEntity::new_at_frame_zero_for_test(
            2,
            6,
            6,
            0,
            0,
            owner,
            crate::sim::components::Health {
                current: 100,
                max: 100,
            },
            fact_type,
            EntityCategory::Structure,
            0,
            5,
            false,
        );
        sim.substrate.entities.insert(fact);
        sim.substrate
            .entities
            .get_mut(1)
            .unwrap()
            .radio_contacts
            .insert(2);

        sim.mission_queue_exact(
            1,
            MissionId::from_known(MissionType::Guard),
            0,
            0,
            &crate::sim::mission::authority::EntityReadyInputProvider,
        )
        .unwrap();
        sim.mission_host_promote(1, 7, &rules);
        let e = sim.substrate.entities.get(1).unwrap();
        assert_eq!(
            e.mission.current(),
            MissionId::NONE,
            "weapons-factory slot-0 contact holds a queued Guard"
        );
        assert_eq!(e.mission.queued().known(), Some(MissionType::Guard));

        // A queued Move is exempt from the slot-0 hold and promotes.
        sim.mission_queue_exact(
            1,
            MissionId::from_known(MissionType::Move),
            0,
            0,
            &crate::sim::mission::authority::EntityReadyInputProvider,
        )
        .unwrap();
        sim.mission_host_promote(1, 9, &rules);
        let e = sim.substrate.entities.get(1).unwrap();
        assert_eq!(e.mission.current().known(), Some(MissionType::Move));
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

    // ===== In-loop dispatch authority =====

    /// Like `scoped_move_unit`, but interned through the SIM's interner so the
    /// unit survives a real `advance_tick` (test_intern ids don't exist in
    /// `sim.interner`, and tick-path resolves would panic).
    fn insert_s2_scoped_move_unit(sim: &mut Simulation, id: u64, rx: u16, ry: u16) {
        let owner = sim.interner.intern("Americans");
        let type_ref = sim.interner.intern("TEST");
        let mut e = GameEntity::new_at_frame_zero_for_test(
            id,
            rx,
            ry,
            0, // z = ground level
            0, // facing = north
            owner,
            crate::sim::components::Health {
                current: 100,
                max: 100,
            },
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

    /// Post-flip: mission state advances only through the verbs — the tick
    /// never invents a mission from the legacy machines. An uncommanded mover
    /// keeps its selector at none across arrival.
    #[test]
    fn tick_never_projects_a_mission_from_the_machines() {
        let mut sim = Simulation::new();
        insert_s2_scoped_move_unit(&mut sim, 1, 5, 5); // default target: arrives tick 1
        sim.set_logic_order_for_test(vec![1]);
        let heights = std::collections::BTreeMap::new();

        let _ = sim.advance_tick(&[], None, &heights, None, None, 67);
        let e = sim.substrate.entities.get(1).unwrap();
        assert!(e.movement_target.is_none(), "fixture must arrive on tick 1");
        assert_eq!(
            e.mission.current(),
            MissionId::NONE,
            "no verb ran — arrival must not project a mission"
        );

        let _ = sim.advance_tick(&[], None, &heights, None, None, 67);
        let e = sim.substrate.entities.get(1).unwrap();
        assert_eq!(
            e.mission.current(),
            MissionId::NONE,
            "idle ticks must not project a mission either"
        );
        assert_eq!(e.mission.ai_counter(), 2, "the host counter still ticks");
    }

    /// Post-flip corpse freeze: a dying Unit is skipped by the host walk, so
    /// its mission state (counter included) freezes at death — the gamemd
    /// corpse behavior the old per-tick tail projection could not express.
    #[test]
    fn dying_unit_mission_state_freezes() {
        let mut sim = Simulation::new();
        insert_s2_scoped_move_unit(&mut sim, 1, 5, 5);
        sim.set_logic_order_for_test(vec![1]);
        sim.object_ai_stage(None);
        assert_eq!(
            sim.substrate.entities.get(1).unwrap().mission.ai_counter(),
            1
        );
        sim.substrate.entities.get_mut(1).unwrap().dying = true;
        sim.object_ai_stage(None);
        assert_eq!(
            sim.substrate.entities.get(1).unwrap().mission.ai_counter(),
            1,
            "a dying Unit's mission state freezes (no host visit)"
        );
    }

    /// S2: exactly one ai_counter increment per unit-tick — in-loop for a
    /// dispatched mover, tail for an idle (never-collected) unit. Double or
    /// zero count is permanent lockstep drift.
    #[test]
    fn s2_ai_counter_increments_exactly_once() {
        let mut sim = Simulation::new();
        insert_s2_scoped_move_unit(&mut sim, 1, 5, 5); // dispatched on tick 1
        insert_s2_scoped_move_unit(&mut sim, 2, 8, 8);
        // never collected; never scoped
        sim.substrate.entities.get_mut(2).unwrap().movement_target = None;
        sim.set_logic_order_for_test(vec![1, 2]);
        let heights = std::collections::BTreeMap::new();

        let _ = sim.advance_tick(&[], None, &heights, None, None, 67);
        assert_eq!(
            sim.substrate.entities.get(1).unwrap().mission.ai_counter(),
            1
        );
        assert_eq!(
            sim.substrate.entities.get(2).unwrap().mission.ai_counter(),
            1
        );
        let _ = sim.advance_tick(&[], None, &heights, None, None, 67);
        assert_eq!(
            sim.substrate.entities.get(1).unwrap().mission.ai_counter(),
            2
        );
        assert_eq!(
            sim.substrate.entities.get(2).unwrap().mission.ai_counter(),
            2
        );
    }

    /// Load trusts serialized MissionCom: a save carrying a verb-committed
    /// mission that no legacy machine could re-derive (Move current, no
    /// movement_target) must restore an IDENTICAL state hash and value.
    /// Guards the deleted post-load re-derive against reintroduction.
    #[test]
    fn save_load_round_trip_trusts_serialized_mission() {
        use crate::sim::snapshot::GameSnapshot;
        let mut sim = Simulation::new();
        insert_s2_scoped_move_unit(&mut sim, 1, 5, 5);
        sim.substrate.entities.get_mut(1).unwrap().movement_target = None;
        update_mission_test_fixture(
            &mut sim.substrate.entities.get_mut(1).unwrap().mission,
            |fixture| {
                fixture.current = MissionId::from_known(MissionType::Move);
                fixture.ai_counter = 9;
            },
        );
        sim.set_logic_order_for_test(vec![1]);
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
            restored
                .substrate
                .entities
                .get(1)
                .unwrap()
                .mission
                .current()
                .known(),
            Some(MissionType::Move),
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
        let e = GameEntity::new_at_frame_zero_for_test(
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
        assert_eq!(
            sim.scenario_rng.state(),
            scen,
            "no scenario draw above ConditionYellow"
        );
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
        assert_eq!(
            sim.scenario_rng.state(),
            expect.state(),
            "exactly one prob-roll draw"
        );
        assert_eq!(
            sim.main_rng.state(),
            main,
            "scenario stream only, never main"
        );
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
        assert_eq!(
            sim.scenario_rng.state(),
            expect.state(),
            "roll + list-pick = two draws"
        );
        assert_eq!(sim.main_rng.state(), main);
        assert_eq!(
            live_until(&sim, 1),
            tick + 5,
            "armed to spawn_tick + Lifetime"
        );
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
        assert_eq!(
            sim.scenario_rng.state(),
            expect.state(),
            "single-spark success = one draw"
        );
        assert_eq!(
            live_until(&sim, 1),
            tick + 5,
            "armed despite the no-draw list-pick"
        );
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
        assert_eq!(
            sim.scenario_rng.state(),
            frozen,
            "live system blocks the draw"
        );
        assert_eq!(live_until(&sim, 1), 5, "hold unchanged while live");

        // At tick 5 the system has expired: clears and re-rolls (2 draws, re-armed).
        sim.session.tick = 5;
        let mut expect = sim.scenario_rng.clone();
        techno_common_post(&mut sim, 1, Some(&rules));
        expect.next_range_u32_inclusive(0, DAMAGE_SPARK_ROLL_MAX);
        expect.next_range_u32_inclusive(0, 1);
        assert_eq!(
            sim.scenario_rng.state(),
            expect.state(),
            "expiry resumes rolling"
        );
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
        assert!(
            !obj.emits_damage_spark(),
            "a vehicle never emits AI_Update sparks"
        );

        let mut sim = Simulation::new();
        let owner = sim.interner.intern("Americans");
        let type_ref = sim.interner.intern("VEHCYB");
        let e = GameEntity::new_at_frame_zero_for_test(
            1,
            5,
            5,
            0,
            0,
            owner,
            crate::sim::components::Health {
                current: 20,
                max: 100,
            },
            type_ref,
            EntityCategory::Unit,
            0,
            5,
            true,
        );
        sim.substrate.entities.insert(e);
        let scen = sim.scenario_rng.state();
        techno_common_post(&mut sim, 1, Some(&rules));
        assert_eq!(
            sim.scenario_rng.state(),
            scen,
            "non-Cyborg-infantry type makes zero draws"
        );
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
        assert_eq!(
            live_until(&sim, 1),
            u64::MAX,
            "Lifetime<=0 → indefinite hold"
        );
        sim.session.tick = 1_000_000;
        let frozen = sim.scenario_rng.state();
        techno_common_post(&mut sim, 1, Some(&rules));
        assert_eq!(
            sim.scenario_rng.state(),
            frozen,
            "permanent hold never re-rolls"
        );
    }
}
