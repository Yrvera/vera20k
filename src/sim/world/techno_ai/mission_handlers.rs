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
        // **Infantry take a leaf override first, and VERA does not model it.**
        // `InfantryClass`'s Move slot is `+0x22C` = `0x0051F660`, which gates on
        // `[this+0x6C4] ∈ {0x1B, 0x1C, 0x1D, 0x1E}` — a small per-infantry state
        // enum, identity UNCHECKED, the same field `0x00521320` reads — and for
        // a human-owned unit (`vtable+0x3C` → `HouseClass::IsControlledByHuman`
        // @ `0x0050B730`) calls `Set_Destination(NULL, true)` through `+0x480`
        // and returns 1, never entering `FootClass::Mission_Move` @ `0x004D4200`
        // and never drawing its jitter. Trigger: a deployed infantryman ordered
        // to move. Player effect: retail drops the destination and re-dispatches
        // next frame; VERA keeps the destination and draws the cadence jitter.
        // Frequency: GI, Guardian GI and Desolator deploy is routine, so this is
        // not rare. Downstream risk: it is one RNG draw per occurrence, so the
        // stream diverges too. **Note the frame trap**: `[this+0x6C4]` is a
        // `UnitTypeClass*` on UnitClass and this state enum on InfantryClass,
        // which caches its own type at `+0x6C0`.
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
        // `FootClass::Mission_Attack`, so vehicles belong on this path.
        //
        // RESIDUAL (GSI-07.06) — **Infantry do NOT**, and that is not modelled.
        // `InfantryClass`'s Attack slot `+0x210` is `0x0051F3E0`, a real
        // override with three branches ahead of the Foot body:
        // - Human-owned infantry whose DoType `[this+0x6C4]` is in
        //   `{0x1B, 0x1C, 0x1D, 0x1E}` call vtable `+0x428` (`0x0051F330`, an
        //   in-place re-acquire: keep firing if the target is still legal, else
        //   rescan in range, else go idle — it never walks), then return the
        //   PLAIN `ftol(Rate) + RandomRanged(0,2)`, skipping the whole Foot
        //   body. So no half-cadence gate and no idle-mode exit. The same field
        //   and value set gate `InfantryClass::Mission_Move` (`0x0051F660`),
        //   recorded on the Move arm above.
        //
        //   The DoType identity is PROVED, not assumed. The sequence-name
        //   pointer table is at `0x008255C8`, 42 entries, bounded by
        //   `InfantryTypeClass::ReadSequenceData @ 0x00523D00`'s
        //   `while (ptr < 0x825670)`. Indices `0x1B`..`0x1E` are `Deploy`,
        //   `Deployed`, `DeployedFire`, `DeployedIdle`. Corroborated by
        //   `InfantryClass::Do_Action @ 0x0051D6F0`, whose land-to-water remap
        //   pairs (Walk/Crawl->Swim, Ready/Prone->Tread, Die1/Die2->WetDie1/2,
        //   FireUp/FireProne->WetAttack) all decode exactly under this table,
        //   and which plays `DeploySound=` on `0x1B` and `UndeploySound=` on
        //   `0x1F`.
        //
        //   Trigger: an EXPLICIT player attack order given to an
        //   already-deployed infantryman. Not merely "a deployed unit firing" —
        //   auto-acquire while deployed does not reach here, because
        //   `Mission_Guard` (`0x0051F620`) and `Mission_AreaGuard`
        //   (`0x0051F640`) both route through `FUN_00521320`, which handles the
        //   deployed state itself and calls `+0x428` directly without changing
        //   mission.
        //   Player effect: VERA runs the half-cadence test and the idle exit
        //   that retail skips, so the dispatch cadence and the scenario-RNG
        //   draw rate diverge for that engagement.
        //   Frequency: several times per match in any game with Allied GIs or
        //   Soviet Desolators — routine micro, not continuous. An earlier draft
        //   of this note said continuous; that was wrong, and the correction is
        //   the auto-acquire route above.
        //
        //   Stock infantry that can enter the set (`Deployer=` on
        //   `InfantryTypeClass+0xEC8`): `E1`, `GGI`, `DESO`, `YURI`, `YURIPR`.
        //   `CAOS` also carries `Deployer=` but is a voxel `UnitClass` and
        //   never reaches these paths. `0x1E` is unreachable for every stock
        //   type — `GISequence`/`GuardianGISequence` author
        //   `DeployedIdle=0,0,0` and `Do_Action` rejects a zero frame count —
        //   and `0x1D` is reachable only for `E1`/`GGI`/`DESO`.
        //
        //   **The frame trap has THREE meanings at this offset, not two.**
        //   `InfantryClass` instance `+0x6C4` is this DoType (init `-1`);
        //   `UnitClass` instance `+0x6C4` is a `UnitTypeClass*`; and
        //   `TechnoTypeClass+0x6C4` is `UndeployDelay=` (stock `YURI=150`,
        //   `YURIPR=75`). `Mission_Move` itself reads the TYPE one at
        //   `0x0051F6A4` — loading `[ESI+0x6C0]` first — to decide whether an
        //   AI-owned deployed unit undeploys immediately, so a port that reads
        //   the instance field there gets Yuri Clone undeploy wrong.
        // - A spy/engineer-class infantryman (`InfType+0xEC2`, or
        //   `HasWeaponAbility(0xE)`) holding a BuildingClass target whose type
        //   has `+0x1577` set and `+0x1701` clear takes
        //   `Set_Destination(target, 1)` then `Assign_Mission(0x11 Sabotage)`
        //   and returns 1. Frequency: low-to-moderate — the ordinary
        //   right-click resolver usually issues the enter action directly, so
        //   this is the force-fire / retarget path.
        // - An AI-owned engineer or medic converts to Capture. Frequency: zero
        //   today, because this project has no AI opponent.
        //
        // RESIDUAL (GSI-07.06) — two further Foot-body steps are absent:
        // - Step 1, the `HoverAttack` re-anchor. When `TechnoType+0x390`
        //   (`HoverAttack`, NOT `DefaultToGuardArea` — the research corpus has
        //   those crossed) is set and `GetHeight() == 0`, native finds a nearby
        //   passable cell and takes it as a destination every dispatch. Live
        //   FootClass carriers in stock: `JUMPJET` (Rocketeer) and
        //   `SCHP`/`SCHD` (Siege Chopper). Trigger: a landed Rocketeer or a
        //   grounded Siege Chopper on Attack. Player effect: retail nudges them
        //   off the spot; VERA's stay put. Frequency: routine in Allied and
        //   Soviet mid-game. The key IS parsed, for locomotor selection only.
        // - Step 2, the `[this+0x68E]` re-acquire, which runs
        //   `Greatest_Threat` once and clears the flag. Frequency: ZERO today —
        //   its only producers are the tank-bunker adjacency scans in
        //   `Mission_Guard @ 0x004D51C5` and `Mission_AreaGuard @ 0x004D7018`,
        //   both already recorded as residuals on their own arms. Downstream
        //   risk is the reason it is written down: the consumer must land in
        //   the same slice as the bunker scan, or a bunker-acquired unit keeps
        //   the bunker as its target instead of re-picking one dispatch later.
        (EntityCategory::Unit | EntityCategory::Infantry, Some(MissionType::Attack)) => {
            let cadence = jittered_mission_cadence(sim, rules, MissionType::Attack);
            let delay = if foot_attack_in_half_cadence_band(sim, rules, id) {
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
            // **VERA-internal, gamemd equivalent UNCHECKED — this mapping is
            // wrong and the arm is dead.** The "three byte latches, then
            // `Assign_Mission(5, 0)`, then `return 1`" shape lives at
            // `0x00740A90`, which is vtable `+0x22C` — the **Move** slot, not
            // Guard — reads `[this+0x6E0]`/`+0x6E1`/`+0x6E2`, and queues
            // **Guard**, not Harvest or Unload. `UnitClass`'s real Guard
            // override `0x00740810` gates its `Queue_Mission(10)`/`return 1` on
            // `UnitTypeClass+0xE0E`/`+0xE0F` plus house and refinery checks, and
            // its `Queue_Mission(0x10)` path returns `ftol(Rate) + Rand(0, 2)`
            // rather than 1.
            //
            // Trigger: none today — both latch bytes have only `#[cfg(test)]`
            // writers (`sim::mission::leaf`), so production never reaches
            // either arm. Player effect: none. Frequency: zero. Downstream
            // risk: the wrong native mapping would be carried straight into any
            // future deploy work; the shape belongs on the Move arm.
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
        // RESIDUAL (GSI-07.20) — Hunt has a cadence and no body. Native
        // `FootClass::Mission_Hunt @ 0x004D5350` selects among Sabotage,
        // Capture and a Move return-to-base; the two arms here only re-arm the
        // dispatch timer, so an object on Hunt is assigned and then does
        // nothing. Trigger: anything queued onto Hunt — the crate and trigger
        // paths assign it today. Player effect: hunting units stand still.
        // Frequency: low in stock skirmish, because the main assigner would be
        // an AI opponent, which this project has not built yet. Downstream
        // risk: the selector needs the Capture/Sabotage target scan, so it
        // lands with the engineer and terrorist mission work, not before.
        //
        // RESIDUAL (GSI-07.09) — Mission 4 Retreat has no handler at all. It is
        // absent from the stub set as well, so a Retreat-committed object falls
        // out of the dispatcher with its timer untouched; native
        // `FootClass::Mission_Retreat @ 0x004DA2C0` oscillates between two
        // states. Trigger: any assigner queuing Retreat. Player effect: the
        // object freezes instead of withdrawing. Frequency: zero today for the
        // same reason as Hunt. Downstream risk: the missing timer touch means
        // the object also never re-enters the dispatch cadence, so a future
        // assigner would strand it rather than merely mis-handle it.
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
/// - the base hook's NavQueue pop and locomotor piggyback unwind. These are
///   **not** early returns that suppress the selector — that reading was wrong.
///   `FootClass::Enter_Idle_Mode` @ `0x004D82B0` pops `[this+0x598]` /
///   `[this+0x58C]`, calls `Assign_Destination(next, 0)` and returns 1, but both
///   leaves discard that return for control flow (`0x00738970` assigns it to a
///   local and then unconditionally calls `+0x4AC`; `0x0051CBA0` the same).
///   Waypointed movement continues because the pop **installs a destination**,
///   so the leaf then reads `[this+0x5A4] != 0` and picks Move(2) on its own.
///   Both are inert here — `nav_queue` has no production writer and the
///   piggyback unwind runs in the movement phase — but a future NavQueue writer
///   must pop *before* the selector reads the destination, not restore an early
///   return that does not exist.
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
    // **VERA-internal, gamemd equivalent UNCHECKED — two ordering/indexing
    // differences, both inert today.** Native is
    // `if (effective != -1) { entry = MissionControl[*(this+0xAC)];
    // if (entry[+7] || entry[+5]) return; }` — the *effective* mission only
    // gates whether to look at all, while the *committed* one at `+0xAC` indexes
    // the table. This looks the entry up on `effective_mission`. And the
    // infantry leaf tests the destination `[this+0x5A4]` **before** the
    // Guard/AreaGuard and Zombie/Paralyzed gates, where this tests them first.
    //
    // Trigger: a caller whose committed and effective missions differ, or one
    // that reaches here with both a destination and a frozen mission. Player
    // effect: none today — the only live entry is the Attack handler's
    // no-target exit, where committed == effective == Attack and `[Attack]`
    // carries neither key. Frequency: zero. Downstream risk: a second producer
    // would inherit both. (Curiosity for whoever ports it: with a current of -1
    // and only a queued mission, native indexes `MissionControl[-1]` — an
    // out-of-bounds read one entry below the array.)
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

/// `FootClass::Mission_AreaGuard` 0x004D6AA0 — "hold this spot and cover it".
///
/// With no target installed and the base can-acquire predicate satisfied
/// (`TechnoClass::CanAcquireTarget` 0x007091D0 at 0x004D6EDB), the handler runs
/// the SAME shared target scanner the common AI body runs for
/// Move/Guard/Harvest — slot `+0x39C`, 0x00709820, called at 0x004D6F06 — but
/// with the Area Guard threat mask (which is what widens the acquisition radius
/// — see `combat::threat_range`). If that installs a target the handler returns
/// one frame. Otherwise the cadence is the object's own `[Area Guard] Rate`
/// plus a `RandomRanged(1, 5)` draw (0x004D7059).
///
/// Deliberately NOT represented, recorded:
/// - **the head hand-off arm.** Before anything else, `if ([this+0x2E4]) {
///   Queue_Mission(Guard, commence=1); return 1; }` (0x004D6AAB). VERA has no
///   field for `+0x2E4` and its identity is UNCHECKED. Trigger: whatever sets
///   that dword. Player effect: retail drops such an object straight back onto
///   plain Guard on its next dispatch; VERA keeps it area-guarding. Frequency:
///   unknown pending the field's identity. Downstream risk: none — it is one
///   queue call away once the field is named.
/// - **the three containment latches** the head shares verbatim with
///   `Mission_Guard` (0x004D6ACC-0x004D6B22) — same residual as recorded on
///   [`evaluate_foot_guard_cadence`].
/// - **the harvester-resume arm** (0x004D6D1A-0x004D6D69): for a `UnitClass`
///   (`What_Am_I()` 1) whose type carries byte `+0xE0E`, the handler runs
///   `Queue_Mission(Harvest, 0)`, calls `[vtable+0x1EC]` (Commence) and returns
///   `RandomRanged(1, 10) + 1` — a different cadence AND a different draw from
///   the `(1, 5)` above. Trigger: a miner on Area Guard. Player effect: retail
///   puts it straight back to work; VERA's keeps area-guarding. Frequency:
///   invisible today — the miner exclusion at the head of
///   [`dispatch_supported_foot_mission_cadence`] keeps any miner not on Guard
///   out of the handler, and the only route onto Area Guard is the Move-arrival
///   promotion that is itself a recorded residual. Downstream risk is why it is
///   written down: the `(1, 10) + 1` return is an RNG-consumption difference
///   that lands the moment either of those two closes. `+0xE0E` is UNCHECKED.
/// - **the tank-bunker adjacency scan** at 0x004D6F44, byte-for-byte the block
///   recorded on [`evaluate_foot_guard_cadence`].
/// - **the infantry idle action.** With still no target the handler calls
///   `[vtable+0x478]` at 0x004D6F25 = `InfantryClass::UpdateIdleAction`
///   0x0051CDB0, which fidgets the facing, can play an idle `VocClass` line, and
///   draws `RandomRanged(0, 0x7FFFFFFE)` plus up to three more values. `Mission_Guard`
///   calls it from the same no-target position. VERA has no infantry idle-action
///   system. Trigger: any idle infantryman on Guard, Sticky or Area Guard.
///   Player effect: retail infantry look around and shuffle while standing
///   guard; VERA's stand still. Frequency: continuous, every idle infantryman
///   in every match. Downstream risk: those draws are scenario-RNG consumption
///   VERA does not make, so the streams cannot be compared frame-for-frame
///   until an idle-action system lands.
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

/// `FootClass::Mission_Guard` 0x004D5070, the body Guard(5) and Sticky(6) share.
///
/// **Shared for units. Infantry reach it only through a leaf override VERA does
/// not model, recorded here.** `InfantryClass`'s slot `+0x21C` is `0x0051F620`,
/// nine instructions: `CALL 0x00521320; CMP EAX,-1; JNZ` returns that value
/// directly, and only `-1` falls through to the Foot body. `0x00521320` owns
/// infantry deploy and undeploy on Guard; its arms return an animation-table
/// duration (`Type+0xE3C` plus `+0x460`/`+0x3D0`/`+0x418`), or `Rate +
/// RandomRanged(0, 2)`, or — on the radiation arm gated by `Type+0xD37`,
/// Desolator-shaped — `Rate + RandomRanged(10, 0x14)`, a draw VERA never makes.
/// Trigger: any infantryman on Guard or Sticky whose type can deploy.
/// Player effect: a deploying GI, Guardian GI or Desolator re-dispatches on the
/// Foot cadence instead of its animation's own length. Frequency: continuous
/// wherever deployed infantry hold ground, which is ordinary play for both
/// Allied and Soviet. Downstream risk: it consumes scenario RNG on the
/// radiation arm, so the stream diverges as well as the cadence.
///
/// The Sticky half of the shared-slot claim is confirmed:
/// `MissionClass::GetMissionTimerEntry` @ `0x005B3A00` is
/// `&MissionControl + *(this+0xAC) * 8`, indexed on the **committed** mission —
/// the same field the dispatcher switches on — so `[Sticky] Rate` reaches the
/// shared body exactly as the arm below assumes, and nothing else in the binary
/// distinguishes Sticky from Guard.
///
/// `mission` is the object's OWN committed selector, not the handler's: the
/// native timer lookup indexes the control table on the committed mission id,
/// so the same handler re-arms a Guard object at `[Guard] Rate` and a Sticky
/// object at `[Sticky] Rate`.
///
/// **The handler performs no target acquisition.** Its only no-target tail call
/// is `[vtable+0x478]` (0x004D51A6 and 0x004D51D8), which is the idle-action
/// virtual — `InfantryClass::UpdateIdleAction` 0x0051CDB0 for infantry and the
/// `XOR AL,AL; RET` stub 0x0041C040 for units — not the scanner. The scanner
/// slot is `+0x39C` (0x00709820), which this handler never calls, so a guarding
/// object acquires solely through the common AI body's block. VERA matches.
///
/// **Cadence.** Bunker delegation returns the base cadence with no draw; the
/// ordinary path is `[Rate] + RandomRanged(0, 2)` (0x004D532F).
///
/// Deliberately NOT represented, recorded:
/// - **the whole target-present arm** (0x004D51E0-0x004D5225). With `[this+0x2B4]`
///   holding a target the handler skips both the bunker scan and the idle action
///   and instead does `if (GetTechnoType()->[+0x390] && ObjectClass::GetHeight()
///   0x005F5F40 == 0) TechnoClass::Set_Destination 0x00741970
///   (FootClass::Find_Nearby_Passable_Cell(...), 1)`. That is a **destination
///   write**, not a cadence value, so it is a behaviour contract rather than a
///   timing detail. Trigger: every Guard dispatch of a grounded object that
///   already holds a target and whose type carries `+0x390`. Player effect:
///   retail shuffles such a guard onto a nearby passable cell while it engages;
///   VERA's stands still. Frequency: the arm itself is entered continuously
///   during any firefight, so until `+0x390` is named — its identity is
///   UNCHECKED — the frequency cannot honestly be called low. Downstream risk:
///   a destination write from the mission handler crosses into movement's
///   ownership, so closing it needs the movement owner in the loop.
/// - **the AI-only Sabotage queue** (0x004D523A-0x004D52A9): for an
///   `InfantryClass` (`What_Am_I()` 0xF) whose house is not human-controlled
///   (0x0050B730 on `[this+0x21C]`), that either carries type byte `+0xEC2` or
///   passes `TechnoClass::HasWeaponAbility(0xE)` 0x0070D0D0, whose current
///   mission is not already Sabotage, and whose target is a `BuildingClass`,
///   `Queue_Mission(0x11, 0)`. Trigger: an AI demolition infantryman standing
///   guard that picks up an enemy building. Player effect: retail's walks in and
///   plants; VERA's shoots it instead. Frequency: **zero today** — the arm is
///   gated on a non-human house and VERA has no AI opponent — and continuous
///   once one lands, which is why it is recorded rather than left unwritten.
///   `+0xEC2` is UNCHECKED.
/// - **the two further containment latches.** The head takes three byte
///   latches in order — `+0x68F` → `[vtable+0x340]` (0x004DFB70), `+0x690` →
///   `[+0x348]`, `+0x691` → `[+0x34C]` — and each delegates, discards the
///   result and returns the flat `ftol(Rate × 900)` with no jitter draw
///   (0x004D5076-0x004D50CD). VERA models the first as `bunker_delegate`; the
///   identity of the other two is UNCHECKED, and VERA carries no state that
///   could be their equivalent, so there is nothing to gate on. The identical
///   three-way head opens `FootClass::Mission_AreaGuard` at 0x004D6ACC, so this
///   is a FootClass-wide containment cluster, not a bunker special case.
///   Trigger: whichever containment those two bytes denote. Player effect:
///   cadence only — a contained object re-dispatches on the flat rate instead
///   of rate-plus-jitter. Frequency: unknown until the bytes are identified,
///   which is why this cannot be closed by guessing. Downstream risk: one
///   scenario-RNG draw per dispatch on those paths.
/// - **the tank-bunker adjacency scan** (0x004D5116-0x004D51D2, repeated
///   verbatim in Mission_AreaGuard at 0x004D6F44). Behind
///   `TechnoClass::GetWeapon(1)` 0x0070E140 returning a live weapon whose
///   warhead byte `+0x158` is set, the handler walks the eight-neighbour offset
///   table at 0x0089F688 for a building whose type byte `+0x1575` is set and
///   whose owner matches its own, then `Assign_Target` (`[+0x3C8]`), sets
///   `+0x68E` and `Queue_Mission(1, 0)`. Trigger: a unit sitting on Guard in a
///   cell orthogonally or diagonally adjacent to a tank bunker its own house
///   owns. Player effect: retail garrisons the bunker by itself; VERA's unit
///   stays outside. Frequency: near zero in ordinary skirmish — tank bunkers
///   are pre-placed map objects on a minority of stock maps and are rarely
///   captured. Downstream risk: none; `bunker_link` already exists, but the two
///   gating flags (`+0x158` on the warhead, `+0x1575` on the building type) are
///   UNCHECKED and would have to be resolved to INI keys first.
/// - **the two cadence-tail short-circuits ahead of the jitter draw**
///   (0x004D52A9-0x004D5341). First, a three-dword object timer at `+0x2EC`
///   (start) / `+0x2F4` (delay): while it is live the handler returns its own
///   remaining frames and draws NO RNG. `TechnoClass::Constructor` 0x006F2E86
///   leaves it start=now, delay=0 — already expired — so the gate is inert
///   until something arms it; the one arming site found is 0x007464AB, in code
///   Ghidra has not bounded into a function, and the timer's role is UNCHECKED.
///   Second, past that gate, `GetTechnoType()->[+0x6B0]` — the **`DistributedFire`**
///   bool, key string 0x00843A64, read by `TechnoTypeClass::ReadINI` at
///   0x00714850/0x00714864 — combined with the object counter `+0x468 > 0`
///   returns **0**, re-dispatching on the next frame and again drawing nothing.
///   Trigger: a `DistributedFire` type on Guard with a live spread-fire count.
///   Player effect: it re-evaluates every frame instead of every ~26. Frequency:
///   the Aegis Cruiser is the only stock `DistributedFire` type, so naval maps
///   with an Allied player only. Downstream risk: VERA implements no
///   distributed-fire mechanism at all (recorded at `passive_target_scan`), so
///   the counter this gate reads has no VERA counterpart to bind to.
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

/// Whether this dispatch takes `FootClass::Mission_Attack`'s halved cadence.
///
/// gamemd-derived: `FootClass::Mission_Attack @ 0x004D4DC0` halves its return
/// only when a target is installed AND the attacker qualifies by TYPE AND the
/// 2D distance falls in the close band. The type half is
/// `(What_Am_I() == 0xF && InfantryType->CloseRange) || primaryWeapon.Range <
/// 0x201` — an infantry type carrying `CloseRange=`, or any type whose primary
/// reaches under 513 leptons.
///
/// That gate was missing, which is the whole point of this function's rewrite:
/// without it every tank, rifle infantryman and artillery piece ran the halved
/// cadence whenever it closed to 1.1–3 cells. 22 of the 93 stock vehicle and
/// infantry types have a short enough primary; the other 71 must return the
/// full cadence. Both branches draw the same jitter, so the cost is not an
/// extra draw — it is that the Attack dispatch, and the scenario-RNG jitter it
/// consumes, ran at roughly double the native rate through every close-quarters
/// engagement.
///
/// UNCHECKED: the band's exact boundaries. The `282..=768` pair below is
/// inherited from the earlier survey that wrote this function; a later reading
/// put it at `[281.6, 769)`, which disagrees at both ends, and neither reading
/// carries an address. Left as found rather than moved on prose.
fn foot_attack_in_half_cadence_band(sim: &Simulation, rules: &RuleSet, id: u64) -> bool {
    const MIN_LEPTONS: i64 = 282;
    const MAX_LEPTONS: i64 = 768;
    /// `CMP ..., 0x201` — the primary-weapon reach below which any type
    /// qualifies, in leptons.
    const CLOSE_PRIMARY_RANGE_LEPTONS: i64 = 0x201;

    let Some(attacker) = sim.substrate.entities.get(id) else {
        return false;
    };
    if !foot_attack_type_takes_half_cadence(sim, rules, attacker, CLOSE_PRIMARY_RANGE_LEPTONS) {
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

/// The type half of the halved-cadence gate.
///
/// `What_Am_I() == 0xF` is InfantryClass, so `CloseRange=` only qualifies an
/// infantry type — a vehicle carrying the key would NOT take the short path in
/// native, and does not here.
fn foot_attack_type_takes_half_cadence(
    sim: &Simulation,
    rules: &RuleSet,
    attacker: &crate::sim::game_entity::GameEntity,
    close_primary_range_leptons: i64,
) -> bool {
    let Some(object) = rules.object(sim.interner.resolve(attacker.type_ref)) else {
        return false;
    };
    if attacker.category == EntityCategory::Infantry && object.close_range {
        return true;
    }
    let Some(primary) = object
        .primary
        .as_deref()
        .and_then(|name| rules.weapon(name))
    else {
        return false;
    };
    (primary.range * crate::util::fixed_math::SimFixed::from_num(256)).to_num::<i64>()
        < close_primary_range_leptons
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
