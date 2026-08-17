//! Evidence-backed Foot/Unit mission-handler cadence evaluation.
//!
//! The object-AI host and its ordering remain in the parent module; this module
//! owns only handler inputs, results, and the single timer epilogue.

use super::{Simulation, can_acquire_target, passive_target_scan};
use crate::map::entities::EntityCategory;
use crate::rules::ruleset::RuleSet;
use crate::sim::mission::authority::EntityReadyInputProvider;
use crate::sim::mission::{MissionId, MissionType};

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
pub(super) fn dispatch_supported_foot_mission_cadence(
    sim: &mut Simulation,
    id: u64,
    rules: &RuleSet,
) {
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
        // Rate=.016` is 14 frames against Guard's 26. Stock skirmish maps park
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
pub(super) struct MissionHandlerInput {
    pub(super) category: EntityCategory,
    /// The committed selector, or `None` for the idle sentinel. The native
    /// dispatcher's bounds test on the mission id is UNSIGNED, so the sentinel
    /// and every out-of-range id take the switch's default arm rather than
    /// being skipped.
    pub(super) mission: Option<MissionType>,
    pub(super) timer_due: bool,
    pub(super) moving_or_queued: bool,
    pub(super) bunker_delegate: bool,
    pub(super) has_attack_target: bool,
    /// The destination slot on its own. The idle-mode selector reads this one
    /// field, not the broader in-motion test [`Self::moving_or_queued`] uses.
    pub(super) has_destination: bool,
    /// Current when present, otherwise queued — the selector the idle-mode
    /// early returns and the control-entry lookups read.
    pub(super) effective_mission: Option<MissionType>,
    pub(super) unit_deploy_begin_active: bool,
    pub(super) unit_deploy_reverse_active: bool,
}

/// The handler result is evaluated before the one common MissionClass timer
/// write, which prevents branch-local epilogues from double-rearming it.
#[derive(Debug, Clone, Copy)]
pub(super) struct MissionHandlerEvaluation {
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
pub(super) fn move_arrival_evaluation(
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
pub(super) fn foot_enter_idle_mode_queue(
    rules: &RuleSet,
    input: MissionHandlerInput,
) -> Option<MissionType> {
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
pub(super) const BASE_MISSION_HANDLER_FRAMES: i32 = 450;

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
pub(super) fn base_mission_handler_delay(
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
