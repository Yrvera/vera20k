//! Refinery docking visual sequence — approach, link, unload, depart.
//!
//! Drives the sub-state machine (`RefineryDockPhase`) when the miner is in
//! `MinerState::Dock`. Mirrors the four-state FSM used by the original
//! game's harvester deploy mission (cases 0/1/3/4): approach the queue,
//! link onto the pad, deposit bales, then drive off the exit cell.
//!
//! `refinery_pad_cell` is a thin wrapper over
//! [`crate::sim::docking::pad_geometry::pad_cell_for`] — that helper owns
//! the single building-center-relative lepton→cell conversion shared with
//! aircraft pad descent.
//!
//! ## Dependency rules
//! - Part of sim/ — depends on sim/miner, sim/miner_dock, sim/components,
//!   sim/movement, sim/docking/pad_geometry, rules/.
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

use crate::rules::ruleset::RuleSet;
use crate::sim::components::BaleDepositEvent;
use crate::sim::miner::{MinerConfig, MinerState, RefineryDockPhase, ResourceType};
use crate::sim::movement;
use crate::sim::movement::locomotor::MovementLayer;
use crate::sim::movement::turret;
use crate::sim::occupancy::OccupancyGrid;
use crate::sim::pathfinding::PathGrid;
use crate::sim::world::{SimSoundEvent, Simulation};
use crate::util::fixed_math::{SIM_TICK_HZ, SimFixed};

use super::miner_dock::ContactAdmission;
use super::miner_system::{MinerSnapshot, effective_purifier_count};
use crate::sim::production::{credits_entry_for_owner, foundation_dimensions};

/// Maximum diamond-ring radius for the post-unload exit-cell spiral search.
/// gamemd's `FootClass::Find_Nearby_Passable_Cell` derives its cap from
/// `Speed + SightRange` (capped at 32). A miner-class unit lands at ~14.
/// 16 covers the same footprint with a small safety margin and still
/// terminates quickly when the area around the refinery is fully blocked.
const EXIT_SEARCH_MAX_RADIUS: i32 = 16;

/// Target body facing on the refinery dock pad — East (0x40 in 8-bit,
/// 0x4000 in 16-bit). The miner's back faces the refinery (which sits west
/// of the dock pad), aligning the dump animation with the open-bay voxel.
/// Verified from gamemd `UnitClass::Receive_Radio` case 0x16 at 0x737430:
/// reads PrimaryFacing rate-timer, calls locomotor `Do_Turn(0x4000)` to
/// rotate toward East. Applies to both HARV and CMIN (no Teleporter gate).
const DOCK_FACING_EAST: u8 = 0x40;

/// Sim tick period in milliseconds, derived from the same constant used by
/// the movement system's rotation step.
const SIM_TICK_MS: u32 = 1000 / SIM_TICK_HZ;

/// Helper: record a dock phase transition to the snapshot's debug buffer.
fn record_dock_phase(snap: &mut MinerSnapshot, old: RefineryDockPhase, new: RefineryDockPhase) {
    snap.debug_dock_events
        .push((format!("{:?}", old), format!("{:?}", new)));
}

// ---------------------------------------------------------------------------
// Cell computation helpers
// ---------------------------------------------------------------------------

/// Queue cell — where the miner waits outside the refinery (pathfindable).
///
/// Uses art.ini `QueueingCell=` when available (merged into ObjectType),
/// otherwise falls back to geometric approximation from foundation dimensions.
pub(super) fn refinery_queue_cell(
    rx: u16,
    ry: u16,
    width: u16,
    height: u16,
    queueing_cell: Option<(u16, u16)>,
) -> (u16, u16) {
    if let Some((qx, qy)) = queueing_cell {
        (rx + qx, ry + qy)
    } else {
        (rx + width, ry + height / 2)
    }
}

/// CAN_DOCK queue target sent by `BuildingClass::Receive_Radio` case 0x0E.
///
/// Verified in gamemd: this path hardcodes building anchor + (3, 1) and does
/// not read art.ini `QueueingCell=`.
pub(super) fn refinery_can_dock_queue_cell(rx: u16, ry: u16) -> (u16, u16) {
    (rx.saturating_add(3), ry.saturating_add(1))
}

/// Pad cell — on the refinery platform inside the building footprint.
///
/// When art.ini declares a `DockingOffset0` (passed through as `docking_offset`),
/// delegates to [`crate::sim::docking::pad_geometry::pad_cell_for`] for the
/// shared building-center-relative lepton→cell conversion. Otherwise falls back
/// to the rightmost foundation column, vertically centred (used by retail
/// refineries which have no `DockingOffset0` in art).
pub(super) fn refinery_pad_cell(
    rx: u16,
    ry: u16,
    width: u16,
    height: u16,
    docking_offset: Option<(i32, i32, i32)>,
) -> (u16, u16) {
    if let Some((dx, dy, dz)) = docking_offset {
        let pad = crate::rules::object_type::DockPad {
            lepton_offset: (dx, dy, dz),
        };
        crate::sim::docking::pad_geometry::pad_cell_for((rx, ry), (width, height), &pad)
    } else {
        (rx + width.saturating_sub(1), ry + height / 2)
    }
}

/// Exit cell — where the miner drives after undocking.
///
/// Reproduces the observable behaviour of the original engine: the
/// harvester exits through the queue cell directly outside the dock
/// pad — the same cell it entered through. For a 4×3 GAREFN at cell
/// (10, 10) with pad at (13, 11), the queue cell at (14, 11) is the
/// only passable cell adjacent to the pad, so every exit goes through
/// it.
///
/// Internal mechanism deliberately diverges from the binary. gamemd's
/// `ReleaseDockedHarvester` (0x4595C0) computes a passable cell
/// anchored at `Location_cell + (−1, +1)` — west of the foundation —
/// and calls `Set_Destination(that_cell)`. That destination is
/// immediately overwritten on the next mission cycle by
/// `Mission_Harvest` case 0 SCAN which calls `Set_Destination(0)` to
/// clear and then `Search_For_Tiberium_And_Move` to re-target to ore.
/// The west-anchored cell never appears as a visible waypoint — the
/// miner drives from pad through the queue cell on its way to ore.
/// We collapse those two stages: drive directly to the queue cell,
/// then `SearchOre` takes over.
///
/// `find_nearby_passable_cell_with_index` provides a fallback when
/// the queue cell is blocked (e.g., another miner already waiting
/// there): ring 1+ picks an adjacent cell, typically still east of
/// the foundation. Falls back to the art.ini `QueueingCell`
/// (or the geometric default from [`refinery_queue_cell`]) when no
/// passable cell exists within [`EXIT_SEARCH_MAX_RADIUS`] or no path
/// grid is available.
///
/// Applies to both war miners and chrono miners — neither warps on the
/// outbound trip; only the inbound (ore → refinery) leg uses the
/// chrono warp.
pub(super) fn refinery_exit_cell(
    rx: u16,
    ry: u16,
    width: u16,
    height: u16,
    queueing_cell: Option<(u16, u16)>,
    path_grid: Option<&PathGrid>,
    occupancy: Option<&OccupancyGrid>,
    tick: u64,
) -> (u16, u16) {
    let queue = refinery_queue_cell(rx, ry, width, height, queueing_cell);

    if let Some(grid) = path_grid {
        if let Some(cell) = find_nearby_passable_cell_with_index(
            queue.0 as i32,
            queue.1 as i32,
            grid,
            occupancy,
            EXIT_SEARCH_MAX_RADIUS,
            tick,
        ) {
            return cell;
        }
    }

    queue
}

/// Whether `(x, y)` is in-bounds, passable on the ground layer, and not
/// occupied by any other ground-layer entity.
///
/// Bridge layer is intentionally ignored: refinery exit cells must drop the
/// miner on land. `occupancy` may be `None` when only path passability is
/// known (used by direct unit tests of the spiral algorithm).
fn is_exit_cell_passable(
    x: i32,
    y: i32,
    grid: &PathGrid,
    occupancy: Option<&OccupancyGrid>,
) -> bool {
    if x < 0 || y < 0 || x >= grid.width() as i32 || y >= grid.height() as i32 {
        return false;
    }
    let cx = x as u16;
    let cy = y as u16;
    if !grid.is_walkable(cx, cy) {
        return false;
    }
    if let Some(occ) = occupancy {
        if !occ.is_empty_on_layer(cx, cy, MovementLayer::Ground) {
            return false;
        }
    }
    true
}

/// Diamond-ring spiral search from `(ox, oy)`.
///
/// Mirrors the iteration order of `FootClass::Find_Nearby_Passable_Cell`
/// (gamemd 0x56DC20): ring `r=0` is the anchor itself; subsequent rings
/// walk top/bottom rows first (delta = -r..=r), then left/right columns
/// excluding corners (delta = 1-r..=r-1). Returns the **first** passable
/// cell encountered.
///
/// Note: gamemd's implementation collects up to 24 candidates from the
/// first non-empty ring and picks one at random (via `g_CurrentFrameCounter
/// % count`); see [`find_nearby_passable_cells_first_ring`] for the
/// candidate-collecting variant used by the refinery exit drive.
pub(crate) fn find_nearby_passable_cell(
    ox: i32,
    oy: i32,
    grid: &PathGrid,
    occupancy: Option<&OccupancyGrid>,
    max_radius: i32,
) -> Option<(u16, u16)> {
    if is_exit_cell_passable(ox, oy, grid, occupancy) {
        return Some((ox as u16, oy as u16));
    }
    for r in 1..=max_radius {
        // Segment 1: top + bottom rows.
        for delta in -r..=r {
            if is_exit_cell_passable(ox + delta, oy - r, grid, occupancy) {
                return Some(((ox + delta) as u16, (oy - r) as u16));
            }
            if is_exit_cell_passable(ox + delta, oy + r, grid, occupancy) {
                return Some(((ox + delta) as u16, (oy + r) as u16));
            }
        }
        // Segment 2: left + right columns (corners already covered by segment 1).
        for delta in (1 - r)..=(r - 1) {
            if is_exit_cell_passable(ox - r, oy + delta, grid, occupancy) {
                return Some(((ox - r) as u16, (oy + delta) as u16));
            }
            if is_exit_cell_passable(ox + r, oy + delta, grid, occupancy) {
                return Some(((ox + r) as u16, (oy + delta) as u16));
            }
        }
    }
    None
}

/// Diamond-ring spiral that collects ALL passable cells in the first
/// non-empty ring, mirroring gamemd's `FootClass::Find_Nearby_Passable_Cell`
/// (0x56DC20) candidate-collection block. The original engine then picks
/// from the collected pool via `g_CurrentFrameCounter % count` — caller
/// supplies the equivalent index.
///
/// Why this matters: a deterministic "return first walkable" picks the
/// same cell every time, which is visually fine when the anchor itself
/// is passable (the common case for the refinery exit cell — queue cell
/// adjacent to pad) but produces "miner always exits at the same spot"
/// drift when the anchor is blocked and several ring-1 candidates exist.
/// The modulo selection over the ring's candidates spreads exits across
/// the available cells the way gamemd does.
///
/// Returns the chosen cell, or `None` if no ring within `max_radius`
/// produces a passable cell. The selection is `candidates[index % count]`,
/// matching gamemd's modulo-based pick.
pub(crate) fn find_nearby_passable_cell_with_index(
    ox: i32,
    oy: i32,
    grid: &PathGrid,
    occupancy: Option<&OccupancyGrid>,
    max_radius: i32,
    index: u64,
) -> Option<(u16, u16)> {
    // Ring 0: the anchor itself. If passable, it's the sole candidate.
    if is_exit_cell_passable(ox, oy, grid, occupancy) {
        return Some((ox as u16, oy as u16));
    }
    let mut candidates: Vec<(u16, u16)> = Vec::with_capacity(24);
    for r in 1..=max_radius {
        candidates.clear();
        // Segment 1: top + bottom rows.
        for delta in -r..=r {
            if is_exit_cell_passable(ox + delta, oy - r, grid, occupancy) {
                candidates.push(((ox + delta) as u16, (oy - r) as u16));
            }
            if is_exit_cell_passable(ox + delta, oy + r, grid, occupancy) {
                candidates.push(((ox + delta) as u16, (oy + r) as u16));
            }
        }
        // Segment 2: left + right columns (corners already covered by segment 1).
        for delta in (1 - r)..=(r - 1) {
            if is_exit_cell_passable(ox - r, oy + delta, grid, occupancy) {
                candidates.push(((ox - r) as u16, (oy + delta) as u16));
            }
            if is_exit_cell_passable(ox + r, oy + delta, grid, occupancy) {
                candidates.push(((ox + r) as u16, (oy + delta) as u16));
            }
        }
        if !candidates.is_empty() {
            // gamemd's `local_60[g_CurrentFrameCounter % count]` selection.
            let pick = (index as usize) % candidates.len();
            return Some(candidates[pick]);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Refinery lookup helpers
// ---------------------------------------------------------------------------

/// Resolve a refinery entity's foundation and compute queue/pad/exit cells.
/// Returns `(queue, pad, exit)` or `None` if the refinery is gone.
fn resolve_refinery_cells(
    sim: &Simulation,
    rules: &RuleSet,
    ref_sid: u64,
    path_grid: Option<&PathGrid>,
) -> Option<((u16, u16), (u16, u16), (u16, u16), usize)> {
    let entity = sim.entities.get(ref_sid)?;
    if entity.dying || entity.health.current == 0 {
        return None;
    }
    let obj = rules.object_case_insensitive(sim.interner.resolve(entity.type_ref));
    let (w, h) = obj
        .map(|o| foundation_dimensions(&o.foundation))
        .unwrap_or((1, 1));
    let qc = obj.and_then(|o| o.queueing_cell);
    let dock_off = obj.and_then(|o| o.pads.first().map(|p| p.lepton_offset));
    let dock_capacity = obj.map(|o| o.number_of_docks.max(1) as usize).unwrap_or(1);
    let rx = entity.position.rx;
    let ry = entity.position.ry;
    Some((
        refinery_can_dock_queue_cell(rx, ry),
        refinery_pad_cell(rx, ry, w, h, dock_off),
        refinery_exit_cell(rx, ry, w, h, qc, path_grid, Some(&sim.occupancy), sim.tick),
        dock_capacity,
    ))
}

/// Look up the UnloadingClass for a miner type from rules.ini.
fn unloading_class(rules: &RuleSet, type_id: &str) -> Option<String> {
    rules
        .object_case_insensitive(type_id)
        .and_then(|obj| obj.unloading_class.clone())
}

fn dock_abort_state(snap: &MinerSnapshot) -> MinerState {
    if snap.miner.forced_return && !snap.miner.cargo.is_empty() {
        MinerState::ForcedReturn
    } else if snap.miner.is_full() {
        MinerState::ReturnToRefinery
    } else {
        MinerState::SearchOre
    }
}

fn abort_invalid_refinery(sim: &mut Simulation, snap: &mut MinerSnapshot, ref_sid: Option<u64>) {
    if let Some(ref_sid) = ref_sid {
        sim.production
            .dock_reservations
            .cancel_miner(ref_sid, snap.entity_id);
    }

    if let Some(entity) = sim.entities.get_mut(snap.entity_id) {
        entity.display_type_override = None;
        entity.facing_target = None;
        entity.movement_target = None;
    }

    snap.miner.reserved_refinery = None;
    snap.miner.dock_queued = false;
    snap.miner.dock_phase = RefineryDockPhase::Approach;
    snap.miner.deposit_cooldown_ticks = 0;
    snap.miner.exit_cell = None;
    snap.miner.unload_timer = 0;
    if snap.miner.is_full() {
        snap.miner.target_ore_cell = None;
    }
    snap.miner.state = dock_abort_state(snap);
}

// ---------------------------------------------------------------------------
// Main dock sequence handler
// ---------------------------------------------------------------------------

/// Process one tick of the refinery docking sequence for a single miner.
pub(super) fn handle_dock_sequence(
    sim: &mut Simulation,
    rules: &RuleSet,
    config: &MinerConfig,
    path_grid: Option<&PathGrid>,
    snap: &mut MinerSnapshot,
) {
    let phase_before = snap.miner.dock_phase;

    let Some(ref_sid) = snap.miner.reserved_refinery else {
        abort_invalid_refinery(sim, snap, None);
        if phase_before != snap.miner.dock_phase {
            record_dock_phase(snap, phase_before, snap.miner.dock_phase);
        }
        return;
    };

    let Some((queue, pad, exit, dock_capacity)) =
        resolve_refinery_cells(sim, rules, ref_sid, path_grid)
    else {
        abort_invalid_refinery(sim, snap, Some(ref_sid));
        if phase_before != snap.miner.dock_phase {
            record_dock_phase(snap, phase_before, snap.miner.dock_phase);
        }
        return;
    };

    match snap.miner.dock_phase {
        RefineryDockPhase::Approach => {
            phase_approach(sim, path_grid, snap, queue, pad, ref_sid, dock_capacity);
        }
        RefineryDockPhase::Linked => {
            phase_linked(sim, rules, snap, pad, ref_sid);
        }
        RefineryDockPhase::Pivoting => {
            phase_pivoting(sim, rules, config, snap);
        }
        RefineryDockPhase::Unloading => {
            phase_unloading(sim, rules, config, snap, ref_sid);
        }
        RefineryDockPhase::DepositCooldown => {
            phase_deposit_cooldown(snap);
        }
        RefineryDockPhase::Departing => {
            phase_departing(sim, path_grid, snap, pad, exit, ref_sid);
        }
    }

    if phase_before != snap.miner.dock_phase {
        record_dock_phase(snap, phase_before, snap.miner.dock_phase);
    }
}

// ---------------------------------------------------------------------------
// Phase handlers
// ---------------------------------------------------------------------------

fn phase_approach(
    sim: &mut Simulation,
    path_grid: Option<&PathGrid>,
    snap: &mut MinerSnapshot,
    queue: (u16, u16),
    pad: (u16, u16),
    ref_sid: u64,
    dock_capacity: usize,
) {
    // Try to acquire the dock reservation. If granted, immediately re-target
    // the pad cell and transition to Linked. RemoveOccupy in art.ini removes
    // the pad cell from the path/occupancy grid, so the move can proceed
    // without bypass_grid.
    let admission =
        sim.production
            .dock_reservations
            .hello_or_wait(ref_sid, snap.entity_id, dock_capacity);

    if admission == ContactAdmission::Accepted {
        snap.miner.dock_queued = false;
        let moving = sim
            .entities
            .get(snap.entity_id)
            .is_some_and(|e| e.movement_target.is_some());
        if (snap.rx, snap.ry) == pad && !moving {
            sim.production
                .dock_reservations
                .link_on_pad(ref_sid, snap.entity_id);
            snap.miner.dock_phase = RefineryDockPhase::Linked;
        } else if !moving {
            movement::issue_direct_move(&mut sim.entities, snap.entity_id, pad, snap.speed);
            snap.miner.dock_phase = RefineryDockPhase::Linked;
        }
        return;
    }
    snap.miner.dock_queued = true;

    if queue == pad && sim.production.dock_reservations.pad_occupied(ref_sid) {
        return;
    }

    // Reservation not granted — keep heading toward QueueingCell.
    if !is_adjacent_or_at((snap.rx, snap.ry), queue) {
        if let Some(grid) = path_grid {
            issue_move_if_idle(&mut sim.entities, grid, snap.entity_id, queue, snap.speed);
        }
    }
}

fn phase_linked(
    sim: &mut Simulation,
    rules: &RuleSet,
    snap: &mut MinerSnapshot,
    pad: (u16, u16),
    ref_sid: u64,
) {
    let arrived = sim
        .entities
        .get(snap.entity_id)
        .is_some_and(|e| e.movement_target.is_none());
    if !arrived {
        return;
    }

    snap.rx = pad.0;
    snap.ry = pad.1;
    sim.production
        .dock_reservations
        .link_on_pad(ref_sid, snap.entity_id);

    if let Some(entity) = sim.entities.get_mut(snap.entity_id) {
        if let Some(uc) = unloading_class(rules, sim.interner.resolve(snap.type_id)) {
            entity.display_type_override = Some(sim.interner.intern(&uc));
        }
        // Kick off the pivot to East. The actual rotation runs in
        // phase_pivoting one tick at a time, mirroring gamemd's
        // RateTimer-driven smooth turn from radio 0x16.
        entity.facing_target = Some(DOCK_FACING_EAST);
    }

    sim.sound_events.push(SimSoundEvent::DockDeploy {
        building_id: ref_sid,
    });

    snap.miner.dock_phase = RefineryDockPhase::Pivoting;
}

/// Smooth in-place rotation toward DOCK_FACING_EAST (0x40). Each tick
/// advances facing by at most `rot_to_facing_delta` units. When facing
/// reaches the target, initialise the unload timer and transition to
/// Unloading — the same handshake gamemd performs via radio 0x15 once
/// the harvester is stationary and the PrimaryFacing rate-timer has
/// completed its target rotation.
fn phase_pivoting(
    sim: &mut Simulation,
    rules: &RuleSet,
    config: &MinerConfig,
    snap: &mut MinerSnapshot,
) {
    let type_name = sim.interner.resolve(snap.type_id).to_string();
    let rot: i32 = rules
        .object_case_insensitive(&type_name)
        .map(|obj| obj.turret_rot.max(1))
        .unwrap_or(10);

    let Some(entity) = sim.entities.get_mut(snap.entity_id) else {
        return;
    };

    let max_delta: u8 = turret::rot_to_facing_delta(rot, SIM_TICK_MS);
    let diff: i16 = turret::shortest_rotation(entity.facing, DOCK_FACING_EAST);

    if diff.unsigned_abs() <= max_delta as u16 {
        // Pivot complete — snap to exact facing, clear target, and start
        // the dump cascade. Timer init mirrors the prior phase_linked
        // behaviour: `interval - 10` lines the first pop up with the
        // 14.4-frame gate after the Linked/Pivoting → Unloading transition.
        entity.facing = DOCK_FACING_EAST;
        entity.facing_target = None;
        snap.miner.unload_timer = (config.unload_tick_interval as i16).saturating_sub(10);
        snap.miner.dock_phase = RefineryDockPhase::Unloading;
    } else {
        // Still rotating — advance facing toward the target. Refresh the
        // screen coordinates so the sprite re-renders at the new heading.
        if diff > 0 {
            entity.facing = entity.facing.wrapping_add(max_delta);
        } else {
            entity.facing = entity.facing.wrapping_sub(max_delta);
        }
        entity.position.refresh_screen_coords();
        entity.facing_target = Some(DOCK_FACING_EAST);
    }
}

fn phase_unloading(
    sim: &mut Simulation,
    rules: &RuleSet,
    config: &MinerConfig,
    snap: &mut MinerSnapshot,
    ref_sid: u64,
) {
    if snap.miner.unload_timer > 0 {
        snap.miner.unload_timer -= 10;
        return;
    }

    // Drain one resource-type "slot" per threshold crossing — all bales
    // of the same type drop in one atomic step. The 14.4-tick interval
    // is the latency between SLOT drains, not between bale credits.
    //
    // Slot order is fixed: Ore first, then Gems, so mixed cargo drains
    // ore first.
    const SLOT_ORDER: [ResourceType; 2] = [ResourceType::Ore, ResourceType::Gem];
    let next_slot = SLOT_ORDER
        .iter()
        .copied()
        .find(|t| snap.miner.cargo.iter().any(|b| b.resource_type == *t));

    if let Some(slot_type) = next_slot {
        let mut slot_value: i32 = 0;
        snap.miner.cargo.retain(|b| {
            if b.resource_type == slot_type {
                slot_value = slot_value.saturating_add(i32::from(b.value));
                false
            } else {
                true
            }
        });

        // Credits go to the REFINERY OWNER, not the harvester's current
        // controller. gamemd reads the building's owner via vtable+0x3C
        // (`GetOwner` on the BuildingClass instance, not on the harvester).
        // Matters under mind-control: a Yuri unit MC'ing an enemy harvester
        // still credits the original refinery owner — the "steal" doesn't
        // work. The single GetOwner result also keys the purifier-count
        // lookup, so base credits and bonus always share one owner.
        let refinery_owner: String = sim
            .entities
            .get(ref_sid)
            .map(|b| sim.interner.resolve(b.owner).to_string())
            .unwrap_or_else(|| sim.interner.resolve(snap.owner).to_string());

        {
            let credits = credits_entry_for_owner(sim, &refinery_owner);
            *credits = credits.saturating_add(slot_value);
        }

        // Purifier bonus applied once per slot drain:
        //   bonus = floor(slot_value × purifier_count × PurifierBonus)
        // Multiplying by the whole slot eliminates the per-bale integer
        // truncation that previously under-paid by ~PurifierBonus% per bale.
        let purifier_count = effective_purifier_count(sim, rules, &refinery_owner);
        if purifier_count > 0 {
            let bonus_pct: i32 = rules.general.purifier_bonus_pct;
            let bonus: i32 = slot_value
                .saturating_mul(purifier_count)
                .saturating_mul(bonus_pct)
                / 100;
            if bonus > 0 {
                let credits = credits_entry_for_owner(sim, &refinery_owner);
                *credits = credits.saturating_add(bonus);
            }
        }

        // One deposit event per slot drain — drives one SpecialAnim play
        // and one smoke-particle spawn per slot.
        sim.bale_events.push(BaleDepositEvent {
            building_id: ref_sid,
            tick: sim.tick,
        });

        snap.miner.unload_timer = snap
            .miner
            .unload_timer
            .saturating_add(config.unload_tick_interval as i16);
        return;
    }

    // Cargo empty — hold for one more dump-gate interval, matching gamemd's
    // post-last-bale idle: the dump counter keeps ticking after the last
    // bale drains, and on the NEXT 14.4-frame gate fire `FindFirstNonEmptySlot`
    // returns -1, transitioning the refinery state machine to state 4
    // (which calls `ReleaseDockedHarvester`). One unload-interval is the
    // correct hold — not the full SpecialAnim duration. The GAREFNOR pipe
    // sprite plays out as a building-side anim and outlasts the miner's
    // departure in gamemd as well.
    snap.miner.home_refinery = Some(ref_sid);
    snap.miner.deposit_cooldown_ticks = unload_interval_in_ticks(config);
    snap.miner.dock_phase = RefineryDockPhase::DepositCooldown;
}

/// One unload-interval in whole ticks. `unload_tick_interval` is stored in
/// tenths-of-a-tick (default 144 = 14.4 ticks); rounding UP gives the
/// gamemd-equivalent post-last-bale hold.
fn unload_interval_in_ticks(config: &MinerConfig) -> u16 {
    (config.unload_tick_interval).div_ceil(10)
}

/// Hold on the pad until the last deposit animation completes, then
/// hand off to `Departing`.
fn phase_deposit_cooldown(snap: &mut MinerSnapshot) {
    if snap.miner.deposit_cooldown_ticks > 0 {
        snap.miner.deposit_cooldown_ticks -= 1;
        return;
    }
    // NOTE: `display_type_override` is deliberately NOT cleared here. The
    // unloading sprite (e.g. CMON) has a different VXL/HVA anchor than the
    // base sprite (CMIN); clearing the override while the miner is still
    // parked on the pad makes the rendered voxel visually snap by the
    // anchor delta. Defer the clear to `phase_departing`, fired the first
    // tick the miner has crossed off the pad cell, so the snap (if any)
    // happens while the miner is already moving and is masked by the drive
    // animation.
    snap.miner.dock_phase = RefineryDockPhase::Departing;
}

fn phase_departing(
    sim: &mut Simulation,
    path_grid: Option<&PathGrid>,
    snap: &mut MinerSnapshot,
    pad: (u16, u16),
    exit: (u16, u16),
    ref_sid: u64,
) {
    // Clear the unloading-sprite override the first tick after the miner
    // has physically crossed off the pad cell. Clearing while still parked
    // on the pad makes the rendered voxel snap by the CMON↔CMIN HVA anchor
    // delta; clearing once the miner is mid-drive masks the snap inside
    // the drive animation. Idempotent — safe to call every tick.
    if (snap.rx, snap.ry) != pad
        && let Some(entity) = sim.entities.get_mut(snap.entity_id)
        && entity.display_type_override.is_some()
    {
        entity.display_type_override = None;
    }

    // Cache the exit cell on first entry. The exit returned by
    // `resolve_refinery_cells` is recomputed every tick from live occupancy
    // and will shift away from the miner the moment it arrives (the spiral
    // search treats the miner's own cell as blocked), causing a ping-pong
    // loop. gamemd avoids this by computing the destination once inside
    // `ReleaseDockedHarvester` and never recomputing it.
    //
    // The same first-entry boundary fires the dock-exit VOC: gamemd's
    // `ReleaseDockedHarvester` (0x4595C0) step 2 calls
    // `VocClass::PlayAt(rules+0x244, building.Location, 0)` at the building
    // location every ore-delivery cycle. The app layer resolves
    // `RefineryExitSfx` to [AudioVisual] `BunkerWallsDownSound`.
    let target = match snap.miner.exit_cell {
        Some(cached) => cached,
        None => {
            sim.production
                .dock_reservations
                .release_on_pad(ref_sid, snap.entity_id);
            sim.production
                .dock_reservations
                .release_contact(ref_sid, snap.entity_id);
            snap.miner.exit_cell = Some(exit);
            if let Some(building) = sim.entities.get(ref_sid) {
                let (rx, ry) = (building.position.rx, building.position.ry);
                sim.sound_events
                    .push(SimSoundEvent::RefineryExitSfx { rx, ry });
            }
            exit
        }
    };

    let moving = sim
        .entities
        .get(snap.entity_id)
        .is_some_and(|e| e.movement_target.is_some());
    let at_exit = (snap.rx, snap.ry) == target;
    let teleporting = sim
        .entities
        .get(snap.entity_id)
        .is_some_and(|e| e.teleport_state.is_some());

    if !moving && !at_exit {
        // Pad→exit drive: the exit lands at the queue cell directly
        // outside the pad (e.g. (14, 11) for GAREFN at (10, 10) with pad
        // (13, 11)). `astar_search` accepts a blocked start cell, so
        // pathing out of the pad works even though the pad sits inside
        // the building footprint.
        //
        // TODO(force_track_0x47_bib_step): gamemd inserts a sub-cell
        // drive-track curve here BEFORE the A* drive — `Force_Track(0x47, ...)`
        // = `TURN_TRACKS[71]` → `RAW_TRACKS[15]` (16-point ESE arc, no cell
        // crossing). It's a half-cell visual "ease-off-the-pad" before the
        // long exit drive. Track 15 already exists in `drive_track.rs`, but
        // the geometry between Force_Track's destination-as-reference and
        // our `head_offset_x = head_dx*256 + 128` formula needs Ghidra
        // verification before wiring it in — otherwise the curve plays in
        // the wrong direction. See `feedback_force_track_bib_step.md`.
        if let Some(grid) = path_grid {
            let _ = movement::issue_move_command(
                &mut sim.entities,
                grid,
                snap.entity_id,
                target,
                snap.speed,
                false,
                None,
                None,
                None,
                false,
            );
        } else {
            // No grid (test-only edge case): fall back to single-cell direct
            // move so the test harness still progresses.
            movement::issue_direct_move(&mut sim.entities, snap.entity_id, target, snap.speed);
        }
        // Mark the exit drive as `bypass_grid` so the per-step occupancy
        // check is skipped. The pad's ONLY adjacent walkable cell is the
        // queue cell (rx+4, ry+1); if another miner is queued there (common
        // with 2+ chrono miners cycling one refinery), the deferred-cell
        // check in `movement_occupancy::detect_deferred_cell_check` halts
        // the exit step indefinitely. gamemd handles this via radio-
        // protocol coordination (the queued miner steps aside as we leave);
        // `bypass_grid` is the closest minimal equivalent — it lets the
        // exit drive push through the queue cell, briefly overlapping any
        // waiting miner. The dock-in path uses the same trick when entering
        // the foundation.
        //
        // Note: facing is intentionally NOT pinned here. `issue_move_command`
        // already sets `facing_target` from the first path step, so the
        // movement system rotates the unit toward the actual direction of
        // travel as it leaves the pad. gamemd's `Force_Track(0x47, ...)` is
        // a DRIVE-TRACK CURVE INDEX, not a facing byte — pinning facing
        // to 0x47 would make the miner drive backwards (body facing ESE
        // while moving west toward the exit cell).
        return;
    }

    if !moving && at_exit && !teleporting {
        sim.production
            .dock_reservations
            .release_on_pad(ref_sid, snap.entity_id);
        sim.production
            .dock_reservations
            .release_contact(ref_sid, snap.entity_id);
        snap.miner.reserved_refinery = None;
        snap.miner.dock_queued = false;
        snap.miner.forced_return = false;
        // Clear the pending ore target (it has been consumed) and the
        // cached exit cell (only valid for this Departing run). Preserve
        // `last_harvest_cell` — the ghost-cell archive must survive the
        // entire dock cycle so the next `SearchOre` returns directly to
        // the productive patch saved when this miner became full.
        snap.miner.target_ore_cell = None;
        snap.miner.exit_cell = None;
        snap.miner.dock_phase = RefineryDockPhase::Approach;
        snap.miner.state = MinerState::SearchOre;
        return;
    }

    if let Some(entity) = sim.entities.get(snap.entity_id) {
        snap.rx = entity.position.rx;
        snap.ry = entity.position.ry;
    }
}

// ---------------------------------------------------------------------------
// Utility (re-exported from miner_system for shared use)
// ---------------------------------------------------------------------------

/// True if `pos` is at `target` or cardinally/diagonally adjacent (1 cell away).
fn is_adjacent_or_at(pos: (u16, u16), target: (u16, u16)) -> bool {
    let dx = (pos.0 as i32 - target.0 as i32).unsigned_abs();
    let dy = (pos.1 as i32 - target.1 as i32).unsigned_abs();
    dx <= 1 && dy <= 1
}

/// Issue a move command only if the entity isn't already pathing to this target.
fn issue_move_if_idle(
    entities: &mut crate::sim::entity_store::EntityStore,
    grid: &PathGrid,
    entity_id: u64,
    target: (u16, u16),
    speed: SimFixed,
) {
    if target.0 >= grid.width() || target.1 >= grid.height() {
        return;
    }
    let already = entities
        .get(entity_id)
        .and_then(|e| e.movement_target.as_ref())
        .and_then(|mt| mt.path.last().copied())
        .is_some_and(|goal| goal == target);
    if !already {
        let _ = movement::issue_move_command(
            entities, grid, entity_id, target, speed, false, None, None, None, false,
        );
    }
}
