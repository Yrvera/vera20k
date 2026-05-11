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
use crate::sim::miner::{MinerConfig, MinerState, RefineryDockPhase};
use crate::sim::movement;
use crate::sim::pathfinding::PathGrid;
use crate::sim::world::{SimSoundEvent, Simulation};
use crate::util::fixed_math::SimFixed;

use super::miner_system::{MinerSnapshot, player_has_purifier};
use crate::sim::production::{credits_entry_for_owner, foundation_dimensions};

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
/// Anchor is the foundation centroid (`BuildingClass::GetCoords()` overrides
/// `ObjectClass::GetCoords()` and adds `W*128 - 128` / `H*128 - 128` to the
/// top-left cell *center*). The undock helper then offsets that anchor by
/// `(-0x80, +0x80)` leptons before integer-dividing into cell space:
///   exit_lepton.x = rx*256 + W*128 - 128
///   exit_lepton.y = ry*256 + H*128 + 128
/// For a 4×3 refinery at (10, 10) this resolves to (11, 12) — one cell
/// south-east of the foundation centroid, on the bib row. Larger refineries
/// push the exit point further SE proportionally.
pub(super) fn refinery_exit_cell(rx: u16, ry: u16, width: u16, height: u16) -> (u16, u16) {
    let exit_x = (rx as i32 * 256 + width as i32 * 128 - 0x80) / 256;
    let exit_y = (ry as i32 * 256 + height as i32 * 128 + 0x80) / 256;
    (exit_x.max(0) as u16, exit_y.max(0) as u16)
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
) -> Option<((u16, u16), (u16, u16), (u16, u16))> {
    let entity = sim.entities.get(ref_sid)?;
    let obj = rules.object_case_insensitive(sim.interner.resolve(entity.type_ref));
    let (w, h) = obj
        .map(|o| foundation_dimensions(&o.foundation))
        .unwrap_or((1, 1));
    let qc = obj.and_then(|o| o.queueing_cell);
    let dock_off = obj.and_then(|o| o.pads.first().map(|p| p.lepton_offset));
    let rx = entity.position.rx;
    let ry = entity.position.ry;
    Some((
        refinery_queue_cell(rx, ry, w, h, qc),
        refinery_pad_cell(rx, ry, w, h, dock_off),
        refinery_exit_cell(rx, ry, w, h),
    ))
}

/// Look up the UnloadingClass for a miner type from rules.ini.
fn unloading_class(rules: &RuleSet, type_id: &str) -> Option<String> {
    rules
        .object_case_insensitive(type_id)
        .and_then(|obj| obj.unloading_class.clone())
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
        snap.miner.state = MinerState::SearchOre;
        snap.miner.dock_phase = RefineryDockPhase::Approach;
        if phase_before != snap.miner.dock_phase {
            record_dock_phase(snap, phase_before, snap.miner.dock_phase);
        }
        return;
    };

    let Some((queue, pad, exit)) = resolve_refinery_cells(sim, rules, ref_sid) else {
        snap.miner.reserved_refinery = None;
        snap.miner.state = MinerState::SearchOre;
        snap.miner.dock_phase = RefineryDockPhase::Approach;
        if phase_before != snap.miner.dock_phase {
            record_dock_phase(snap, phase_before, snap.miner.dock_phase);
        }
        return;
    };

    match snap.miner.dock_phase {
        RefineryDockPhase::Approach => {
            phase_approach(sim, path_grid, snap, queue, pad, ref_sid);
        }
        RefineryDockPhase::Linked => {
            phase_linked(sim, rules, snap, pad, ref_sid);
        }
        RefineryDockPhase::Unloading => {
            phase_unloading(sim, rules, config, snap, ref_sid);
        }
        RefineryDockPhase::Departing => {
            phase_departing(sim, snap, exit);
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
) {
    // Try to acquire the dock reservation. If granted, immediately re-target
    // the pad cell and transition to Linked. RemoveOccupy in art.ini removes
    // the pad cell from the path/occupancy grid, so the move can proceed
    // without bypass_grid.
    if sim
        .production
        .dock_reservations
        .try_reserve(ref_sid, snap.entity_id)
    {
        snap.miner.dock_queued = false;
        movement::issue_direct_move(&mut sim.entities, snap.entity_id, pad, snap.speed);
        snap.miner.dock_phase = RefineryDockPhase::Linked;
        return;
    }
    snap.miner.dock_queued = true;

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

    if let Some(uc) = unloading_class(rules, sim.interner.resolve(snap.type_id))
        && let Some(entity) = sim.entities.get_mut(snap.entity_id)
    {
        entity.display_type_override = Some(sim.interner.intern(&uc));
    }

    sim.sound_events.push(SimSoundEvent::DockDeploy {
        building_id: ref_sid,
    });

    // Initialize unload_timer to 0 — first bale fires after one full
    // unload_tick_interval, matching gamemd's per-bale gate.
    snap.miner.unload_timer = 0;
    snap.miner.dock_phase = RefineryDockPhase::Unloading;
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

    if let Some(bale) = snap.miner.cargo.pop() {
        let value: i32 = i32::from(bale.value);
        let owner_str = sim.interner.resolve(snap.owner).to_string();

        {
            let credits = credits_entry_for_owner(sim, &owner_str);
            *credits = credits.saturating_add(value);
        }

        // Per-bale purifier bonus (matches gamemd's per-bale credit application).
        if player_has_purifier(sim, rules, &owner_str) {
            let bonus_pct: i32 = rules.general.purifier_bonus_pct;
            let bonus: i32 = value * bonus_pct / 100;
            if bonus > 0 {
                let credits = credits_entry_for_owner(sim, &owner_str);
                *credits = credits.saturating_add(bonus);
            }
        }

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

    // Cargo empty — release dock and depart.
    sim.production.dock_reservations.release(ref_sid);
    snap.miner.home_refinery = Some(ref_sid);

    if let Some(entity) = sim.entities.get_mut(snap.entity_id) {
        entity.display_type_override = None;
    }

    snap.miner.dock_phase = RefineryDockPhase::Departing;
}

fn phase_departing(sim: &mut Simulation, snap: &mut MinerSnapshot, exit: (u16, u16)) {
    let moving = sim
        .entities
        .get(snap.entity_id)
        .is_some_and(|e| e.movement_target.is_some());
    let at_exit = (snap.rx, snap.ry) == exit;
    let teleporting = sim
        .entities
        .get(snap.entity_id)
        .is_some_and(|e| e.teleport_state.is_some());

    if !moving && !at_exit {
        movement::issue_direct_move(&mut sim.entities, snap.entity_id, exit, snap.speed);
        // Exit cell sits on the foundation south edge (centroid + (-0x80,
        // +0x80) leptons). The pad→exit straight line crosses interior
        // foundation cells which are blocked in path_grid; mirror gamemd's
        // locomotor head_to (which ignores cell occupancy during dock
        // departure) by setting bypass_grid for this one move.
        if let Some(entity) = sim.entities.get_mut(snap.entity_id)
            && let Some(ref mut mt) = entity.movement_target
        {
            mt.bypass_grid = true;
        }
        return;
    }

    if !moving && at_exit && !teleporting {
        if let Some(entity) = sim.entities.get_mut(snap.entity_id) {
            entity.facing = 0x47;
        }
        snap.miner.reserved_refinery = None;
        snap.miner.dock_queued = false;
        snap.miner.forced_return = false;
        // Clear stale ore targets so SearchOre re-scans from the exit cell.
        snap.miner.target_ore_cell = None;
        snap.miner.last_harvest_cell = None;
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
