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

use crate::rules::art_data::BuildingAnimKind;
use crate::rules::ruleset::RuleSet;
use crate::sim::components::BaleDepositEvent;
use crate::sim::miner::{MinerConfig, MinerState, RefineryDockPhase};
use crate::sim::movement;
use crate::sim::movement::locomotor::MovementLayer;
use crate::sim::occupancy::OccupancyGrid;
use crate::sim::pathfinding::PathGrid;
use crate::sim::world::{SimSoundEvent, Simulation};
use crate::util::fixed_math::{SIM_TICK_HZ, SimFixed};

use super::miner_system::{MinerSnapshot, effective_purifier_count};
use crate::sim::production::{credits_entry_for_owner, foundation_dimensions};

/// Maximum diamond-ring radius for the post-unload exit-cell spiral search.
/// gamemd's `FootClass::Find_Nearby_Passable_Cell` derives its cap from
/// `Speed + SightRange` (capped at 32). A miner-class unit lands at ~14.
/// 16 covers the same footprint with a small safety margin and still
/// terminates quickly when the area around the refinery is fully blocked.
const EXIT_SEARCH_MAX_RADIUS: i32 = 16;

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
/// Mirrors gamemd's `ReleaseDockedHarvester` (0x4595C0): the harvester is
/// released next to the refinery's west edge (foundation top-left minus one
/// X, plus one Y) and `FootClass::Find_Nearby_Passable_Cell` then picks the
/// nearest in-bounds, passable, unoccupied cell starting from that anchor.
/// The shape works uniformly for every refinery foundation (4×3 GAREFN,
/// 2×2 YAREFN, 3×3 Slave Miner refinery) because the anchor offset is
/// foundation-relative and the spiral expands outward from there.
///
/// Returns the first passable cell found in a CW diamond-ring spiral.
/// Falls back to the art.ini `QueueingCell` (or the geometric default from
/// [`refinery_queue_cell`]) if no passable cell exists within
/// [`EXIT_SEARCH_MAX_RADIUS`], or if no grid is available.
pub(super) fn refinery_exit_cell(
    rx: u16,
    ry: u16,
    width: u16,
    height: u16,
    queueing_cell: Option<(u16, u16)>,
    path_grid: Option<&PathGrid>,
    occupancy: Option<&OccupancyGrid>,
) -> (u16, u16) {
    let anchor_x = rx as i32 - 1;
    let anchor_y = ry as i32 + 1;

    if let Some(grid) = path_grid {
        if let Some(cell) =
            find_nearby_passable_cell(anchor_x, anchor_y, grid, occupancy, EXIT_SEARCH_MAX_RADIUS)
        {
            return cell;
        }
    }

    refinery_queue_cell(rx, ry, width, height, queueing_cell)
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
/// cell encountered (a simplification of the original's 24-candidate +
/// frame-modulo tie-break — we just need one valid drop-off, not a randomized
/// pick among many).
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
        refinery_exit_cell(rx, ry, w, h, qc, path_grid, Some(&sim.occupancy)),
    ))
}

/// Look up the UnloadingClass for a miner type from rules.ini.
fn unloading_class(rules: &RuleSet, type_id: &str) -> Option<String> {
    rules
        .object_case_insensitive(type_id)
        .and_then(|obj| obj.unloading_class.clone())
}

/// Resolve the refinery's deposit-anim cycle length, in sim ticks.
///
/// Picks the longest SpecialAnim slot on the building (multiple slots
/// are allowed in art.ini; the cooldown must outlast the latest one).
/// Returns 0 when the refinery type has no SpecialAnim — in that case
/// `DepositCooldown` falls through to `Departing` on its first tick.
fn deposit_anim_duration_ticks(sim: &Simulation, rules: &RuleSet, refinery_sid: u64) -> u16 {
    let Some(building) = sim.entities.get(refinery_sid) else {
        return 0;
    };
    let type_str = sim.interner.resolve(building.type_ref);
    let Some(obj) = rules.object_case_insensitive(type_str) else {
        return 0;
    };
    let Some(art_entry) = rules
        .art_registry
        .resolve_metadata_entry(type_str, &obj.image)
    else {
        return 0;
    };
    let max_ms: u32 = art_entry
        .building_anims
        .iter()
        .filter(|a| matches!(a.kind, BuildingAnimKind::Special))
        .filter(|a| a.loop_end > a.loop_start)
        .map(|a| u32::from(a.loop_end - a.loop_start) * u32::from(a.rate))
        .max()
        .unwrap_or(0);
    if max_ms == 0 {
        return 0;
    }
    // SIM_TICK_HZ ticks/sec → 1000/SIM_TICK_HZ ms/tick. Round UP so a
    // partial-tick remainder still completes the anim before the hold
    // ends.
    let tick_ms: u32 = 1000 / SIM_TICK_HZ;
    let ticks: u32 = max_ms.div_ceil(tick_ms.max(1));
    ticks.min(u16::MAX as u32) as u16
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

    let Some((queue, pad, exit)) = resolve_refinery_cells(sim, rules, ref_sid, path_grid) else {
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
            phase_linked(sim, rules, config, snap, pad, ref_sid);
        }
        RefineryDockPhase::Unloading => {
            phase_unloading(sim, rules, config, snap, ref_sid);
        }
        RefineryDockPhase::DepositCooldown => {
            phase_deposit_cooldown(sim, snap);
        }
        RefineryDockPhase::Departing => {
            phase_departing(sim, path_grid, snap, exit, ref_sid);
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
    config: &MinerConfig,
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

    // First dock bale must wait ceil(14.4) = 15 frames after dock-link,
    // matching the per-bale gate where the dump counter starts at 0 on
    // dock-link and a bale deposits only once the counter reaches
    // HarvesterDumpRate × 900 = 14.4. With phase_unloading's
    // decrement-then-check structure (`if timer > 0 { timer -= 10;
    // return; }` before the pop), the pop fires on the tick when the
    // timer crosses ≤ 0. Initialising the timer one decrement step below
    // the full interval (`interval - 10`) lines that crossing up with
    // tick 15 after the Linked transition. Previously initialised to 0,
    // which dropped the first bale instantly.
    snap.miner.unload_timer = (config.unload_tick_interval as i16).saturating_sub(10);
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

        // Purifier bonus = real_purifiers + AI_virtual_purifiers, all
        // sourced from the refinery's owner. Bonus credits are routed to
        // the miner's owner alongside the base credits — matching the
        // existing credit-destination convention. The per-bale split here
        // is a downstream of the per-bale cargo model (Finding 1 will
        // collapse this into one whole-slot dump); the formula itself is
        // count-correct as of this fix.
        let refinery_owner_id = sim.entities.get(ref_sid).map(|b| b.owner);
        let refinery_owner: String = refinery_owner_id
            .map(|id| sim.interner.resolve(id).to_string())
            .unwrap_or_else(|| owner_str.clone());
        let purifier_count = effective_purifier_count(sim, rules, &refinery_owner);
        if purifier_count > 0 {
            let bonus_pct: i32 = rules.general.purifier_bonus_pct;
            let bonus: i32 = value
                .saturating_mul(purifier_count)
                .saturating_mul(bonus_pct)
                / 100;
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

    // Cargo empty — seed the deposit-anim cooldown and transition. The
    // dock reservation and the unloading-model override are both held
    // through DepositCooldown so the visual matches a miner still parked
    // on the pad while the deposit animation finishes. Both are released
    // by `phase_departing` once the miner reaches the exit cell.
    snap.miner.home_refinery = Some(ref_sid);
    snap.miner.deposit_cooldown_ticks = deposit_anim_duration_ticks(sim, rules, ref_sid);
    snap.miner.dock_phase = RefineryDockPhase::DepositCooldown;
}

/// Hold on the pad until the last deposit animation completes, then
/// hand off to `Departing`.
fn phase_deposit_cooldown(sim: &mut Simulation, snap: &mut MinerSnapshot) {
    if snap.miner.deposit_cooldown_ticks > 0 {
        snap.miner.deposit_cooldown_ticks -= 1;
        return;
    }
    if let Some(entity) = sim.entities.get_mut(snap.entity_id) {
        entity.display_type_override = None;
    }
    snap.miner.dock_phase = RefineryDockPhase::Departing;
}

fn phase_departing(
    sim: &mut Simulation,
    path_grid: Option<&PathGrid>,
    snap: &mut MinerSnapshot,
    exit: (u16, u16),
    ref_sid: u64,
) {
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
        // Pad→exit drive: the exit is outside the foundation but the pad is
        // on the foundation east edge. A* must route AROUND the foundation
        // (south or north side) — `astar_search` accepts a blocked start
        // cell, so pathing out of the pad works even though the pad is
        // inside the building footprint.
        if let Some(grid) = path_grid {
            let _ = movement::issue_move_command(
                &mut sim.entities,
                grid,
                snap.entity_id,
                exit,
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
            movement::issue_direct_move(&mut sim.entities, snap.entity_id, exit, snap.speed);
        }
        // Force facing 0x47 at the START of the drive. gamemd's Force_Track
        // call in ReleaseDockedHarvester sets track_index to 0x47 before any
        // movement step, so the unit visually faces ESE throughout the exit
        // drive. The arrival branch below re-snaps for safety in case the
        // movement system rotated facing during travel.
        if let Some(entity) = sim.entities.get_mut(snap.entity_id) {
            entity.facing = 0x47;
            entity.facing_target = Some(0x47);
        }
        return;
    }

    if !moving && at_exit && !teleporting {
        if let Some(entity) = sim.entities.get_mut(snap.entity_id) {
            entity.facing = 0x47;
        }
        // Release the dock here, not at cargo-empty, so the next queued
        // miner cannot try_reserve onto a pad that the previous miner is
        // still standing on / driving off.
        sim.production.dock_reservations.release(ref_sid);
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
