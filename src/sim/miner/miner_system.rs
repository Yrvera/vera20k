//! Miner state machine tick — drives the SearchOre→Harvest→Return→Unload loop.
//!
//! Called once per sim tick from `tick_resource_economy()`. Uses the two-phase
//! snapshot pattern: snapshot all miners, process deterministically by stable_id,
//! then apply mutations back to the EntityStore.
//!
//! ## Dependency rules
//! - Part of sim/ — depends on sim/miner, sim/miner_dock, sim/components,
//!   sim/movement, sim/pathfinding, rules/.
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

use std::collections::BTreeSet;

use crate::map::entities::EntityCategory;
use crate::rules::locomotor_type::MovementZone;
use crate::rules::ruleset::RuleSet;
use crate::sim::miner::miner_dock::ContactAdmission;
use crate::sim::miner::{
    CargoBale, Miner, MinerConfig, MinerKind, MinerState, RefineryDockPhase, ResourceNode,
    ResourceType,
};
use crate::sim::movement;
use crate::sim::movement::locomotor::MovementLayer;
use crate::sim::movement::teleport_movement::issue_teleport_command;
use crate::sim::occupancy::OccupancyGrid;
use crate::sim::pathfinding::PathGrid;
use crate::sim::pathfinding::zone_map::{ZONE_INVALID, ZoneGrid};
use crate::sim::production::pick_best_resource_node;
use crate::sim::world::{SimSoundEvent, Simulation};
use crate::util::fixed_math::{SimFixed, ra2_speed_to_leptons_per_second};

use crate::sim::debug_event_log::DebugEventKind;
use crate::sim::intern::InternedId;

use crate::sim::production::foundation_dimensions;
use crate::util::lepton::LEPTONS_PER_LEVEL;

/// Chrono far-return compares object-coordinate distance in leptons against
/// `ChronoHarvTooFarDistance * 256`. The stock branch is strict `>`, so a
/// miner exactly at the threshold still uses the close radio path.
fn chrono_return_exceeds_too_far_threshold(
    sim: &Simulation,
    miner_sid: u64,
    refinery_sid: u64,
    threshold_cells: u16,
) -> Option<bool> {
    let miner = sim.entities.get(miner_sid)?;
    let refinery = sim.entities.get(refinery_sid)?;
    if refinery.dying || refinery.health.current == 0 {
        return None;
    }

    let miner_x = i64::from(miner.position.rx) * 256 + miner.position.sub_x.to_num::<i64>();
    let miner_y = i64::from(miner.position.ry) * 256 + miner.position.sub_y.to_num::<i64>();
    let miner_z = i64::from(miner.position.z) * LEPTONS_PER_LEVEL;
    let refinery_x =
        i64::from(refinery.position.rx) * 256 + refinery.position.sub_x.to_num::<i64>();
    let refinery_y =
        i64::from(refinery.position.ry) * 256 + refinery.position.sub_y.to_num::<i64>();
    let refinery_z = i64::from(refinery.position.z) * LEPTONS_PER_LEVEL;

    let dx = miner_x - refinery_x;
    let dy = miner_y - refinery_y;
    let dz = miner_z - refinery_z;
    let distance_sq = dx * dx + dy * dy + dz * dz;
    let threshold = i64::from(threshold_cells.max(1)) * 256;
    Some(distance_sq > threshold * threshold)
}

/// Snapshot of one miner entity for two-phase processing.
pub(super) struct MinerSnapshot {
    pub(super) entity_id: u64,
    pub(super) owner: InternedId,
    pub(super) type_id: InternedId,
    pub(super) rx: u16,
    pub(super) ry: u16,
    pub(super) speed: SimFixed,
    pub(super) miner: Miner,
    /// Buffered miner state change events — flushed to entity in Phase 3.
    pub(super) debug_events: Vec<(String, String)>,
    /// Buffered dock phase change events — flushed to entity in Phase 3.
    pub(super) debug_dock_events: Vec<(String, String)>,
}

/// Main entry point: tick all entities with the Miner component.
///
/// Deterministic: snapshots sorted by stable_id, mutations applied in order.
pub(crate) fn tick_miners(
    sim: &mut Simulation,
    rules: &RuleSet,
    config: &MinerConfig,
    path_grid: Option<&PathGrid>,
) {
    // Phase 1: Snapshot all miners from EntityStore.
    let keys = sim.entities.keys_sorted();
    let mut snapshots: Vec<MinerSnapshot> = Vec::new();
    for &id in &keys {
        let Some(entity) = sim.entities.get(id) else {
            continue;
        };
        let Some(ref miner) = entity.miner else {
            continue;
        };
        // Slave Miners use their own system (slave_miner.rs) — skip here.
        if miner.kind == MinerKind::Slave {
            continue;
        }
        // Use the authentic RA2 speed formula: Speed=4 → ~0.586 cells/sec.
        let raw_speed: i32 = rules
            .object_case_insensitive(sim.interner.resolve(entity.type_ref))
            .map(|obj| obj.speed.max(1))
            .unwrap_or(4);
        let speed: SimFixed = ra2_speed_to_leptons_per_second(raw_speed);
        snapshots.push(MinerSnapshot {
            entity_id: id,
            owner: entity.owner,
            type_id: entity.type_ref,
            rx: entity.position.rx,
            ry: entity.position.ry,
            speed,
            miner: miner.clone(),
            debug_events: Vec::new(),
            debug_dock_events: Vec::new(),
        });
    }
    // Already sorted by stable_id since keys_sorted() returns sorted order.
    log::debug!(
        "tick_miners: {} miners, {} resource_nodes",
        snapshots.len(),
        sim.production.resource_nodes.len(),
    );

    if snapshots.is_empty() {
        return;
    }

    // Dying entities are still in `sim.entities` until the death animation
    // finishes, but their refinery reservation must release immediately so
    // queued miners can be promoted without waiting through the death anim.
    let alive_sids: BTreeSet<u64> = sim
        .entities
        .values()
        .filter(|e| !e.dying)
        .map(|e| e.stable_id)
        .collect();
    sim.production.dock_reservations.cleanup_dead(&alive_sids);

    // Phase 2: Process each miner through its state machine.
    for snap in &mut snapshots {
        process_miner(sim, rules, config, path_grid, snap);
    }

    // Phase 3: Write miner state back to EntityStore and flush debug events.
    for snap in &snapshots {
        if let Some(entity) = sim.entities.get_mut(snap.entity_id) {
            entity.miner = Some(snap.miner.clone());
            for (from, to) in &snap.debug_events {
                entity.push_debug_event(
                    sim.tick as u32,
                    DebugEventKind::MinerStateChange {
                        from: from.clone(),
                        to: to.clone(),
                    },
                );
            }
            for (from, to) in &snap.debug_dock_events {
                entity.push_debug_event(
                    sim.tick as u32,
                    DebugEventKind::DockPhaseChange {
                        from: from.clone(),
                        to: to.clone(),
                    },
                );
            }
        }
    }

    // Phase 4: Drive VoxelAnimation playing state from miner Harvest state.
    for snap in &snapshots {
        let is_harvesting: bool = snap.miner.state == MinerState::Harvest;
        if let Some(entity) = sim.entities.get_mut(snap.entity_id) {
            if let Some(ref mut va) = entity.voxel_animation {
                va.playing = is_harvesting;
                if !is_harvesting {
                    va.frame = 0;
                    va.elapsed_ms = 0;
                }
            }
        }
    }

    // Phase 4b: Drive HarvestOverlay (oregath.shp) visibility from miner Harvest state.
    for snap in &snapshots {
        let is_harvesting: bool = snap.miner.state == MinerState::Harvest;
        if let Some(entity) = sim.entities.get_mut(snap.entity_id) {
            if let Some(ref mut ho) = entity.harvest_overlay {
                if is_harvesting && !ho.visible {
                    ho.visible = true;
                    ho.frame = 0;
                    ho.elapsed_ms = 0;
                } else if !is_harvesting && ho.visible {
                    ho.visible = false;
                    ho.frame = 0;
                    ho.elapsed_ms = 0;
                }
            }
        }
    }
}

/// Process one miner through one tick of its state machine.
fn process_miner(
    sim: &mut Simulation,
    rules: &RuleSet,
    config: &MinerConfig,
    path_grid: Option<&PathGrid>,
    snap: &mut MinerSnapshot,
) {
    if sim
        .entities
        .get(snap.entity_id)
        .is_some_and(|entity| entity.forced_drive_track.is_some())
    {
        return;
    }

    let state_before = format!("{:?}", snap.miner.state);
    match snap.miner.state {
        MinerState::SearchOre => handle_search_ore(sim, config, path_grid, snap),
        MinerState::MoveToOre => handle_move_to_ore(sim, rules, config, path_grid, snap),
        MinerState::Harvest => handle_harvest(sim, rules, config, path_grid, snap),
        MinerState::ReturnToRefinery => handle_return(sim, rules, config, path_grid, snap),
        MinerState::Dock => {
            super::miner_dock_sequence::handle_dock_sequence(sim, rules, config, path_grid, snap)
        }
        MinerState::Unload => {
            // Legacy state — production code never enters this path. If we
            // encounter it (e.g., a save from before the FSM rewrite), fall
            // through to SearchOre.
            snap.miner.state = MinerState::SearchOre;
        }
        MinerState::WaitNoOre => handle_wait_no_ore(config, snap),
        MinerState::ForcedReturn => handle_forced_return(sim, rules, config, path_grid, snap),
    }
    let state_after = format!("{:?}", snap.miner.state);
    if state_before != state_after {
        log::info!(
            "MINER {} state: {} → {} pos=({},{}) target_ore={:?} cargo={} timer={}",
            snap.entity_id,
            state_before,
            state_after,
            snap.rx,
            snap.ry,
            snap.miner.target_ore_cell,
            snap.miner.cargo.len(),
            snap.miner.harvest_timer,
        );
        snap.debug_events.push((state_before, state_after));
    }
}

// -- State handlers --

/// Build the combined scan filter — zone reachability AND cell occupancy.
///
/// Mirrors gamemd's `FootClass::Is_Cell_Harvestable`, which gates each
/// ring-1+ candidate cell through a zone-connectivity check plus a
/// per-cell `Can_Enter_Cell` call (cell occupancy: vehicles, terrain
/// objects, building footprints).
///
/// Returns `None` if no zone grid or anchor is available — caller falls
/// back to an unfiltered scan for this tick.
fn build_scan_filter<'a>(
    sim: &'a Simulation,
    path_grid: Option<&'a PathGrid>,
    snap: &MinerSnapshot,
) -> Option<Box<dyn Fn((u16, u16)) -> bool + 'a>> {
    let entity = sim.entities.get(snap.entity_id);
    let mz = entity
        .and_then(|e| e.locomotor.as_ref())
        .map(|loc| loc.movement_zone)
        .unwrap_or(MovementZone::Normal);
    let layer = entity
        .map(|e| e.movement_layer_or_ground())
        .unwrap_or(MovementLayer::Ground);
    let zone_grid = sim.zone_grid.as_ref()?;
    let anchor = effective_zone_cell(zone_grid, mz, snap.rx, snap.ry)?;
    let occupancy = &sim.occupancy;
    let self_id = snap.entity_id;

    Some(Box::new(move |ore_cell: (u16, u16)| {
        if !ore_reachable(zone_grid, mz, layer, anchor, ore_cell) {
            return false;
        }
        is_cell_path_clear_for_scan(occupancy, path_grid, ore_cell, self_id)
    }))
}

/// True if the cell has no static blocker (terrain object, building
/// footprint set in PathGrid) and no non-self vehicle/structure occupant
/// (OccupancyGrid). Infantry are not blockers.
///
/// Used by ring-1+ scan candidates only — ring 0 is always allowed (the
/// harvester is allowed to harvest its own cell even if it appears as a
/// blocker to itself).
pub(crate) fn is_cell_path_clear_for_scan(
    occupancy: &OccupancyGrid,
    path_grid: Option<&PathGrid>,
    cell: (u16, u16),
    self_id: u64,
) -> bool {
    if let Some(grid) = path_grid
        && !grid.is_walkable(cell.0, cell.1)
    {
        return false;
    }
    if let Some(occ) = occupancy.get(cell.0, cell.1) {
        let any_non_self_blocker = occ.blockers(MovementLayer::Ground).any(|id| id != self_id);
        if any_non_self_blocker {
            return false;
        }
    }
    true
}

fn handle_search_ore(
    sim: &Simulation,
    config: &MinerConfig,
    path_grid: Option<&PathGrid>,
    snap: &mut MinerSnapshot,
) {
    // gamemd's Mission_Harvest state 0 checks full storage before scanning
    // ore, so a full miner that lost its refinery keeps trying to return.
    if snap.miner.is_full() {
        snap.miner.target_ore_cell = None;
        snap.miner.state = MinerState::ReturnToRefinery;
        return;
    }

    // Combined scan filter — zone reachability + cell occupancy.
    // Returns None if zone_grid / anchor is missing; caller falls back to
    // an unfiltered scan that tick.
    let scan_filter = build_scan_filter(sim, path_grid, snap);
    let filter_ref: Option<&dyn Fn((u16, u16)) -> bool> = scan_filter.as_deref();

    // Archive ghost-cell consumption: if `last_harvest_cell` is set,
    // drive straight to it and clear. The archive is written by
    // `save_archive_via_short_scan` when the miner becomes full.
    // Reachability is re-checked because the patch may have been walled
    // off between the save and the next cycle.
    if let Some(archive) = snap.miner.last_harvest_cell {
        let archive_has_ore = sim.production.resource_nodes.contains_key(&archive);
        let archive_reachable = filter_ref.is_none_or(|f| f(archive));
        if archive_has_ore && archive_reachable {
            snap.miner.target_ore_cell = Some(archive);
            snap.miner.state = MinerState::MoveToOre;
            snap.miner.last_harvest_cell = None;
            return;
        }
        // Stale archive (depleted or unreachable) — drop it so we don't
        // keep retrying.
        snap.miner.last_harvest_cell = None;
    }

    // Long-range bounded scan from the miner's current position
    // (TiberiumLongScan). Single scan with no separate short-scan
    // pre-pass — the search expands outward and picks the best cell
    // within radius. Used for both war miners and chrono miners.
    //
    // Chrono miners DRIVE to ore, not warp — the original's
    // Mission_Harvest state 0 forces a DriveLocomotion piggyback before
    // calling Set_Destination, so the teleport-vs-drive branch in
    // Set_Destination resolves to drive. Only the inbound trip
    // (ore → refinery) uses the warp; outbound is a normal drive.
    if let Some(cell) = search_local_ore(
        &sim.production.resource_nodes,
        (snap.rx, snap.ry),
        config.long_scan_radius,
        filter_ref,
        config.ore_bale_value,
        config.gem_bale_value,
    ) {
        snap.miner.target_ore_cell = Some(cell);
        snap.miner.state = MinerState::MoveToOre;
        return;
    }

    // Global search — find nearest reachable ore anywhere on the map.
    if let Some(cell) = pick_best_resource_node(
        &sim.production.resource_nodes,
        (snap.rx, snap.ry),
        filter_ref,
    ) {
        snap.miner.target_ore_cell = Some(cell);
        snap.miner.state = MinerState::MoveToOre;
        return;
    }

    // No reachable ore anywhere.
    snap.miner.state = MinerState::WaitNoOre;
    snap.miner.rescan_cooldown = config.rescan_cooldown_ticks;
}

fn handle_move_to_ore(
    sim: &mut Simulation,
    _rules: &RuleSet,
    config: &MinerConfig,
    path_grid: Option<&PathGrid>,
    snap: &mut MinerSnapshot,
) {
    let Some(current_target) = snap.miner.target_ore_cell else {
        snap.miner.state = MinerState::SearchOre;
        return;
    };

    // Check if current target has been depleted.
    let still_has_ore = sim
        .production
        .resource_nodes
        .get(&current_target)
        .is_some_and(|n| n.remaining > 0);
    if !still_has_ore {
        snap.miner.target_ore_cell = None;
        snap.miner.state = MinerState::SearchOre;
        return;
    }

    // Wait for any in-progress teleport to complete (chrono delay).
    // Must be checked BEFORE the arrival check — during ChronoDelay the
    // entity is already at the target position but still materializing
    // (50% translucent). Transitioning to Harvest during delay would skip
    // the warp-in visual.
    let has_teleport = sim
        .entities
        .get(snap.entity_id)
        .is_some_and(|e| e.teleport_state.is_some());
    if has_teleport {
        return;
    }

    // Per-tick rescan — gamemd's Mission_Harvest state 0 re-runs the
    // ore scan every tick from the harvester's current cell. If the
    // best-available cell shifts (current target became blocked by a
    // tree / other miner, or a closer ore opened up), retarget. The
    // scan is deterministic given unchanged inputs, so when nothing
    // changes it returns the same cell and the assignment is a no-op.
    let new_target = {
        let scan_filter = build_scan_filter(sim, path_grid, snap);
        let filter_ref: Option<&dyn Fn((u16, u16)) -> bool> = scan_filter.as_deref();
        search_local_ore(
            &sim.production.resource_nodes,
            (snap.rx, snap.ry),
            config.long_scan_radius,
            filter_ref,
            config.ore_bale_value,
            config.gem_bale_value,
        )
    };
    let target = new_target.unwrap_or(current_target);
    if target != current_target {
        snap.miner.target_ore_cell = Some(target);
        // Clear existing movement so it gets re-issued to the new target.
        if let Some(entity) = sim.entities.get_mut(snap.entity_id) {
            entity.movement_target = None;
        }
    }

    // Arrived?
    if (snap.rx, snap.ry) == target {
        snap.miner.state = MinerState::Harvest;
        // Original requires 9 StepTimer steps before first bale (18 frames at default rate).
        snap.miner.harvest_timer = config.harvest_tick_interval;
        return;
    }

    // Check if entity still has an active movement target (may have just
    // been cleared above on retarget).
    let has_movement = sim
        .entities
        .get(snap.entity_id)
        .is_some_and(|e| e.movement_target.is_some());
    // Adjacent to ore? The passability matrix blocks Tiberium terrain for
    // Track-type units, so A* can't path onto the ore cell itself. Use a
    // direct (non-pathfinding) move for the final step — harvesters must
    // be able to reach ore regardless of terrain passability rules.
    // Only issue the move if not already heading there (avoid re-issuing
    // every tick before the entity physically arrives).
    let dx = (snap.rx as i32 - target.0 as i32).unsigned_abs();
    let dy = (snap.ry as i32 - target.1 as i32).unsigned_abs();

    if dx <= 1 && dy <= 1 {
        if !has_movement {
            movement::issue_direct_move(&mut sim.entities, snap.entity_id, target, snap.speed);
        }
        return;
    }

    // Issue movement if not already pathing.
    // After issuing the A* move, mark it as ignore_terrain_cost so the
    // movement tick doesn't block at Tiberium cells along the path.
    // Harvesters must be able to traverse ore fields freely.
    if !has_movement && let Some(grid) = path_grid {
        issue_move_if_idle(&mut sim.entities, grid, snap.entity_id, target, snap.speed);
        // Mark the newly created movement as terrain-cost-exempt.
        if let Some(entity) = sim.entities.get_mut(snap.entity_id)
            && let Some(ref mut mt) = entity.movement_target
        {
            mt.ignore_terrain_cost = true;
        }
    }
}

fn handle_harvest(
    sim: &mut Simulation,
    rules: &RuleSet,
    config: &MinerConfig,
    path_grid: Option<&PathGrid>,
    snap: &mut MinerSnapshot,
) {
    // Timer countdown.
    if snap.miner.harvest_timer > 0 {
        snap.miner.harvest_timer -= 1;
        return;
    }

    let cell = (snap.rx, snap.ry);
    let empty: u16 = snap
        .miner
        .capacity_bales
        .saturating_sub(snap.miner.cargo.len() as u16);

    // One extraction call drains min(empty_capacity, cell_density) bales
    // in a single atomic mutation (matches gamemd's Harvest_Ore_Tick).
    let bales = extract_bales_max(sim, cell, config, empty);

    if !bales.is_empty() {
        snap.miner.cargo.extend(bales);

        if snap.miner.is_full() {
            // Becoming-full: save an archive ghost cell pointing at a
            // nearby still-productive patch so the next `SearchOre`
            // (after dock) returns directly to it.
            save_archive_via_short_scan(sim, config, path_grid, snap);
            begin_return(sim, rules, config, path_grid, snap);
            return;
        }
        // Bales extracted but miner not full → cell has either been
        // drained (multi-bale exhausted it) or still has more density
        // (capacity capped this call). Reset timer; next tick re-enters
        // Harvest. If the cell is now empty the next call returns 0 and
        // we fall through to short-scan; if it still has density we wait
        // 18 frames per gamemd's step-counter gate.
        snap.miner.harvest_timer = config.harvest_tick_interval;
        return;
    }

    // No bales extracted (cell empty). Three sub-paths:
    //   1. Full → return, save archive via short scan.
    //   2. Otherwise run a short continuation scan from the current
    //      cell. Hit → keep harvesting (we use MoveToOre, which
    //      re-enters Harvest on arrival).
    //   3. Miss while not full → return, clear archive.
    if snap.miner.is_full() {
        save_archive_via_short_scan(sim, config, path_grid, snap);
        begin_return(sim, rules, config, path_grid, snap);
        return;
    }

    // Short scan. The filter's closure captures `&sim`; scope it so the
    // immutable borrow drops before `begin_return` needs `&mut sim` below.
    let continuation_target = {
        let scan_filter = build_scan_filter(sim, path_grid, snap);
        let filter_ref: Option<&dyn Fn((u16, u16)) -> bool> = scan_filter.as_deref();
        search_local_ore(
            &sim.production.resource_nodes,
            (snap.rx, snap.ry),
            config.local_continuation_radius,
            filter_ref,
            config.ore_bale_value,
            config.gem_bale_value,
        )
    };
    if let Some(next_cell) = continuation_target {
        snap.miner.target_ore_cell = Some(next_cell);
        snap.miner.state = MinerState::MoveToOre;
        return;
    }

    // Scan miss while not full → return to refinery, clear archive.
    snap.miner.last_harvest_cell = None;
    begin_return(sim, rules, config, path_grid, snap);
}

/// Save a fresh ghost-cell archive by running a short-radius scan from
/// the miner's current position. Called when the miner becomes full so
/// the next `SearchOre` cycle can return directly to a nearby still-
/// productive patch. On scan miss, clears the archive.
fn save_archive_via_short_scan(
    sim: &Simulation,
    config: &MinerConfig,
    path_grid: Option<&PathGrid>,
    snap: &mut MinerSnapshot,
) {
    let scan_filter = build_scan_filter(sim, path_grid, snap);
    let filter_ref: Option<&dyn Fn((u16, u16)) -> bool> = scan_filter.as_deref();
    snap.miner.last_harvest_cell = search_local_ore(
        &sim.production.resource_nodes,
        (snap.rx, snap.ry),
        config.local_continuation_radius,
        filter_ref,
        config.ore_bale_value,
        config.gem_bale_value,
    );
}

fn handle_return(
    sim: &mut Simulation,
    rules: &RuleSet,
    config: &MinerConfig,
    path_grid: Option<&PathGrid>,
    snap: &mut MinerSnapshot,
) {
    let has_teleport = sim
        .entities
        .get(snap.entity_id)
        .is_some_and(|e| e.teleport_state.is_some());
    if has_teleport {
        return;
    }

    let Some(ref_sid) = snap.miner.reserved_refinery else {
        if let Some((rsid, _dock)) = find_nearest_refinery(
            sim,
            rules,
            sim.interner.resolve(snap.owner),
            sim.interner.resolve(snap.type_id),
            (snap.rx, snap.ry),
        ) {
            snap.miner.reserved_refinery = Some(rsid);
            if try_issue_chrono_far_return_teleport(sim, rules, config, path_grid, snap, rsid) {
                return;
            }
            if try_begin_chrono_close_return_radio(sim, rules, config, path_grid, snap, rsid) {
                return;
            }
        } else {
            snap.miner.state = MinerState::WaitNoOre;
        }
        return;
    };

    let Some(dock) = refinery_dock_for_sid(sim, rules, ref_sid) else {
        sim.production
            .dock_reservations
            .cancel_miner(ref_sid, snap.entity_id);
        snap.miner.reserved_refinery = None;
        snap.miner.dock_queued = false;
        snap.miner.dock_phase = RefineryDockPhase::Approach;
        snap.miner.exit_cell = None;
        if snap.miner.is_full() {
            snap.miner.target_ore_cell = None;
            snap.miner.state = MinerState::ReturnToRefinery;
        } else {
            snap.miner.state = MinerState::SearchOre;
        }
        return;
    };

    let moving = sim
        .entities
        .get(snap.entity_id)
        .is_some_and(|entity| entity.movement_target.is_some());
    if !moving && try_issue_chrono_far_return_teleport(sim, rules, config, path_grid, snap, ref_sid)
    {
        return;
    }
    if try_begin_chrono_close_return_radio(sim, rules, config, path_grid, snap, ref_sid) {
        return;
    }

    let at_dock = (snap.rx, snap.ry) == dock;
    let contact = if snap.miner.kind == MinerKind::Chrono {
        at_dock
    } else {
        let stopped_close_enough = sim.entities.get(snap.entity_id).is_some_and(|entity| {
            entity.movement_target.is_none()
                && is_within_close_enough((snap.rx, snap.ry), dock, rules.general.close_enough)
        });
        is_adjacent_or_at((snap.rx, snap.ry), dock) || stopped_close_enough
    };

    if contact {
        snap.miner.state = MinerState::Dock;
        snap.miner.dock_phase = RefineryDockPhase::Approach;
        return;
    }

    if let Some(grid) = path_grid {
        issue_move_if_idle(&mut sim.entities, grid, snap.entity_id, dock, snap.speed);
    }
}

fn handle_wait_no_ore(_config: &MinerConfig, snap: &mut MinerSnapshot) {
    if snap.miner.rescan_cooldown > 0 {
        snap.miner.rescan_cooldown -= 1;
        return;
    }
    snap.miner.state = MinerState::SearchOre;
}

fn handle_forced_return(
    sim: &mut Simulation,
    rules: &RuleSet,
    config: &MinerConfig,
    path_grid: Option<&PathGrid>,
    snap: &mut MinerSnapshot,
) {
    let has_teleport = sim
        .entities
        .get(snap.entity_id)
        .is_some_and(|e| e.teleport_state.is_some());
    if has_teleport {
        return;
    }

    if snap.miner.reserved_refinery.is_none() {
        if let Some((rsid, _dock)) = find_nearest_refinery(
            sim,
            rules,
            sim.interner.resolve(snap.owner),
            sim.interner.resolve(snap.type_id),
            (snap.rx, snap.ry),
        ) {
            snap.miner.reserved_refinery = Some(rsid);
            if try_issue_chrono_far_return_teleport(sim, rules, config, path_grid, snap, rsid) {
                return;
            }
        } else {
            snap.miner.state = MinerState::WaitNoOre;
            snap.miner.rescan_cooldown = config.rescan_cooldown_ticks;
            return;
        }
    }

    handle_return(sim, rules, config, path_grid, snap);
}

// -- Helpers --

/// Extract one bale from a resource node cell.
///
/// Each bale drains one richness level from the cell (base units).
/// base = 120 for ore, 180 for gems — matching seed_resource_nodes_from_overlays.
/// This keeps remaining aligned with the overlay frame formula (remaining/base = richness),
/// so the visual depletion in the renderer tracks correctly.
pub(crate) fn extract_bale(
    sim: &mut Simulation,
    cell: (u16, u16),
    config: &MinerConfig,
) -> Option<CargoBale> {
    let node = sim.production.resource_nodes.get_mut(&cell)?;
    if node.remaining == 0 {
        return None;
    }
    let (value, base): (u16, u16) = match node.resource_type {
        ResourceType::Ore => (config.ore_bale_value, 120),
        ResourceType::Gem => (config.gem_bale_value, 180),
    };
    let res_type = node.resource_type;
    node.remaining = node.remaining.saturating_sub(base);
    if node.remaining == 0 {
        sim.production.resource_nodes.remove(&cell);
        // Fully depleted — clear overlay so rendering skips this cell.
        if let Some(grid) = &mut sim.overlay_grid {
            grid.clear_overlay(cell.0, cell.1);
        }
        return Some(CargoBale {
            resource_type: res_type,
            value,
        });
    }
    // Partial depletion — sync overlay frame to new density.
    if let Some(grid) = &mut sim.overlay_grid {
        let frame = (node.remaining / base).saturating_sub(1).min(11) as u8;
        grid.set_overlay_data(cell.0, cell.1, frame);
    }
    Some(CargoBale {
        resource_type: node.resource_type,
        value,
    })
}

/// Drain as many bales from `cell` as fit within `empty_capacity_bales`.
///
/// Mirrors gamemd's harvester per-tick extraction:
///   amount    = ftol(Storage - current_load)   // bales requested
///   extracted = Reduce_Tiberium(amount)        // clamped to cell density
///   AddAmount(extracted, type)                 // one storage update
///
/// One call drains `min(empty_capacity_bales, cell_density_levels)` bales
/// in a single atomic mutation: one `node.remaining` decrement and one
/// overlay update (or removal). Returns an empty Vec when the cell is
/// missing, has `remaining == 0`, or `empty_capacity_bales == 0`.
pub(crate) fn extract_bales_max(
    sim: &mut Simulation,
    cell: (u16, u16),
    config: &MinerConfig,
    empty_capacity_bales: u16,
) -> Vec<CargoBale> {
    if empty_capacity_bales == 0 {
        return Vec::new();
    }
    let Some(node) = sim.production.resource_nodes.get(&cell) else {
        return Vec::new();
    };
    if node.remaining == 0 {
        return Vec::new();
    }
    let (value, base): (u16, u16) = match node.resource_type {
        ResourceType::Ore => (config.ore_bale_value, 120),
        ResourceType::Gem => (config.gem_bale_value, 180),
    };
    let resource_type = node.resource_type;
    let density_levels = node.remaining / base;
    if density_levels == 0 {
        return Vec::new();
    }
    let n: u16 = empty_capacity_bales.min(density_levels);
    let remaining_after: u16 = node.remaining - n * base;

    let bales: Vec<CargoBale> = (0..n)
        .map(|_| CargoBale {
            resource_type,
            value,
        })
        .collect();

    if remaining_after == 0 {
        sim.production.resource_nodes.remove(&cell);
        if let Some(grid) = &mut sim.overlay_grid {
            grid.clear_overlay(cell.0, cell.1);
        }
    } else {
        sim.production
            .resource_nodes
            .get_mut(&cell)
            .expect("node existed above")
            .remaining = remaining_after;
        if let Some(grid) = &mut sim.overlay_grid {
            let frame = (remaining_after / base).saturating_sub(1).min(11) as u8;
            grid.set_overlay_data(cell.0, cell.1, frame);
        }
    }

    bales
}

/// Begin the return-to-refinery sequence.
///
/// Chrono miners inside `ChronoHarvTooFarDistance` keep the normal refinery
/// radio/contact path to the accepted dock cell. Miners beyond that threshold
/// use the far-return destination: the `QueueingCell` passable-cell search
/// result, not the pad/contact cell.
fn begin_return(
    sim: &mut Simulation,
    rules: &RuleSet,
    config: &MinerConfig,
    path_grid: Option<&PathGrid>,
    snap: &mut MinerSnapshot,
) {
    if let Some((rsid, _dock)) = find_nearest_refinery(
        sim,
        rules,
        sim.interner.resolve(snap.owner),
        sim.interner.resolve(snap.type_id),
        (snap.rx, snap.ry),
    ) {
        snap.miner.reserved_refinery = Some(rsid);
        if try_issue_chrono_far_return_teleport(sim, rules, config, path_grid, snap, rsid) {
            return;
        }
        if try_begin_chrono_close_return_radio(sim, rules, config, path_grid, snap, rsid) {
            return;
        }
        snap.miner.state = MinerState::ReturnToRefinery;
    } else {
        snap.miner.state = MinerState::WaitNoOre;
    }
}

fn try_begin_chrono_close_return_radio(
    sim: &mut Simulation,
    rules: &RuleSet,
    config: &MinerConfig,
    path_grid: Option<&PathGrid>,
    snap: &mut MinerSnapshot,
    ref_sid: u64,
) -> bool {
    if snap.miner.kind != MinerKind::Chrono {
        return false;
    }

    match chrono_return_exceeds_too_far_threshold(
        sim,
        snap.entity_id,
        ref_sid,
        config.too_far_threshold_chrono,
    ) {
        Some(false) => {}
        Some(true) | None => return false,
    }

    let Some(dock_capacity) = refinery_dock_capacity_for_sid(sim, rules, ref_sid) else {
        return false;
    };

    let admission =
        sim.production
            .dock_reservations
            .hello_or_wait(ref_sid, snap.entity_id, dock_capacity);

    if let Some(entity) = sim.entities.get_mut(snap.entity_id) {
        entity.movement_target = None;
    }

    snap.miner.state = MinerState::Dock;
    snap.miner.dock_queued = admission != ContactAdmission::Accepted;
    snap.miner.dock_phase = if admission == ContactAdmission::Accepted {
        RefineryDockPhase::MissionEnter
    } else {
        RefineryDockPhase::Approach
    };

    if admission != ContactAdmission::Accepted {
        if let Some(staging) = chrono_return_staging_cell_for_sid(sim, rules, ref_sid, path_grid)
            && !is_adjacent_or_at((snap.rx, snap.ry), staging)
            && let Some(grid) = path_grid
        {
            issue_move_if_idle(&mut sim.entities, grid, snap.entity_id, staging, snap.speed);
        }
    }

    true
}

fn try_issue_chrono_far_return_teleport(
    sim: &mut Simulation,
    rules: &RuleSet,
    config: &MinerConfig,
    path_grid: Option<&PathGrid>,
    snap: &MinerSnapshot,
    ref_sid: u64,
) -> bool {
    if snap.miner.kind != MinerKind::Chrono {
        return false;
    }

    if !chrono_return_exceeds_too_far_threshold(
        sim,
        snap.entity_id,
        ref_sid,
        config.too_far_threshold_chrono,
    )
    .unwrap_or(false)
    {
        return false;
    }

    let Some(staging) = chrono_return_staging_cell_for_sid(sim, rules, ref_sid, path_grid) else {
        return false;
    };

    let z = sim
        .entities
        .get(snap.entity_id)
        .map(|entity| entity.position.z)
        .unwrap_or(0);
    spawn_warp_effects(
        sim,
        rules,
        snap.type_id,
        (snap.rx, snap.ry, z),
        (staging.0, staging.1, z),
    );
    issue_teleport_command(
        &mut sim.entities,
        snap.entity_id,
        staging,
        &rules.general,
        true,
    )
}

fn spawn_warp_effects(
    sim: &mut Simulation,
    rules: &RuleSet,
    type_id: InternedId,
    depart: (u16, u16, u8),
    arrive: (u16, u16, u8),
) {
    use crate::sim::components::WorldEffect;

    const FALLBACK_FRAME_COUNT: u16 = 20;

    let anim_name: &str = &rules.general.warp_out.name;
    let anim_rate: u32 = rules.general.warp_out.rate_ms;
    let anim_interned = sim.interner.intern(anim_name);

    let anim_frames: u16 = sim
        .effect_frame_counts
        .get(&anim_interned)
        .copied()
        .unwrap_or(FALLBACK_FRAME_COUNT);

    for (rx, ry, z) in [depart, arrive] {
        sim.world_effects.push(WorldEffect {
            shp_name: anim_interned,
            rx,
            ry,
            sub_x: crate::util::lepton::CELL_CENTER_LEPTON,
            sub_y: crate::util::lepton::CELL_CENTER_LEPTON,
            z,
            frame: 0,
            total_frames: anim_frames,
            rate_ms: anim_rate,
            elapsed_ms: 0,
            translucent: true,
            delay_ms: 0,
            start_sound_id: None,
            start_sound_emitted: false,
        });
    }

    let obj = rules.object_case_insensitive(sim.interner.resolve(type_id));
    let chrono_out = obj
        .and_then(|o| o.chrono_out_sound.clone())
        .or_else(|| rules.general.chrono_out_sound.clone());
    let chrono_in = obj
        .and_then(|o| o.chrono_in_sound.clone())
        .or_else(|| rules.general.chrono_in_sound.clone());
    if let Some(name) = chrono_out {
        let sound_id = sim.interner.intern(&name);
        sim.sound_events.push(SimSoundEvent::ChronoTeleport {
            sound_id,
            rx: depart.0,
            ry: depart.1,
        });
    }
    if let Some(name) = chrono_in {
        let sound_id = sim.interner.intern(&name);
        sim.sound_events.push(SimSoundEvent::ChronoTeleport {
            sound_id,
            rx: arrive.0,
            ry: arrive.1,
        });
    }
}

/// Find the nearest friendly refinery. Returns (stable_id, dock_cell).
///
/// TibSun legacy: checks alliance (not just same-owner), building health,
/// and construction state. Matches original `BuildingClass::CanDock` guards.
fn find_nearest_refinery(
    sim: &Simulation,
    rules: &RuleSet,
    owner: &str,
    harvester_type_id: &str,
    from: (u16, u16),
) -> Option<(u64, (u16, u16))> {
    let mut best: Option<(u32, u64, u16, u16)> = None;
    for entity in sim.entities.values() {
        let e_owner = sim.interner.resolve(entity.owner);
        let e_type = sim.interner.resolve(entity.type_ref);
        if entity.category != EntityCategory::Structure
            // TibSun legacy: accept allied refineries, not just same-owner.
            || !crate::map::houses::are_houses_friendly(
                &sim.house_alliances,
                owner,
                e_owner,
            )
            || !rules.is_refinery_type(e_type)
            || !rules.harvester_can_dock_at(harvester_type_id, e_type)
            // Death animations keep the building entity around, but gamemd
            // calls UndockUnit from damage/sell paths before accepting more cargo.
            || entity.dying
            // TibSun legacy: skip dead buildings (CanDock checks HP > 0).
            || entity.health.current == 0
            // TibSun legacy: skip buildings under construction (CanDock rejects mission 0x13).
            || entity.building_up.is_some()
        {
            continue;
        }
        let obj = rules.object_case_insensitive(e_type);
        let (w, h) = obj
            .map(|o| foundation_dimensions(&o.foundation))
            .unwrap_or((1, 1));
        let qc = obj.and_then(|o| o.queueing_cell);
        let dock = refinery_dock_cell(entity.position.rx, entity.position.ry, w, h, qc);
        let dx = i64::from(dock.0) - i64::from(from.0);
        let dy = i64::from(dock.1) - i64::from(from.1);
        let dist_sq = (dx * dx + dy * dy) as u32;
        match best {
            Some((d, _, _, _)) if dist_sq >= d => {}
            _ => best = Some((dist_sq, entity.stable_id, dock.0, dock.1)),
        }
    }
    best.map(|(_, sid, dx, dy)| (sid, (dx, dy)))
}

/// Resolve a refinery's dock cell from its stable_id.
fn refinery_dock_for_sid(sim: &Simulation, rules: &RuleSet, ref_sid: u64) -> Option<(u16, u16)> {
    let entity = sim.entities.get(ref_sid)?;
    if entity.dying || entity.health.current == 0 {
        return None;
    }
    let obj = rules.object_case_insensitive(sim.interner.resolve(entity.type_ref));
    let (w, h) = obj
        .map(|o| foundation_dimensions(&o.foundation))
        .unwrap_or((1, 1));
    let qc = obj.and_then(|o| o.queueing_cell);
    Some(refinery_dock_cell(
        entity.position.rx,
        entity.position.ry,
        w,
        h,
        qc,
    ))
}

fn refinery_dock_capacity_for_sid(
    sim: &Simulation,
    rules: &RuleSet,
    ref_sid: u64,
) -> Option<usize> {
    let entity = sim.entities.get(ref_sid)?;
    if entity.dying || entity.health.current == 0 {
        return None;
    }
    rules
        .object_case_insensitive(sim.interner.resolve(entity.type_ref))
        .map(|o| o.number_of_docks.max(1) as usize)
        .or(Some(1))
}

/// Chrono far-return staging cell from `QueueingCell`, then the same nearby
/// passable-cell search gamemd runs before assigning a teleport destination.
fn chrono_return_staging_cell_for_sid(
    sim: &Simulation,
    rules: &RuleSet,
    ref_sid: u64,
    path_grid: Option<&PathGrid>,
) -> Option<(u16, u16)> {
    let entity = sim.entities.get(ref_sid)?;
    let obj = rules.object_case_insensitive(sim.interner.resolve(entity.type_ref));
    let (w, h) = obj
        .map(|o| foundation_dimensions(&o.foundation))
        .unwrap_or((1, 1));
    let qc = obj.and_then(|o| o.queueing_cell);
    let seed = super::miner_dock_sequence::refinery_queue_cell(
        entity.position.rx,
        entity.position.ry,
        w,
        h,
        qc,
    );

    if let Some(grid) = path_grid {
        return super::miner_dock_sequence::find_nearby_passable_cell_with_index(
            seed.0 as i32,
            seed.1 as i32,
            grid,
            None,
            super::miner_dock_sequence::EXIT_SEARCH_MAX_RADIUS,
            sim.tick,
        );
    }

    Some(seed)
}

pub(crate) fn refinery_dock_cell(
    rx: u16,
    ry: u16,
    _width: u16,
    _height: u16,
    _queueing_cell: Option<(u16, u16)>,
) -> (u16, u16) {
    super::miner_dock_sequence::refinery_can_dock_queue_cell(rx, ry)
}

/// 8-neighbor offsets in clockwise order starting from north. Used by the
/// effective-zone-cell probe and the ore-reachability check.
const ADJACENT_8: [(i32, i32); 8] = [
    (0, -1),
    (1, -1),
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
    (-1, -1),
];

/// Return a cell whose zone serves as the harvester's reachability anchor.
///
/// The harvester's own cell may be on Tiberium (impassable in the path grid,
/// hence `ZONE_INVALID`); when so, probe its 8 neighbors and return the
/// first cell with a valid zone. Returns `None` if neither the harvester's
/// cell nor any neighbor has a valid zone — caller falls back to no-filter
/// behavior for that tick.
fn effective_zone_cell(
    zone_grid: &ZoneGrid,
    mz: MovementZone,
    rx: u16,
    ry: u16,
) -> Option<(u16, u16)> {
    let zone_map = zone_grid.map_for(mz)?;
    if zone_map.zone_at(rx, ry, MovementLayer::Ground) != ZONE_INVALID {
        return Some((rx, ry));
    }
    for &(dx, dy) in &ADJACENT_8 {
        let nx = (rx as i32) + dx;
        let ny = (ry as i32) + dy;
        if nx < 0 || ny < 0 || nx > u16::MAX as i32 || ny > u16::MAX as i32 {
            continue;
        }
        let (nx, ny) = (nx as u16, ny as u16);
        if zone_map.zone_at(nx, ny, MovementLayer::Ground) != ZONE_INVALID {
            return Some((nx, ny));
        }
    }
    None
}

/// True if any 8-neighbor of `ore_cell` is in the harvester's connected zone
/// component. Ore cells themselves are `ZONE_INVALID` because Tiberium is
/// blocked in the path grid (so A* doesn't path through ore fields), so we
/// probe the ore's neighbors instead — mirroring how a harvester actually
/// approaches an ore patch.
fn ore_reachable(
    zone_grid: &ZoneGrid,
    mz: MovementZone,
    layer: MovementLayer,
    harvester_zone_cell: (u16, u16),
    ore_cell: (u16, u16),
) -> bool {
    for &(dx, dy) in &ADJACENT_8 {
        let nx = (ore_cell.0 as i32) + dx;
        let ny = (ore_cell.1 as i32) + dy;
        if nx < 0 || ny < 0 || nx > u16::MAX as i32 || ny > u16::MAX as i32 {
            continue;
        }
        let (nx, ny) = (nx as u16, ny as u16);
        if zone_grid.can_reach(mz, harvester_zone_cell, layer, (nx, ny), layer) {
            return true;
        }
    }
    false
}

/// Search for ore within `radius` cells of `center`. Returns best cell.
///
/// Mirrors gamemd's `FootClass::Scan_For_Tiberium` (0x4DD0A0): a diamond
/// ring expansion that returns as soon as any ring contains harvestable ore,
/// then picks the highest-value cell within that ring. Value = `base × (density+1)`
/// per tiberium type (Ore base default 25, Gems default 50).
///
/// Critical: nearer rings win unconditionally — a closer ore patch always
/// beats a richer-but-farther gem patch. This is the opposite of "globally
/// best in radius" and is the reason harvesters pick local ore even when
/// gems exist elsewhere on the map.
pub(crate) fn search_local_ore(
    nodes: &std::collections::BTreeMap<(u16, u16), ResourceNode>,
    center: (u16, u16),
    radius: u16,
    filter: Option<&dyn Fn((u16, u16)) -> bool>,
    ore_base: u16,
    gem_base: u16,
) -> Option<(u16, u16)> {
    let value_of = |node: &ResourceNode| -> u32 {
        let base = match node.resource_type {
            ResourceType::Ore => ore_base as u32,
            ResourceType::Gem => gem_base as u32,
        };
        base * (node.remaining as u32 + 1)
    };

    // Ring 0 fast path: if the center cell has ore, return immediately.
    // gamemd checks LandType==Tiberium with no harvestability filter for the
    // center — a unit standing on ore harvests it without zone/passability tests.
    if let Some(node) = nodes.get(&center)
        && node.remaining > 0
    {
        return Some(center);
    }

    // Ring 1..radius expansion (Chebyshev distance, diamond perimeter).
    // For each ring we walk the four arms and track the highest-value
    // harvestable cell. As soon as any ring yields a hit, return it —
    // gamemd's early-exit-per-ring is what makes nearer-always-wins.
    let radius_i = radius as i32;
    let cx = center.0 as i32;
    let cy = center.1 as i32;

    for ring in 1..radius_i {
        let mut best_in_ring: Option<(u32, (u16, u16))> = None;

        for col in -ring..=ring {
            // The four diamond arms at Chebyshev distance == ring.
            // Corner cells (col == ±ring) are visited twice across arms;
            // gamemd does the same, no dedup needed (same cell re-evaluated).
            let arms: [(i32, i32); 4] = [
                (cx + col, cy - ring), // top
                (cx + col, cy + ring), // bottom
                (cx - ring, cy + col), // left
                (cx + ring, cy + col), // right
            ];
            for (nx, ny) in arms {
                if nx < 0 || ny < 0 || nx > u16::MAX as i32 || ny > u16::MAX as i32 {
                    continue;
                }
                let cell = (nx as u16, ny as u16);
                let Some(node) = nodes.get(&cell) else {
                    continue;
                };
                if node.remaining == 0 {
                    continue;
                }
                if let Some(f) = filter
                    && !f(cell)
                {
                    continue;
                }
                let value = value_of(node);
                // gamemd: strict `if (old < new)` — first-seen wins on ties.
                match best_in_ring {
                    Some((cur, _)) if value <= cur => {}
                    _ => best_in_ring = Some((value, cell)),
                }
            }
        }

        if let Some((_, cell)) = best_in_ring {
            return Some(cell);
        }
    }

    None
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

/// True if `pos` is at `target` or cardinally/diagonally adjacent (1 cell away).
/// Used for dock arrival checks — buildings occupy their cells, so miners
/// park adjacent to the refinery rather than on top of it.
fn is_adjacent_or_at(pos: (u16, u16), target: (u16, u16)) -> bool {
    let dx = (pos.0 as i32 - target.0 as i32).unsigned_abs();
    let dy = (pos.1 as i32 - target.1 as i32).unsigned_abs();
    dx <= 1 && dy <= 1
}

/// Movement can legitimately stop short when blocked but within
/// `[General] CloseEnough`; refinery return must treat that as contact so the
/// dock radio/enter sequence can take over instead of reissuing the same path.
fn is_within_close_enough(pos: (u16, u16), target: (u16, u16), close_enough: SimFixed) -> bool {
    let dx = (pos.0 as i32 - target.0 as i32).abs();
    let dy = (pos.1 as i32 - target.1 as i32).abs();
    SimFixed::from_num((dx + dy) * 256) < close_enough
}

/// Check whether the player owns at least one Ore Purifier building.
///
/// Retained for callers that only need a boolean signal (e.g., UI hints).
/// For deposit-time credit math use [`count_purifiers_for_owner`] — gamemd
/// multiplies the bonus by the live count, so a 2-purifier player should
/// receive +50%, not +25%.
pub(crate) fn player_has_purifier(sim: &Simulation, rules: &RuleSet, owner: &str) -> bool {
    count_purifiers_for_owner(sim, rules, owner) > 0
}

/// Count alive Ore Purifier buildings owned by `owner` (case-insensitive).
///
/// Used by the deposit-bonus formula in `phase_unloading` and by the Slave
/// Miner deposit path. The bonus is `count × PurifierBonus × amount`, so
/// every real purifier stacks the bonus linearly.
pub(crate) fn count_purifiers_for_owner(sim: &Simulation, rules: &RuleSet, owner: &str) -> i32 {
    sim.entities
        .values()
        .filter(|e| {
            e.category == EntityCategory::Structure
                && sim.interner.resolve(e.owner).eq_ignore_ascii_case(owner)
                && rules
                    .object_case_insensitive(sim.interner.resolve(e.type_ref))
                    .is_some_and(|obj| obj.ore_purifier)
        })
        .count() as i32
}

/// Effective purifier count used in the deposit bonus formula.
///
/// Returns `real_purifiers + AI_virtual_purifiers`, where the AI term is
/// `general.ai_virtual_purifiers[difficulty]` for non-human houses in
/// skirmish/campaign play, and 0 otherwise. Both terms are sourced from
/// the refinery's owner — credit destination is a separate concern.
pub(crate) fn effective_purifier_count(
    sim: &Simulation,
    rules: &RuleSet,
    refinery_owner: &str,
) -> i32 {
    let real = count_purifiers_for_owner(sim, rules, refinery_owner);
    // Apply the AI virtual bonus only when a HouseState explicitly says
    // the refinery's owner is non-human. Real games seed every house
    // through app init with the correct flag; tests/edge cases that fall
    // through to the credits_entry_for_owner auto-create get is_human=true
    // (the safer default) and therefore skip the AI bonus, as intended.
    let is_ai =
        crate::sim::house_state::house_state_for_owner(&sim.houses, refinery_owner, &sim.interner)
            .is_some_and(|h| !h.is_human);
    if !is_ai {
        return real;
    }
    let difficulty = sim.game_options.ai_difficulty;
    let table = rules.general.ai_virtual_purifiers;
    // INI ordering is `[Brutal, Medium, Easy]`. Defensive bounds-check in
    // case the difficulty index drifts out of range.
    let virtual_count = if (0..3).contains(&difficulty) {
        table[difficulty as usize]
    } else {
        0
    };
    real + virtual_count
}
