//! Building docking system — repair depot (`UnitRepair=yes`) admission,
//! waiting, service and release.
//!
//! Native shape (gamemd.exe, all read this task):
//!
//! - **Admission is contact-slot capacity, no stored queue.** The depot's
//!   `RadioClass` contact array is sized once at construction:
//!   `BuildingClass::Constructor` 0x0043BCBD reads `Type+0x1780`
//!   (`NumberOfDocks=`), clamps `< 1 ⇒ 1` and calls `Set_Contact_Count`
//!   (0x0043BCD0). Nothing in the binary keeps a waiting list.
//! - **Every waiter re-probes itself.** `FootClass::Mission_Enter` 0x004D9290
//!   sends `0x0E` to `Contacts[0]` (or to the archive target `+0x218`,
//!   0x004D929F) on every dispatch, then re-arms
//!   `ftol([Enter] Rate * 900) + RandomRanged(0, 2)` (0x004D946C..0x004D9497,
//!   Scenario stream). Whichever waiter's dispatch lands first after the pad
//!   frees wins the slot.
//! - **`BuildingClass::Receive_Radio(0x0E)` 0x0043C2D0 for a UnitRepair
//!   building** (0x0043C7E9..): `+0x660` power flag off ⇒ 10; already linked
//!   AND `Transmit(0x22)` == 10 (0x0043C824..C842) ⇒ 10; the Hospital/Armory
//!   branch (`Type+0x16C1/+0x16C2`, 0x0043CB0C) is NOT taken, so a depot never
//!   evicts a repaired occupant on a waiter's probe; not linked and
//!   `Has_Free_Or_Own_Contact_Slot` 0x0065ADF0 ⇒ the building HELLOs the sender
//!   (0x0043C8B0..C8C3); then `0x13` and return 1 (0x0043C9F5..CA37). The
//!   waiter therefore stays in Enter and keeps probing; there is no `0x12`
//!   move assignment for depots — the unit's own player-order NavCom drives it.
//! - **`ObjectClass::Receive_Radio(0x22)` 0x005F5320**: health ratio ≥
//!   `Rules+0x16F8` ⇒ 10 else 1. `Rules+0x16F8` is not an INI key:
//!   `RulesClass::ReadAudioVisual` 0x0066B323/0x0066B32D stores the double 1.0
//!   unconditionally, so "repaired" ⇔ `hp >= max_hp`.
//! - **Reply 10 to the waiter**: `Mission_Enter` 0x004D92D0 sends BREAK and
//!   calls `Enter_Idle_Mode` (+0x484 = 0x00738970 → Guard for a plain unit).
//! - **Release of a repaired occupant** is the building's repair mission
//!   (`BuildingClass::MissionRepairAndProduce` 0x0044B780, 0x0044C2AE..C4B0 and
//!   0x0044BD5E..): on `0x1C` reply 0x21 the occupant gets `Queue_Mission(Move)`
//!   + `Set_Destination(exit cell)` + BREAK. The exit cell is
//!   `BuildingClass::GetDockCellForObject` 0x0044EFB0: the foundation exit list
//!   (`Type+0xED4`, initializer 0x0045C300) walked in order until the unit can
//!   enter the cell. Scatter (`FootClass::Receive_Radio 0x17`) is only reached
//!   from the Hospital/Armory loop and is not part of the depot flow.
//!
//! The repair step itself (Techno `0x1C`, cost via the type vtable) is an
//! adjacent lane: `repair_tick` keeps the pre-existing VERA-internal cost math
//! (gamemd equivalent UNCHECKED). Native never ejects for insufficient funds
//! (0x20 ⇒ state 1 and retry); the `NO_FUNDS_GRACE_TICKS` eject below is
//! VERA-internal drift retained from the previous FSM.
//!
//! ## Dependency rules
//! - Part of sim/ — depends on rules/, sim/radio, sim/movement, sim/mission.
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

use crate::rules::ruleset::RuleSet;
use crate::sim::intern::InternedId;
use crate::sim::mission::authority::EntityReadyInputProvider;
use crate::sim::mission::timer::MissionTimer;
use crate::sim::mission::{MissionId, MissionType};
use crate::sim::movement;
use crate::sim::movement::locomotor::MovementLayer;
use crate::sim::pathfinding::PathGrid;
use crate::sim::radio::{self, RadioMessage, RadioPayload, RadioResponse};
use crate::sim::world::Simulation;
use crate::util::fixed_math::ra2_speed_to_leptons_per_second;

use crate::sim::production::foundation_dimensions;

/// Dock state machine phase for a unit interacting with a repair depot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DockPhase {
    /// Driving toward the depot on the player's order; not yet a contact.
    Approach,
    /// Stopped short of the pad (occupied), not yet a contact; re-probing on
    /// the `[Enter]` cadence.
    WaitForDock,
    /// Holds a contact slot, moving onto the exact dock cell.
    EnterDock,
    /// On the dock pad, receiving repair (HP restored, credits deducted).
    Servicing,
    /// Repaired or out of funds — being released off the pad.
    ExitDock,
}

/// Per-entity docking state, stored as `Option<DockState>` on `GameEntity`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DockState {
    /// StableEntityId of the target repair depot building.
    pub dock_building_id: u64,
    /// Current phase of the dock state machine.
    pub phase: DockPhase,
    /// Ticks remaining until the next repair step fires.
    pub service_timer: u32,
    /// Consecutive ticks with insufficient credits (triggers exit after grace).
    pub no_funds_ticks: u32,
    /// `Mission_Enter` dispatch cadence: the next `0x0E` probe is due when this
    /// expires (`ftol([Enter] Rate*900) + RandomRanged(0,2)`, 0x004D946C).
    /// Unarmed ⇒ due, so the first probe fires on the first tick after the
    /// order, like the freshly assigned Enter mission's zero dispatch delay.
    #[serde(default)]
    pub enter_retry: MissionTimer,
}

impl DockState {
    /// A fresh player-ordered depot entry: Approach, no timers.
    pub fn approach(dock_building_id: u64) -> Self {
        Self {
            dock_building_id,
            phase: DockPhase::Approach,
            service_timer: 0,
            no_funds_ticks: 0,
            enter_retry: MissionTimer::default(),
        }
    }
}

/// Grace period in ticks before a docked unit exits due to insufficient funds.
/// ~2 seconds at 15 Hz. VERA-internal (native retries without ejecting).
const NO_FUNDS_GRACE_TICKS: u32 = 30;

/// `[Enter] Rate=.016` ⇒ `ftol(0.016 * 900)` = 14 (stock); used only when the
/// mission table carries no `[Enter]` rate.
const ENTER_RETRY_BASE_FRAMES: u32 = 14;
/// `RandomRanged(0, 2)` jitter added to every `Mission_Enter` epilogue.
const ENTER_RETRY_JITTER_MAX_FRAMES: u32 = 2;

/// Outcome of one repair-depot service step — the depot's `REPAIR_TICK`
/// trichotomy. Carries the per-step payload the caller applies, so the money/
/// heal math lives in one pure place (`repair_tick`) instead of inline in the
/// dock FSM. Maps to the dock-bus [`RadioResponse`] code via [`Self::radio_response`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepairResponse {
    /// A repair step fired: heal `heal` HP and deduct `cost` credits.
    Roger { heal: u16, cost: i32 },
    /// Not enough credits for this step. `grace` is the incremented no-funds
    /// counter; the caller exits the dock once it reaches [`NO_FUNDS_GRACE_TICKS`].
    InsufficientFunds { grace: u32 },
    /// Fully repaired — exit the dock.
    RepairComplete,
}

impl RepairResponse {
    /// The `RadioClass` response code this maps to on the dock bus.
    pub fn radio_response(self) -> RadioResponse {
        match self {
            RepairResponse::Roger { .. } => RadioResponse::Roger,
            RepairResponse::InsufficientFunds { .. } => RadioResponse::InsufficientFunds,
            RepairResponse::RepairComplete => RadioResponse::RepairComplete,
        }
    }
}

/// Decide one repair-depot service step (the `REPAIR_TICK` trichotomy). Pure
/// integer math — byte-identical to the inline `Servicing` arm it replaces:
/// `total = cost * repair_percent / 100`, `cost_per_step = max(1, total *
/// repair_step / max_hp)`, funded ⇒ `Roger`, unfunded ⇒ `InsufficientFunds`
/// (grace incremented), already-full ⇒ `RepairComplete`. No clock/RNG/float.
/// VERA-internal cost math, gamemd equivalent UNCHECKED (Techno `0x1C` reads
/// the type vtable `+0xB0/+0xB4`; adjacent lane).
pub fn repair_tick(
    hp: u16,
    max_hp: u16,
    unit_cost: i32,
    repair_percent: u16,
    repair_step: u16,
    credits: i32,
    no_funds_ticks: u32,
) -> RepairResponse {
    if hp >= max_hp {
        return RepairResponse::RepairComplete;
    }
    let total_repair_cost = (unit_cost as i64 * repair_percent as i64 / 100) as i32;
    let cost_per_step = if max_hp > 0 {
        (total_repair_cost as i64 * repair_step as i64 / max_hp as i64).max(1) as i32
    } else {
        1
    };
    if credits >= cost_per_step {
        RepairResponse::Roger {
            heal: repair_step,
            cost: cost_per_step,
        }
    } else {
        RepairResponse::InsufficientFunds {
            grace: no_funds_ticks + 1,
        }
    }
}

/// Compute the dock cell (center of foundation) for a building.
///
/// `GetDockCoord` for `UnitRepair` with `NumberOfDocks == 1` is the building
/// coord plus `DockingOffset0`: GADEPT 3x3 (no offset) ⇒ centre; NADEPT 4x3
/// `DockingOffset0=128,0,0` ⇒ origin + (2, 1). Both equal `w/2, h/2`.
pub fn depot_dock_cell(building_rx: u16, building_ry: u16, foundation: &str) -> (u16, u16) {
    let (w, h) = foundation_dimensions(foundation);
    (building_rx + w / 2, building_ry + h / 2)
}

/// The foundation exit list `Type+0xED4` (`0x0089D368 + foundation_id * 0x78`),
/// as written by the static initializer at 0x0045C300 (decoded by simulating
/// its straight-line stores). Relative to the building's NW cell, terminated
/// by `(0x7FFF, 0x7FFF)` in the binary.
///
/// - `1x1` (id 0, row 0x0089D368): S, SW, SE, W, E, N, NW, NE.
/// - `3x3` (id 6, row 0x0089D638): south row, S corners, W/E columns, north
///   row, N corners.
/// - `4x3` (id 12, row 0x0089D908): byte-identical to the `3x3` row — the
///   binary reuses the 3x3 list, so `(3, y)` entries fall inside a 4x3
///   footprint and are skipped by the enter check.
///
/// Other foundations: generated in the same ring order from their own
/// width/height — VERA-internal, gamemd rows UNCHECKED (no stock `UnitRepair`
/// building uses them).
pub fn foundation_exit_list(foundation: &str) -> Vec<(i32, i32)> {
    let def = crate::rules::foundation::foundation_def(foundation);
    match def.id {
        0 => vec![
            (0, 1),
            (-1, 1),
            (1, 1),
            (-1, 0),
            (1, 0),
            (0, -1),
            (-1, -1),
            (1, -1),
        ],
        6 | 12 => ring_exit_list(3, 3),
        _ => ring_exit_list(i32::from(def.width), i32::from(def.height)),
    }
}

fn ring_exit_list(w: i32, h: i32) -> Vec<(i32, i32)> {
    let mut out = Vec::with_capacity(((w + 2) * (h + 2) - w * h) as usize);
    for x in 0..w {
        out.push((x, h));
    }
    out.push((-1, h));
    out.push((w, h));
    for y in (0..h).rev() {
        out.push((-1, y));
        out.push((w, y));
    }
    for x in 0..w {
        out.push((x, -1));
    }
    out.push((-1, -1));
    out.push((w, -1));
    out
}

/// `BuildingClass::GetDockCellForObject` 0x0044EFB0 for a depot: the first
/// exit-list cell the unit can enter (in bounds, walkable, no vehicle/building
/// occupant on the ground layer). Native asks the unit's `vtable+0x1AC`
/// (cell, -1, -1, 0, 0); VERA stands the grid + occupancy test in for it
/// (infantry-only occupants are not rejected — UNCHECKED against native).
pub fn depot_exit_cell(
    sim: &Simulation,
    path_grid: Option<&PathGrid>,
    building_rx: u16,
    building_ry: u16,
    foundation: &str,
) -> Option<(u16, u16)> {
    for (dx, dy) in foundation_exit_list(foundation) {
        let x = i32::from(building_rx) + dx;
        let y = i32::from(building_ry) + dy;
        if x < 0 || y < 0 || x > i32::from(u16::MAX) || y > i32::from(u16::MAX) {
            continue;
        }
        let (x, y) = (x as u16, y as u16);
        if let Some(grid) = path_grid {
            if x >= grid.width() || y >= grid.height() || !grid.is_walkable(x, y) {
                continue;
            }
        }
        let blocked = sim
            .substrate
            .occupancy
            .get(x, y)
            .is_some_and(|cell| cell.has_blockers_on(MovementLayer::Ground));
        if blocked {
            continue;
        }
        return Some((x, y));
    }
    None
}

/// Manhattan distance between two cell coordinates.
fn cell_distance(ax: u16, ay: u16, bx: u16, by: u16) -> u32 {
    let dx = (ax as i32 - bx as i32).unsigned_abs();
    let dy = (ay as i32 - by as i32).unsigned_abs();
    dx.max(dy)
}

/// Arm the `Mission_Enter` epilogue: `ftol([Enter] Rate*900)` first (no RNG),
/// then one `RandomRanged(0, 2)` on the Scenario stream (0x004D9483..0x004D9497),
/// summed. Same draw site and order as the miner's `schedule_enter_retry`.
fn arm_enter_retry(sim: &mut Simulation, rules: &RuleSet, timer: &mut MissionTimer) {
    let base = match rules.mission_control.rate_frames(MissionType::Enter) {
        0 => ENTER_RETRY_BASE_FRAMES,
        frames => frames,
    };
    let jitter = sim
        .miner_jitter_rng()
        .next_range_u32_inclusive(0, ENTER_RETRY_JITTER_MAX_FRAMES);
    timer.arm(sim.session.binary_frame, base.saturating_add(jitter));
}

/// Drive the unit straight onto/off the pad (footprint cells are not grid
/// walkable, so bypass the grid like the refinery pad entry does).
fn issue_pad_move(sim: &mut Simulation, rules: &RuleSet, id: u64, target: (u16, u16)) {
    let speed = sim
        .resolve_move_info(id, Some(rules))
        .map(|info| info.speed)
        .unwrap_or_else(|| ra2_speed_to_leptons_per_second(4));
    if movement::issue_direct_move(&mut sim.substrate.entities, id, target, speed) {
        if let Some(target) = sim
            .substrate
            .entities
            .get_mut(id)
            .and_then(|entity| entity.movement_target.as_mut())
        {
            target.bypass_grid = true;
        }
    }
}

/// BREAK the unit↔depot link over the bus (both slots cleared). No-op when the
/// unit holds no contact with the depot.
fn break_depot_contact(sim: &mut Simulation, unit_id: u64, depot_id: u64) {
    let linked = sim
        .substrate
        .entities
        .get(unit_id)
        .is_some_and(|unit| unit.radio_contacts.contains(depot_id));
    if linked {
        let _ = radio::transmit(
            sim,
            unit_id,
            depot_id,
            RadioMessage::Break,
            RadioPayload::default(),
        );
    }
}

fn queue_mission(sim: &mut Simulation, id: u64, mission: MissionType) {
    let now = sim.session.binary_frame;
    let _ = sim.mission_queue_exact(
        id,
        MissionId::from_known(mission),
        0,
        now,
        &EntityReadyInputProvider,
    );
}

/// Advance building dock state machines for all entities with `dock_state`.
///
/// Called once per tick from `advance_tick()`, after `tick_repairs()`.
/// Uses the two-phase snapshot pattern to avoid borrow conflicts.
pub fn tick_building_docks(sim: &mut Simulation, rules: &RuleSet, path_grid: Option<&PathGrid>) {
    struct DockSnapshot {
        id: u64,
        owner: InternedId,
        type_ref: InternedId,
        rx: u16,
        ry: u16,
        hp: u16,
        max_hp: u16,
        moving: bool,
        dock_building_id: u64,
        phase: DockPhase,
        service_timer: u32,
        no_funds_ticks: u32,
        enter_retry: MissionTimer,
    }

    let snapshots: Vec<DockSnapshot> = sim
        .substrate
        .entities
        .values()
        .filter_map(|e| {
            let ds = e.dock_state.as_ref()?;
            Some(DockSnapshot {
                id: e.stable_id,
                owner: e.owner,
                type_ref: e.type_ref,
                rx: e.position.rx,
                ry: e.position.ry,
                hp: e.health.current,
                max_hp: e.health.max,
                moving: e.movement_target.is_some(),
                dock_building_id: ds.dock_building_id,
                phase: ds.phase,
                service_timer: ds.service_timer,
                no_funds_ticks: ds.no_funds_ticks,
                enter_retry: ds.enter_retry,
            })
        })
        .collect();

    if snapshots.is_empty() {
        return;
    }

    struct DockMutation {
        id: u64,
        new_phase: Option<DockPhase>,
        new_timer: Option<u32>,
        new_no_funds: Option<u32>,
        new_enter_retry: Option<MissionTimer>,
        heal_amount: u16,
        deduct_credits: i32,
        clear_dock: bool,
        clear_movement: bool,
    }

    let mut mutations: Vec<DockMutation> = Vec::new();

    for snap in &snapshots {
        let mut m = DockMutation {
            id: snap.id,
            new_phase: None,
            new_timer: None,
            new_no_funds: None,
            new_enter_retry: None,
            heal_amount: 0,
            deduct_credits: 0,
            clear_dock: false,
            clear_movement: false,
        };

        // Verify depot still exists and is alive/friendly.
        //
        // DRIFT (VERA-internal, gamemd equivalent UNCHECKED beyond the gate):
        // `BuildingClass::Receive_Radio(0x0E)` @ 0x0043C2D0 answers 10 to a
        // probe while the building is offline (`+0x660 == 0`, test at
        // 0x0043C7FB), so a linked waiter BREAKs and idles during low power.
        // VERA has no per-building online latch on entities, so a depot keeps
        // admitting during low power. Trigger: player low-power while units
        // queue at a depot; effect: repairs continue where gamemd pauses them.
        let depot_info = sim
            .substrate
            .entities
            .get(snap.dock_building_id)
            .and_then(|depot| {
                if depot.health.current == 0 || depot.dying {
                    return None;
                }
                if depot.owner != snap.owner {
                    return None;
                }
                let obj = sim.object_type(depot.type_ref, rules)?;
                if !obj.unit_repair {
                    return None;
                }
                Some((
                    depot.position.rx,
                    depot.position.ry,
                    obj.foundation.clone(),
                    usize::from(obj.number_of_docks.max(1)),
                ))
            });

        let Some((depot_rx, depot_ry, foundation, dock_capacity)) = depot_info else {
            // Depot gone or invalid — abort docking.
            break_depot_contact(sim, snap.id, snap.dock_building_id);
            m.clear_dock = true;
            mutations.push(m);
            continue;
        };

        let (dock_rx, dock_ry) = depot_dock_cell(depot_rx, depot_ry, &foundation);
        let dist = cell_distance(snap.rx, snap.ry, dock_rx, dock_ry);

        match snap.phase {
            DockPhase::Approach | DockPhase::WaitForDock | DockPhase::EnterDock => {
                let mut linked = sim
                    .substrate
                    .entities
                    .get(snap.id)
                    .is_some_and(|unit| unit.radio_contacts.contains(snap.dock_building_id));

                // Mission_Enter dispatch: one 0x0E probe per cadence window.
                if snap.enter_retry.due(sim.session.binary_frame) {
                    if linked && snap.hp >= snap.max_hp {
                        // 0x0043C824..C842: linked sender whose 0x22 answers 10
                        // (ratio >= 1.0) gets 10 back; Mission_Enter 0x004D92D0
                        // then BREAKs and calls Enter_Idle_Mode (Guard for a
                        // plain unit, 0x00738970). No re-arm — the unit leaves
                        // the Enter mission.
                        break_depot_contact(sim, snap.id, snap.dock_building_id);
                        queue_mission(sim, snap.id, MissionType::Guard);
                        m.clear_dock = true;
                        m.clear_movement = true;
                        mutations.push(m);
                        continue;
                    }
                    if !linked {
                        // 0x0043C8A4..C8C3: not a contact and a slot is free or
                        // own ⇒ the building HELLOs the sender. VERA sends the
                        // HELLO unit→depot; the linked end state (both slots)
                        // is identical. Capacity is the ctor's
                        // max(NumberOfDocks, 1) (0x0043BCBD..BCD0), sized
                        // grow-only here rather than at spawn.
                        if let Some(depot) = sim.substrate.entities.get_mut(snap.dock_building_id) {
                            depot.radio_contacts.set_capacity(dock_capacity);
                        }
                        let reply = radio::transmit(
                            sim,
                            snap.id,
                            snap.dock_building_id,
                            RadioMessage::Hello,
                            RadioPayload::default(),
                        );
                        linked = reply == RadioResponse::Roger;
                    }
                    // Epilogue: ftol(Rate*900) + RandomRanged(0,2), every dispatch.
                    let mut timer = snap.enter_retry;
                    arm_enter_retry(sim, rules, &mut timer);
                    m.new_enter_retry = Some(timer);
                }

                if linked {
                    if dist == 0 {
                        // Pad arrival (PerCellProcess 0x15 ⇒ building repair
                        // mission). Native also queues Sleep on the unit; the
                        // exact mission stays represented as Enter here.
                        m.clear_movement = true;
                        m.new_phase = Some(DockPhase::Servicing);
                        m.new_timer = Some(rules.general.unit_repair_rate_ticks);
                    } else {
                        if !snap.moving {
                            // The player-order NavCom keeps driving natively;
                            // VERA re-issues the pad move once the slot is
                            // held (VERA-internal stand-in, UNCHECKED).
                            issue_pad_move(sim, rules, snap.id, (dock_rx, dock_ry));
                        }
                        if snap.phase != DockPhase::EnterDock {
                            m.new_phase = Some(DockPhase::EnterDock);
                        }
                    }
                } else if snap.phase == DockPhase::Approach && !snap.moving {
                    m.new_phase = Some(DockPhase::WaitForDock);
                } else if snap.phase == DockPhase::EnterDock {
                    // Link lost (depot BREAK); fall back to waiting.
                    m.new_phase = Some(DockPhase::WaitForDock);
                }
            }
            DockPhase::Servicing => {
                if snap.hp >= snap.max_hp {
                    m.new_phase = Some(DockPhase::ExitDock);
                } else {
                    let timer = snap.service_timer.saturating_sub(1);
                    if timer == 0 {
                        // A repair step is due — resolve the REPAIR_TICK trichotomy.
                        let unit_cost = sim
                            .object_type(snap.type_ref, rules)
                            .map(|obj| obj.cost)
                            .unwrap_or(0);
                        let credits = crate::sim::house_state::house_state_for_owner(
                            &sim.houses,
                            sim.interner.resolve(snap.owner),
                            &sim.interner,
                        )
                        .map(|h| h.credits)
                        .unwrap_or(0);

                        match repair_tick(
                            snap.hp,
                            snap.max_hp,
                            unit_cost,
                            rules.general.repair_percent,
                            rules.general.repair_step,
                            credits,
                            snap.no_funds_ticks,
                        ) {
                            RepairResponse::Roger { heal, cost } => {
                                m.heal_amount = heal;
                                m.deduct_credits = cost;
                                m.new_no_funds = Some(0);
                            }
                            RepairResponse::InsufficientFunds { grace } => {
                                if grace >= NO_FUNDS_GRACE_TICKS {
                                    m.new_phase = Some(DockPhase::ExitDock);
                                } else {
                                    m.new_no_funds = Some(grace);
                                }
                            }
                            RepairResponse::RepairComplete => {
                                m.new_phase = Some(DockPhase::ExitDock);
                            }
                        }
                        m.new_timer = Some(rules.general.unit_repair_rate_ticks);
                    } else {
                        m.new_timer = Some(timer);
                    }
                }
            }
            DockPhase::ExitDock => {
                // MissionRepairAndProduce 0x0044C4B0..: Queue_Mission(Move) +
                // Set_Destination(GetDockCellForObject) + BREAK, only when an
                // exit cell exists (0x0044C48E: an invalid cell leaves the unit
                // linked on the pad and the building retries).
                match depot_exit_cell(sim, path_grid, depot_rx, depot_ry, &foundation) {
                    Some(exit) => {
                        queue_mission(sim, snap.id, MissionType::Move);
                        issue_pad_move(sim, rules, snap.id, exit);
                        break_depot_contact(sim, snap.id, snap.dock_building_id);
                        m.clear_dock = true;
                    }
                    None => {}
                }
            }
        }

        mutations.push(m);
    }

    // Apply mutations.
    for m in &mutations {
        let Some(entity) = sim.substrate.entities.get_mut(m.id) else {
            continue;
        };

        if m.clear_movement {
            entity.movement_target = None;
        }

        if m.clear_dock {
            entity.dock_state = None;
            continue;
        }

        if let Some(ref mut ds) = entity.dock_state {
            if let Some(phase) = m.new_phase {
                ds.phase = phase;
            }
            if let Some(timer) = m.new_timer {
                ds.service_timer = timer;
            }
            if let Some(nf) = m.new_no_funds {
                ds.no_funds_ticks = nf;
            }
            if let Some(retry) = m.new_enter_retry {
                ds.enter_retry = retry;
            }
        }

        if m.heal_amount > 0 {
            entity.health.current = (entity.health.current + m.heal_amount).min(entity.health.max);
            // Service depots normally repair units/aircraft; this remains a no-op
            // for them and protects the gate if a structure ever reaches this path.
            entity.refresh_building_damage_state_gate(rules.general.condition_yellow_x1000);
        }

        if m.deduct_credits > 0 {
            if let Some(house) = crate::sim::house_state::house_state_for_owner_mut(
                &mut sim.houses,
                sim.interner.resolve(entity.owner),
                &sim.interner,
            ) {
                house.credits = (house.credits - m.deduct_credits).max(0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::entities::EntityCategory;
    use crate::rules::ini_parser::IniFile;
    use crate::sim::command::Command;
    use crate::sim::components::Health;
    use crate::sim::game_entity::GameEntity;
    use crate::sim::occupancy::CellListInsertion;
    use std::collections::BTreeMap;

    #[test]
    fn dock_cell_for_3x3_foundation() {
        let (rx, ry) = depot_dock_cell(10, 20, "3x3");
        assert_eq!((rx, ry), (11, 21));
    }

    #[test]
    fn dock_cell_for_4x3_foundation() {
        // NADEPT: DockingOffset0=128,0,0 ⇒ origin + (2, 1).
        assert_eq!(depot_dock_cell(10, 20, "4x3"), (12, 21));
    }

    #[test]
    fn dock_cell_for_2x2_foundation() {
        let (rx, ry) = depot_dock_cell(10, 20, "2x2");
        assert_eq!((rx, ry), (11, 21));
    }

    #[test]
    fn dock_cell_for_1x1_foundation() {
        let (rx, ry) = depot_dock_cell(10, 20, "1x1");
        assert_eq!((rx, ry), (10, 20));
    }

    #[test]
    fn cell_distance_same() {
        assert_eq!(cell_distance(5, 5, 5, 5), 0);
    }

    #[test]
    fn cell_distance_diagonal() {
        assert_eq!(cell_distance(5, 5, 8, 9), 4);
    }

    /// Exit rows decoded from the 0x0045C300 initializer (rows 0x0089D368,
    /// 0x0089D638, 0x0089D908).
    #[test]
    fn foundation_exit_lists_match_native_rows() {
        assert_eq!(
            foundation_exit_list("1x1"),
            vec![
                (0, 1),
                (-1, 1),
                (1, 1),
                (-1, 0),
                (1, 0),
                (0, -1),
                (-1, -1),
                (1, -1)
            ]
        );
        let three = vec![
            (0, 3),
            (1, 3),
            (2, 3),
            (-1, 3),
            (3, 3),
            (-1, 2),
            (3, 2),
            (-1, 1),
            (3, 1),
            (-1, 0),
            (3, 0),
            (0, -1),
            (1, -1),
            (2, -1),
            (-1, -1),
            (3, -1),
        ];
        assert_eq!(foundation_exit_list("3x3"), three);
        assert_eq!(
            foundation_exit_list("4x3"),
            three,
            "the binary reuses the 3x3 row for 4x3"
        );
    }

    // --- 7c: repair-depot REPAIR_TICK trichotomy (`repair_tick`) ---
    // cost=1000, percent=15 -> total=150; step=8, max_hp=300 -> cost_per_step=4.

    #[test]
    fn depot_repair_full_hp_returns_complete() {
        assert_eq!(
            repair_tick(300, 300, 1000, 15, 8, 0, 0),
            RepairResponse::RepairComplete
        );
    }

    #[test]
    fn depot_repair_funded_returns_roger_with_step_and_cost() {
        assert_eq!(
            repair_tick(100, 300, 1000, 15, 8, 10, 0),
            RepairResponse::Roger { heal: 8, cost: 4 }
        );
        assert_eq!(
            repair_tick(100, 300, 1000, 15, 8, 4, 0),
            RepairResponse::Roger { heal: 8, cost: 4 }
        );
    }

    #[test]
    fn depot_repair_unfunded_returns_insufficient_with_incremented_grace() {
        assert_eq!(
            repair_tick(100, 300, 1000, 15, 8, 3, 0),
            RepairResponse::InsufficientFunds { grace: 1 }
        );
        assert_eq!(
            repair_tick(100, 300, 1000, 15, 8, 0, 5),
            RepairResponse::InsufficientFunds { grace: 6 }
        );
    }

    #[test]
    fn depot_repair_cost_per_step_clamps_to_one() {
        assert_eq!(
            repair_tick(100, 300, 10, 15, 8, 1, 0),
            RepairResponse::Roger { heal: 8, cost: 1 }
        );
        assert_eq!(
            repair_tick(100, 300, 10, 15, 8, 0, 0),
            RepairResponse::InsufficientFunds { grace: 1 }
        );
    }

    #[test]
    fn depot_repair_funded_step_ignores_prior_grace() {
        assert_eq!(
            repair_tick(100, 300, 1000, 15, 8, 10, NO_FUNDS_GRACE_TICKS - 1),
            RepairResponse::Roger { heal: 8, cost: 4 }
        );
    }

    #[test]
    fn depot_repair_grace_reaches_cap() {
        assert_eq!(
            repair_tick(100, 300, 1000, 15, 8, 0, NO_FUNDS_GRACE_TICKS - 1),
            RepairResponse::InsufficientFunds {
                grace: NO_FUNDS_GRACE_TICKS
            }
        );
    }

    #[test]
    fn depot_repair_response_maps_to_radio_codes() {
        assert_eq!(
            RepairResponse::Roger { heal: 8, cost: 4 }.radio_response(),
            RadioResponse::Roger
        );
        assert_eq!(
            RepairResponse::InsufficientFunds { grace: 1 }.radio_response(),
            RadioResponse::InsufficientFunds
        );
        assert_eq!(
            RepairResponse::RepairComplete.radio_response(),
            RadioResponse::RepairComplete
        );
        assert_eq!(RadioResponse::Roger as u8, 0x01);
        assert_eq!(RadioResponse::InsufficientFunds as u8, 0x20);
        assert_eq!(RadioResponse::RepairComplete as u8, 0x21);
    }

    // --- Admission / waiting / release through the production order path ---

    const DEPOT: u64 = 500;
    const DEPOT_RX: u16 = 10;
    const DEPOT_RY: u16 = 10;

    fn depot_rules() -> RuleSet {
        let ini = IniFile::from_str(
            "[General]\n\
             RepairPercent=15%\n\
             RepairStep=8\n\
             URepairRate=.016\n\
             [Enter]\n\
             Rate=.016\n\
             [InfantryTypes]\n\
             [VehicleTypes]\n\
             0=MTNK\n\
             [AircraftTypes]\n\
             [BuildingTypes]\n\
             0=GADEPT\n\
             [MTNK]\n\
             Name=Grizzly\n\
             Cost=700\n\
             Strength=300\n\
             Speed=6\n\
             [GADEPT]\n\
             Name=Depot\n\
             Foundation=3x3\n\
             UnitRepair=yes\n\
             Strength=1000\n",
        );
        RuleSet::from_ini(&ini).expect("depot rules")
    }

    fn spawn_entity(
        sim: &mut Simulation,
        sid: u64,
        type_id: &str,
        category: EntityCategory,
        rx: u16,
        ry: u16,
        hp: u16,
        max: u16,
    ) {
        let owner_id = sim.interner.intern("Americans");
        let type_id = sim.interner.intern(type_id);
        let mut ge = GameEntity::new_at_frame_zero_for_test(
            sid,
            rx,
            ry,
            0,
            0,
            owner_id,
            Health { current: hp, max },
            type_id,
            category,
            0,
            5,
            category == EntityCategory::Unit,
        );
        ge.lifecycle.in_limbo = false;
        sim.substrate.entities.insert(ge);
        if sim.substrate.next_stable_object_id <= sid {
            sim.substrate.next_stable_object_id = sid + 1;
        }
    }

    fn spawn_depot(sim: &mut Simulation) {
        spawn_entity(
            sim,
            DEPOT,
            "GADEPT",
            EntityCategory::Structure,
            DEPOT_RX,
            DEPOT_RY,
            1000,
            1000,
        );
        for y in DEPOT_RY..DEPOT_RY + 3 {
            for x in DEPOT_RX..DEPOT_RX + 3 {
                sim.substrate.occupancy.add(
                    x,
                    y,
                    DEPOT,
                    MovementLayer::Ground,
                    None,
                    CellListInsertion::AppendBuilding,
                );
            }
        }
    }

    fn spawn_tank(sim: &mut Simulation, sid: u64, rx: u16, ry: u16) {
        spawn_entity(sim, sid, "MTNK", EntityCategory::Unit, rx, ry, 100, 300);
    }

    fn setup(tank_count: u64) -> (Simulation, RuleSet, PathGrid) {
        let rules = depot_rules();
        let mut sim = Simulation::new();
        {
            use crate::sim::house_state::HouseState;
            let owner_id = sim.interner.intern("Americans");
            let mut house = HouseState::new(owner_id, 0, None, false, 0, 10);
            house.credits = 10_000;
            sim.houses.insert(owner_id, house);
        }
        spawn_depot(&mut sim);
        for i in 0..tank_count {
            spawn_tank(&mut sim, 1 + i, 14 + i as u16, 11);
        }
        (sim, rules, PathGrid::new(64, 64))
    }

    fn order_repair(sim: &mut Simulation, rules: &RuleSet, grid: &PathGrid, tank: u64) -> bool {
        let height_map: BTreeMap<(u16, u16), u8> = BTreeMap::new();
        sim.apply_command(
            "Americans",
            &Command::RepairAtDepot {
                entity_id: tank,
                depot_id: DEPOT,
            },
            Some(rules),
            Some(grid),
            &height_map,
        )
    }

    fn tick(sim: &mut Simulation, rules: &RuleSet, grid: &PathGrid) {
        sim.session.binary_frame = sim.session.binary_frame.wrapping_add(1);
        tick_building_docks(sim, rules, Some(grid));
        crate::sim::movement::tick_movement(
            &mut sim.substrate.entities,
            &mut sim.interner,
            &mut sim.pending_lifecycle_requests,
        );
        sim.session.tick += 1;
    }

    fn linked(sim: &Simulation, tank: u64) -> bool {
        sim.substrate
            .entities
            .get(tank)
            .is_some_and(|e| e.radio_contacts.contains(DEPOT))
            && sim
                .substrate
                .entities
                .get(DEPOT)
                .is_some_and(|d| d.radio_contacts.contains(tank))
    }

    fn phase(sim: &Simulation, tank: u64) -> Option<DockPhase> {
        sim.substrate
            .entities
            .get(tank)
            .and_then(|e| e.dock_state.as_ref().map(|d| d.phase))
    }

    fn pos(sim: &Simulation, tank: u64) -> (u16, u16) {
        let e = sim.substrate.entities.get(tank).unwrap();
        (e.position.rx, e.position.ry)
    }

    /// Production path: the RepairAtDepot command installs the FSM (mission
    /// Enter), the first probe links the first orderer, the other two stay
    /// unlinked with an armed Enter-cadence timer and no stored queue.
    #[test]
    fn depot_order_installs_enter_and_first_probe_links_one_unit() {
        let (mut sim, rules, grid) = setup(3);
        for tank in 1..=3 {
            assert!(order_repair(&mut sim, &rules, &grid, tank));
            let e = sim.substrate.entities.get(tank).unwrap();
            assert_eq!(e.dock_state.as_ref().unwrap().phase, DockPhase::Approach);
            assert_eq!(
                e.mission.queued().known(),
                Some(MissionType::Enter),
                "player depot order queues Enter (7) through the exact authority"
            );
            assert_eq!(e.derived_mission().0, MissionType::Enter);
        }
        tick(&mut sim, &rules, &grid);
        assert!(linked(&sim, 1));
        assert!(!linked(&sim, 2));
        assert!(!linked(&sim, 3));
        let depot = sim.substrate.entities.get(DEPOT).unwrap();
        assert_eq!(
            depot.radio_contacts.capacity(),
            1,
            "NumberOfDocks default 1"
        );
        assert_eq!(depot.radio_contacts.len(), 1);
        for tank in 2..=3 {
            let ds = sim
                .substrate
                .entities
                .get(tank)
                .unwrap()
                .dock_state
                .clone()
                .unwrap();
            assert!(ds.enter_retry.is_armed());
            assert!((14..=16).contains(&ds.enter_retry.duration));
        }
    }

    /// Each probe draws exactly one `RandomRanged(0,2)` on the Scenario
    /// stream (the Mission_Enter epilogue), none between probes.
    #[test]
    fn waiter_probe_draws_one_scenario_random_per_dispatch() {
        let (mut sim, rules, grid) = setup(2);
        for tank in 1..=2 {
            assert!(order_repair(&mut sim, &rules, &grid, tank));
        }
        // Frame 1: both units probe (two draws).
        let mut shadow = sim.clone_scenario_rng();
        tick(&mut sim, &rules, &grid);
        shadow.next_range_u32_inclusive(0, 2);
        shadow.next_range_u32_inclusive(0, 2);
        assert_eq!(
            sim.scenario_rng.next_range_u32_inclusive(0, 1000),
            shadow.next_range_u32_inclusive(0, 1000),
            "two probes ⇒ two RandomRanged(0,2) draws"
        );
        // Between probes the cadence window draws nothing.
        let mut shadow = sim.clone_scenario_rng();
        let waiter_due = {
            let ds = sim
                .substrate
                .entities
                .get(2)
                .unwrap()
                .dock_state
                .clone()
                .unwrap();
            ds.enter_retry.start_frame + ds.enter_retry.duration
        };
        while sim.session.binary_frame + 1 < waiter_due {
            tick(&mut sim, &rules, &grid);
        }
        assert_eq!(
            sim.scenario_rng.next_range_u32_inclusive(0, 1000),
            shadow.next_range_u32_inclusive(0, 1000),
            "no draw between dispatches"
        );
    }

    /// Admission follows whichever waiter's Enter timer fires first after the
    /// pad frees, not arrival order: unit 3's timer is set to fire before
    /// unit 2's, so 3 docks next.
    #[test]
    fn freed_pad_goes_to_the_first_reprobe_not_the_first_arrival() {
        let (mut sim, rules, grid) = setup(3);
        for tank in 1..=3 {
            assert!(order_repair(&mut sim, &rules, &grid, tank));
        }
        // Run until unit 1 is repaired and released.
        let mut released_at = None;
        for _ in 0..2000 {
            tick(&mut sim, &rules, &grid);
            if phase(&sim, 1).is_none() {
                released_at = Some(sim.session.binary_frame);
                break;
            }
        }
        let released_at = released_at.expect("unit 1 releases");
        assert!(!linked(&sim, 1));
        assert_eq!(sim.substrate.entities.get(1).unwrap().health.current, 300);
        assert!(!linked(&sim, 2) && !linked(&sim, 3));
        // Force the re-probe order: 3 fires before 2.
        {
            let e2 = sim.substrate.entities.get_mut(2).unwrap();
            e2.dock_state
                .as_mut()
                .unwrap()
                .enter_retry
                .arm(released_at, 10);
            let e3 = sim.substrate.entities.get_mut(3).unwrap();
            e3.dock_state
                .as_mut()
                .unwrap()
                .enter_retry
                .arm(released_at, 3);
        }
        for _ in 0..5 {
            tick(&mut sim, &rules, &grid);
        }
        assert!(linked(&sim, 3), "the first re-prober wins the freed slot");
        assert!(!linked(&sim, 2), "the earlier arrival is not promoted");
        assert_eq!(phase(&sim, 2), Some(DockPhase::WaitForDock));
    }

    /// Release drives the repaired unit off the pad to the foundation exit
    /// list's first enterable cell — (0, 3) below the NW corner for a 3x3 —
    /// with BREAK on both ends and a queued Move mission.
    #[test]
    fn repaired_unit_is_released_to_the_native_exit_cell_and_pad_is_vacated() {
        let (mut sim, rules, grid) = setup(1);
        assert!(order_repair(&mut sim, &rules, &grid, 1));
        let pad = depot_dock_cell(DEPOT_RX, DEPOT_RY, "3x3");
        let mut reached_pad = false;
        let mut released = false;
        for _ in 0..2000 {
            tick(&mut sim, &rules, &grid);
            if phase(&sim, 1) == Some(DockPhase::Servicing) {
                reached_pad = true;
                assert_eq!(pos(&sim, 1), pad);
            }
            if reached_pad && phase(&sim, 1).is_none() {
                released = true;
                break;
            }
        }
        assert!(reached_pad && released);
        let e = sim.substrate.entities.get(1).unwrap();
        assert_eq!(e.health.current, 300);
        assert!(!linked(&sim, 1));
        assert_eq!(e.mission.queued().known(), Some(MissionType::Move));
        let exit = (DEPOT_RX, DEPOT_RY + 3);
        assert_eq!(
            e.movement_target.as_ref().map(|t| *t.path.last().unwrap()),
            Some(exit)
        );
        for _ in 0..200 {
            tick(&mut sim, &rules, &grid);
        }
        assert_eq!(
            pos(&sim, 1),
            exit,
            "pad vacated, unit parked on the exit cell"
        );
        assert!(
            sim.substrate
                .entities
                .get(DEPOT)
                .unwrap()
                .radio_contacts
                .is_empty()
        );
    }

    /// A linked unit that is already at full health answers 0x22 with 10, so
    /// its own probe returns 10: BREAK + Enter_Idle_Mode (Guard), no scatter.
    #[test]
    fn linked_full_health_waiter_breaks_and_goes_idle_on_its_next_probe() {
        let (mut sim, rules, grid) = setup(2);
        for tank in 1..=2 {
            assert!(order_repair(&mut sim, &rules, &grid, tank));
        }
        tick(&mut sim, &rules, &grid);
        assert!(linked(&sim, 1));
        sim.substrate.entities.get_mut(1).unwrap().health.current = 300;
        let due = {
            let ds = sim
                .substrate
                .entities
                .get(1)
                .unwrap()
                .dock_state
                .clone()
                .unwrap();
            ds.enter_retry.start_frame + ds.enter_retry.duration
        };
        while sim.session.binary_frame < due {
            tick(&mut sim, &rules, &grid);
        }
        assert!(!linked(&sim, 1));
        assert!(phase(&sim, 1).is_none());
        assert_eq!(
            sim.substrate
                .entities
                .get(1)
                .unwrap()
                .mission
                .queued()
                .known(),
            Some(MissionType::Guard)
        );
        // The slot is free for the next waiter's probe.
        let due2 = {
            let ds = sim
                .substrate
                .entities
                .get(2)
                .unwrap()
                .dock_state
                .clone()
                .unwrap();
            ds.enter_retry.start_frame + ds.enter_retry.duration
        };
        while sim.session.binary_frame < due2 {
            tick(&mut sim, &rules, &grid);
        }
        assert!(linked(&sim, 2));
    }

    /// Exit-cell selection skips foundation/vehicle-blocked cells in list order.
    #[test]
    fn exit_cell_skips_blocked_cells_in_list_order() {
        let (mut sim, _rules, grid) = setup(0);
        assert_eq!(
            depot_exit_cell(&sim, Some(&grid), DEPOT_RX, DEPOT_RY, "3x3"),
            Some((DEPOT_RX, DEPOT_RY + 3))
        );
        spawn_tank(&mut sim, 7, DEPOT_RX, DEPOT_RY + 3);
        sim.substrate.occupancy.add(
            DEPOT_RX,
            DEPOT_RY + 3,
            7,
            MovementLayer::Ground,
            None,
            CellListInsertion::AppendBuilding,
        );
        assert_eq!(
            depot_exit_cell(&sim, Some(&grid), DEPOT_RX, DEPOT_RY, "3x3"),
            Some((DEPOT_RX + 1, DEPOT_RY + 3))
        );
    }
}
