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

use crate::sim::movement::locomotion::LocomotorSlot;

use super::Simulation;
use crate::map::entities::EntityCategory;
use crate::map::overlay_types::OverlayTypeRegistry;
use crate::rules::particle_system_type::ParticleSystemBehavesLike;
use crate::rules::ruleset::RuleSet;
use crate::sim::miner::MinerConfig;
use crate::sim::mission::authority::EntityReadyInputProvider;
use crate::sim::mission::{MissionId, MissionType};
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
    pub(crate) animation_sequences:
        Option<&'a std::collections::BTreeMap<String, crate::sim::animation::SequenceSet>>,
}

// P3 oracle probe import — used only by the `#[cfg(test)]` factory_oracle_step_trace.
#[cfg(test)]
use crate::sim::production::StepOutcome;

impl Simulation {
    /// Object-AI stage: the authoritative per-object Mission host.
    ///
    /// Walks the live LogicVector order via `for_each_live_object` — the same
    /// re-read contract the native scheduler uses. Every present slot receives
    /// its owner-local visit: anims and dying objects run lifecycle work, while
    /// ordinary Techno objects enter `techno_ai_shell` for the `+0xC4`
    /// AI-counter increment and queued-mission promotion at the verified
    /// per-category AI position (see `Simulation::mission_host_promote`).
    pub(crate) fn object_ai_stage(&mut self, rules: Option<&RuleSet>) {
        self.object_ai_stage_with(rules, ObjectAiCtx::default());
    }

    /// The **second** Ready→Commence checkpoint, run after the movement phases.
    ///
    /// `InfantryClass::AI` and `UnitClass::AI` each gate twice per tick, once on
    /// either side of the object's own locomotion, and both checkpoints are the
    /// same pair of virtuals — the readiness predicate at self-vtable `+0x200`
    /// followed by `Commence` at `+0x1ec` when it returns true. Our object-AI
    /// stage is the first checkpoint (it runs immediately before Phase-1 ground
    /// movement); this is the second.
    ///
    /// Without it a unit that came to rest during its own movement step could
    /// not commence until the next tick, so every mission commencement that
    /// depends on having stopped was one tick late. That compounds through
    /// chained handoffs — harvest dock/unload/exit, guard→attack — at one extra
    /// tick per stage.
    ///
    /// Scope, deliberately: **Unit and Infantry only.** Those are the two
    /// categories verified to gate twice. Aircraft gate exactly once and do it
    /// *after* their locomotion, so their single checkpoint sits at the wrong
    /// end in our tick today — recorded, not fixed here, to keep this change's
    /// hash delta attributable to one mechanism. Buildings gate twice inside
    /// their own update, which is not a movement bracket; unchanged.
    ///
    /// The AI counter is NOT ticked here. Native increments it once per AI pass,
    /// and the first checkpoint already did.
    pub(crate) fn object_ai_post_movement_promote(&mut self, rules: Option<&RuleSet>) {
        let Some(rules) = rules else {
            return;
        };
        let now = self.session.binary_frame;
        self.for_each_live_object(|sim, id| {
            if sim.substrate.anims.contains_key(id) {
                return;
            }
            let Some(entity) = sim.substrate.entities.get(id) else {
                return;
            };
            if entity.dying {
                return;
            }
            if !matches!(
                entity.category,
                EntityCategory::Unit | EntityCategory::Infantry
            ) {
                return;
            }
            sim.mission_host_promote(id, now, rules);
        });
    }

    /// The second Unit/Infantry Ready-to-Commence checkpoint for one object,
    /// called immediately after that object's own locomotion.
    pub(crate) fn object_ai_post_movement_promote_one(&mut self, id: u64, rules: Option<&RuleSet>) {
        let Some(rules) = rules else {
            return;
        };
        let Some(entity) = self.substrate.entities.get(id) else {
            return;
        };
        if entity.dying
            || !matches!(
                entity.category,
                EntityCategory::Unit | EntityCategory::Infantry
            )
        {
            return;
        }
        self.mission_host_promote(id, self.session.binary_frame, rules);
    }

    /// Dispatch one current LogicVector slot. A finishing death sequence calls
    /// UnInit synchronously here, so compacting removal is visible before the
    /// scheduler increments its cursor.
    pub(crate) fn object_ai_visit_one(
        &mut self,
        id: u64,
        rules: Option<&RuleSet>,
        ctx: ObjectAiCtx<'_>,
    ) -> bool {
        if self.substrate.anims.contains_key(id) {
            if let Some(rules) = rules {
                self.visit_anim(id, rules);
            }
            return true;
        }
        if self.substrate.particle_systems.contains_key(id) {
            if let Some(rules) = rules {
                crate::sim::particles::system_ai::tick_particle_system(self, rules, id);
            }
            return true;
        }

        let Some(entity) = self.substrate.entities.get(id) else {
            return false;
        };
        if entity.dying {
            let Some(sequences) = ctx.animation_sequences else {
                return true;
            };
            let type_ref = entity.type_ref;
            let type_name = self.interner.resolve(type_ref);
            let finished = crate::sim::animation::tick_dying_animation(
                self.substrate
                    .entities
                    .get_mut(id)
                    .expect("dying object remained present"),
                sequences,
                &self.session.game_options,
                type_name,
            );
            if finished {
                self.release_move_sound(id);
                self.uninit(id);
            }
            return true;
        }

        // UnitClass::AI / InfantryClass::AI test the object-owned TubeMovement
        // index before entering the ordinary Foot/mission body.  The active
        // leaf runs later in this same LogicVector slot (the world host owns
        // the mutable movement substrates), so this visit must contribute no
        // mission-counter, queued-mission, cadence, or passive-target work.
        if matches!(
            entity.category,
            EntityCategory::Unit | EntityCategory::Infantry
        ) && entity.low_bridge_tube_state.is_some()
        {
            return true;
        }

        let category = entity.category;
        techno_ai_shell(self, id, category, rules, ctx);
        true
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

    /// The walk: visit every live, present object slot once in live order.
    /// Dying objects retain their slot while their death sequence runs but do
    /// not enter the ordinary per-category shell. When `record`, return the
    /// visited ids in order (debug/test observation); otherwise the returned
    /// `Vec` is empty and unallocated.
    fn object_ai_walk(
        &mut self,
        record: bool,
        rules: Option<&RuleSet>,
        ctx: ObjectAiCtx<'_>,
    ) -> Vec<u64> {
        let mut visited: Vec<u64> = Vec::new();
        self.for_each_live_object(|sim, id| {
            if sim.object_ai_visit_one(id, rules, ctx) && record {
                visited.push(id);
            }
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
        // Infantry reach the common Techno AI body through the same foot-leaf
        // call units do, so they run the same off-mission clear and passive
        // block in the same order. Idle infantry are the majority of on-map
        // objects; without this a squad holding a chokepoint does nothing until
        // something shoots it. A passenger inside a transport is gated out by
        // the mission read, and a garrisoned occupant fires through the garrison
        // path instead.
        // The supported Foot mission cadence branches run here as well.
        EntityCategory::Infantry => {
            clear_passive_target_off_mission(sim, id);
            mission_common_step(sim, id, rules);
            if let Some(rules) = rules {
                dispatch_supported_foot_mission_cadence(sim, id, rules);
            }
            passive_acquire_step(sim, id, rules);
        }
        EntityCategory::Structure => {
            if let Some(rules) = rules {
                sim.update_building_damage_fire(id, rules);
            }
            // Buildings run the SAME common Techno AI body units do — it is the
            // only acquisition path a base defence has. Same order: off-mission
            // clear, then the counter/promotion, then the passive block. There
            // is deliberately no Guard→Attack mission flip at the dispatch point
            // between them — see the block comment above
            // `passive_target_scan`'s neighbours for why.
            //
            // The clear is DEAD for structures as things stand, and is kept only
            // so the arm keeps the body's shape: a structure never carries a
            // destination, a navigation goal or a standing order, so its
            // committed mission always reads as finished, the derived Guard
            // reading always wins, and Guard is not one of the twelve missions
            // that strip a scanner target. It starts doing work the moment a
            // structure gains live mission machinery.
            //
            // RESIDUAL, same root cause: a structure being sold holds the
            // Selling mission with nothing running, so it reads Guard and keeps
            // scanning, acquiring and firing for the couple of seconds the sale
            // takes. Same shape as the `building_up` residual noted on
            // `passive_acquire_step`.
            clear_passive_target_off_mission(sim, id);
            // BuildingClass::Update consumes its ready latch via Ready→Commence
            // (`0x0043FE43`/`0x0043FFA3`); with no latch writers live the
            // promotion evaluates to not-ready (recorded residual).
            mission_common_step(sim, id, rules);
            passive_acquire_step(sim, id, rules);
            // BuildingClass::Update consumes the shared C4/PostMortem latch at
            // its late tail. Keep the forced receiver inline in this object's
            // LogicVector visit so nested death effects precede the next slot.
            if let Some(rules) = rules {
                sim.tick_pending_building_detonation(id, rules, ctx.overlay_registry);
            }
        }
        // AircraftClass::AI promotes via Ready→Commence (`0x00415058`).
        //
        // RESIDUAL — no passive block on this arm. Aircraft reach the common
        // Techno AI body in the original through the same foot-leaf call the
        // Unit and Infantry leaves use, so the block is shared with them there;
        // whether it does anything for a YR aircraft in practice is UNCHECKED.
        // It is omitted here because VERA's aircraft mission machine owns firing
        // and return-to-base, and the idle/parked/docked aircraft states all read
        // as Guard — so wiring this in would install targets on helipad-parked
        // aircraft outside the system that decides when they may shoot. Doing it
        // properly means choosing which aircraft states may acquire and routing
        // the pick through that machine, which is its own slice.
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
// the handler-body execution remains with legacy per-system phases except for
// the timer-only Move reschedule below and the absorbed Harvest handler.

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
    // The off-mission passive-target clear runs BEFORE the +0xC4 counter.
    clear_passive_target_off_mission(sim, id);
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
    if let Some(rules) = rules {
        dispatch_supported_foot_mission_cadence(sim, id, rules);
    }
    // Passive / opportunity target acquisition sits between mission dispatch
    // and the second IsAlive guard, before the object's own locomotion.
    passive_acquire_step(sim, id, rules);
    // Guard E (post-dispatch IsAlive): the dispatched handler may have
    // destroyed the Unit; a dead Unit runs no post-mission block.
    if !sim.substrate.entities.get(id).is_some_and(|e| e.is_alive()) {
        return BracketReach::Dispatched;
    }
    techno_common_post(sim, id, rules);
    BracketReach::Dispatched
}

// ===== Passive / opportunity target acquisition =====
//
// This is what makes an idle Grizzly shoot a tank that drives past and a
// Patriot Missile engage on its own. The original runs it inside the common
// Techno AI body, after mission dispatch and before the object's locomotion,
// behind a per-object cadence timer: when the timer expires and the object is
// on Move, Harvest or Guard, the shared target scanner runs, re-arms the timer
// and (with no target already installed) installs one.
//
// The whole block is behind the object's OWN mission and type flags — no
// order, no prior damage, and no player input is involved.

/// Largest value of the scanner's timer jitter draw (`RandomRanged(0, 2)` —
/// three outcomes, inclusive).
const PASSIVE_SCAN_DELAY_JITTER_MAX: u32 = 2;

/// Missions on which a passively-acquired target is dropped, before the AI
/// counter runs. Meaning: the moment an object takes a job that should not be
/// shooting, a target it picked up on its own goes away. The mission *numbers*
/// are verified ({0, 7, 13, 14, 16, 18, 19, 20, 22, 23, 24, 28}); the names
/// below are this project's mission table for those indices.
const PASSIVE_TARGET_CLEAR_MISSIONS: [MissionType; 12] = [
    MissionType::Sleep,
    MissionType::Enter,
    MissionType::Stop,
    MissionType::Ambush,
    MissionType::Unload,
    MissionType::Construction,
    MissionType::Selling,
    MissionType::Repair,
    MissionType::Missile,
    MissionType::Harmless,
    MissionType::Open,
    MissionType::Deliberate,
];

/// The passive-acquire gate predicate (pure; the testable core).
///
/// An object reaches the target scanner iff its mission is one of
/// {Move, Harvest, Guard}, it passes the base can-acquire check, AND
/// (`OpportunityFire` OR mission == Guard). The Guard term is the verified
/// behavior and the one that matters most: an object sitting on Guard acquires
/// even with `OpportunityFire=no`, which is why idle units and base defences
/// engage at all (no stock base defence carries `OpportunityFire`).
///
/// The three-mission admission is the caller's, and it is exhaustive: the
/// original tests the mission slot against 2, then 10, then 5, and skips the
/// whole block otherwise. **Area Guard is not in that set on purpose** — its
/// own handler owns its acquisition (see `evaluate_foot_area_guard`), so
/// widening this predicate would scan such an object twice per cadence and
/// double its scanner RNG draw.
///
/// Deferred, deliberately (recorded, not approximated): the AI-team Move
/// shortcut at the head of the original's gate (needs a live team; there is no
/// AI opponent yet), the two Move sub-branches whose INI keys are unresolved,
/// and the weapon sub-clause on the no-`OpportunityFire` Guard arm (its weapon
/// flag is unresolved).
fn passive_acquire_gate(mission: MissionType, can_acquire: bool, opportunity_fire: bool) -> bool {
    matches!(
        mission,
        MissionType::Move | MissionType::Guard | MissionType::Harvest
    ) && can_acquire
        && (opportunity_fire || mission == MissionType::Guard)
}

/// Re-arm the evidence-backed Foot/Unit handler subset without duplicating the
/// legacy movement, combat, or target-selection systems.
///
/// YR `MissionClass::AI` at `0x005B3060` gates the current handler on the
/// dispatch timer and writes `(current frame, handler return)` afterward. The
/// existing movement/combat phases remain the sole owners of their path and
/// target side effects; this absorbs only proven handler-return cadence. Harvest
/// has its own full handler and epilogue, so miners are excluded to avoid a
/// second write. Target acquisition and approach-result producers are still
/// absent; their native routes remain explicit no-ops rather than guessed AI.
fn dispatch_supported_foot_mission_cadence(sim: &mut Simulation, id: u64, rules: &RuleSet) {
    let now = sim.session.binary_frame;
    let input = {
        let Some(entity) = sim.substrate.entities.get(id) else {
            return;
        };
        if entity.dying {
            return;
        }
        let category = entity.category;
        if !matches!(category, EntityCategory::Unit | EntityCategory::Infantry) {
            return;
        }
        let mission = entity.mission.current().known();
        // A miner's dispatch is owned by the absorbed Harvest handler, which
        // writes its own epilogue — except on Guard, which the Harvest handler
        // now declines. That is the native split: a vehicle on Guard enters the
        // harvester Guard override, which layers the slave/refinery checks and
        // then tail-calls the same FootClass Guard handler every other unit
        // uses; a vehicle on Harvest enters the Harvest handler. Exactly one of
        // the two runs, so the timer keeps a single writer.
        //
        // RESIDUAL, not modelled: the harvester Guard override's player arm
        // re-queues Harvest when a Refinery the house owns sits in one of the
        // eight neighbouring cells, so a retail miner stopped next to its
        // refinery goes back to work on its own. VERA's stays put.
        if entity.miner.is_some() && mission != Some(MissionType::Guard) {
            return;
        }
        let moving = entity.movement_target.is_some()
            || entity.navigation.nav_com.is_some()
            || entity.drive_track.is_some()
            || entity.forced_drive_track.is_some();
        MissionHandlerInput {
            category,
            mission,
            timer_due: entity.mission.dispatch_timer().due(now),
            moving_or_queued: moving || entity.mission.queued() != MissionId::NONE,
            bunker_delegate: entity.bunker_link.installed_in().is_some(),
            has_attack_target: entity.attack_target.is_some(),
            // The destination slot alone, NOT the wider "is this object in
            // motion" test above: the idle-mode selector branches on exactly
            // that one field.
            has_destination: entity.navigation.nav_com.is_some(),
            effective_mission: entity.mission.effective().known(),
            unit_deploy_begin_active: entity
                .mission_leaf
                .as_unit()
                .is_some_and(|leaf| leaf.deploy_begin_active() != 0),
            unit_deploy_reverse_active: entity
                .mission_leaf
                .as_unit()
                .is_some_and(|leaf| leaf.deploy_reverse_active() != 0),
        }
    };
    if !input.timer_due {
        return;
    }

    let evaluation = match (input.category, input.mission) {
        // `FootClass::Mission_Move` is the native named location for this
        // handler-return cadence; movement execution remains in movement/.
        (EntityCategory::Unit | EntityCategory::Infantry, Some(MissionType::Move)) => {
            if input.moving_or_queued {
                MissionHandlerEvaluation::cadence(jittered_mission_cadence(
                    sim,
                    rules,
                    MissionType::Move,
                ))
            } else {
                // The arrival branch. `FootClass::Mission_Move` calls the class
                // arrival hook and returns one frame; the hook is the ONLY
                // thing that takes an object back off Move. Without it a
                // finished move order leaves the unit on Move for the rest of
                // the match, re-dispatched every single frame instead of
                // settling onto Guard's cadence — and never eligible for the
                // Guard-only arm of the passive-acquire gate.
                move_arrival_evaluation(rules, input)
            }
        }
        // `UnitClass::Mission_Attack @ 0x007447A0` is a tail jump to
        // `FootClass::Mission_Attack`; keep both categories on this one path.
        (EntityCategory::Unit | EntityCategory::Infantry, Some(MissionType::Attack)) => {
            let cadence = jittered_mission_cadence(sim, rules, MissionType::Attack);
            let delay = if foot_attack_in_half_cadence_band(sim, id) {
                cadence / 2
            } else {
                cadence
            };
            // The handler's ONLY exit. With no shoot-at target installed it
            // runs the idle-mode selector, which picks a replacement mission
            // and queues it; with one installed it takes the firing step
            // instead, which the combat phase already owns.
            //
            // This is NOT an "is my target still reachable" or "is my target
            // dead" test — the original has neither. A blocker that simply
            // walks away and stays alive never releases its attacker, by any
            // route: the attacker keeps Attack, keeps closing, and only stops
            // when something outside the handler nulls its target. What does
            // null it is the two detach sweeps (target destroyed, target
            // detached alive) and a fresh player order.
            //
            // Without this arm an object parked on Attack never returns to a
            // mission the passive-acquire gate admits, so it stops scanning for
            // targets permanently. Both branches draw the cadence jitter, so
            // this adds no RNG draw; the half-cadence band needs a live target
            // and is already skipped here.
            let idle_queue = (!input.has_attack_target)
                .then(|| foot_enter_idle_mode_queue(rules, input))
                .flatten();
            MissionHandlerEvaluation {
                delay,
                // Stale entity IDs are an authoritative target-loss input, and
                // clearing an invalid handle is not the native selector: the
                // clear lands this dispatch, the idle exit reads the target as
                // it stood at entry and fires on the next one.
                clear_stale_attack_target: input.has_attack_target
                    && attack_target_is_stale(sim, id),
                clear_attack_target: false,
                queue: idle_queue,
            }
        }
        (EntityCategory::Unit, Some(MissionType::Guard)) => {
            // `UnitClass`'s Guard override: the two Unit-local deploy latches
            // queue before the FootClass delegate and return 1.
            if input.unit_deploy_begin_active {
                MissionHandlerEvaluation::queue(1, MissionType::Harvest)
            } else if input.unit_deploy_reverse_active {
                MissionHandlerEvaluation::queue(1, MissionType::Unload)
            } else {
                evaluate_foot_guard_cadence(sim, rules, MissionType::Guard, input.bunker_delegate)
            }
        }
        (EntityCategory::Infantry, Some(MissionType::Guard)) => {
            evaluate_foot_guard_cadence(sim, rules, MissionType::Guard, input.bunker_delegate)
        }
        // Sticky dispatches through the SAME slot as Guard — one handler, two
        // selectors — so it runs the Guard body. The cadence still comes from
        // the object's own mission slot (the timer lookup indexes on the
        // committed mission id, not on the handler's identity), and `[Sticky]
        // Rate=.016` is 14 frames against Guard's 27. Stock skirmish maps park
        // neutral civilian traffic on this.
        (EntityCategory::Unit | EntityCategory::Infantry, Some(MissionType::Sticky)) => {
            evaluate_foot_guard_cadence(sim, rules, MissionType::Sticky, input.bunker_delegate)
        }
        // Area Guard is NOT a Guard alias — it has its own slot and its own
        // handler, and that handler owns its acquisition. The common Techno AI
        // body's passive-acquire block admits missions {Move, Harvest, Guard}
        // and nothing else, so an Area Guard object is deliberately never
        // scanned there; this arm is its single acquisition route.
        (EntityCategory::Unit | EntityCategory::Infantry, Some(MissionType::AreaGuard)) => {
            evaluate_foot_area_guard(sim, id, rules)
        }
        // `FootClass::Mission_Hunt`: the observed Capture/Sabotage/Move routes
        // need an authoritative selector. Until one exists, retain its cadence
        // and do not manufacture a target or queued mission.
        (EntityCategory::Infantry, Some(MissionType::Hunt)) => MissionHandlerEvaluation::cadence(
            jittered_mission_cadence(sim, rules, MissionType::Hunt),
        ),
        // The `UnitClass` Hunt override retries the strict target probe, then
        // queues Enter only if its separate approach virtual returns one.
        // Neither producer exists here, so preserve its exact no-jitter
        // fallback rather than infer arrival from target presence.
        (EntityCategory::Unit, Some(MissionType::Hunt)) => {
            MissionHandlerEvaluation::cadence(mission_cadence(rules, MissionType::Hunt))
        }
        // Everything else: the object still reaches a handler and still re-arms
        // its timer. Where that handler is the un-overridden base one, the
        // return value is a verified constant and no RNG is drawn; where the
        // leaf class overrides the slot with a real handler VERA has not
        // absorbed yet, leave the timer alone rather than install a value the
        // original never writes.
        (category, mission) => match base_mission_handler_delay(category, mission) {
            Some(delay) => MissionHandlerEvaluation::cadence(delay),
            None => return,
        },
    };

    if evaluation.clear_stale_attack_target || evaluation.clear_attack_target {
        if let Some(entity) = sim.substrate.entities.get_mut(id) {
            entity.attack_target = None;
            if evaluation.clear_attack_target {
                entity.passively_acquired_target = false;
            }
        }
    }
    if let Some(queued_mission) = evaluation.queue {
        let _ = sim.mission_queue_exact(
            id,
            MissionId::from_known(queued_mission),
            0,
            now,
            &EntityReadyInputProvider,
        );
    }
    if let Some(entity) = sim.substrate.entities.get_mut(id) {
        entity
            .mission
            .write_dispatch_epilogue(now as i32, evaluation.delay);
    }
}

#[derive(Debug, Clone, Copy)]
struct MissionHandlerInput {
    category: EntityCategory,
    /// The committed selector, or `None` for the idle sentinel. The native
    /// dispatcher's bounds test on the mission id is UNSIGNED, so the sentinel
    /// and every out-of-range id take the switch's default arm rather than
    /// being skipped.
    mission: Option<MissionType>,
    timer_due: bool,
    moving_or_queued: bool,
    bunker_delegate: bool,
    has_attack_target: bool,
    /// The destination slot on its own. The idle-mode selector reads this one
    /// field, not the broader in-motion test [`Self::moving_or_queued`] uses.
    has_destination: bool,
    /// Current when present, otherwise queued — the selector the idle-mode
    /// early returns and the control-entry lookups read.
    effective_mission: Option<MissionType>,
    unit_deploy_begin_active: bool,
    unit_deploy_reverse_active: bool,
}

/// The handler result is evaluated before the one common MissionClass timer
/// write, which prevents branch-local epilogues from double-rearming it.
#[derive(Debug, Clone, Copy)]
struct MissionHandlerEvaluation {
    delay: i32,
    clear_stale_attack_target: bool,
    /// The handler itself dropped the shoot-at target (the arrival hook's
    /// `Assign_Target(NULL)`), as opposed to the stale-handle cleanup above.
    clear_attack_target: bool,
    queue: Option<MissionType>,
}

impl MissionHandlerEvaluation {
    const fn cadence(delay: i32) -> Self {
        Self {
            delay,
            clear_stale_attack_target: false,
            clear_attack_target: false,
            queue: None,
        }
    }

    const fn queue(delay: i32, mission: MissionType) -> Self {
        Self {
            delay,
            clear_stale_attack_target: false,
            clear_attack_target: false,
            queue: Some(mission),
        }
    }
}

/// The Move handler's arrival hook, reduced to the parts VERA can commit.
///
/// `UnitClass`'s override, ordinary-vehicle arm: drop the shoot-at target,
/// clear the destination, then queue **Guard**. `InfantryClass`'s override: a
/// live target queues **Attack** and the target is *kept*; otherwise Guard —
/// and the whole infantry selector is skipped when the *current* mission's
/// control entry carries `Zombie=` or `Paralyzed=` (both absent from `[Move]`,
/// so on this path they never fire; they are read anyway because the gate is on
/// the object's own mission slot and a later caller may arrive on another one).
///
/// The destination clear is a no-op here by construction — this branch is only
/// taken when nothing is moving or queued, which is exactly the state
/// `Set_Destination(NULL, true)` produces.
///
/// Deliberately NOT represented, each recorded rather than guessed:
/// - the vehicle Unload / Harvest arms and the Area-Guard promotion, which key
///   off house-threat, deploy and harvester type fields VERA does not model;
/// - the infantry Guard-vs-Area-Guard choice, same reason — the ordinary arm
///   for a player-controlled infantryman with no veteran self-heal ability is
///   Guard, which is what this returns;
/// - the vehicle suppression byte that skips the queue entirely (its writer is
///   UNKNOWN, so modelling it would be inventing a gate);
/// - the base hook's two EARLY returns, which suppress the whole selector for
///   that arrival: a NavQueue pop, and a locomotor piggyback unwind. Both are
///   inert here — `nav_queue` still has no production writer, and the piggyback
///   unwind runs in the movement phase — but a future NavQueue writer must
///   restore the early return or a waypointed unit will fall to Guard at the
///   first leg instead of continuing to the next one.
fn move_arrival_evaluation(
    rules: &RuleSet,
    input: MissionHandlerInput,
) -> MissionHandlerEvaluation {
    let infantry = input.category == EntityCategory::Infantry;
    if infantry {
        let frozen = rules
            .mission_control
            .entry(MissionType::Move)
            .is_some_and(|entry| entry.zombie || entry.paralyzed);
        if frozen {
            return MissionHandlerEvaluation::cadence(1);
        }
        let next = if input.has_attack_target {
            MissionType::Attack
        } else {
            MissionType::Guard
        };
        return MissionHandlerEvaluation::queue(1, next);
    }
    MissionHandlerEvaluation {
        delay: 1,
        clear_stale_attack_target: false,
        clear_attack_target: true,
        queue: Some(MissionType::Guard),
    }
}

/// The idle-mode selector reached from the Attack handler's no-target exit.
///
/// `Enter_Idle_Mode` is the shared "you have nothing to do; commit the mission
/// that says so" virtual, and both leaf overrides on this path — the Infantry
/// one and the Unit one — begin by running the base arrival hook and then pick
/// a replacement selector. VERA already models the *arrival* entry into it as
/// [`move_arrival_evaluation`]; this is the same virtual entered from the other
/// direction, so only the arms that differ are re-derived here.
///
/// Reached only with no shoot-at target, so the two leaves agree on the whole
/// remaining selection and it collapses to one function:
/// - **a destination is installed** → `Move`. Both leaves take it; the Infantry
///   one substitutes Capture or Sabotage when that is the effective selector,
///   which cannot happen from the Attack handler.
/// - **no destination** → `Guard`, after two early returns that suppress the
///   assignment entirely.
///
/// The Unit leaf additionally nulls its (already null) target and destination
/// on the no-destination arm, and the Infantry leaf's own already-null
/// destination write is likewise inert.
///
/// The early returns, each read from the leaf bodies rather than assumed:
/// - the effective selector is already `Guard` or `Area Guard` — the object is
///   idle, and re-assigning would restart its mission timer for free;
/// - the effective selector's control entry carries `Zombie=` or `Paralyzed=`.
///   `[Attack]` carries neither in stock rules, so this cannot fire from the
///   Attack handler; it is read anyway because the gate is on the object's own
///   selector and the same virtual is entered from other missions.
/// - the *committed* selector is `Patrol` or `Area Guard` (the Unit leaf also
///   excludes `Unload` and `Eaten`) — the tail gate that skips the assign.
///
/// Deliberately NOT represented, recorded rather than guessed:
/// - the head gate both leaves share, an early return on a Foot field whose
///   writer and meaning are UNKNOWN. Modelling it would be inventing a gate;
///   leaving it out can only make the selector run where the original skipped
///   it, and the skip case is unidentified.
/// - the `Area Guard` arm of the no-destination branch. Choosing it over
///   `Guard` turns on a weapon-ability flag and a type flag that are both
///   unresolved; the ordinary arm for a player-controlled object is `Guard`,
///   which is what [`move_arrival_evaluation`] already commits for the same
///   unresolved branch. Keeping the two consistent matters more than guessing.
/// - the AI-only sub-arms, which need a live team and a house-threat field.
fn foot_enter_idle_mode_queue(rules: &RuleSet, input: MissionHandlerInput) -> Option<MissionType> {
    // The tail gate, evaluated on the committed selector.
    let committed_blocks_assign = matches!(
        input.mission,
        Some(MissionType::Patrol) | Some(MissionType::AreaGuard)
    ) || (input.category == EntityCategory::Unit
        && matches!(
            input.mission,
            Some(MissionType::Unload) | Some(MissionType::Eaten)
        ));
    if committed_blocks_assign {
        return None;
    }

    if input.has_destination {
        return Some(MissionType::Move);
    }

    if matches!(
        input.effective_mission,
        Some(MissionType::Guard) | Some(MissionType::AreaGuard)
    ) {
        return None;
    }
    let frozen = input.effective_mission.is_some_and(|mission| {
        rules
            .mission_control
            .entry(mission)
            .is_some_and(|entry| entry.zombie || entry.paralyzed)
    });
    if frozen {
        return None;
    }

    Some(MissionType::Guard)
}

/// Smallest value of the Area Guard cadence jitter draw (`RandomRanged(1, 5)`).
/// Every other absorbed handler draws `(0, 2)`; this one does not.
const AREA_GUARD_CADENCE_JITTER_MIN: u32 = 1;
/// Largest value of the Area Guard cadence jitter draw (`RandomRanged(1, 5)`).
const AREA_GUARD_CADENCE_JITTER_MAX: u32 = 5;

/// `FootClass::Mission_AreaGuard` — "hold this spot and cover it".
///
/// With no target installed and the base can-acquire predicate satisfied, the
/// handler runs the SAME shared target scanner the common AI body runs for
/// Move/Guard/Harvest, but with the Area Guard threat mask (which is what
/// widens the acquisition radius — see `combat::threat_range`). If that
/// installs a target the handler returns one frame. Otherwise the cadence is
/// the object's own `[Area Guard] Rate` plus a `RandomRanged(1, 5)` draw.
///
/// Deliberately NOT represented, recorded:
/// - **the guard post.** The original anchors the scan on a stored guard-post
///   target, defaulting it to the object's own cell the first time the handler
///   runs with none. VERA has no such field, so the scan is always anchored on
///   the object — identical while the object is standing on its post, which is
///   the state Area Guard exists to hold.
/// - **the post leash.** When the object drifts further from its post than its
///   own area-guard range, the original drops the target and sends it home.
///   Nothing in VERA moves an Area Guard object off its post today.
/// - **the aircraft cadence doubling** (the original doubles the rate for
///   aircraft only, and aircraft never reach this arm) and the extra
///   short-range close-band divisor, whose second gate is an unresolved
///   infantry type flag.
/// - one further per-object predicate the original checks between can-acquire
///   and the scan, whose identity is UNKNOWN.
fn evaluate_foot_area_guard(
    sim: &mut Simulation,
    id: u64,
    rules: &RuleSet,
) -> MissionHandlerEvaluation {
    let needs_target = sim
        .substrate
        .entities
        .get(id)
        .is_some_and(|entity| entity.attack_target.is_none());
    if needs_target && can_acquire_target(sim, id, rules) {
        passive_target_scan(sim, id, rules, MissionType::AreaGuard);
        let acquired = sim
            .substrate
            .entities
            .get(id)
            .is_some_and(|entity| entity.attack_target.is_some());
        if acquired {
            // The original returns one frame the moment the scan installs a
            // target, ahead of the cadence tail — so this path draws the
            // scanner's jitter and NOT the `(1, 5)` cadence jitter.
            return MissionHandlerEvaluation::cadence(1);
        }
    }
    let base = mission_cadence(rules, MissionType::AreaGuard);
    let jitter = sim
        .scenario_rng
        .next_range_u32_inclusive(AREA_GUARD_CADENCE_JITTER_MIN, AREA_GUARD_CADENCE_JITTER_MAX)
        as i32;
    MissionHandlerEvaluation::cadence(base.saturating_add(jitter))
}

/// `FootClass::Mission_Guard`, the body Guard(5) and Sticky(6) share.
///
/// `mission` is the object's OWN committed selector, not the handler's: the
/// native timer lookup indexes the control table on the committed mission id,
/// so the same handler re-arms a Guard object at `[Guard] Rate` and a Sticky
/// object at `[Sticky] Rate`.
///
/// Bunker delegation returns the base cadence; all represented local-guard
/// paths take exactly one `RandomRanged(0, 2)`.
fn evaluate_foot_guard_cadence(
    sim: &mut Simulation,
    rules: &RuleSet,
    mission: MissionType,
    bunker_delegate: bool,
) -> MissionHandlerEvaluation {
    if bunker_delegate {
        MissionHandlerEvaluation::cadence(mission_cadence(rules, mission))
    } else {
        MissionHandlerEvaluation::cadence(jittered_mission_cadence(sim, rules, mission))
    }
}

/// The un-overridden `MissionClass` handler's return value, in frames.
///
/// Every base mission stub in the original is the same two instructions —
/// load 450, return — so a slot no leaf class overrides does nothing and comes
/// back in 30 seconds at 15 fps. It reads no INI: `[Sleep] Rate=1` would be 900
/// frames and is dead data for this path.
const BASE_MISSION_HANDLER_FRAMES: i32 = 450;

/// Which committed missions still sit on the un-overridden base handler for a
/// given category, i.e. which ones re-arm with the flat
/// [`BASE_MISSION_HANDLER_FRAMES`] and consume no RNG.
///
/// Read directly out of the `UnitClass` and `InfantryClass` mission-handler
/// vtable blocks (the slots holding the shared 450-frame stubs), so this is a
/// per-category fact, not an inference: `Repair` is a base stub for Infantry
/// and a real override for Units.
///
/// `None` — the idle sentinel — belongs here because the dispatcher's bounds
/// test is unsigned: the sentinel takes the switch default, which calls the
/// same slot `Sleep(0)` does.
///
/// A mission that is NOT in this set has a real leaf handler VERA has not
/// absorbed; returning `None` leaves its timer untouched rather than writing a
/// cadence the original never produces.
fn base_mission_handler_delay(
    category: EntityCategory,
    mission: Option<MissionType>,
) -> Option<i32> {
    let Some(mission) = mission else {
        // The `-1` idle sentinel takes the unsigned default arm.
        return Some(BASE_MISSION_HANDLER_FRAMES);
    };
    let shared_base_stub = matches!(
        mission,
        MissionType::Sleep
            | MissionType::QMove
            | MissionType::Return
            | MissionType::Stop
            | MissionType::Ambush
            | MissionType::Construction
            | MissionType::Selling
            | MissionType::Missile
            | MissionType::Harmless
            | MissionType::Open
            | MissionType::ParadropApproach
            | MissionType::ParadropOverfly
            | MissionType::Deliberate
            | MissionType::AttackMove
            | MissionType::SpyplaneApproach
            | MissionType::SpyplaneOverfly
    );
    // Repair is the one slot the two categories disagree on.
    let category_base_stub = category == EntityCategory::Infantry && mission == MissionType::Repair;
    (shared_base_stub || category_base_stub).then_some(BASE_MISSION_HANDLER_FRAMES)
}

#[inline]
fn mission_cadence(rules: &RuleSet, mission: MissionType) -> i32 {
    rules
        .mission_control
        .rate_frames(mission)
        .min(i32::MAX as u32) as i32
}

#[inline]
fn jittered_mission_cadence(sim: &mut Simulation, rules: &RuleSet, mission: MissionType) -> i32 {
    let base = mission_cadence(rules, mission);
    let jitter = sim.scenario_rng.next_range_u32_inclusive(0, 2) as i32;
    base.saturating_add(jitter)
}

/// The representative Foot Attack cadence halves only inside the observed
/// 282..=768 lepton band. Entity target positions are authoritative here;
/// force-fire cell geometry is intentionally deferred with target routing.
fn foot_attack_in_half_cadence_band(sim: &Simulation, id: u64) -> bool {
    const MIN_LEPTONS: i64 = 282;
    const MAX_LEPTONS: i64 = 768;

    let Some(attacker) = sim.substrate.entities.get(id) else {
        return false;
    };
    let Some(crate::sim::combat::AttackTarget {
        target: crate::sim::combat::TargetKind::Entity(target_id),
        ..
    }) = attacker.attack_target.as_ref()
    else {
        return false;
    };
    let Some(target) = sim.substrate.entities.get(*target_id) else {
        return false;
    };
    let distance_sq = crate::sim::combat::lepton_distance_sq(&attacker.position, &target.position);
    distance_sq >= MIN_LEPTONS * MIN_LEPTONS && distance_sq <= MAX_LEPTONS * MAX_LEPTONS
}

fn attack_target_is_stale(sim: &Simulation, id: u64) -> bool {
    let Some(attacker) = sim.substrate.entities.get(id) else {
        return false;
    };
    let Some(crate::sim::combat::AttackTarget {
        target: crate::sim::combat::TargetKind::Entity(target_id),
        ..
    }) = attacker.attack_target.as_ref()
    else {
        return false;
    };
    !sim.substrate
        .entities
        .get(*target_id)
        .is_some_and(|target| !target.dying && target.is_alive())
}

/// The base can-acquire check every passive path sits behind.
///
/// VERIFIED and modelled: the type-level `CanPassiveAquire` opt-out (the INI
/// key and its default were read out of the binary's key table), and the
/// equipped-weapon requirement.
///
/// SUBSTITUTED, not verified: the building arm. The original tests a building-
/// type flag whose INI key was NOT resolved, combined with a virtual whose slot
/// role is UNCHECKED — the pair reads as "an unpowered defence". VERA uses a
/// DIFFERENT predicate here, `Powered=` plus the owner's low-power state, which
/// is the same predicate that already blocks such a building from firing. The
/// intent matches; the exact condition is UNCHECKED and the two can disagree
/// (for example on a building disabled by something other than low power).
///
/// SUBSTITUTED, not verified: the mind-control term. The original's condition
/// is a capture-manager pointer plus a helper call, neither decoded; VERA reads
/// its own mind-controlled flag.
///
/// Not modelled (recorded): the first disabled/limbo-ish virtual, the second
/// early-out field, and the player-control virtual whose slot role is
/// UNCHECKED. Leaving the last one out makes VERA *more* permissive than the
/// original for some player-controlled objects; inventing a predicate for it
/// would be worse.
fn can_acquire_target(sim: &Simulation, id: u64, rules: &RuleSet) -> bool {
    let Some(entity) = sim.substrate.entities.get(id) else {
        return false;
    };
    let Some(obj) = rules.object(sim.interner.resolve(entity.type_ref)) else {
        return false;
    };
    if !obj.can_passive_acquire {
        return false;
    }
    if entity.category == EntityCategory::Structure
        && !crate::sim::power_system::is_building_powered(
            &sim.power_states,
            rules,
            entity,
            &sim.interner,
        )
    {
        return false;
    }
    if entity.mind_controlled {
        return false;
    }
    obj.primary.is_some() || obj.secondary.is_some()
}

/// The shared passive target scanner — the same routine every Techno class
/// reaches through the same slot, buildings included.
///
/// Entry order, and the RNG contract, both matter for lockstep:
/// 1. stamp the last-scan frame;
/// 2. re-arm the cadence timer to `[General]`'s targeting delay for this
///    mission **plus one `RandomRanged(0, 2)` draw on the scenario RNG**. That
///    draw is UNCONDITIONAL — it happens here, before any target work, whether
///    or not a target is found and whether or not one already exists. It is a
///    *separate, additional* draw from the mission-dispatch epilogue jitter
///    that also runs on the scenario stream; the two must never be folded;
/// 3. drop a stale target the scanner itself installed;
/// 4. with no target installed, install one.
///
/// Steps 3 and 4 are merged below. The original drops the pointer and
/// immediately re-acquires, which lands back on the same value whenever the
/// same candidate still wins. VERA must not perform that round trip literally:
/// the weapon's rearm cooldown lives on the target record here, not on the
/// object, so a no-op drop-and-reinstall would restart ROF on every cadence and
/// a unit whose ROF exceeds the ~28-frame scan interval would never get a shot
/// off. Installing the scan result directly is the same observable outcome —
/// the target setter is a no-op when the pick is unchanged.
///
/// The Area Guard delay branch is written because it belongs to the scanner,
/// but Area Guard is not one of the three missions that reach here from the AI
/// body — it becomes live when the Area Guard mission handler (a separate
/// caller of this scanner) lands.
fn passive_target_scan(sim: &mut Simulation, id: u64, rules: &RuleSet, mission: MissionType) {
    let now = sim.session.binary_frame;
    let base_delay = if mission == MissionType::AreaGuard {
        rules.general.guard_area_targeting_delay
    } else {
        rules.general.normal_targeting_delay
    };
    let jitter = sim
        .scenario_rng
        .next_range_u32_inclusive(0, PASSIVE_SCAN_DELAY_JITTER_MAX);
    if let Some(entity) = sim.substrate.entities.get_mut(id) {
        entity.last_target_scan_frame = now;
        entity
            .passive_scan_timer
            .arm(now, base_delay.saturating_add(jitter));
    }

    // An ordered or retaliation target is never touched here — the original
    // only re-evaluates a target its own scanner installed.
    let Some((has_target, holds_passive_target)) = sim
        .substrate
        .entities
        .get(id)
        .map(|e| (e.attack_target.is_some(), e.passively_acquired_target))
    else {
        return;
    };
    if has_target && !holds_passive_target {
        return;
    }
    // A `DistributedFire` type takes the spread-fire assignment instead of this
    // single-target one. That mechanism is not implemented, and approximating it
    // with a single target would be a different behavior, so those types install
    // nothing here. Stock YR: the Aegis Cruiser only.
    //
    // The divert happens at the ASSIGNMENT, after the drop — so a spread-fire
    // type still lets go of a target it had picked up, it just does not take a
    // new single one. Returning ahead of the drop instead would let it sit on a
    // stale victim forever.
    let spreads_fire = sim
        .substrate
        .entities
        .get(id)
        .and_then(|e| rules.object(sim.interner.resolve(e.type_ref)))
        .is_some_and(|obj| obj.distributed_fire);
    if spreads_fire {
        if holds_passive_target {
            let _ = sim.set_archive_target_represented(id, None);
        }
        if let Some(entity) = sim.substrate.entities.get_mut(id) {
            entity.passively_acquired_target = false;
        }
        return;
    }

    // Re-evaluate. The original gates its drop on three action codes whose
    // meanings are UNCHECKED, so this drops and re-picks on every cadence,
    // which is what it does whenever it drops at all.
    //
    // DRIFT — target choice: this ranks candidates nearest-first (with threat
    // class and stable id as tie-breakers). The original ranks by a per-
    // candidate threat score over an expanding-ring cell walk and keeps only
    // strictly-greater scores, which is not a distance order. Whenever two or
    // more enemies are in range the two engines can pick different targets, and
    // since this is now the authoritative acquisition path that is every
    // engagement with more than one candidate. Replacing the ranking is
    // deliberately out of scope here; approximating the score would be worse
    // than a recorded, honest difference.
    //
    // RESIDUAL — scan-side RNG. This scan draws NOTHING; the whole per-scan
    // cost is the one timer-jitter draw above. The original's candidate
    // evaluation draws `RandomRanged(0, 99)` on the SAME scenario instance, at
    // one callsite, per evaluated candidate — so its per-scan cost is 1 + K, not
    // 1. That draw is short-circuited by a player-control test on the
    // candidate's owning house plus a frame-window comparison, so it is dead for
    // human-controlled houses and VERA has no AI opponent to open it. How often
    // the gate opens in a real match is UNCHECKED. This is an AI-parity blocker
    // to settle before any AI house ships, not a live desync today.
    let pick = crate::sim::combat::acquire_best_target_for_entity(
        &sim.substrate.entities,
        rules,
        &sim.interner,
        id,
        Some(&sim.fog),
        sim.resolved_terrain.as_ref(),
    );
    // Install the target only — no mission, no destination, and nothing fires
    // this tick. A unit that acquires while driving keeps driving, and an idle
    // unit that acquires does NOT walk toward what it found.
    //
    // Swinging an existing attack onto a different victim goes through the
    // shared in-place retarget so the weapon's rearm countdown, burst counter
    // and inter-shot delay survive. Rebuilding the attack record instead would
    // zero all three and hand out a free shot on every re-pick — and with a
    // ~28-frame cadence against stock ROF values that mostly exceed it, a Guard
    // unit would fire at roughly double its stock rate whenever two enemies
    // traded places as nearest.
    let pick_kind = pick.map(crate::sim::combat::TargetKind::Entity);
    let current_kind = sim
        .substrate
        .entities
        .get(id)
        .and_then(|e| e.attack_target.as_ref().map(|t| t.target));
    match (current_kind, pick) {
        (Some(current), Some(sid)) if current != crate::sim::combat::TargetKind::Entity(sid) => {
            if let Some(entity) = sim.substrate.entities.get_mut(id) {
                crate::sim::combat::retarget_preserving_rearm(entity, sid);
            }
        }
        // Fresh install, or a clear: no rearm state exists to carry over.
        _ => {
            let _ = sim.set_archive_target_represented(id, pick_kind);
        }
    }
    // DRIFT — passive-flag set condition, and VERA-INTERNAL by decision.
    //
    // Original: the target assignment clears the flag on every call (modelled in
    // the shared target setter), and the scanner's CALLER re-sets it only when
    // the scan actually CHANGED the target. An object whose rescan re-picks the
    // same victim therefore ends with the flag false, drops out of the drop step
    // from then on, and settles on that victim.
    //
    // VERA: the flag is set whenever a target is installed, so the object keeps
    // re-evaluating every cadence.
    //
    // Trigger and player effect: any object that holds a scanner target for more
    // than one cadence — so every idle unit, infantryman and defence in a
    // standing engagement. The original settles on its first re-picked victim;
    // VERA re-picks nearest-first every ~28 frames and can swing onto a closer
    // enemy mid-reload. Frequency: continuous while anything is idle near a
    // contact, which in ordinary play is most of a match.
    //
    // Why it is kept: this one step stands in for BOTH the scanner and the
    // mission-handler re-evaluation the original splits into its Guard and
    // Attack building/foot missions, and VERA has neither handler. Adopting the
    // native set condition reintroduces a permanent target latch (a defence
    // holds a scout that walked out of range for the rest of the match) and
    // un-gates pursuit, since the flag is what keeps a scanner target from being
    // chased. Both are covered by tests that go red when it is adopted.
    // Downstream risk: retiring this DRIFT means adding the real mission
    // handlers first, not flipping this line. The drop step's own action-code
    // gate is UNCHECKED either way.
    if let Some(entity) = sim.substrate.entities.get_mut(id) {
        entity.passively_acquired_target = pick.is_some();
    }
}

/// The passive-acquire block, at its position in the common Techno AI body:
/// after mission dispatch, before the object's own movement.
///
/// Order is the original's: cadence-timer expiry, then the mission test, then
/// the gate, then the scanner. Nothing before the scanner draws RNG.
///
/// Not modelled: the divert gate the original checks between the timer and the
/// mission test — for a foot unit it means "a pending player attack-order
/// handoff skips passive acquire this tick", and buildings never divert (their
/// slot is a constant false).
///
/// Recorded residuals, all VERA-side and none of them acted on here:
/// - A building still in its `building_up` deployment animation reaches the
///   scanner and burns its jitter draw. It cannot fire (the fire gate blocks it)
///   and the window is a few seconds once per building, but the draw is real.
/// - Holding a scanner target suppresses retaliation, because the retaliation
///   pass skips anything that already has a target. So an idle unit that has
///   picked something up will not switch to whatever shoots it in the back. The
///   original's equivalent is UNCHECKED and worth a dedicated pass.
/// - This scan runs in the object-AI pass, ahead of the order-intent
///   acquisition stage later in the tick, so it wins for any object that would
///   have been served by both. Consequence: a unit put on guard stance no longer
///   walks out to close on a target it cannot reach — the scanner installs an
///   in-range target first, and pursuit skips scanner targets by design.
/// - A deployed Desolator that picks something up on its own suppresses its own
///   radiation self-target re-arm, because that path only fires for a structure
///   or unit with no target installed.
fn passive_acquire_step(sim: &mut Simulation, id: u64, rules: Option<&RuleSet>) {
    let Some(rules) = rules else {
        return;
    };
    let Some(entity) = sim.substrate.entities.get(id) else {
        return;
    };
    if entity.dying {
        return;
    }
    if !entity.passive_scan_timer.due(sim.session.binary_frame) {
        return;
    }
    let mission = entity.passive_acquire_mission();
    if !matches!(
        mission,
        MissionType::Move | MissionType::Guard | MissionType::Harvest
    ) {
        return;
    }
    let opportunity_fire = rules
        .object(sim.interner.resolve(entity.type_ref))
        .is_some_and(|obj| obj.opportunity_fire);
    if !passive_acquire_gate(
        mission,
        can_acquire_target(sim, id, rules),
        opportunity_fire,
    ) {
        return;
    }
    passive_target_scan(sim, id, rules, mission);
}

/// The off-mission clear, which runs before the AI counter: a passively
/// acquired target is dropped the moment the object takes a job that should not
/// be shooting (see [`PASSIVE_TARGET_CLEAR_MISSIONS`]).
fn clear_passive_target_off_mission(sim: &mut Simulation, id: u64) {
    let drop = sim.substrate.entities.get(id).is_some_and(|entity| {
        entity.attack_target.is_some()
            && entity.passively_acquired_target
            && PASSIVE_TARGET_CLEAR_MISSIONS.contains(&entity.passive_acquire_mission())
    });
    if !drop {
        return;
    }
    let _ = sim.set_archive_target_represented(id, None);
    if let Some(entity) = sim.substrate.entities.get_mut(id) {
        entity.passively_acquired_target = false;
    }
}

// ===== Why there is no building Guard->Attack mission flip here =====
//
// In the original, a Guard-mission building that holds a target commits
// Mission_Attack, and that mission is NOT a latch: it re-derives an action from
// the live target on every dispatch through an action jumptable, and when the
// target pointer goes null it clears the target, re-assigns Guard and commences
// — read out of the original's building Attack-mission handler this session,
// whose null-target arm is exactly assign-target-null, assign-mission-Guard,
// commence. The flip is safe there because the Attack mission owns the
// re-evaluation.
//
// VERA has no building Mission_Attack handler, and firing here does not read
// the mission at all: the fire gate and the attacker snapshot never look at it,
// so a structure holding a target fires whatever mission it is on. So writing
// Attack would buy exactly zero firing while costing the rescan — the passive
// gate only admits {Move, Harvest, Guard}, and nothing in VERA would ever move
// the building back off Attack, because combat deliberately does not clear a
// target that has merely gone out of range and buildings are excluded from
// pursuit. A Tesla Coil that acquired a scout at 6 cells would stay locked on it
// after it backed off to 8 and stayed in vision — silent for the rest of the
// match, for near-certain in the first minutes of any game.
//
// VERA-INTERNAL: the flip is deliberately omitted, so a building stays on the
// bridged Guard reading and the scanner's own cadence owns target selection —
// re-picking the best in-range candidate every ~28 frames and clearing the
// target when nothing is in range. That reproduces the observable result of the
// original's re-evaluating Attack mission more closely than the latch would.
// Restoring the flip requires a real Mission_Attack handler first.

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
    use crate::sim::components::{
        DriveCoord, DriveLocomotionRuntime, MovementTarget, NavTargetRef,
    };
    use crate::sim::docking::building_dock::{DockPhase, DockState};
    use crate::sim::game_entity::{BunkerLink, GameEntity};
    use crate::sim::miner::{Miner, MinerConfig, MinerKind};
    use crate::sim::mission::leaf::MissionLeafState;
    use crate::sim::mission::state::MissionTestFixture;
    use crate::sim::mission::{
        MissionCom, MissionControl, MissionDispatchTimer, MissionId, MissionType,
    };
    use crate::sim::movement::drive_track::begin_forced_turn_track;
    use crate::sim::movement::locomotor::{LocomotorState, MovementLayer};
    use crate::sim::movement::tube_movement::LowBridgeTubeMovementState;
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

    fn register_entity(sim: &mut Simulation, mut entity: GameEntity) {
        entity.owner = sim.interner.intern("Americans");
        entity.type_ref = sim.interner.intern("TEST");
        sim.substrate.entities.insert(entity);
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
    fn object_ai_stage_visits_dying_object_in_its_live_slot() {
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
            vec![1, 2],
            "a dying object keeps receiving its live scheduler slot until UnInit"
        );
        // With no sequence table this fixture cannot finish the death action,
        // so the second visit must leave the object registered for a later pass.
        sim.object_ai_stage(None);
        assert_eq!(sim.live_object_order_snapshot(), vec![1, 2]);
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

    // ===== Passive-acquire gate predicate =====

    #[test]
    fn passive_gate_move_with_opportunity_fire_and_weapon_eligible() {
        assert!(passive_acquire_gate(MissionType::Move, true, true));
    }

    #[test]
    fn passive_gate_guard_with_weapon_eligible_without_opportunity_fire() {
        // Guard objects auto-acquire regardless of OpportunityFire — the arm
        // every stock base defence and every idle vehicle relies on.
        assert!(passive_acquire_gate(MissionType::Guard, true, false));
    }

    #[test]
    fn passive_gate_harvest_with_opportunity_fire_eligible() {
        assert!(passive_acquire_gate(MissionType::Harvest, true, true));
    }

    #[test]
    fn passive_gate_move_without_opportunity_fire_not_eligible() {
        assert!(!passive_acquire_gate(MissionType::Move, true, false));
    }

    #[test]
    fn passive_gate_cannot_acquire_not_eligible_even_on_guard() {
        // The base can-acquire check applies to ALL paths, including Guard.
        assert!(!passive_acquire_gate(MissionType::Guard, false, true));
        assert!(!passive_acquire_gate(MissionType::Move, false, true));
    }

    #[test]
    fn passive_gate_off_mission_not_eligible() {
        // Missions outside {Move,Guard,Harvest} never reach the passive block.
        assert!(!passive_acquire_gate(MissionType::Attack, true, true));
        assert!(!passive_acquire_gate(MissionType::Sleep, true, true));
    }

    // ===== Passive acquisition — production line =====

    /// Rules for the passive-acquire tests. `MTNK` is an ordinary armed tank
    /// (no `OpportunityFire` — it must still acquire through the Guard arm),
    /// `NOACQ` is the same tank with the type-level opt-out, and `NASAM` is an
    /// armed `Powered=yes` defence that drains power.
    fn passive_rules() -> RuleSet {
        RuleSet::from_ini(&IniFile::from_str(
            "[General]\nNormalTargetingDelay=27\nGuardAreaTargetingDelay=36\n\n\
             [InfantryTypes]\n0=GI\n[AircraftTypes]\n\
             [VehicleTypes]\n0=MTNK\n1=NOACQ\n2=UNARM\n\
             [BuildingTypes]\n0=NASAM\n1=GAPOWR\n\n\
             [GAPOWR]\nStrength=750\nArmor=wood\nFoundation=2x2\nSight=5\nPower=100\n\n\
             [GI]\nLocomotor={4A582744-9839-11d1-B709-00A024DDAFD1}\n\
             Strength=125\nArmor=none\nSpeed=4\nSight=10\nPrimary=105mm\n\n\
             [MTNK]\nLocomotor={4A582741-9839-11d1-B709-00A024DDAFD1}\n\
             Strength=300\nArmor=heavy\nSpeed=6\nSight=10\nPrimary=105mm\n\n\
             [NOACQ]\nLocomotor={4A582741-9839-11d1-B709-00A024DDAFD1}\n\
             Strength=300\nArmor=heavy\nSpeed=6\nSight=10\nPrimary=105mm\nCanPassiveAquire=no\n\n\
             [UNARM]\nLocomotor={4A582741-9839-11d1-B709-00A024DDAFD1}\n\
             Strength=300\nArmor=heavy\nSpeed=6\nSight=10\n\n\
             [NASAM]\nStrength=1000\nArmor=wood\nFoundation=1x1\nSight=10\n\
             Primary=105mm\nPowered=yes\nPower=-50\n\n\
             [105mm]\nDamage=65\nROF=50\nRange=6\nWarhead=AP\n\n\
             [AP]\nVerses=100%,100%,90%,75%,75%,75%,60%,30%,20%,0%,0%\n",
        ))
        .expect("passive-acquire test rules parse")
    }

    fn passive_map_entity(
        owner: &str,
        type_id: &str,
        cx: u16,
        cy: u16,
        category: EntityCategory,
    ) -> crate::map::entities::MapEntity {
        crate::map::entities::MapEntity {
            owner: owner.to_string(),
            type_id: type_id.to_string(),
            health: 256,
            cell_x: cx,
            cell_y: cy,
            facing: 64,
            category,
            sub_cell: 0,
            veterancy: 0,
            high: false,
            mission: None,
        }
    }

    /// Two hostile vehicles parked three cells apart, neither of them given any
    /// order. Runs `ticks` real ticks through `advance_tick` and returns the
    /// sim. Entity 1 is the Allied vehicle, entity 2 the Soviet one.
    ///
    /// Nothing in this fixture issues a command, assigns a target, or deals
    /// damage on purpose, so the ONLY way a target can appear on a unit whose
    /// enemy never shoots is the passive scanner.
    fn run_idle_pair(allied_type: &str, soviet_type: &str, ticks: u64) -> Simulation {
        let rules = passive_rules();
        let heights: std::collections::BTreeMap<(u16, u16), u8> = std::collections::BTreeMap::new();
        let grid = crate::sim::pathfinding::PathGrid::new(64, 64);
        let mut sim = Simulation::with_seed(0x5CA1_AB1E_0001);
        sim.spawn_from_map(
            &[
                passive_map_entity("Americans", allied_type, 20, 20, EntityCategory::Unit),
                passive_map_entity("Soviet", soviet_type, 23, 20, EntityCategory::Unit),
            ],
            Some(&rules),
            &heights,
        );
        for _ in 0..ticks {
            let _ = sim.advance_tick(&[], Some(&rules), &heights, Some(&grid), None, 67);
        }
        sim
    }

    #[test]
    fn idle_guard_unit_acquires_a_target_with_no_order_at_all() {
        // The headline behavior: a parked tank opens fire on an enemy that is
        // simply standing in range. No order, no damage taken, no attack-move.
        // The target is unarmed so receiver-synchronous retaliation cannot
        // replace the scanner-owned bookkeeping under test here.
        let sim = run_idle_pair("MTNK", "UNARM", 90);
        let allied = sim.substrate.entities.get(1).expect("allied tank present");
        assert!(
            allied.attack_target.is_some(),
            "an idle Guard-mission unit must passively acquire a hostile in range"
        );
        assert!(
            allied.passively_acquired_target,
            "the target must be flagged as scanner-acquired (it gates the drop/clear blocks)"
        );
    }

    #[test]
    fn a_unit_that_passively_acquires_does_not_move_toward_the_target() {
        // Pursuit regression guard. The passive commit writes the target
        // pointer only; a Guard unit fires from where it stands. If pursuit
        // ever picks these units up again they walk off across the map with no
        // OrderIntent to bring them home.
        //
        // A target can only be acquired IN range, so the scenario has to open a
        // range gap afterwards: the unarmed Allied scout parks two cells away
        // until the Soviet tank picks it up, then drives off. That is exactly
        // the "an enemy scouted past my base" case. The scout is unarmed, so it
        // never shoots and no retaliation can install a target another way.
        let rules = passive_rules();
        let heights: std::collections::BTreeMap<(u16, u16), u8> = std::collections::BTreeMap::new();
        let grid = crate::sim::pathfinding::PathGrid::new(64, 64);
        let mut sim = Simulation::with_seed(0x5CA1_AB1E_0002);
        sim.spawn_from_map(
            &[
                passive_map_entity("Americans", "UNARM", 22, 20, EntityCategory::Unit),
                passive_map_entity("Soviet", "MTNK", 20, 20, EntityCategory::Unit),
            ],
            Some(&rules),
            &heights,
        );
        let allied = sim.interner.get("Americans").expect("Americans interned");
        let start = sim
            .substrate
            .entities
            .get(2)
            .map(|e| (e.position.rx, e.position.ry))
            .expect("soviet tank present");

        // Let the Soviet tank acquire the parked scout.
        for _ in 0..60 {
            let _ = sim.advance_tick(&[], Some(&rules), &heights, Some(&grid), None, 67);
        }
        let soviet = sim.substrate.entities.get(2).expect("soviet tank present");
        assert!(
            soviet.attack_target.is_some() && soviet.passively_acquired_target,
            "precondition: the idle tank must have picked the scout up on its own"
        );

        // Now the scout runs. The tank must not follow it.
        let scram = crate::sim::command::CommandEnvelope::new(
            allied,
            61,
            crate::sim::command::Command::Move {
                entity_id: 1,
                target_rx: 55,
                target_ry: 20,
                queue: false,
                group_id: None,
            },
        );
        for tick in 60..260u64 {
            let due: Vec<crate::sim::command::CommandEnvelope> = if tick + 1 == 61 {
                vec![scram.clone()]
            } else {
                Vec::new()
            };
            let _ = sim.advance_tick(&due, Some(&rules), &heights, Some(&grid), None, 67);
        }
        let scout = sim.substrate.entities.get(1).expect("scout present");
        assert!(
            scout.position.rx > 30,
            "precondition: the scout actually ran out of the tank's weapon range"
        );
        let soviet = sim.substrate.entities.get(2).expect("soviet tank present");
        assert_eq!(
            (soviet.position.rx, soviet.position.ry),
            start,
            "a passively-acquired target must never be pursued — the unit holds its ground"
        );
        assert!(
            soviet.movement_target.is_none(),
            "no movement may be issued for a scanner-acquired target"
        );
    }

    #[test]
    fn passive_scan_keeps_rescanning_on_cadence_instead_of_latching() {
        // The scanner must not go dormant after one acquisition. With a target
        // installed by the scanner itself the object stays on Guard, so every
        // cadence expiry draws again and re-picks. Latching would leave the
        // timer expired forever and re-acquire with zero delay after combat
        // cleared the target.
        let rules = passive_rules();
        let mut sim = Simulation::new();
        insert_scannable(&mut sim, 1, "Americans", "MTNK", EntityCategory::Unit);

        // First scan: no hostile, so nothing is installed but the draw happens.
        let before = sim.scenario_rng.state();
        passive_acquire_step(&mut sim, 1, Some(&rules));
        assert_ne!(sim.scenario_rng.state(), before);

        // Pretend the scan had found something.
        {
            let e = sim.substrate.entities.get_mut(1).unwrap();
            e.attack_target = Some(AttackTarget::new(9));
            e.passively_acquired_target = true;
            e.passive_scan_timer.clear(); // cadence expired again
        }
        assert_eq!(
            sim.substrate
                .entities
                .get(1)
                .unwrap()
                .passive_acquire_mission(),
            MissionType::Guard,
            "a scanner-installed target must not move the object off Guard"
        );

        let armed = sim.scenario_rng.state();
        passive_acquire_step(&mut sim, 1, Some(&rules));
        assert_ne!(
            sim.scenario_rng.state(),
            armed,
            "the object must still be scanning on cadence with a target installed"
        );
        assert!(
            sim.substrate
                .entities
                .get(1)
                .unwrap()
                .passive_scan_timer
                .is_armed(),
            "the cadence timer is re-armed by the rescan, not left expired"
        );
    }

    #[test]
    fn rescan_that_repicks_the_same_target_does_not_reset_the_weapon_cooldown() {
        // The rearm cooldown lives on the target record here rather than on the
        // object, so a literal drop-and-reinstall every cadence would restart
        // ROF and a slow-firing unit would never fire.
        let rules = passive_rules();
        let heights: std::collections::BTreeMap<(u16, u16), u8> = std::collections::BTreeMap::new();
        let grid = crate::sim::pathfinding::PathGrid::new(64, 64);
        let mut sim = Simulation::with_seed(0x5CA1_AB1E_0003);
        // Spawn through the real path so vision is established; the only
        // candidate is the unarmed Soviet vehicle two cells away.
        sim.spawn_from_map(
            &[
                passive_map_entity("Americans", "MTNK", 20, 20, EntityCategory::Unit),
                passive_map_entity("Soviet", "UNARM", 22, 20, EntityCategory::Unit),
            ],
            Some(&rules),
            &heights,
        );
        let _ = sim.advance_tick(&[], Some(&rules), &heights, Some(&grid), None, 67);
        {
            let mut target = AttackTarget::new(2);
            target.cooldown_ticks = 40;
            let e = sim.substrate.entities.get_mut(1).unwrap();
            e.attack_target = Some(target);
            e.passively_acquired_target = true;
            e.passive_scan_timer.clear();
        }
        passive_acquire_step(&mut sim, 1, Some(&rules));
        let attack = sim
            .substrate
            .entities
            .get(1)
            .unwrap()
            .attack_target
            .as_ref()
            .expect("target retained");
        assert_eq!(attack.target, TargetKind::Entity(2));
        assert_eq!(
            attack.cooldown_ticks, 40,
            "re-picking the same target must leave the ROF cooldown untouched"
        );
    }

    #[test]
    fn rescan_that_changes_target_also_preserves_the_weapon_cooldown() {
        // The sibling of the test above, and the one that matters more: a
        // CHANGED pick must not restart the weapon either. Rebuilding the attack
        // record zeroes the rearm countdown, and since the scanner re-picks
        // nearest-first every ~28 frames while most stock ROF values are longer
        // than that, every time two enemies trade places as nearest the attacker
        // would get a free shot.
        let rules = passive_rules();
        let heights: std::collections::BTreeMap<(u16, u16), u8> = std::collections::BTreeMap::new();
        let grid = crate::sim::pathfinding::PathGrid::new(64, 64);
        let mut sim = Simulation::with_seed(0x5CA1_AB1E_0006);
        sim.spawn_from_map(
            &[
                passive_map_entity("Americans", "MTNK", 20, 20, EntityCategory::Unit),
                // Two candidates; the nearer one (id 3) is what a rescan picks.
                passive_map_entity("Soviet", "UNARM", 24, 20, EntityCategory::Unit),
                passive_map_entity("Soviet", "UNARM", 21, 20, EntityCategory::Unit),
            ],
            Some(&rules),
            &heights,
        );
        let _ = sim.advance_tick(&[], Some(&rules), &heights, Some(&grid), None, 67);
        {
            // Hold the FARTHER one, mid-reload, so the rescan must swing over.
            let mut target = AttackTarget::new(2);
            target.cooldown_ticks = 40;
            let e = sim.substrate.entities.get_mut(1).unwrap();
            e.attack_target = Some(target);
            e.passively_acquired_target = true;
            e.passive_scan_timer.clear();
        }
        passive_acquire_step(&mut sim, 1, Some(&rules));
        let attack = sim
            .substrate
            .entities
            .get(1)
            .unwrap()
            .attack_target
            .as_ref()
            .expect("target retained");
        assert_eq!(
            attack.target,
            TargetKind::Entity(3),
            "precondition: the rescan swung onto the nearer candidate"
        );
        assert_eq!(
            attack.cooldown_ticks, 40,
            "swinging onto a new target must not restart the weapon"
        );
    }

    #[test]
    fn auto_retarget_after_a_scanner_victim_dies_keeps_the_target_scanner_owned() {
        // The state that must never exist: a live target with the provenance
        // flag false on an object that was never given an order. Combat's own
        // auto-retarget fires when the current victim dies and a second
        // candidate is in range — it is a continuation of the same unordered
        // acquisition, so the flag has to carry over. If it does not, the object
        // silently leaves the passive block, starts getting chased by pursuit,
        // and never releases a target that walks out of range.
        let rules = passive_rules();
        let heights: std::collections::BTreeMap<(u16, u16), u8> = std::collections::BTreeMap::new();
        let grid = crate::sim::pathfinding::PathGrid::new(64, 64);
        let mut sim = Simulation::with_seed(0x5CA1_AB1E_0007);
        sim.spawn_from_map(
            &[
                passive_map_entity("Americans", "MTNK", 20, 20, EntityCategory::Unit),
                passive_map_entity("Soviet", "UNARM", 21, 20, EntityCategory::Unit),
                passive_map_entity("Soviet", "UNARM", 23, 20, EntityCategory::Unit),
            ],
            Some(&rules),
            &heights,
        );
        for _ in 0..60 {
            let _ = sim.advance_tick(&[], Some(&rules), &heights, Some(&grid), None, 67);
        }
        assert_eq!(
            sim.substrate
                .entities
                .get(1)
                .unwrap()
                .attack_target
                .as_ref()
                .map(|t| t.target),
            Some(TargetKind::Entity(2)),
            "precondition: the scanner picked the nearer candidate"
        );

        // Put the current victim one hit from death and let combat finish it.
        // The invariant is checked EVERY tick, not just at the end: the leak
        // opens on the exact tick the victim dies and can be papered over by the
        // next cadence rescan, so an end-state assertion would miss it.
        sim.substrate.entities.get_mut(2).unwrap().health.current = 1;
        for tick in 0..80 {
            let _ = sim.advance_tick(&[], Some(&rules), &heights, Some(&grid), None, 67);
            let tank = sim.substrate.entities.get(1).expect("tank present");
            assert!(
                tank.attack_target.is_none() || tank.passively_acquired_target,
                "tick {tick}: the tank was never ordered to attack anything, so a target it \
                 holds must always be scanner-owned — a live target with the flag false takes \
                 it out of the passive block, into pursuit, and it never releases"
            );
        }
        assert!(
            sim.substrate
                .entities
                .get(2)
                .is_none_or(|e| e.health.current == 0 || e.dying),
            "precondition: the first victim died"
        );
        let tank = sim.substrate.entities.get(1).expect("tank present");
        assert!(
            tank.attack_target.is_some() && tank.passively_acquired_target,
            "precondition: the second candidate is still in range and was taken up"
        );
    }

    #[test]
    fn the_shared_retarget_preserves_provenance_and_rearm() {
        // The contract at the one function that owns swinging an attack onto a
        // new victim. Combat's auto-retarget is a continuation of whatever
        // acquisition installed the target, so provenance carries over; and the
        // rearm state must survive (covered end-to-end elsewhere, pinned here).
        let mut e = GameEntity::test_default(1, "MTNK", "Americans", 5, 5);
        let mut target = AttackTarget::new(2);
        target.cooldown_ticks = 33;
        target.burst_remaining = 2;
        e.attack_target = Some(target);
        e.passively_acquired_target = true;

        crate::sim::combat::retarget_preserving_rearm(&mut e, 7);

        let attack = e.attack_target.as_ref().expect("target retained");
        assert_eq!(attack.target, TargetKind::Entity(7));
        assert_eq!(attack.cooldown_ticks, 33, "rearm must survive the swing");
        assert_eq!(attack.burst_remaining, 2, "burst must survive the swing");
        assert!(
            e.passively_acquired_target,
            "an auto-retarget is not a new order — the target stays scanner-owned"
        );

        // An ordered target stays ordered through the same swing.
        e.passively_acquired_target = false;
        crate::sim::combat::retarget_preserving_rearm(&mut e, 9);
        assert!(!e.passively_acquired_target);
    }

    #[test]
    fn a_defence_actually_damages_what_it_passively_acquires() {
        // Structures carry gates units do not — the deploy animation and the
        // power check — so acquisition alone does not prove a defence shoots.
        let rules = passive_rules();
        let heights: std::collections::BTreeMap<(u16, u16), u8> = std::collections::BTreeMap::new();
        let grid = crate::sim::pathfinding::PathGrid::new(64, 64);
        let mut sim = Simulation::with_seed(0x5CA1_AB1E_0008);
        sim.spawn_from_map(
            &[
                passive_map_entity("Soviet", "UNARM", 22, 20, EntityCategory::Unit),
                passive_map_entity("Americans", "NASAM", 20, 20, EntityCategory::Structure),
                passive_map_entity("Americans", "GAPOWR", 14, 26, EntityCategory::Structure),
            ],
            Some(&rules),
            &heights,
        );
        for _ in 0..160 {
            let _ = sim.advance_tick(&[], Some(&rules), &heights, Some(&grid), None, 67);
        }
        let scout = sim.substrate.entities.get(1).expect("scout present");
        assert!(
            scout.health.current < scout.health.max,
            "an unordered base defence must actually shoot what it picks up: {}/{}",
            scout.health.current,
            scout.health.max,
        );
    }

    #[test]
    fn a_unit_that_finished_a_move_order_still_acquires_and_shoots() {
        // The dominant case in real play: a unit is ordered somewhere, arrives,
        // and sits. Five Grizzlies sent to a chokepoint must not watch an enemy
        // drive past. Since the Move handler's arrival hook landed, the unit
        // gets there the way retail does — the hook drops the target, queues
        // Guard, and the host promotes it — rather than by the derived-reading
        // bridge that used to carry it while the selector stayed stuck on Move.
        let rules = passive_rules();
        let heights: std::collections::BTreeMap<(u16, u16), u8> = std::collections::BTreeMap::new();
        let grid = crate::sim::pathfinding::PathGrid::new(64, 64);
        let mut sim = Simulation::with_seed(0x5CA1_AB1E_0009);
        sim.spawn_from_map(
            &[
                passive_map_entity("Americans", "MTNK", 10, 20, EntityCategory::Unit),
                passive_map_entity("Soviet", "UNARM", 22, 20, EntityCategory::Unit),
            ],
            Some(&rules),
            &heights,
        );
        let allied = sim.interner.get("Americans").expect("Americans interned");
        // Drive to a cell three away from the parked enemy, then never touch it
        // again. The enemy is unarmed, so nothing can provoke a retaliation.
        let order = crate::sim::command::CommandEnvelope::new(
            allied,
            2,
            crate::sim::command::Command::Move {
                entity_id: 1,
                target_rx: 19,
                target_ry: 20,
                queue: false,
                group_id: None,
            },
        );
        for tick in 0..220u64 {
            let due: Vec<crate::sim::command::CommandEnvelope> = if tick + 1 == 2 {
                vec![order.clone()]
            } else {
                Vec::new()
            };
            let _ = sim.advance_tick(&due, Some(&rules), &heights, Some(&grid), None, 67);
        }

        let tank = sim.substrate.entities.get(1).expect("tank present");
        assert_eq!(
            tank.mission.current().known(),
            Some(MissionType::Guard),
            "the arrival hook committed Guard — the unit must not be left on Move"
        );
        assert!(
            tank.movement_target.is_none(),
            "precondition: the move actually finished"
        );
        assert_eq!(
            tank.passive_acquire_mission(),
            MissionType::Guard,
            "a finished order must not leave the unit stuck outside the gate"
        );
        assert!(
            tank.attack_target.is_some() && tank.passively_acquired_target,
            "an ordered-then-idle unit must acquire like any other idle unit"
        );
        let enemy = sim.substrate.entities.get(2).expect("enemy present");
        assert!(
            enemy.health.current < enemy.health.max,
            "and it must actually open fire: {}/{}",
            enemy.health.current,
            enemy.health.max,
        );
    }

    #[test]
    fn a_stopped_unit_is_not_deafened_by_the_stop_mission() {
        // Stop(13) is one of the twelve missions that strip a scanner target, so
        // a literal read of the committed selector would make pressing S both
        // silence the unit and take away what it had found.
        let mut e = GameEntity::test_default(1, "MTNK", "Americans", 5, 5);
        e.mission
            .apply_test_fixture(fixture_with_current(&e.mission, MissionType::Stop));
        assert_eq!(
            e.passive_acquire_mission(),
            MissionType::Guard,
            "a finished Stop reads as Guard, not as the target-stripping Stop mission"
        );
        // While the order is still live, the committed selector wins.
        e.movement_target = Some(MovementTarget::default());
        assert_eq!(e.passive_acquire_mission(), MissionType::Stop);
    }

    fn fixture_with_current(mission: &MissionCom, current: MissionType) -> MissionTestFixture {
        let mut fixture = mission_test_fixture(mission);
        fixture.current = MissionId::from_known(current);
        fixture
    }

    #[test]
    fn a_passively_acquired_target_actually_gets_shot() {
        // Acquire AND fire. Everything else here stops at "a target is
        // installed"; this is the one that proves the round trip reaches damage.
        // Both tanks are armed and neither is ordered to do anything.
        let sim = run_idle_pair("MTNK", "MTNK", 160);
        let allied = sim.substrate.entities.get(1).expect("allied tank present");
        let soviet = sim.substrate.entities.get(2).expect("soviet tank present");
        assert!(
            allied.health.current < allied.health.max || soviet.health.current < soviet.health.max,
            "a passively acquired target must actually be fired on: \
             allied {}/{}, soviet {}/{}",
            allied.health.current,
            allied.health.max,
            soviet.health.current,
            soviet.health.max,
        );
    }

    #[test]
    fn idle_infantry_acquires_a_target_with_no_order_at_all() {
        // Infantry reach the same block through the same foot leaf. A rifleman
        // standing next to an enemy must open fire on his own.
        let rules = passive_rules();
        let heights: std::collections::BTreeMap<(u16, u16), u8> = std::collections::BTreeMap::new();
        let grid = crate::sim::pathfinding::PathGrid::new(64, 64);
        let mut sim = Simulation::with_seed(0x5CA1_AB1E_0004);
        sim.spawn_from_map(
            &[
                passive_map_entity("Americans", "UNARM", 22, 20, EntityCategory::Unit),
                passive_map_entity("Soviet", "GI", 20, 20, EntityCategory::Infantry),
            ],
            Some(&rules),
            &heights,
        );
        for _ in 0..90 {
            let _ = sim.advance_tick(&[], Some(&rules), &heights, Some(&grid), None, 67);
        }
        let rifleman = sim.substrate.entities.get(2).expect("infantry present");
        assert!(
            rifleman.attack_target.is_some() && rifleman.passively_acquired_target,
            "idle infantry must passively acquire a hostile in range"
        );
    }

    #[test]
    fn can_passive_aquire_no_type_never_acquires() {
        // The enemy is UNARMED in both runs, so it can neither acquire nor
        // shoot — retaliation cannot muddy the result and a target on the
        // vehicle under test can only have come from its own scanner.
        let control = run_idle_pair("UNARM", "MTNK", 90);
        assert!(
            control
                .substrate
                .entities
                .get(2)
                .expect("soviet vehicle present")
                .attack_target
                .is_some(),
            "control: an ordinary type acquires the unarmed enemy in this fixture"
        );

        let opted_out = run_idle_pair("UNARM", "NOACQ", 90);
        let soviet = opted_out
            .substrate
            .entities
            .get(2)
            .expect("soviet vehicle present");
        assert!(
            soviet.attack_target.is_none(),
            "CanPassiveAquire=no must keep the type out of the scanner entirely"
        );
        assert!(!soviet.passively_acquired_target);
    }

    /// Insert one object of `type_id` owned by `owner` with the passive-scan
    /// timer already due, so the next `passive_acquire_step` reaches the gate.
    fn insert_scannable(
        sim: &mut Simulation,
        id: u64,
        owner: &str,
        type_id: &str,
        category: EntityCategory,
    ) {
        let owner_ref = sim.interner.intern(owner);
        let type_ref = sim.interner.intern(type_id);
        let mut e = GameEntity::new_at_frame_zero_for_test(
            id,
            5,
            5,
            0,
            0,
            owner_ref,
            crate::sim::components::Health {
                current: 100,
                max: 100,
            },
            type_ref,
            category,
            0,
            5,
            true,
        );
        e.category = category;
        e.passive_scan_timer.clear(); // due now
        sim.substrate.entities.insert(e);
    }

    #[test]
    fn scan_rearms_the_timer_to_the_ini_delay_plus_the_draw() {
        // The cadence comes from [General], never from a constant in code: the
        // re-armed duration is NormalTargetingDelay plus the 0..=2 jitter, and
        // the anchor is the current frame.
        let rules = passive_rules();
        let mut sim = Simulation::new();
        insert_scannable(&mut sim, 1, "Americans", "MTNK", EntityCategory::Unit);

        let mut probe = sim.scenario_rng.clone();
        let expected_jitter = probe.next_range_u32_inclusive(0, PASSIVE_SCAN_DELAY_JITTER_MAX);

        passive_acquire_step(&mut sim, 1, Some(&rules));

        let timer = sim.substrate.entities.get(1).unwrap().passive_scan_timer;
        assert_eq!(timer.start_frame, sim.session.binary_frame);
        assert_eq!(
            timer.duration,
            rules.general.normal_targeting_delay + expected_jitter,
            "re-armed duration must be the INI delay plus this scan's jitter draw"
        );
        assert_eq!(
            sim.substrate
                .entities
                .get(1)
                .unwrap()
                .last_target_scan_frame,
            sim.session.binary_frame
        );
    }

    #[test]
    fn scan_draws_exactly_one_scenario_value_and_moves_no_other_stream() {
        // Lockstep contract. Exactly one RandomRanged(0,2) per scan, on the
        // SCENARIO stream, and it is unconditional — this fixture has no
        // hostile at all, so no target is found and the draw still happens.
        let rules = passive_rules();
        let mut sim = Simulation::new();
        insert_scannable(&mut sim, 1, "Americans", "MTNK", EntityCategory::Unit);

        let mut expected_scenario = sim.scenario_rng.clone();
        expected_scenario.next_range_u32_inclusive(0, PASSIVE_SCAN_DELAY_JITTER_MAX);
        let main_before = sim.main_rng.state();
        let mapgen_before = sim.mapgen_rng.state();

        passive_acquire_step(&mut sim, 1, Some(&rules));

        assert_eq!(
            sim.scenario_rng.state(),
            expected_scenario.state(),
            "the scan must consume EXACTLY one scenario draw"
        );
        assert_eq!(
            sim.main_rng.state(),
            main_before,
            "main stream must not move"
        );
        assert_eq!(
            sim.mapgen_rng.state(),
            mapgen_before,
            "mapgen stream must not move"
        );
        assert!(
            sim.substrate
                .entities
                .get(1)
                .unwrap()
                .attack_target
                .is_none(),
            "no hostile exists, so no target is installed — the draw still happened"
        );

        // A second call before the timer expires draws nothing at all.
        let after_first = sim.scenario_rng.state();
        passive_acquire_step(&mut sim, 1, Some(&rules));
        assert_eq!(
            sim.scenario_rng.state(),
            after_first,
            "the cadence timer gates the scan; a non-due object draws nothing"
        );
    }

    #[test]
    fn unpowered_defence_never_reaches_the_scanner() {
        // An unpowered base defence does not look for targets. Observable
        // without a hostile present: the scan's unconditional draw is the FIRST
        // thing the scanner does, so zero draws means the gate held.
        let rules = passive_rules();
        let mut sim = Simulation::new();
        insert_scannable(&mut sim, 1, "Americans", "NASAM", EntityCategory::Structure);
        let owner = sim.interner.intern("Americans");
        sim.power_states.insert(
            owner,
            crate::sim::power_system::PowerState {
                is_low_power: true,
                ..Default::default()
            },
        );

        let before = sim.scenario_rng.state();
        passive_acquire_step(&mut sim, 1, Some(&rules));
        assert_eq!(
            sim.scenario_rng.state(),
            before,
            "an unpowered defence must not reach the scanner"
        );

        // Control: with power restored the same defence scans.
        sim.power_states
            .get_mut(&owner)
            .expect("power state present")
            .is_low_power = false;
        sim.substrate
            .entities
            .get_mut(1)
            .unwrap()
            .passive_scan_timer
            .clear();
        passive_acquire_step(&mut sim, 1, Some(&rules));
        assert_ne!(
            sim.scenario_rng.state(),
            before,
            "a powered defence reaches the scanner"
        );
    }

    #[test]
    fn a_defence_releases_a_target_that_leaves_range_and_does_not_latch() {
        // The base-defence loop must keep re-evaluating. Nothing in VERA ever
        // moves a building off a mission, and combat deliberately keeps a target
        // that has only gone out of range, so if the defence ever left the
        // passive gate it would stay locked on one scout for the whole match and
        // stay silent through everything that came after.
        //
        // The gap must be range-only, not vision: combat already drops a target
        // whose cell stops being visible, so a scout that ran off the map edge
        // would prove nothing. [NASAM] has Range=6 and Sight=10, so the scout
        // backs off from 2 cells to 8 — outside the weapon, still plainly in
        // sight. Only the scanner's own re-evaluation can release it there.
        let rules = passive_rules();
        let heights: std::collections::BTreeMap<(u16, u16), u8> = std::collections::BTreeMap::new();
        let grid = crate::sim::pathfinding::PathGrid::new(64, 64);
        let mut sim = Simulation::with_seed(0x5CA1_AB1E_0005);
        sim.spawn_from_map(
            &[
                passive_map_entity("Soviet", "UNARM", 22, 20, EntityCategory::Unit),
                passive_map_entity("Americans", "NASAM", 20, 20, EntityCategory::Structure),
                // The defence drains power; without a plant the unpowered arm of
                // the can-acquire check would (correctly) keep it out entirely.
                passive_map_entity("Americans", "GAPOWR", 14, 26, EntityCategory::Structure),
            ],
            Some(&rules),
            &heights,
        );
        let soviet = sim.interner.get("Soviet").expect("Soviet interned");
        for _ in 0..60 {
            let _ = sim.advance_tick(&[], Some(&rules), &heights, Some(&grid), None, 67);
        }
        assert!(
            sim.substrate
                .entities
                .get(2)
                .expect("defence present")
                .attack_target
                .is_some(),
            "precondition: the defence acquires the scout parked in range"
        );

        let scram = crate::sim::command::CommandEnvelope::new(
            soviet,
            61,
            crate::sim::command::Command::Move {
                entity_id: 1,
                target_rx: 28,
                target_ry: 20,
                queue: false,
                group_id: None,
            },
        );
        for tick in 60..300u64 {
            let due: Vec<crate::sim::command::CommandEnvelope> = if tick + 1 == 61 {
                vec![scram.clone()]
            } else {
                Vec::new()
            };
            let _ = sim.advance_tick(&due, Some(&rules), &heights, Some(&grid), None, 67);
        }
        let scout = sim.substrate.entities.get(1).expect("scout present");
        assert!(
            scout.position.rx >= 27,
            "precondition: the scout actually left the defence's Range=6"
        );
        assert!(
            sim.fog.is_cell_visible(
                sim.interner.get("Americans").expect("Americans interned"),
                scout.position.rx,
                scout.position.ry,
            ),
            "precondition: the scout is still in sight, so only a rescan can release it"
        );
        let defence = sim.substrate.entities.get(2).expect("defence present");
        assert!(
            defence.attack_target.is_none(),
            "the defence must release a target that walked out of range, not latch onto it"
        );
        assert!(
            defence.passive_scan_timer.is_armed(),
            "and it must still be scanning on cadence, ready for the next contact"
        );
    }

    #[test]
    fn off_mission_clear_drops_a_scanner_target_but_not_an_ordered_one() {
        let mut sim = Simulation::new();
        insert_scannable(&mut sim, 1, "Americans", "MTNK", EntityCategory::Unit);
        let now = sim.session.binary_frame;
        // Unload is one of the twelve missions that drop a scanner target. The
        // job has to be LIVE for the committed selector to be read — a mission
        // whose machinery has gone quiet defers to the derived reading, which is
        // what stops a finished order from deafening a unit forever.
        sim.mission_assign_exact(1, MissionId::from_known(MissionType::Unload), now)
            .unwrap();
        {
            let e = sim.substrate.entities.get_mut(1).unwrap();
            e.movement_target = Some(MovementTarget::default());
            e.attack_target = Some(AttackTarget::new(9));
            e.passively_acquired_target = true;
        }
        clear_passive_target_off_mission(&mut sim, 1);
        let e = sim.substrate.entities.get(1).unwrap();
        assert!(e.attack_target.is_none(), "a scanner target is dropped");
        assert!(!e.passively_acquired_target);

        // An ordered target on the same mission is left alone.
        {
            let e = sim.substrate.entities.get_mut(1).unwrap();
            e.attack_target = Some(AttackTarget::new(9));
            e.passively_acquired_target = false;
        }
        clear_passive_target_off_mission(&mut sim, 1);
        assert!(
            sim.substrate
                .entities
                .get(1)
                .unwrap()
                .attack_target
                .is_some(),
            "only a scanner-acquired target is dropped"
        );
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
        entity.owner = sim.interner.intern("Americans");
        entity.type_ref = sim.interner.intern("TEST");
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

    fn move_cadence_rules() -> RuleSet {
        RuleSet::from_ini(&IniFile::from_str("[General]\n\n[Move]\nRate=.016\n"))
            .expect("move cadence rules parse")
    }

    fn representative_foot_handler_rules() -> RuleSet {
        RuleSet::from_ini(&IniFile::from_str(
            "[General]\n\n[Move]\nRate=.016\n\n[Attack]\nRate=.016\n\n[Guard]\nRate=.016\n\n[Hunt]\nRate=.016\n",
        ))
        .expect("representative Foot handler rules parse")
    }

    #[test]
    fn move_handler_rearms_from_the_authoritative_object_ai_host() {
        let mut sim = ordinary_drive_host_sim(0xCAFE);
        let rules = move_cadence_rules();
        let mut expected_rng = sim.clone_scenario_rng();
        let jitter = expected_rng.next_range_u32_inclusive(0, 2) as i32;

        assert!(sim.object_ai_visit_one(
            ORDINARY_DRIVE_HOST_ID,
            Some(&rules),
            ObjectAiCtx::default(),
        ));

        let entity = sim
            .substrate
            .entities
            .get(ORDINARY_DRIVE_HOST_ID)
            .expect("fixture entity");
        assert_eq!(entity.mission.dispatch_timer().start_frame(), 0);
        assert_eq!(entity.mission.dispatch_timer().delay(), 14 + jitter);
        assert_eq!(
            sim.scenario_rng.logical_state(),
            expected_rng.logical_state()
        );

        let hash = sim.state_hash();
        let bytes = GameSnapshot::save(&sim, 0, 0, "move_cadence", 0);
        let restored = GameSnapshot::load(&bytes).expect("snapshot").sim;
        assert_eq!(restored.state_hash(), hash);
        assert_eq!(
            restored
                .substrate
                .entities
                .get(ORDINARY_DRIVE_HOST_ID)
                .expect("restored fixture entity")
                .mission
                .dispatch_timer(),
            entity.mission.dispatch_timer()
        );
    }

    #[test]
    fn move_handler_preserves_pending_timer_without_consuming_rng() {
        let mut sim = ordinary_drive_host_sim(0xCAFE);
        let rules = move_cadence_rules();
        let pending = MissionDispatchTimer::from_raw(0, 5);
        let fixture = mission_test_fixture(
            &sim.substrate
                .entities
                .get(ORDINARY_DRIVE_HOST_ID)
                .expect("fixture entity")
                .mission,
        );
        sim.substrate
            .entities
            .get_mut(ORDINARY_DRIVE_HOST_ID)
            .expect("fixture entity")
            .mission
            .apply_test_fixture(MissionTestFixture {
                dispatch_timer: pending,
                ..fixture
            });
        let before_rng = sim.scenario_rng.logical_state();

        sim.object_ai_visit_one(ORDINARY_DRIVE_HOST_ID, Some(&rules), ObjectAiCtx::default());

        assert_eq!(
            sim.substrate
                .entities
                .get(ORDINARY_DRIVE_HOST_ID)
                .expect("fixture entity")
                .mission
                .dispatch_timer(),
            pending
        );
        assert_eq!(sim.scenario_rng.logical_state(), before_rng);
    }

    #[test]
    fn move_handler_arrival_returns_one_frame_without_rng() {
        let mut sim = ordinary_drive_host_sim(0xCAFE);
        let rules = move_cadence_rules();
        sim.substrate
            .entities
            .get_mut(ORDINARY_DRIVE_HOST_ID)
            .expect("fixture entity")
            .navigation
            .nav_com = None;
        let before_rng = sim.scenario_rng.logical_state();

        sim.object_ai_visit_one(ORDINARY_DRIVE_HOST_ID, Some(&rules), ObjectAiCtx::default());

        assert_eq!(
            sim.substrate
                .entities
                .get(ORDINARY_DRIVE_HOST_ID)
                .expect("fixture entity")
                .mission
                .dispatch_timer(),
            MissionDispatchTimer::from_raw(0, 1)
        );
        assert_eq!(sim.scenario_rng.logical_state(), before_rng);
    }

    #[test]
    fn infantry_attack_handler_halves_jittered_cadence_inside_the_proven_band() {
        let mut sim = Simulation::with_seed(0xA771);
        let rules = representative_foot_handler_rules();
        let mut infantry = entity_of(1, EntityCategory::Infantry);
        infantry.attack_target = Some(AttackTarget::new(2));
        update_mission_test_fixture(&mut infantry.mission, |fixture| {
            fixture.current = MissionId::from_known(MissionType::Attack);
            fixture.dispatch_timer = MissionDispatchTimer::at_frame(0);
        });
        let target = entity_of(2, EntityCategory::Unit);
        assert_eq!((target.position.rx, target.position.ry), (5, 5));
        infantry.position.rx = 7; // 512 leptons: within FootClass::Mission_Attack band.
        sim.substrate.entities.insert(infantry);
        sim.substrate.entities.insert(target);

        let mut expected_rng = sim.clone_scenario_rng();
        let expected_delay = (14 + expected_rng.next_range_u32_inclusive(0, 2) as i32) / 2;
        sim.object_ai_visit_one(1, Some(&rules), ObjectAiCtx::default());

        assert_eq!(
            sim.substrate
                .entities
                .get(1)
                .unwrap()
                .mission
                .dispatch_timer(),
            MissionDispatchTimer::from_raw(0, expected_delay)
        );
        assert_eq!(
            sim.scenario_rng.logical_state(),
            expected_rng.logical_state()
        );
    }

    #[test]
    fn unit_attack_delegates_to_the_foot_attack_cadence() {
        let mut sim = Simulation::with_seed(0xA772);
        let rules = representative_foot_handler_rules();
        let mut unit = entity_of(1, EntityCategory::Unit);
        unit.attack_target = Some(AttackTarget::new(2));
        update_mission_test_fixture(&mut unit.mission, |fixture| {
            fixture.current = MissionId::from_known(MissionType::Attack);
            fixture.dispatch_timer = MissionDispatchTimer::at_frame(0);
        });
        let target = entity_of(2, EntityCategory::Unit);
        unit.position.rx = 7; // 512 leptons: FootClass::Mission_Attack band.
        register_entity(&mut sim, unit);
        register_entity(&mut sim, target);

        let mut expected_rng = sim.clone_scenario_rng();
        let expected_delay = (14 + expected_rng.next_range_u32_inclusive(0, 2) as i32) / 2;
        sim.object_ai_visit_one(1, Some(&rules), ObjectAiCtx::default());

        assert_eq!(
            sim.substrate
                .entities
                .get(1)
                .unwrap()
                .mission
                .dispatch_timer(),
            MissionDispatchTimer::from_raw(0, expected_delay)
        );
        assert_eq!(
            sim.scenario_rng.logical_state(),
            expected_rng.logical_state()
        );
    }

    /// The Attack handler's only exit. With the shoot-at target gone and no
    /// destination left, the idle-mode selector commits Guard — and Guard is
    /// inside the passive-acquire gate, so the object starts scanning again.
    /// Without this arm an object parked on Attack never scans for a target
    /// for the rest of the match.
    #[test]
    fn attack_handler_with_no_target_enters_idle_mode_and_regains_passive_acquire() {
        let mut sim = Simulation::with_seed(0xA774);
        let rules = representative_foot_handler_rules();
        let mut infantry = entity_of(1, EntityCategory::Infantry);
        infantry.attack_target = None;
        update_mission_test_fixture(&mut infantry.mission, |fixture| {
            fixture.current = MissionId::from_known(MissionType::Attack);
            fixture.dispatch_timer = MissionDispatchTimer::at_frame(0);
        });
        register_entity(&mut sim, infantry);

        let mut expected_rng = sim.clone_scenario_rng();
        // Both arms of the target branch reach the cadence tail, so the idle
        // exit adds no draw; the half-cadence band needs a live target.
        let expected_delay = 14 + expected_rng.next_range_u32_inclusive(0, 2) as i32;
        sim.object_ai_visit_one(1, Some(&rules), ObjectAiCtx::default());

        let infantry = sim.substrate.entities.get(1).unwrap();
        assert_eq!(
            infantry.mission.queued().known(),
            Some(MissionType::Guard),
            "the idle selector queued a replacement instead of leaving it on Attack"
        );
        assert_eq!(
            infantry.mission.dispatch_timer(),
            MissionDispatchTimer::from_raw(0, expected_delay)
        );
        assert_eq!(
            sim.scenario_rng.logical_state(),
            expected_rng.logical_state(),
            "the idle exit consumes no extra RNG"
        );
        assert!(
            passive_acquire_gate(MissionType::Guard, true, false),
            "the replacement mission is one the passive-acquire gate admits"
        );
        assert!(
            !passive_acquire_gate(MissionType::Attack, true, true),
            "Attack itself is not, which is why the exit is load-bearing"
        );
    }

    /// Same exit, but the object still has somewhere to be — the destination a
    /// Restore just gave back. The idle selector picks Move, not Guard.
    #[test]
    fn attack_handler_with_no_target_but_a_destination_enters_move() {
        let mut sim = Simulation::with_seed(0xA775);
        let rules = representative_foot_handler_rules();
        let mut infantry = entity_of(1, EntityCategory::Infantry);
        infantry.attack_target = None;
        infantry.navigation.nav_com =
            Some(crate::sim::components::NavTargetRef::Cell { rx: 20, ry: 21 });
        update_mission_test_fixture(&mut infantry.mission, |fixture| {
            fixture.current = MissionId::from_known(MissionType::Attack);
            fixture.dispatch_timer = MissionDispatchTimer::at_frame(0);
        });
        register_entity(&mut sim, infantry);

        sim.object_ai_visit_one(1, Some(&rules), ObjectAiCtx::default());

        assert_eq!(
            sim.substrate
                .entities
                .get(1)
                .unwrap()
                .mission
                .queued()
                .known(),
            Some(MissionType::Move)
        );
    }

    /// The idle selector's whole decision table, driven directly so the arms the
    /// Attack handler cannot reach are still pinned.
    #[test]
    fn enter_idle_mode_selector_matches_the_leaf_decision_table() {
        let rules = representative_foot_handler_rules();
        let base = MissionHandlerInput {
            category: EntityCategory::Infantry,
            mission: Some(MissionType::Attack),
            timer_due: true,
            moving_or_queued: false,
            bunker_delegate: false,
            has_attack_target: false,
            has_destination: false,
            effective_mission: Some(MissionType::Attack),
            unit_deploy_begin_active: false,
            unit_deploy_reverse_active: false,
        };

        assert_eq!(
            foot_enter_idle_mode_queue(&rules, base),
            Some(MissionType::Guard),
            "no destination: Guard"
        );
        assert_eq!(
            foot_enter_idle_mode_queue(
                &rules,
                MissionHandlerInput {
                    has_destination: true,
                    ..base
                }
            ),
            Some(MissionType::Move),
            "a destination is installed: Move"
        );
        for already_idle in [MissionType::Guard, MissionType::AreaGuard] {
            assert_eq!(
                foot_enter_idle_mode_queue(
                    &rules,
                    MissionHandlerInput {
                        mission: Some(MissionType::Hunt),
                        effective_mission: Some(already_idle),
                        ..base
                    }
                ),
                None,
                "{already_idle:?} is already idle, so nothing is assigned"
            );
        }
        assert_eq!(
            foot_enter_idle_mode_queue(
                &rules,
                MissionHandlerInput {
                    mission: Some(MissionType::Patrol),
                    effective_mission: Some(MissionType::Patrol),
                    has_destination: true,
                    ..base
                }
            ),
            None,
            "the committed-selector tail gate blocks the assign"
        );
        for blocked in [MissionType::Unload, MissionType::Eaten] {
            assert_eq!(
                foot_enter_idle_mode_queue(
                    &rules,
                    MissionHandlerInput {
                        category: EntityCategory::Unit,
                        mission: Some(blocked),
                        effective_mission: Some(blocked),
                        has_destination: true,
                        ..base
                    }
                ),
                None,
                "the Unit tail gate also excludes {blocked:?}"
            );
            assert_eq!(
                foot_enter_idle_mode_queue(
                    &rules,
                    MissionHandlerInput {
                        mission: Some(blocked),
                        effective_mission: Some(blocked),
                        has_destination: true,
                        ..base
                    }
                ),
                Some(MissionType::Move),
                "but the Infantry tail gate does not"
            );
        }
    }

    /// The `Zombie=`/`Paralyzed=` early return, read off the object's own
    /// control entry. Neither key is present in stock `[Attack]`, so this cannot
    /// fire from the Attack handler; the gate is modelled because the same
    /// virtual is entered from other missions.
    #[test]
    fn enter_idle_mode_selector_honours_frozen_control_entries() {
        let rules = RuleSet::from_ini(&IniFile::from_str(
            "[General]\n\n[Attack]\nRate=.016\n\n[Hunt]\nRate=.016\nParalyzed=yes\n\n[Sticky]\nRate=.016\nZombie=yes\n",
        ))
        .expect("frozen control rules parse");
        let base = MissionHandlerInput {
            category: EntityCategory::Infantry,
            mission: Some(MissionType::Attack),
            timer_due: true,
            moving_or_queued: false,
            bunker_delegate: false,
            has_attack_target: false,
            has_destination: false,
            effective_mission: Some(MissionType::Attack),
            unit_deploy_begin_active: false,
            unit_deploy_reverse_active: false,
        };

        for frozen in [MissionType::Hunt, MissionType::Sticky] {
            assert_eq!(
                foot_enter_idle_mode_queue(
                    &rules,
                    MissionHandlerInput {
                        effective_mission: Some(frozen),
                        ..base
                    }
                ),
                None,
                "{frozen:?} carries a frozen control entry"
            );
        }
        assert_eq!(
            foot_enter_idle_mode_queue(&rules, base),
            Some(MissionType::Guard),
            "stock [Attack] carries neither key"
        );
    }

    #[test]
    fn attack_handler_clears_only_an_authoritatively_stale_entity_target() {
        let mut sim = Simulation::with_seed(0xA773);
        let rules = representative_foot_handler_rules();
        let mut unit = entity_of(1, EntityCategory::Unit);
        unit.attack_target = Some(AttackTarget::new(99));
        update_mission_test_fixture(&mut unit.mission, |fixture| {
            fixture.current = MissionId::from_known(MissionType::Attack);
            fixture.dispatch_timer = MissionDispatchTimer::at_frame(0);
        });
        register_entity(&mut sim, unit);

        sim.object_ai_visit_one(1, Some(&rules), ObjectAiCtx::default());

        let unit = sim.substrate.entities.get(1).unwrap();
        assert!(unit.attack_target.is_none());
        assert_eq!(unit.mission.queued(), MissionId::NONE);
    }

    #[test]
    fn infantry_guard_handler_skips_jitter_for_a_bunker_delegate() {
        let mut sim = Simulation::with_seed(0x6A2D);
        let rules = representative_foot_handler_rules();
        let mut infantry = entity_of(1, EntityCategory::Infantry);
        infantry.bunker_link = BunkerLink::Installed(99);
        update_mission_test_fixture(&mut infantry.mission, |fixture| {
            fixture.current = MissionId::from_known(MissionType::Guard);
            fixture.dispatch_timer = MissionDispatchTimer::at_frame(0);
        });
        sim.substrate.entities.insert(infantry);
        let before_rng = sim.scenario_rng.logical_state();

        sim.object_ai_visit_one(1, Some(&rules), ObjectAiCtx::default());

        assert_eq!(
            sim.substrate
                .entities
                .get(1)
                .unwrap()
                .mission
                .dispatch_timer(),
            MissionDispatchTimer::from_raw(0, 14)
        );
        assert_eq!(sim.scenario_rng.logical_state(), before_rng);
    }

    #[test]
    fn unit_guard_deploy_begin_queues_harvest_without_rng() {
        let mut sim = Simulation::with_seed(0x6A2E);
        let rules = representative_foot_handler_rules();
        let mut unit = entity_of(1, EntityCategory::Unit);
        unit.mission_leaf = MissionLeafState::unit_raw_for_test(1, 0, 0, 0);
        update_mission_test_fixture(&mut unit.mission, |fixture| {
            fixture.current = MissionId::from_known(MissionType::Guard);
            fixture.dispatch_timer = MissionDispatchTimer::at_frame(0);
        });
        register_entity(&mut sim, unit);
        let before_rng = sim.scenario_rng.logical_state();

        sim.object_ai_visit_one(1, Some(&rules), ObjectAiCtx::default());

        let unit = sim.substrate.entities.get(1).unwrap();
        assert_eq!(unit.mission.queued().known(), Some(MissionType::Harvest));
        assert_eq!(
            unit.mission.dispatch_timer(),
            MissionDispatchTimer::from_raw(0, 1)
        );
        assert_eq!(sim.scenario_rng.logical_state(), before_rng);
    }

    #[test]
    fn unit_guard_deploy_reverse_queues_unload_without_rng() {
        let mut sim = Simulation::with_seed(0x6A2F);
        let rules = representative_foot_handler_rules();
        let mut unit = entity_of(1, EntityCategory::Unit);
        unit.mission_leaf = MissionLeafState::unit_raw_for_test(0, 1, 0, 0);
        update_mission_test_fixture(&mut unit.mission, |fixture| {
            fixture.current = MissionId::from_known(MissionType::Guard);
            fixture.dispatch_timer = MissionDispatchTimer::at_frame(0);
        });
        register_entity(&mut sim, unit);
        let before_rng = sim.scenario_rng.logical_state();

        sim.object_ai_visit_one(1, Some(&rules), ObjectAiCtx::default());

        let unit = sim.substrate.entities.get(1).unwrap();
        assert_eq!(unit.mission.queued().known(), Some(MissionType::Unload));
        assert_eq!(
            unit.mission.dispatch_timer(),
            MissionDispatchTimer::from_raw(0, 1)
        );
        assert_eq!(sim.scenario_rng.logical_state(), before_rng);
    }

    #[test]
    fn infantry_hunt_handler_uses_foot_jittered_cadence() {
        let mut sim = Simulation::with_seed(0x487A);
        let rules = representative_foot_handler_rules();
        let mut infantry = entity_of(1, EntityCategory::Infantry);
        update_mission_test_fixture(&mut infantry.mission, |fixture| {
            fixture.current = MissionId::from_known(MissionType::Hunt);
            fixture.dispatch_timer = MissionDispatchTimer::at_frame(0);
        });
        sim.substrate.entities.insert(infantry);
        let mut expected_rng = sim.clone_scenario_rng();
        let expected_delay = 14 + expected_rng.next_range_u32_inclusive(0, 2) as i32;

        sim.object_ai_visit_one(1, Some(&rules), ObjectAiCtx::default());

        assert_eq!(
            sim.substrate
                .entities
                .get(1)
                .unwrap()
                .mission
                .dispatch_timer(),
            MissionDispatchTimer::from_raw(0, expected_delay)
        );
        assert_eq!(
            sim.scenario_rng.logical_state(),
            expected_rng.logical_state()
        );
    }

    #[test]
    fn unit_hunt_handler_uses_base_cadence_without_foot_jitter() {
        let mut sim = Simulation::with_seed(0xF007);
        let rules = representative_foot_handler_rules();
        let mut unit = entity_of(1, EntityCategory::Unit);
        unit.owner = sim.interner.intern("Americans");
        unit.type_ref = sim.interner.intern("TEST");
        update_mission_test_fixture(&mut unit.mission, |fixture| {
            fixture.current = MissionId::from_known(MissionType::Hunt);
            fixture.dispatch_timer = MissionDispatchTimer::at_frame(0);
        });
        sim.substrate.entities.insert(unit);
        let before_rng = sim.scenario_rng.logical_state();

        sim.object_ai_visit_one(1, Some(&rules), ObjectAiCtx::default());

        assert_eq!(
            sim.substrate
                .entities
                .get(1)
                .unwrap()
                .mission
                .dispatch_timer(),
            MissionDispatchTimer::from_raw(0, 14)
        );
        assert_eq!(sim.scenario_rng.logical_state(), before_rng);
    }

    #[test]
    fn unit_hunt_does_not_infer_enter_without_an_approach_result() {
        let mut sim = Simulation::with_seed(0xF008);
        let rules = representative_foot_handler_rules();
        let mut unit = entity_of(1, EntityCategory::Unit);
        unit.attack_target = Some(AttackTarget::new(2));
        update_mission_test_fixture(&mut unit.mission, |fixture| {
            fixture.current = MissionId::from_known(MissionType::Hunt);
            fixture.dispatch_timer = MissionDispatchTimer::at_frame(0);
        });
        register_entity(&mut sim, unit);
        let target = entity_of(2, EntityCategory::Structure);
        register_entity(&mut sim, target);
        let before_rng = sim.scenario_rng.logical_state();

        sim.object_ai_visit_one(1, Some(&rules), ObjectAiCtx::default());

        let unit = sim.substrate.entities.get(1).unwrap();
        assert_eq!(unit.mission.queued(), MissionId::NONE);
        assert_eq!(
            unit.mission.dispatch_timer(),
            MissionDispatchTimer::from_raw(0, 14)
        );
        assert_eq!(sim.scenario_rng.logical_state(), before_rng);
    }

    // ===== The base (un-overridden) mission handler =====

    /// A mission whose slot no leaf class overrides re-arms with a flat 450
    /// frames and draws nothing. Without this arm the dispatch timer stayed at
    /// whatever Commence wrote — `{now, 0}`, i.e. permanently due — so the
    /// mission timer gated nothing for those objects.
    #[test]
    fn base_stub_missions_rearm_450_frames_without_touching_rng() {
        for (category, mission) in [
            (EntityCategory::Unit, MissionType::Stop),
            (EntityCategory::Unit, MissionType::Selling),
            (EntityCategory::Unit, MissionType::Sleep),
            (EntityCategory::Unit, MissionType::Harmless),
            (EntityCategory::Infantry, MissionType::Stop),
            // Repair is a base stub for Infantry only.
            (EntityCategory::Infantry, MissionType::Repair),
        ] {
            let mut sim = Simulation::with_seed(0x5B2E);
            let rules = representative_foot_handler_rules();
            let mut unit = entity_of(1, category);
            update_mission_test_fixture(&mut unit.mission, |fixture| {
                fixture.current = MissionId::from_known(mission);
                fixture.dispatch_timer = MissionDispatchTimer::at_frame(0);
            });
            register_entity(&mut sim, unit);
            let before_rng = sim.scenario_rng.logical_state();

            sim.object_ai_visit_one(1, Some(&rules), ObjectAiCtx::default());

            assert_eq!(
                sim.substrate
                    .entities
                    .get(1)
                    .unwrap()
                    .mission
                    .dispatch_timer(),
                MissionDispatchTimer::from_raw(0, BASE_MISSION_HANDLER_FRAMES),
                "{category:?} on {mission:?} must re-arm the base handler's flat delay"
            );
            assert_eq!(
                sim.scenario_rng.logical_state(),
                before_rng,
                "{category:?} on {mission:?} must draw no RNG — the base stub touches nothing"
            );
        }
    }

    /// The dispatcher's bounds test on the mission id is unsigned, so the idle
    /// sentinel takes the switch default — the same base handler `Sleep(0)` uses
    /// — rather than being skipped.
    #[test]
    fn the_idle_sentinel_takes_the_default_arm() {
        let mut sim = Simulation::with_seed(0x5B34);
        let rules = representative_foot_handler_rules();
        let mut unit = entity_of(1, EntityCategory::Unit);
        update_mission_test_fixture(&mut unit.mission, |fixture| {
            fixture.current = MissionId::NONE;
            fixture.dispatch_timer = MissionDispatchTimer::at_frame(0);
        });
        register_entity(&mut sim, unit);
        let before_rng = sim.scenario_rng.logical_state();

        sim.object_ai_visit_one(1, Some(&rules), ObjectAiCtx::default());

        assert_eq!(
            sim.substrate
                .entities
                .get(1)
                .unwrap()
                .mission
                .dispatch_timer(),
            MissionDispatchTimer::from_raw(0, BASE_MISSION_HANDLER_FRAMES)
        );
        assert_eq!(sim.scenario_rng.logical_state(), before_rng);
    }

    /// A mission whose leaf class DOES override the slot with a real handler
    /// must not be given the base handler's value — that would install a 30
    /// second timer where the original installs its own, much shorter one.
    #[test]
    fn overridden_leaf_handlers_are_left_alone_by_the_default_arm() {
        // Area Guard used to sit in this list; it now has an absorbed handler
        // arm of its own, so the dispatcher reaches it and re-arms its timer.
        // `base_mission_handler_delay` still reports `None` for it — the two
        // facts are independent.
        assert_eq!(
            base_mission_handler_delay(EntityCategory::Unit, Some(MissionType::AreaGuard)),
            None
        );
        for (category, mission) in [
            (EntityCategory::Unit, MissionType::Enter),
            (EntityCategory::Unit, MissionType::Unload),
            // Repair overrides on Units even though it is a stub on Infantry.
            (EntityCategory::Unit, MissionType::Repair),
            (EntityCategory::Infantry, MissionType::Enter),
        ] {
            assert_eq!(
                base_mission_handler_delay(category, Some(mission)),
                None,
                "{category:?} on {mission:?} has a real leaf handler"
            );
            let mut sim = Simulation::with_seed(0x5B2F);
            let rules = representative_foot_handler_rules();
            let mut unit = entity_of(1, category);
            update_mission_test_fixture(&mut unit.mission, |fixture| {
                fixture.current = MissionId::from_known(mission);
                fixture.dispatch_timer = MissionDispatchTimer::at_frame(0);
            });
            register_entity(&mut sim, unit);

            sim.object_ai_visit_one(1, Some(&rules), ObjectAiCtx::default());

            assert_eq!(
                sim.substrate
                    .entities
                    .get(1)
                    .unwrap()
                    .mission
                    .dispatch_timer(),
                MissionDispatchTimer::at_frame(0),
                "{category:?} on {mission:?} must keep its untouched timer"
            );
        }
    }

    /// Sticky and Guard dispatch through the same slot, so Sticky runs the
    /// Guard handler — but the cadence comes from the object's OWN mission
    /// slot, so `[Sticky] Rate` (14) applies, not `[Guard] Rate` (27).
    #[test]
    fn sticky_runs_the_guard_handler_at_its_own_rate() {
        let mut sim = Simulation::with_seed(0x21C0);
        let rules = RuleSet::from_ini(&IniFile::from_str(
            "[General]\n\n[Guard]\nRate=.030\n\n[Sticky]\nRate=.016\n",
        ))
        .expect("sticky cadence rules parse");
        let mut unit = entity_of(1, EntityCategory::Unit);
        update_mission_test_fixture(&mut unit.mission, |fixture| {
            fixture.current = MissionId::from_known(MissionType::Sticky);
            fixture.dispatch_timer = MissionDispatchTimer::at_frame(0);
        });
        register_entity(&mut sim, unit);
        let mut expected_rng = sim.clone_scenario_rng();
        let jitter = expected_rng.next_range_u32_inclusive(0, 2) as i32;

        sim.object_ai_visit_one(1, Some(&rules), ObjectAiCtx::default());

        assert_eq!(
            sim.substrate
                .entities
                .get(1)
                .unwrap()
                .mission
                .dispatch_timer(),
            MissionDispatchTimer::from_raw(0, 14 + jitter),
            "[Sticky] Rate=.016 is 14 frames; [Guard] Rate=.030 would be 27"
        );
        assert_eq!(
            sim.scenario_rng.logical_state(),
            expected_rng.logical_state()
        );
    }

    /// A vehicle that finishes a move order drops its shoot-at target and
    /// queues Guard — the arrival hook's ordinary-vehicle arm. Without this the
    /// unit stays on Move for the rest of the match.
    #[test]
    fn move_arrival_drops_the_vehicle_target_and_queues_guard() {
        let mut sim = Simulation::with_seed(0x7389);
        let rules = move_cadence_rules();
        let mut unit = entity_of(1, EntityCategory::Unit);
        unit.attack_target = Some(AttackTarget::new(2));
        unit.passively_acquired_target = true;
        update_mission_test_fixture(&mut unit.mission, |fixture| {
            fixture.current = MissionId::from_known(MissionType::Move);
            fixture.dispatch_timer = MissionDispatchTimer::at_frame(0);
        });
        register_entity(&mut sim, unit);
        let before_rng = sim.scenario_rng.logical_state();

        sim.object_ai_visit_one(1, Some(&rules), ObjectAiCtx::default());

        let entity = sim.substrate.entities.get(1).expect("fixture entity");
        assert_eq!(
            entity.mission.queued(),
            MissionId::from_known(MissionType::Guard)
        );
        assert!(
            entity.attack_target.is_none(),
            "Assign_Target(NULL) on arrival"
        );
        assert!(!entity.passively_acquired_target);
        assert_eq!(
            entity.mission.dispatch_timer(),
            MissionDispatchTimer::from_raw(0, 1),
            "the arrival branch still returns one frame"
        );
        // The arrival branch draws nothing: the jitter belongs to the still-
        // moving branch only.
        assert_eq!(sim.scenario_rng.logical_state(), before_rng);
    }

    /// Infantry's arrival selector is NOT the vehicle one: a live target queues
    /// Attack and the target is kept, where a vehicle would drop it and fall to
    /// Guard.
    #[test]
    fn move_arrival_promotes_an_infantry_target_to_attack() {
        let mut sim = Simulation::with_seed(0x51CB);
        let rules = move_cadence_rules();
        let mut man = entity_of(1, EntityCategory::Infantry);
        man.attack_target = Some(AttackTarget::new(2));
        update_mission_test_fixture(&mut man.mission, |fixture| {
            fixture.current = MissionId::from_known(MissionType::Move);
            fixture.dispatch_timer = MissionDispatchTimer::at_frame(0);
        });
        register_entity(&mut sim, man);

        sim.object_ai_visit_one(1, Some(&rules), ObjectAiCtx::default());

        let entity = sim.substrate.entities.get(1).expect("fixture entity");
        assert_eq!(
            entity.mission.queued(),
            MissionId::from_known(MissionType::Attack)
        );
        assert!(
            entity.attack_target.is_some(),
            "the infantry arm keeps the target it promotes"
        );
    }

    /// A targetless infantryman arriving falls to Guard, same as a vehicle.
    #[test]
    fn move_arrival_queues_guard_for_a_targetless_infantryman() {
        let mut sim = Simulation::with_seed(0x51CC);
        let rules = move_cadence_rules();
        let mut man = entity_of(1, EntityCategory::Infantry);
        update_mission_test_fixture(&mut man.mission, |fixture| {
            fixture.current = MissionId::from_known(MissionType::Move);
            fixture.dispatch_timer = MissionDispatchTimer::at_frame(0);
        });
        register_entity(&mut sim, man);

        sim.object_ai_visit_one(1, Some(&rules), ObjectAiCtx::default());

        assert_eq!(
            sim.substrate
                .entities
                .get(1)
                .expect("fixture entity")
                .mission
                .queued(),
            MissionId::from_known(MissionType::Guard)
        );
    }

    /// The infantry arrival selector is skipped entirely when the mission the
    /// object arrived ON carries `Zombie=` or `Paralyzed=`. `[Move]` carries
    /// neither in stock rules, so this uses a fixture that sets one.
    #[test]
    fn move_arrival_infantry_selector_is_skipped_by_a_paralyzed_move_entry() {
        let mut sim = Simulation::with_seed(0x51CD);
        let rules = RuleSet::from_ini(&IniFile::from_str(
            "[General]\n\n[Move]\nRate=.016\nParalyzed=yes\n",
        ))
        .expect("paralyzed move rules parse");
        let mut man = entity_of(1, EntityCategory::Infantry);
        update_mission_test_fixture(&mut man.mission, |fixture| {
            fixture.current = MissionId::from_known(MissionType::Move);
            fixture.dispatch_timer = MissionDispatchTimer::at_frame(0);
        });
        register_entity(&mut sim, man);

        sim.object_ai_visit_one(1, Some(&rules), ObjectAiCtx::default());

        let entity = sim.substrate.entities.get(1).expect("fixture entity");
        assert_eq!(entity.mission.queued(), MissionId::NONE);
        assert_eq!(
            entity.mission.dispatch_timer(),
            MissionDispatchTimer::from_raw(0, 1)
        );
    }

    /// Area Guard has its own handler and its own cadence: `[Area Guard]
    /// Rate=.040` is 36 frames, and the jitter draw is `RandomRanged(1, 5)` —
    /// NOT the `(0, 2)` every other absorbed handler takes. The fixture type
    /// carries no weapon, so the can-acquire predicate fails and the scan (with
    /// its own separate draw) never runs.
    #[test]
    fn area_guard_rearms_at_its_own_rate_with_a_one_to_five_jitter() {
        let mut sim = Simulation::with_seed(0x4D6A);
        let rules = RuleSet::from_ini(&IniFile::from_str(
            "[General]\n\n[Guard]\nRate=.030\n\n[Area Guard]\nRate=.040\nAARate=.032\n",
        ))
        .expect("area guard cadence rules parse");
        let mut unit = entity_of(1, EntityCategory::Unit);
        update_mission_test_fixture(&mut unit.mission, |fixture| {
            fixture.current = MissionId::from_known(MissionType::AreaGuard);
            fixture.dispatch_timer = MissionDispatchTimer::at_frame(0);
        });
        register_entity(&mut sim, unit);
        let mut expected_rng = sim.clone_scenario_rng();
        let jitter = expected_rng.next_range_u32_inclusive(1, 5) as i32;
        assert!((1..=5).contains(&jitter));

        sim.object_ai_visit_one(1, Some(&rules), ObjectAiCtx::default());

        assert_eq!(
            sim.substrate
                .entities
                .get(1)
                .expect("fixture entity")
                .mission
                .dispatch_timer(),
            MissionDispatchTimer::from_raw(0, 36 + jitter),
            "[Area Guard] Rate=.040 is 36 frames; [Guard] Rate=.030 would be 27"
        );
        assert_eq!(
            sim.scenario_rng.logical_state(),
            expected_rng.logical_state(),
            "exactly one RandomRanged(1, 5) and nothing else"
        );
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
        if entity.low_bridge_tube_state.is_some() {
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
            || entity.rocket_state.is_some()
            || entity.homing_state.is_some()
            || entity.parachute_state.is_some()
        {
            return Err(HostTraceError::SpecialLocomotorPath);
        }

        let locomotor = entity
            .locomotor
            .as_ref()
            .ok_or(HostTraceError::SpecialLocomotorPath)?;
        if locomotor.active_kind() != LocomotorKind::Drive
            || locomotor.effective_kind() != LocomotorKind::Drive
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
            target: DriveCoord {
                x: 6 * 256 + 128,
                y: 5 * 256 + 128,
                z: 0,
            },
        });
        assert_ordinary_drive_host_error(
            &low_bridge_tube,
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
            .slot = LocomotorSlot::from_kind(LocomotorKind::Teleport);
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
            .begin_piggyback(LocomotorKind::Teleport, MovementLayer::Ground);
        assert_ordinary_drive_host_error(
            &piggyback,
            &control,
            120,
            ordinary,
            HostTraceError::SpecialLocomotorPath,
        );
        // There used to be a second scenario here for the separate "override"
        // mechanism. The two collapsed into one when the piggyback slot became
        // single, so the case it covered is the one directly above.

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

    /// An interned Infantry carrying a Walk locomotor, so the readiness producer
    /// has a real family to map and the moving gate is live.
    fn insert_interned_walker(sim: &mut Simulation, id: u64, rx: u16, ry: u16) {
        let owner = sim.interner.intern("Americans");
        let type_ref = sim.interner.intern("TEST");
        let mut e = GameEntity::new_at_frame_zero_for_test(
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
            EntityCategory::Infantry,
            0,
            5,
            true,
        );
        e.locomotor = Some(
            crate::sim::movement::locomotor::LocomotorState::for_test_kind(
                crate::rules::locomotor_type::LocomotorKind::Walk,
            ),
        );
        e.mission_leaf = crate::sim::mission::leaf::MissionLeafState::infantry_raw_for_test(0, -1);
        sim.substrate.entities.insert(e);
        register_in_logic(sim, id);
    }

    /// The post-movement checkpoint walks the logic scheduler, not the entity
    /// store, so a fixture entity has to be a scheduler member or the walk skips
    /// it and every assertion passes vacuously.
    fn register_in_logic(sim: &mut Simulation, id: u64) {
        sim.substrate.logic.try_push(id).expect("logic slot");
        sim.substrate
            .entities
            .get_mut(id)
            .expect("fixture entity")
            .in_logic_vector = true;
    }

    /// Drive the Walk readiness producer's inputs: a live movement target with a
    /// remaining path step reads as moving, its absence as stopped.
    fn set_walking(sim: &mut Simulation, id: u64, walking: bool) {
        let entity = sim.substrate.entities.get_mut(id).expect("fixture entity");
        entity.movement_target = walking.then(|| crate::sim::components::MovementTarget {
            path: vec![(5, 5), (6, 5)],
            next_index: 1,
            current_speed: crate::util::fixed_math::SimFixed::from_num(4),
            ..Default::default()
        });
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

    /// An infantry that is still walking when the pre-movement checkpoint runs,
    /// and has come to rest by the post-movement one, must commence in the SAME
    /// tick — not the next.
    ///
    /// This is the whole point of the second checkpoint. gamemd's
    /// `InfantryClass::AI` gates either side of the object's own locomotion, so
    /// stopping mid-tick makes the unit eligible before the tick ends. With only
    /// the pre-movement gate the promotion slipped a tick, and that lag compounds
    /// through every chained handoff.
    ///
    /// Infantry rather than Unit deliberately: the Unit moving-defer branch is
    /// inert in production today because `signed_height` has no producer and the
    /// resulting error maps to permissive-ready, so a Unit fixture would pass for
    /// the wrong reason. Infantry readiness reads no height.
    #[test]
    fn post_movement_checkpoint_commences_a_walker_that_stopped_this_tick() {
        let rules = promotion_rules();
        let mut sim = Simulation::new();
        insert_interned_walker(&mut sim, 1, 5, 5);

        sim.mission_queue_exact(
            1,
            MissionId::from_known(MissionType::Move),
            0,
            0,
            &crate::sim::mission::authority::EntityReadyInputProvider,
        )
        .unwrap();

        // Still walking: the pre-movement checkpoint must NOT commence it.
        set_walking(&mut sim, 1, true);
        sim.object_ai_post_movement_promote(Some(&rules));
        let e = sim.substrate.entities.get(1).unwrap();
        assert_eq!(
            e.mission.queued().known(),
            Some(MissionType::Move),
            "a walking infantry must still be deferred: the moving gate is what \
             this checkpoint exists to re-evaluate, so if it commences here the \
             test proves nothing"
        );
        assert_eq!(e.mission.current(), MissionId::NONE);

        // Movement ended during the tick's movement phases.
        set_walking(&mut sim, 1, false);
        sim.object_ai_post_movement_promote(Some(&rules));

        let e = sim.substrate.entities.get(1).unwrap();
        assert_eq!(
            e.mission.current().known(),
            Some(MissionType::Move),
            "coming to rest during the movement phases must let the second \
             checkpoint commence the queued mission in the same tick"
        );
        assert_eq!(e.mission.queued(), MissionId::NONE);
    }

    /// The second checkpoint must not touch categories gamemd gates only once.
    /// Aircraft gate a single time, after their locomotion; buildings gate twice
    /// inside their own update, which is not a movement bracket.
    #[test]
    fn post_movement_checkpoint_skips_aircraft_and_structures() {
        let rules = promotion_rules();
        let mut sim = Simulation::new();
        for (id, category) in [
            (1u64, EntityCategory::Aircraft),
            (2, EntityCategory::Structure),
        ] {
            let owner = sim.interner.intern("Americans");
            let type_ref = sim.interner.intern("TEST");
            let e = GameEntity::new_at_frame_zero_for_test(
                id,
                5,
                5,
                0,
                0,
                owner,
                crate::sim::components::Health {
                    current: 100,
                    max: 100,
                },
                type_ref,
                category,
                0,
                5,
                true,
            );
            sim.substrate.entities.insert(e);
            register_in_logic(&mut sim, id);
            sim.mission_queue_exact(
                id,
                MissionId::from_known(MissionType::Move),
                0,
                0,
                &crate::sim::mission::authority::EntityReadyInputProvider,
            )
            .ok();
        }
        let before: Vec<_> = [1u64, 2]
            .iter()
            .map(|id| sim.substrate.entities.get(*id).unwrap().mission)
            .collect();

        sim.object_ai_post_movement_promote(Some(&rules));

        for (index, id) in [1u64, 2].iter().enumerate() {
            assert_eq!(
                sim.substrate.entities.get(*id).unwrap().mission,
                before[index],
                "entity {id} is not a twice-gated category and must be untouched"
            );
        }
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

    /// Post-flip corpse freeze: a dying Unit still owns its live scheduler slot,
    /// but only its death action runs there. Its ordinary mission state (counter
    /// included) therefore freezes at death.
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
            "a dying Unit's mission state freezes while its death slot runs"
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
