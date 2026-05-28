//! Refinery docking visual sequence — approach, link, unload, depart.
//!
//! Drives the sub-state machine (`RefineryDockPhase`) when the miner is in
//! `MinerState::Dock`. Mirrors the four-state FSM used by the original
//! game's harvester deploy mission (cases 0/1/3/4): approach the queue,
//! link onto the pad, deposit bales, then hand back to harvest scheduling.
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

use crate::map::entities::EntityCategory;
use crate::rules::ruleset::RuleSet;
use crate::sim::components::BaleDepositEvent;
use crate::sim::miner::{MinerConfig, MinerState, RefineryDockPhase, ResourceType};
use crate::sim::movement;
use crate::sim::movement::facing_class::FacingClass;
use crate::sim::movement::locomotor::MovementLayer;
use crate::sim::occupancy::OccupancyGrid;
use crate::sim::pathfinding::PathGrid;
use crate::sim::world::Simulation;
use crate::util::fixed_math::SimFixed;

use super::miner_dock::ContactAdmission;
use super::miner_system::{MinerSnapshot, effective_purifier_count};
use crate::sim::production::{credits_entry_for_owner, foundation_dimensions};
use crate::util::fixed_math::ra2_speed_to_leptons_per_second;

/// Maximum diamond-ring radius for the post-unload exit-cell spiral search.
/// gamemd's `FootClass::Find_Nearby_Passable_Cell` derives its cap from
/// `Speed + SightRange` (capped at 32). A miner-class unit lands at ~14.
/// 16 covers the same footprint with a small safety margin and still
/// terminates quickly when the area around the refinery is fully blocked.
pub(super) const EXIT_SEARCH_MAX_RADIUS: i32 = 16;

/// Target body facing on the refinery dock pad — East (0x40 in 8-bit,
/// 0x4000 in 16-bit). The miner's back faces the refinery (which sits west
/// of the dock pad), aligning the dump animation with the open-bay voxel.
/// Verified from gamemd `UnitClass::Receive_Radio` case 0x16 at 0x737430:
/// reads PrimaryFacing rate-timer, calls locomotor `Do_Turn(0x4000)` to
/// rotate toward East. Applies to both HARV and CMIN (no Teleporter gate).
const DOCK_FACING_EAST: u8 = 0x40;
const DOCK_FACING_EAST_DIR: u16 = (DOCK_FACING_EAST as u16) << 8;
const ENTER_RETRY_BASE_FRAMES: u8 = 14;
const ENTER_RETRY_JITTER_MAX_FRAMES: u32 = 2;
const MISSION_DEPLOY_FACING_WAIT_FRAMES: u8 = 5;
const MISSION_DEPLOY_UNLOAD_BASE_FRAMES: u8 = 14;
const MISSION_DEPLOY_UNLOAD_JITTER_MAX_FRAMES: u32 = 2;
const REFINERY_EXIT_FORCE_TRACK: u8 = 0x47;
const REFINERY_EXIT_FORCE_HEAD_OFFSET_X: i32 = 0;
const REFINERY_EXIT_FORCE_HEAD_OFFSET_Y: i32 = 256;

/// Helper: record a dock phase transition to the snapshot's debug buffer.
fn record_dock_phase(snap: &mut MinerSnapshot, old: RefineryDockPhase, new: RefineryDockPhase) {
    snap.debug_dock_events
        .push((format!("{:?}", old), format!("{:?}", new)));
}

fn facing8_to_dir16(facing: u8) -> u16 {
    (facing as u16) << 8
}

fn dock_pivot_accepts(dir: u16) -> bool {
    ((((dir as u32) >> 7) + 1) & 0x1FE) == 0x80
}

fn dock_pivot_rot_byte(sim: &Simulation, rules: &RuleSet, snap: &MinerSnapshot) -> u8 {
    rules
        .object_case_insensitive(sim.interner.resolve(snap.type_id))
        .map(|obj| obj.turret_rot.clamp(0, 0xFF) as u8)
        .unwrap_or(10)
}

fn schedule_enter_retry(sim: &mut Simulation, snap: &mut MinerSnapshot) {
    let jitter = sim
        .rng
        .next_range_u32_inclusive(0, ENTER_RETRY_JITTER_MAX_FRAMES) as u8;
    snap.miner.dock_enter_retry_start_frame = Some(sim.binary_frame);
    snap.miner.dock_enter_retry_duration = ENTER_RETRY_BASE_FRAMES.saturating_add(jitter);
}

fn enter_retry_due(sim: &Simulation, snap: &MinerSnapshot) -> bool {
    match snap.miner.dock_enter_retry_start_frame {
        Some(start) => {
            let elapsed = sim.binary_frame.saturating_sub(start);
            elapsed >= u32::from(snap.miner.dock_enter_retry_duration)
        }
        None => true,
    }
}

fn clear_enter_retry(snap: &mut MinerSnapshot) {
    snap.miner.dock_enter_retry_start_frame = None;
    snap.miner.dock_enter_retry_duration = 0;
}

fn schedule_mission_deploy_delay(snap: &mut MinerSnapshot, frame: u32, duration: u8) {
    snap.miner.mission_deploy_start_frame = Some(frame);
    snap.miner.mission_deploy_duration = duration;
}

fn mission_deploy_due(sim: &Simulation, snap: &MinerSnapshot) -> bool {
    match snap.miner.mission_deploy_start_frame {
        Some(start) => {
            sim.binary_frame.saturating_sub(start) >= u32::from(snap.miner.mission_deploy_duration)
        }
        None => true,
    }
}

fn clear_mission_deploy_delay(snap: &mut MinerSnapshot) {
    snap.miner.mission_deploy_start_frame = None;
    snap.miner.mission_deploy_duration = 0;
}

fn clear_unload_timer_cluster(snap: &mut MinerSnapshot) {
    snap.miner.unload_accumulator = 0;
    snap.miner.unload_timer_fired = false;
    snap.miner.unload_cluster_start_frame = None;
    snap.miner.unload_cluster_scratch = 0;
    snap.miner.unload_cluster_duration = 0;
    snap.miner.unload_cluster_repeat = 0;
}

fn clear_unload_cluster(snap: &mut MinerSnapshot) {
    snap.miner.unload_active = false;
    clear_unload_timer_cluster(snap);
}

fn mark_refinery_contact(sim: &mut Simulation, miner_id: u64, ref_sid: u64) {
    if let Some(entity) = sim.entities.get_mut(miner_id) {
        entity.mark_live_contact_with(ref_sid);
    }
}

fn clear_refinery_contact(sim: &mut Simulation, miner_id: u64, ref_sid: u64) {
    if let Some(entity) = sim.entities.get_mut(miner_id) {
        entity.clear_live_contact_with(ref_sid);
    }
}

fn tick_unload_accumulator(sim: &Simulation, snap: &mut MinerSnapshot) {
    let Some(start) = snap.miner.unload_cluster_start_frame else {
        snap.miner.unload_timer_fired = false;
        return;
    };
    if snap.miner.unload_cluster_repeat == 0 {
        snap.miner.unload_timer_fired = false;
        return;
    }
    let elapsed = sim.binary_frame.saturating_sub(start);
    if elapsed < snap.miner.unload_cluster_duration {
        snap.miner.unload_timer_fired = false;
        return;
    }

    snap.miner.unload_accumulator = snap
        .miner
        .unload_accumulator
        .saturating_add(snap.miner.unload_accumulator_step);
    snap.miner.unload_timer_fired = true;
    snap.miner.unload_cluster_start_frame = Some(sim.binary_frame);
    snap.miner.unload_cluster_scratch = 0;
    snap.miner.unload_cluster_duration = snap.miner.unload_cluster_repeat;
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
/// to the stock refinery pad opened by the live building object-list scan:
/// `(+3 cells, +1 cell)` from the NW corner.
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
        let _ = (width, height);
        (rx.saturating_add(3), ry.saturating_add(1))
    }
}

/// Conditional reciprocal-link exit cell.
///
/// Stock zero-link refinery unload completion does not use this helper:
/// `UnitClass::Mission_Deploy_Building` state 4 queues/continues Harvest
/// without installing a new passable-cell destination. This remains for
/// conditional reciprocal-link/interrupt modelling and for legacy tests
/// that pin the old helper's geometry.
///
/// `find_nearby_passable_cell_with_index` provides a fallback when
/// the queue cell is blocked (e.g., another miner already waiting
/// there): ring 1+ picks an adjacent cell, typically still east of
/// the foundation. Falls back to the art.ini `QueueingCell`
/// (or the geometric default from [`refinery_queue_cell`]) when no
/// passable cell exists within [`EXIT_SEARCH_MAX_RADIUS`] or no path
/// grid is available.
///
#[cfg(test)]
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
/// candidate-collecting variant used by conditional reciprocal-link release
/// geometry.
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
/// same cell every time, which is visually fine when the anchor itself is
/// passable, but produces "miner always exits at the same spot" drift when
/// a conditional release anchor is blocked and several ring-1 candidates
/// exist.
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

/// Resolve a refinery entity's foundation and compute stock dock cells.
/// Returns `(wait_queue, accepted_cell, pad, dock_capacity)` or `None` if the
/// refinery is gone.
fn resolve_refinery_cells(
    sim: &Simulation,
    rules: &RuleSet,
    ref_sid: u64,
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
    let wait_queue = refinery_queue_cell(rx, ry, w, h, qc);
    Some((
        wait_queue,
        refinery_can_dock_queue_cell(rx, ry),
        refinery_pad_cell(rx, ry, w, h, dock_off),
        dock_capacity,
    ))
}

/// Look up the UnloadingClass for a miner type from rules.ini.
fn unloading_class(rules: &RuleSet, type_id: &str) -> Option<String> {
    rules
        .object_case_insensitive(type_id)
        .and_then(|obj| obj.unloading_class.clone())
}

fn mission_deploy_unload_building(sim: &Simulation, miner_id: u64) -> Option<u64> {
    let miner = sim.entities.get(miner_id)?;
    if miner.position.rx == 0 {
        return None;
    }
    let lookup_rx = miner.position.rx - 1;
    let lookup_ry = miner.position.ry;
    let layer = miner
        .occupancy_list_layer()
        .unwrap_or(MovementLayer::Ground);
    sim.occupancy
        .get(lookup_rx, lookup_ry)?
        .iter_layer(layer)
        .find_map(|occupant| {
            let entity = sim.entities.get(occupant.entity_id)?;
            if entity.category == EntityCategory::Structure
                && !entity.dying
                && entity.health.current > 0
            {
                Some(entity.stable_id)
            } else {
                None
            }
        })
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

fn dock_abort_state_from_miner(miner: &super::Miner) -> MinerState {
    if miner.forced_return && !miner.cargo.is_empty() {
        MinerState::ForcedReturn
    } else if miner.is_full() {
        MinerState::ReturnToRefinery
    } else {
        MinerState::SearchOre
    }
}

fn start_refinery_exit_force_track(
    entity: &mut crate::sim::game_entity::GameEntity,
    speed: SimFixed,
) -> bool {
    let Some(forced) = movement::drive_track::begin_forced_turn_track(
        REFINERY_EXIT_FORCE_TRACK,
        REFINERY_EXIT_FORCE_HEAD_OFFSET_X,
        REFINERY_EXIT_FORCE_HEAD_OFFSET_Y,
        speed,
        false,
    ) else {
        return false;
    };
    entity.drive_track = None;
    entity.forced_drive_track = Some(forced);
    entity.facing_target = None;
    true
}

fn entity_full_speed(sim: &Simulation, rules: &RuleSet, entity_id: u64) -> SimFixed {
    sim.entities
        .get(entity_id)
        .and_then(|entity| rules.object_case_insensitive(sim.interner.resolve(entity.type_ref)))
        .map(|obj| ra2_speed_to_leptons_per_second(obj.speed.max(1)))
        .unwrap_or_else(|| ra2_speed_to_leptons_per_second(4))
}

/// Apply gamemd's interrupt `BuildingClass::UndockUnit` shape for miners that
/// are actually linked to this refinery before the building is removed.
pub(crate) fn interrupt_refinery_docked_miners(
    sim: &mut Simulation,
    rules: &RuleSet,
    ref_sid: u64,
) -> usize {
    let candidates: Vec<(u64, bool)> = sim
        .entities
        .keys_sorted()
        .iter()
        .copied()
        .filter_map(|entity_id| {
            let Some(entity) = sim.entities.get(entity_id) else {
                return None;
            };
            let Some(miner) = entity.miner.as_ref() else {
                return None;
            };
            if miner.reserved_refinery != Some(ref_sid) || miner.state != MinerState::Dock {
                return None;
            }
            let is_on_pad = sim
                .production
                .dock_reservations
                .is_on_pad(ref_sid, entity_id);
            let has_contact = sim
                .production
                .dock_reservations
                .has_contact(ref_sid, entity_id);
            if is_on_pad || has_contact {
                Some((entity_id, is_on_pad))
            } else {
                None
            }
        })
        .collect();

    let mut interrupted = 0;
    for (entity_id, was_on_pad) in candidates {
        sim.production
            .dock_reservations
            .cancel_miner(ref_sid, entity_id);
        clear_refinery_contact(sim, entity_id, ref_sid);
        let speed = entity_full_speed(sim, rules, entity_id);
        let Some(entity) = sim.entities.get_mut(entity_id) else {
            continue;
        };
        let Some(miner) = entity.miner.as_mut() else {
            continue;
        };
        let next_state = dock_abort_state_from_miner(miner);
        miner.reserved_refinery = None;
        miner.dock_queued = false;
        miner.dock_phase = RefineryDockPhase::Approach;
        miner.dock_pivot_facing = None;
        miner.dock_enter_retry_start_frame = None;
        miner.dock_enter_retry_duration = 0;
        miner.mission_deploy_start_frame = None;
        miner.mission_deploy_duration = 0;
        miner.unload_active = false;
        miner.unload_accumulator = 0;
        miner.unload_timer_fired = false;
        miner.unload_cluster_start_frame = None;
        miner.unload_cluster_scratch = 0;
        miner.unload_cluster_duration = 0;
        miner.unload_cluster_repeat = 0;
        miner.deposit_cooldown_ticks = 0;
        miner.exit_cell = None;
        miner.unload_timer = 0;
        if miner.is_full() {
            miner.target_ore_cell = None;
        }
        miner.state = next_state;

        entity.display_type_override = None;
        entity.movement_target = None;
        entity.drive_track = None;
        if was_on_pad && start_refinery_exit_force_track(entity, speed) {
            interrupted += 1;
        }
    }
    interrupted
}

fn abort_invalid_refinery(sim: &mut Simulation, snap: &mut MinerSnapshot, ref_sid: Option<u64>) {
    if let Some(ref_sid) = ref_sid {
        sim.production
            .dock_reservations
            .cancel_miner(ref_sid, snap.entity_id);
        clear_refinery_contact(sim, snap.entity_id, ref_sid);
    }

    if let Some(entity) = sim.entities.get_mut(snap.entity_id) {
        entity.display_type_override = None;
        entity.facing_target = None;
        entity.movement_target = None;
        entity.drive_track = None;
        entity.forced_drive_track = None;
    }

    snap.miner.reserved_refinery = None;
    snap.miner.dock_queued = false;
    snap.miner.dock_phase = RefineryDockPhase::Approach;
    snap.miner.dock_pivot_facing = None;
    clear_enter_retry(snap);
    clear_mission_deploy_delay(snap);
    clear_unload_cluster(snap);
    snap.miner.deposit_cooldown_ticks = 0;
    snap.miner.exit_cell = None;
    snap.miner.unload_timer = 0;
    if snap.miner.is_full() {
        snap.miner.target_ore_cell = None;
    }
    snap.miner.state = dock_abort_state(snap);
}

fn abort_missing_unload_building(sim: &mut Simulation, snap: &mut MinerSnapshot, ref_sid: u64) {
    sim.production
        .dock_reservations
        .cancel_miner(ref_sid, snap.entity_id);
    clear_refinery_contact(sim, snap.entity_id, ref_sid);

    if let Some(entity) = sim.entities.get_mut(snap.entity_id) {
        entity.facing_target = None;
        entity.movement_target = None;
        entity.drive_track = None;
        entity.forced_drive_track = None;
        snap.rx = entity.position.rx;
        snap.ry = entity.position.ry;
    }

    snap.miner.reserved_refinery = None;
    snap.miner.dock_queued = false;
    snap.miner.dock_phase = RefineryDockPhase::Approach;
    snap.miner.dock_pivot_facing = None;
    clear_enter_retry(snap);
    clear_mission_deploy_delay(snap);
    clear_unload_timer_cluster(snap);
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

    match snap.miner.dock_phase {
        RefineryDockPhase::Approach => {
            let Some((wait_queue, _accepted_cell, _pad, dock_capacity)) =
                resolve_refinery_cells(sim, rules, ref_sid)
            else {
                abort_invalid_refinery(sim, snap, Some(ref_sid));
                if phase_before != snap.miner.dock_phase {
                    record_dock_phase(snap, phase_before, snap.miner.dock_phase);
                }
                return;
            };
            phase_approach(sim, path_grid, snap, wait_queue, ref_sid, dock_capacity);
        }
        RefineryDockPhase::MissionEnter => {
            let Some((wait_queue, accepted_cell, _pad, dock_capacity)) =
                resolve_refinery_cells(sim, rules, ref_sid)
            else {
                abort_invalid_refinery(sim, snap, Some(ref_sid));
                if phase_before != snap.miner.dock_phase {
                    record_dock_phase(snap, phase_before, snap.miner.dock_phase);
                }
                return;
            };
            phase_mission_enter(
                sim,
                rules,
                path_grid,
                snap,
                wait_queue,
                accepted_cell,
                ref_sid,
                dock_capacity,
            );
        }
        RefineryDockPhase::AwaitingAcceptedCell => {
            phase_awaiting_accepted_cell(sim, snap);
        }
        RefineryDockPhase::FaceSync => {
            if resolve_refinery_cells(sim, rules, ref_sid).is_none() {
                abort_invalid_refinery(sim, snap, Some(ref_sid));
                if phase_before != snap.miner.dock_phase {
                    record_dock_phase(snap, phase_before, snap.miner.dock_phase);
                }
                return;
            }
            phase_face_sync(sim, rules, snap, ref_sid);
        }
        RefineryDockPhase::MissionQueued => {
            phase_mission_queued(snap);
        }
        RefineryDockPhase::Pivoting => {
            if resolve_refinery_cells(sim, rules, ref_sid).is_none() {
                abort_invalid_refinery(sim, snap, Some(ref_sid));
                if phase_before != snap.miner.dock_phase {
                    record_dock_phase(snap, phase_before, snap.miner.dock_phase);
                }
                return;
            }
            phase_pivoting(sim, rules, config, snap, ref_sid);
        }
        RefineryDockPhase::Unloading => {
            phase_unloading(sim, rules, config, snap, ref_sid);
        }
        RefineryDockPhase::DepositCooldown => {
            phase_deposit_cooldown(snap);
        }
        RefineryDockPhase::Departing => {
            phase_departing(sim, rules, snap, ref_sid);
        }
    }

    tick_unload_accumulator(sim, snap);

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
    wait_queue: (u16, u16),
    ref_sid: u64,
    dock_capacity: usize,
) {
    // Mission_Harvest state 2 sends only HELLO(0x02). On ROGER it queues
    // Mission_Enter for the next tick instead of jumping straight to the
    // accepted-cell move or unload pivot.
    let admission =
        sim.production
            .dock_reservations
            .hello_or_wait(ref_sid, snap.entity_id, dock_capacity);

    snap.miner.dock_queued = admission != ContactAdmission::Accepted;
    if admission == ContactAdmission::Accepted {
        mark_refinery_contact(sim, snap.entity_id, ref_sid);
        snap.miner.dock_phase = RefineryDockPhase::MissionEnter;
    }

    // Reservation not granted — keep heading toward QueueingCell.
    if admission != ContactAdmission::Accepted && !is_adjacent_or_at((snap.rx, snap.ry), wait_queue)
    {
        if let Some(grid) = path_grid {
            issue_move_if_idle(
                &mut sim.entities,
                grid,
                snap.entity_id,
                wait_queue,
                snap.speed,
            );
        }
    }
}

fn phase_mission_enter(
    sim: &mut Simulation,
    rules: &RuleSet,
    path_grid: Option<&PathGrid>,
    snap: &mut MinerSnapshot,
    wait_queue: (u16, u16),
    accepted_cell: (u16, u16),
    ref_sid: u64,
    dock_capacity: usize,
) {
    if !enter_retry_due(sim, snap) {
        return;
    }

    let admission =
        sim.production
            .dock_reservations
            .hello_or_wait(ref_sid, snap.entity_id, dock_capacity);
    if admission == ContactAdmission::Accepted {
        mark_refinery_contact(sim, snap.entity_id, ref_sid);
    }
    let already_entered = sim
        .production
        .dock_reservations
        .has_contact_entered(ref_sid, snap.entity_id);
    let pad_clear_or_self = !sim.production.dock_reservations.pad_occupied(ref_sid)
        || sim
            .production
            .dock_reservations
            .is_on_pad(ref_sid, snap.entity_id);
    if admission != ContactAdmission::Accepted && !already_entered {
        snap.miner.dock_queued = true;
        if !is_adjacent_or_at((snap.rx, snap.ry), wait_queue) {
            if let Some(grid) = path_grid {
                issue_move_if_idle(
                    &mut sim.entities,
                    grid,
                    snap.entity_id,
                    wait_queue,
                    snap.speed,
                );
            }
        }
        schedule_enter_retry(sim, snap);
        return;
    }

    let can_start_enter_handshake =
        (admission == ContactAdmission::Accepted || already_entered) && pad_clear_or_self;
    snap.miner.dock_queued =
        (admission != ContactAdmission::Accepted && !already_entered) || !pad_clear_or_self;

    let moving = sim
        .entities
        .get(snap.entity_id)
        .is_some_and(|e| e.movement_target.is_some());

    if (snap.rx, snap.ry) == accepted_cell && !moving {
        if can_start_enter_handshake {
            sim.production
                .dock_reservations
                .mark_contact_entered(ref_sid, snap.entity_id);
            sync_dock_facing(sim, rules, snap);
            snap.miner.dock_phase = RefineryDockPhase::FaceSync;
        }
        schedule_enter_retry(sim, snap);
        return;
    }

    if !moving {
        // Building 0x0E sends 0x12 with anchor+(3,1). The accepted cell is
        // inside the refinery footprint for stock GAREFN/NAREFN, so use the
        // direct move path already used for refinery pad entry.
        if movement::issue_direct_move(&mut sim.entities, snap.entity_id, accepted_cell, snap.speed)
        {
            if let Some(target) = sim
                .entities
                .get_mut(snap.entity_id)
                .and_then(|entity| entity.movement_target.as_mut())
            {
                target.bypass_grid = true;
            }
        }
    }
    schedule_enter_retry(sim, snap);
    snap.miner.dock_phase = RefineryDockPhase::AwaitingAcceptedCell;
}

fn phase_awaiting_accepted_cell(sim: &mut Simulation, snap: &mut MinerSnapshot) {
    let moving = sim
        .entities
        .get(snap.entity_id)
        .is_some_and(|e| e.movement_target.is_some());
    if moving {
        return;
    }

    if let Some(entity) = sim.entities.get(snap.entity_id) {
        snap.rx = entity.position.rx;
        snap.ry = entity.position.ry;
    }

    // Movement completion is not enough to start the dock pivot. The next
    // Mission_Enter/CAN_DOCK pass must receive 0x12 as already-there before
    // the building emits the 0x18/0x16 entered/pivot handshake.
    snap.miner.dock_phase = RefineryDockPhase::MissionEnter;
}

fn phase_face_sync(sim: &mut Simulation, rules: &RuleSet, snap: &mut MinerSnapshot, ref_sid: u64) {
    let arrived = sim
        .entities
        .get(snap.entity_id)
        .is_some_and(|e| e.movement_target.is_none());
    if !arrived {
        return;
    }

    if let Some(entity) = sim.entities.get(snap.entity_id) {
        snap.rx = entity.position.rx;
        snap.ry = entity.position.ry;
    }

    sim.production
        .dock_reservations
        .mark_contact_entered(ref_sid, snap.entity_id);

    let accepted = sync_dock_facing(sim, rules, snap);
    if !enter_retry_due(sim, snap) {
        return;
    }

    if accepted {
        clear_enter_retry(snap);
        snap.miner.dock_phase = RefineryDockPhase::MissionQueued;
    } else {
        schedule_enter_retry(sim, snap);
    }
}

fn phase_mission_queued(snap: &mut MinerSnapshot) {
    snap.miner.dock_phase = RefineryDockPhase::Pivoting;
}

fn sync_dock_facing(sim: &mut Simulation, rules: &RuleSet, snap: &mut MinerSnapshot) -> bool {
    let rot = dock_pivot_rot_byte(sim, rules, snap);
    let binary_frame = sim.binary_frame;
    let Some(entity) = sim.entities.get(snap.entity_id) else {
        return false;
    };

    if snap.miner.dock_pivot_facing.is_none() {
        let mut pivot = FacingClass::new(facing8_to_dir16(entity.facing), rot);
        pivot.set(DOCK_FACING_EAST_DIR, binary_frame);
        snap.miner.dock_pivot_facing = Some(pivot);
    }

    let pivot = snap
        .miner
        .dock_pivot_facing
        .as_mut()
        .expect("pivot timer initialized");
    pivot.set_rot(rot);
    pivot.set(DOCK_FACING_EAST_DIR, binary_frame);

    let current_dir = pivot.current(binary_frame);
    dock_pivot_accepts(current_dir)
}

fn start_unload_deploy(sim: &mut Simulation, rules: &RuleSet, snap: &mut MinerSnapshot) {
    if let Some(entity) = sim.entities.get_mut(snap.entity_id) {
        if let Some(uc) = unloading_class(rules, sim.interner.resolve(snap.type_id)) {
            entity.display_type_override = Some(sim.interner.intern(&uc));
        }
        entity.facing_target = None;
    }

    snap.miner.unload_active = true;
    snap.miner.unload_accumulator = 0;
    snap.miner.unload_timer_fired = false;
    snap.miner.unload_cluster_start_frame = Some(sim.binary_frame);
    snap.miner.unload_cluster_scratch = 0;
    snap.miner.unload_cluster_duration = 1;
    snap.miner.unload_cluster_repeat = 1;
    snap.miner.unload_timer = 0;
    snap.miner.dock_phase = RefineryDockPhase::Unloading;
}

/// Smooth in-place rotation toward DOCK_FACING_EAST (0x40). gamemd starts
/// this via radio 0x16 by calling the locomotor's `Do_Turn(0x4000)`, then
/// accepts deploy once the 16-bit PrimaryFacing timer enters the same
/// quantized East-facing window.
fn phase_pivoting(
    sim: &mut Simulation,
    rules: &RuleSet,
    config: &MinerConfig,
    snap: &mut MinerSnapshot,
    ref_sid: u64,
) {
    if !mission_deploy_due(sim, snap) {
        return;
    }

    if sync_dock_facing(sim, rules, snap) {
        // Mission 0x10 has reached its facing gate. Radio 0x15 only queued
        // that mission; unload-active effects begin here.
        snap.miner.dock_pivot_facing = None;
        start_unload_deploy(sim, rules, snap);
        let jitter = sim
            .rng
            .next_range_u32_inclusive(0, MISSION_DEPLOY_UNLOAD_JITTER_MAX_FRAMES)
            as u8;
        schedule_mission_deploy_delay(
            snap,
            sim.binary_frame,
            MISSION_DEPLOY_UNLOAD_BASE_FRAMES.saturating_add(jitter),
        );
    } else {
        let _ = config;
        let _ = ref_sid;
        schedule_mission_deploy_delay(snap, sim.binary_frame, MISSION_DEPLOY_FACING_WAIT_FRAMES);
        snap.miner.dock_phase = RefineryDockPhase::Pivoting;
    }
}

fn phase_unloading(
    sim: &mut Simulation,
    rules: &RuleSet,
    config: &MinerConfig,
    snap: &mut MinerSnapshot,
    ref_sid: u64,
) {
    if !snap.miner.unload_active && snap.miner.unload_cluster_start_frame.is_none() {
        // Compatibility for old saves and tests that entered Unloading before
        // the byte-field cluster existed.
        snap.miner.unload_active = true;
        snap.miner.unload_accumulator = i32::from(config.unload_tick_interval.div_ceil(10));
        snap.miner.unload_cluster_start_frame = Some(sim.binary_frame);
        snap.miner.unload_cluster_duration = 1;
        snap.miner.unload_cluster_repeat = 1;
    }

    if !mission_deploy_due(sim, snap) {
        return;
    }

    if snap.miner.unload_accumulator.saturating_mul(10) < i32::from(config.unload_tick_interval) {
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
        let Some(unload_building_id) = mission_deploy_unload_building(sim, snap.entity_id) else {
            abort_missing_unload_building(sim, snap, ref_sid);
            return;
        };

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
            .get(unload_building_id)
            .map(|b| sim.interner.resolve(b.owner).to_string())
            .expect("west-cell unload building should exist");

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
            building_id: unload_building_id,
            tick: sim.tick,
        });

        snap.miner.unload_accumulator = 0;
        schedule_mission_deploy_delay(snap, sim.binary_frame, 1);
        return;
    }

    // Cargo empty at a dump-gate crossing: `FindFirstNonEmptySlot` has
    // returned -1, so stock Mission_Deploy_Building state 3 advances to
    // state 4. Do not seed another dump-gate cooldown here; the due
    // mission-deploy delay and accumulator gate have already fired.
    snap.miner.home_refinery = mission_deploy_unload_building(sim, snap.entity_id);
    snap.miner.deposit_cooldown_ticks = 0;
    schedule_mission_deploy_delay(snap, sim.binary_frame, 1);
    snap.miner.dock_phase = RefineryDockPhase::Departing;
}

/// Legacy/pass-through phase retained for older save/test states. Stock
/// unload reaches Departing directly from the empty-slot gate in
/// `phase_unloading`.
fn phase_deposit_cooldown(snap: &mut MinerSnapshot) {
    if snap.miner.deposit_cooldown_ticks > 0 {
        snap.miner.deposit_cooldown_ticks -= 1;
        return;
    }
    // `phase_departing` owns the stock state-4 cleanup and clears the
    // unloading sprite override during that handoff.
    snap.miner.dock_phase = RefineryDockPhase::Departing;
}

fn phase_departing(sim: &mut Simulation, _rules: &RuleSet, snap: &mut MinerSnapshot, ref_sid: u64) {
    let teleporting = sim
        .entities
        .get(snap.entity_id)
        .is_some_and(|e| e.teleport_state.is_some());
    if teleporting {
        return;
    }

    // Stock CMIN/HARV -> GAREFN/NAREFN completion is the zero-link
    // Mission_Deploy_Building state-4 branch: clear the unload-active
    // bookkeeping and hand directly back to Harvest/SearchOre scheduling.
    // Do not seed ReleaseDockedHarvester effects here: no Force_Track(0x47),
    // no BunkerWallsDownSound, and no cached queue-cell destination.
    sim.production
        .dock_reservations
        .release_on_pad(ref_sid, snap.entity_id);
    sim.production
        .dock_reservations
        .release_contact(ref_sid, snap.entity_id);
    clear_refinery_contact(sim, snap.entity_id, ref_sid);

    if let Some(entity) = sim.entities.get_mut(snap.entity_id) {
        entity.display_type_override = None;
        entity.movement_target = None;
        entity.drive_track = None;
        entity.forced_drive_track = None;
        entity.facing_target = None;
        snap.rx = entity.position.rx;
        snap.ry = entity.position.ry;
    }

    snap.miner.reserved_refinery = None;
    snap.miner.dock_queued = false;
    snap.miner.forced_return = false;
    snap.miner.dock_pivot_facing = None;
    clear_enter_retry(snap);
    clear_mission_deploy_delay(snap);
    clear_unload_cluster(snap);
    snap.miner.deposit_cooldown_ticks = 0;
    // Clear the pending ore target and stale exit cache. Preserve
    // `last_harvest_cell`; the ghost-cell archive survives the dock cycle
    // so the next `SearchOre` can return to the productive patch saved when
    // this miner became full.
    snap.miner.target_ore_cell = None;
    snap.miner.exit_cell = None;
    snap.miner.dock_phase = RefineryDockPhase::Approach;
    snap.miner.state = MinerState::SearchOre;
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
