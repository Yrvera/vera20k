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
use crate::sim::miner::{
    CargoBale, Miner, MinerConfig, MinerKind, MinerState, RefineryDockPhase, ResourceNode,
    ResourceType,
};
use crate::sim::movement;
use crate::sim::movement::locomotor::MovementLayer;
use crate::sim::movement::teleport_movement::issue_teleport_command;
use crate::sim::pathfinding::PathGrid;
use crate::sim::pathfinding::zone_map::{ZONE_INVALID, ZoneGrid};
use crate::sim::production::pick_best_resource_node;
use crate::sim::world::{SimSoundEvent, Simulation};
use crate::util::fixed_math::{SimFixed, ra2_speed_to_leptons_per_second};

use crate::sim::debug_event_log::DebugEventKind;
use crate::sim::intern::InternedId;

use crate::sim::production::foundation_dimensions;

/// Snapshot of one miner entity for two-phase processing.
pub(super) struct MinerSnapshot {
    pub(super) entity_id: u64,
    pub(super) owner: InternedId,
    pub(super) type_id: InternedId,
    pub(super) rx: u16,
    pub(super) ry: u16,
    pub(super) z: u8,
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
            z: entity.position.z,
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

    // Cleanup dead entities from dock reservations.
    let alive_sids: BTreeSet<u64> = sim.entities.values().map(|e| e.stable_id).collect();
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

/// Build a zone-grid-based reachability filter for ore scans.
///
/// Returns `None` if any of (zone_grid, locomotor, effective zone cell)
/// is missing. In that case the caller falls back to an unfiltered scan
/// for this tick — the next tick will likely succeed once the harvester
/// moves to a passable cell.
///
/// Shared by `handle_search_ore` (State 0 fresh search) and
/// `handle_harvest` (State 1 cell-depletion continuation scan).
fn build_reachable_filter<'a>(
    sim: &'a Simulation,
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
    let harvester_anchor = sim
        .zone_grid
        .as_ref()
        .and_then(|zg| effective_zone_cell(zg, mz, snap.rx, snap.ry));

    match (sim.zone_grid.as_ref(), harvester_anchor) {
        (Some(zg), Some(anchor)) => Some(Box::new(move |ore_cell: (u16, u16)| {
            ore_reachable(zg, mz, layer, anchor, ore_cell)
        })),
        _ => None,
    }
}

fn handle_search_ore(
    sim: &Simulation,
    config: &MinerConfig,
    _path_grid: Option<&PathGrid>,
    snap: &mut MinerSnapshot,
) {
    let search_center = snap.miner.last_harvest_cell.unwrap_or((snap.rx, snap.ry));

    // Reachability filter — see build_reachable_filter for the fallback
    // semantics when zone_grid / locomotor / anchor is missing.
    let reachable_filter = build_reachable_filter(sim, snap);
    let filter_ref: Option<&dyn Fn((u16, u16)) -> bool> = reachable_filter.as_deref();

    // Try local continuation scan first (short radius around last harvest spot).
    if let Some(cell) = search_local_ore(
        &sim.production.resource_nodes,
        search_center,
        config.local_continuation_radius,
        filter_ref,
    ) {
        snap.miner.target_ore_cell = Some(cell);
        snap.miner.state = MinerState::MoveToOre;
        return;
    }

    // gamemd.exe (0x0073E844): both war miners and chrono miners use
    // TiberiumLongScan for the initial search — no early exit for chrono.
    // The only chrono-specific behavior is stopping piggybacked locomotion
    // before the scan, which we handle elsewhere.

    // ArchiveTarget pattern (from RA1): if we remember a productive patch and it
    // still has ore AND it's reachable, go back there before doing a full global
    // search. Skipping the reachability check here would re-target a patch the
    // harvester can't reach (e.g., walled off mid-cycle).
    if let Some(archive) = snap.miner.last_harvest_cell {
        let archive_has_ore = sim.production.resource_nodes.contains_key(&archive);
        let archive_reachable = filter_ref.is_none_or(|f| f(archive));
        if archive_has_ore && archive_reachable {
            snap.miner.target_ore_cell = Some(archive);
            snap.miner.state = MinerState::MoveToOre;
            // Clear archive so we don't loop back forever if it depletes on arrival.
            snap.miner.last_harvest_cell = None;
            return;
        }
    }

    // Long-range bounded scan from the miner's current position (TiberiumLongScan).
    // Finds a new ore patch within a larger radius before falling back to unbounded global.
    if let Some(cell) = search_local_ore(
        &sim.production.resource_nodes,
        (snap.rx, snap.ry),
        config.long_scan_radius,
        filter_ref,
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
    let Some(target) = snap.miner.target_ore_cell else {
        snap.miner.state = MinerState::SearchOre;
        return;
    };

    // Check if target cell has been depleted.
    let still_has_ore = sim
        .production
        .resource_nodes
        .get(&target)
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

    // Arrived?
    if (snap.rx, snap.ry) == target {
        snap.miner.state = MinerState::Harvest;
        // Original requires 9 StepTimer steps before first bale (18 frames at default rate).
        snap.miner.harvest_timer = config.harvest_tick_interval;
        return;
    }

    // Check if entity still has an active movement target.
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

    // Try to extract one bale from the cell.
    let bale = extract_bale(sim, cell, config);
    if let Some(bale) = bale {
        snap.miner.cargo.push(bale);
        snap.miner.last_harvest_cell = Some(cell);

        if snap.miner.is_full() {
            // Full — begin return.
            begin_return(sim, rules, config, path_grid, snap);
            return;
        }

        // Cell still has resources? Continue harvesting.
        let still_has = sim
            .production
            .resource_nodes
            .get(&cell)
            .is_some_and(|n| n.remaining > 0);
        if still_has {
            snap.miner.harvest_timer = config.harvest_tick_interval;
            return;
        }
    }

    // Cell depleted (or was already empty). If we have some cargo, return.
    if !snap.miner.cargo.is_empty() {
        begin_return(sim, rules, config, path_grid, snap);
        return;
    }

    // No cargo — search for more ore (local continuation).
    snap.miner.state = MinerState::SearchOre;
}

fn handle_return(
    sim: &mut Simulation,
    rules: &RuleSet,
    config: &MinerConfig,
    path_grid: Option<&PathGrid>,
    snap: &mut MinerSnapshot,
) {
    // Wait for any in-progress chrono teleport to complete before acting.
    let has_teleport = sim
        .entities
        .get(snap.entity_id)
        .is_some_and(|e| e.teleport_state.is_some());
    if has_teleport {
        return;
    }

    let Some(ref_sid) = snap.miner.reserved_refinery else {
        // Lost reservation — find a new refinery.
        if let Some((rsid, dock)) = find_nearest_refinery(
            sim,
            rules,
            sim.interner.resolve(snap.owner),
            sim.interner.resolve(snap.type_id),
            (snap.rx, snap.ry),
        ) {
            snap.miner.reserved_refinery = Some(rsid);
            if snap.miner.kind == MinerKind::Chrono {
                let threshold = config.too_far_threshold_chrono as u32;
                let far_enough = cell_dist_sq((snap.rx, snap.ry), dock) > threshold * threshold;
                if far_enough {
                    // Warp to queue cell via the teleport locomotor system.
                    spawn_warp_effects(
                        sim,
                        rules,
                        snap.type_id,
                        (snap.rx, snap.ry, snap.z),
                        (dock.0, dock.1, snap.z),
                    );
                    issue_teleport_command(&mut sim.entities, snap.entity_id, dock, &rules.general, true);
                    // Stay in ReturnToRefinery — the teleport guard above
                    // will wait one tick for Relocate to land, then adjacency
                    // check below transitions to Dock/WaitForDock.
                    return;
                }
                // Close enough — fall through to drive path.
            }
        } else {
            snap.miner.state = MinerState::WaitNoOre;
            return;
        }
        return;
    };

    // Resolve refinery entity and dock cell (queue cell).
    let Some(dock) = refinery_dock_for_sid(sim, rules, ref_sid) else {
        // Refinery destroyed — find a new one.
        snap.miner.reserved_refinery = None;
        snap.miner.state = MinerState::SearchOre;
        return;
    };

    // Arrive when at dock cell or adjacent — transition to Dock FSM.
    // The Approach phase polls the dock reservation each tick.
    if is_adjacent_or_at((snap.rx, snap.ry), dock) {
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
    // Wait for any in-progress chrono teleport to complete before acting.
    let has_teleport = sim
        .entities
        .get(snap.entity_id)
        .is_some_and(|e| e.teleport_state.is_some());
    if has_teleport {
        return;
    }

    // Same as ReturnToRefinery, but player-triggered.
    // If no refinery reserved yet, find one.
    if snap.miner.reserved_refinery.is_none() {
        if let Some((rsid, dock)) = find_nearest_refinery(
            sim,
            rules,
            sim.interner.resolve(snap.owner),
            sim.interner.resolve(snap.type_id),
            (snap.rx, snap.ry),
        ) {
            snap.miner.reserved_refinery = Some(rsid);
            // Chrono Miners teleport on forced return — but only if far enough.
            if snap.miner.kind == MinerKind::Chrono {
                let threshold = config.too_far_threshold_chrono as u32;
                let far_enough = cell_dist_sq((snap.rx, snap.ry), dock) > threshold * threshold;
                if far_enough {
                    spawn_warp_effects(
                        sim,
                        rules,
                        snap.type_id,
                        (snap.rx, snap.ry, snap.z),
                        (dock.0, dock.1, snap.z),
                    );
                    issue_teleport_command(&mut sim.entities, snap.entity_id, dock, &rules.general, true);
                    // Stay in ForcedReturn — teleport guard waits one tick for
                    // Relocate to land, then handle_return below takes over.
                    return;
                }
                // Close enough — fall through to drive path.
            }
        } else {
            snap.miner.state = MinerState::WaitNoOre;
            snap.miner.rescan_cooldown = config.rescan_cooldown_ticks;
            return;
        }
    }
    // Delegate to normal return logic.
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

/// Begin the return-to-refinery sequence.
///
/// Chrono miners warp to the queue cell (outside the building footprint) via
/// `issue_teleport_command(is_harvester=true)`, which skips the chrono lock
/// and finishes the teleport in a single tick. `handle_return` then detects
/// adjacency and enters the normal dock sequence.
fn begin_return(
    sim: &mut Simulation,
    rules: &RuleSet,
    config: &MinerConfig,
    _path_grid: Option<&PathGrid>,
    snap: &mut MinerSnapshot,
) {
    if let Some((rsid, dock)) = find_nearest_refinery(
        sim,
        rules,
        sim.interner.resolve(snap.owner),
        sim.interner.resolve(snap.type_id),
        (snap.rx, snap.ry),
    ) {
        snap.miner.reserved_refinery = Some(rsid);
        if snap.miner.kind == MinerKind::Chrono {
            // Original engine (0x0073EE51): chrono miners only teleport if
            // distance > ChronoHarvTooFarDistance (default 50 cells). When
            // close enough, they drive like a War Miner.
            let threshold = config.too_far_threshold_chrono as u32;
            let far_enough = cell_dist_sq((snap.rx, snap.ry), dock) > threshold * threshold;

            if far_enough {
                // Warp to queue cell (outside building footprint) via the
                // teleport locomotor system. With is_harvester=true the warp
                // resolves in one tick; handle_return then detects adjacency
                // and enters Dock/WaitForDock.
                spawn_warp_effects(
                    sim,
                    rules,
                    snap.type_id,
                    (snap.rx, snap.ry, snap.z),
                    (dock.0, dock.1, snap.z),
                );
                issue_teleport_command(&mut sim.entities, snap.entity_id, dock, &rules.general, true);
            }
            // Both far (teleporting) and close (driving) → ReturnToRefinery.
            snap.miner.state = MinerState::ReturnToRefinery;
        } else {
            snap.miner.state = MinerState::ReturnToRefinery;
        }
    } else {
        snap.miner.state = MinerState::WaitNoOre;
    }
}

/// Spawn WarpOut visual effects at departure and arrival.
///
/// Self-teleport (chrono miner, chrono legionnaire) spawns the
/// `[General] WarpOut=` anim at both endpoints — same anim object twice,
/// once at the source cell and once at the destination cell. WarpIn and
/// WarpAway are reserved for the Chronosphere superweapon path; ChronoSparkle1
/// is parsed but unused by self-teleport.
///
/// Also emits chrono teleport sound events at both locations:
/// `ChronoOutSound=` at the source, `ChronoInSound=` at the destination.
/// If a sound is not configured on the unit type the corresponding event
/// is skipped.
fn spawn_warp_effects(
    sim: &mut Simulation,
    rules: &RuleSet,
    type_id: InternedId,
    depart: (u16, u16, u8),
    arrive: (u16, u16, u8),
) {
    use crate::sim::components::WorldEffect;

    /// Fallback frame count when the SHP wasn't found in the atlas.
    const FALLBACK_FRAME_COUNT: u16 = 20;

    let anim_name: &str = &rules.general.warp_out.name;
    let anim_rate: u32 = rules.general.warp_out.rate_ms;
    let anim_interned = sim.interner.intern(anim_name);

    let anim_frames: u16 = sim
        .effect_frame_counts
        .get(&anim_interned)
        .copied()
        .unwrap_or(FALLBACK_FRAME_COUNT);

    // Departure WarpOut.
    sim.world_effects.push(WorldEffect {
        shp_name: anim_interned,
        rx: depart.0,
        ry: depart.1,
        z: depart.2,
        frame: 0,
        total_frames: anim_frames,
        rate_ms: anim_rate,
        elapsed_ms: 0,
        translucent: true,
        delay_ms: 0,
    });

    // Arrival WarpOut.
    sim.world_effects.push(WorldEffect {
        shp_name: anim_interned,
        rx: arrive.0,
        ry: arrive.1,
        z: arrive.2,
        frame: 0,
        total_frames: anim_frames,
        rate_ms: anim_rate,
        elapsed_ms: 0,
        translucent: true,
        delay_ms: 0,
    });

    // Resolve per-unit ChronoOut/InSound and emit positional sound events.
    // Source cell gets ChronoOutSound; destination gets ChronoInSound.
    // Per-unit value wins; if absent, fall back to the Rules [General] default.
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

/// Dock cell (queue position) — uses art.ini QueueingCell when available.
///
/// Falls back to geometric approximation: one cell east of the building's
/// east edge, vertically centered. Standard refineries (4x3) produce (rx+4, ry+1)
/// which matches art.ini QueueingCell=4,1.
pub(crate) fn refinery_dock_cell(
    rx: u16,
    ry: u16,
    width: u16,
    height: u16,
    queueing_cell: Option<(u16, u16)>,
) -> (u16, u16) {
    super::miner_dock_sequence::refinery_queue_cell(rx, ry, width, height, queueing_cell)
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
/// Ranking mirrors `pick_best_resource_node`: gems over ore → highest density
/// → nearest → deterministic (ry, rx) tie-break.
pub(crate) fn search_local_ore(
    nodes: &std::collections::BTreeMap<(u16, u16), ResourceNode>,
    center: (u16, u16),
    radius: u16,
    filter: Option<&dyn Fn((u16, u16)) -> bool>,
) -> Option<(u16, u16)> {
    let mut best: Option<((u8, u32, u32, u16, u16), (u16, u16))> = None;
    let min_x = center.0.saturating_sub(radius);
    let max_x = center.0.saturating_add(radius);
    let min_y = center.1.saturating_sub(radius);
    let max_y = center.1.saturating_add(radius);

    for (&(rx, ry), node) in nodes {
        if node.remaining == 0 || rx < min_x || rx > max_x || ry < min_y || ry > max_y {
            continue;
        }
        let dx = rx as i64 - center.0 as i64;
        let dy = ry as i64 - center.1 as i64;
        let dist_sq = (dx * dx + dy * dy) as u32;
        if dist_sq > (radius as u32) * (radius as u32) {
            continue; // circular, not square
        }
        if let Some(f) = filter
            && !f((rx, ry))
        {
            continue;
        }
        let type_rank: u8 = if node.resource_type == ResourceType::Ore {
            1
        } else {
            0
        };
        let density_rank: u32 = u32::MAX - node.remaining as u32;
        let rank = (type_rank, density_rank, dist_sq, ry, rx);
        match best {
            Some((ref cur, _)) if rank >= *cur => {}
            _ => best = Some((rank, (rx, ry))),
        }
    }
    best.map(|(_, cell)| cell)
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

/// True if `pos` is at `target` or cardinally/diagonally adjacent (1 cell away).
/// Used for dock arrival checks — buildings occupy their cells, so miners
/// park adjacent to the refinery rather than on top of it.
fn is_adjacent_or_at(pos: (u16, u16), target: (u16, u16)) -> bool {
    let dx = (pos.0 as i32 - target.0 as i32).unsigned_abs();
    let dy = (pos.1 as i32 - target.1 as i32).unsigned_abs();
    dx <= 1 && dy <= 1
}

/// Squared Euclidean distance between two cells.
///
/// Compare against `threshold * threshold` to avoid sqrt. Matches the original
/// engine's `Sqrt_Approx` pattern for the `ChronoHarvTooFarDistance` check:
/// chrono miners teleport when far, drive when close.
fn cell_dist_sq(a: (u16, u16), b: (u16, u16)) -> u32 {
    let dx = a.0 as i32 - b.0 as i32;
    let dy = a.1 as i32 - b.1 as i32;
    (dx * dx + dy * dy) as u32
}

/// Check whether the player owns at least one Ore Purifier building.
///
/// Scans entities for any alive structure owned by this player where the rules
/// ObjectType has `ore_purifier == true`. When true, all harvested ore receives
/// PurifierBonus (default 25%) extra credits during unloading.
pub(crate) fn player_has_purifier(sim: &Simulation, rules: &RuleSet, owner: &str) -> bool {
    sim.entities.values().any(|e| {
        e.category == EntityCategory::Structure
            && sim.interner.resolve(e.owner).eq_ignore_ascii_case(owner)
            && rules
                .object_case_insensitive(sim.interner.resolve(e.type_ref))
                .is_some_and(|obj| obj.ore_purifier)
    })
}
