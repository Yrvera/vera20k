//! Miner state machine — the Harvest mission handler body
//! (SearchOre→Harvest→Return→Unload loop).
//!
//! Since the handler absorption, each live miner is dispatched individually
//! from the per-object AI host (the Unit arm of `techno_ai_shell`, the
//! Mission_Dispatch position): snapshot the miner, run one FSM step, commit
//! the mutations plus the dispatch epilogue back to the entity. The FSM
//! cursor of record is `MissionCom::handler_state`; the snapshot carries a
//! decoded working copy in `MinerSnapshot::state`.
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
use crate::sim::occupancy::OccupancyGrid;
use crate::sim::pathfinding::PathGrid;
use crate::sim::pathfinding::zone_map::{ZONE_INVALID, ZoneGrid};
use crate::sim::production::pick_best_resource_node;
use crate::sim::world::{SimSoundEvent, Simulation};
use crate::util::fixed_math::{SimFixed, ra2_speed_to_leptons_per_second};

use crate::sim::debug_event_log::DebugEventKind;
use crate::sim::intern::InternedId;

use crate::sim::production::foundation_dimensions;
use crate::util::lepton::ground_height_leptons;

/// Compare object-coordinate distance in leptons against `threshold_cells * 256`.
/// Strict `>` — a miner exactly at the threshold still uses the close radio path.
/// Used by both CMIN (`ChronoHarvTooFarDistance=50`) and HARV
/// (`HarvesterTooFarDistance=5`); caller picks the kind-appropriate threshold.
fn return_exceeds_too_far_threshold(
    sim: &Simulation,
    miner_sid: u64,
    refinery_sid: u64,
    threshold_cells: u16,
) -> Option<bool> {
    let miner = sim.substrate.entities.get(miner_sid)?;
    let refinery = sim.substrate.entities.get(refinery_sid)?;
    if refinery.dying || refinery.health.current == 0 {
        return None;
    }

    let miner_x = i64::from(miner.position.rx) * 256 + miner.position.sub_x.to_num::<i64>();
    let miner_y = i64::from(miner.position.ry) * 256 + miner.position.sub_y.to_num::<i64>();
    let refinery_x =
        i64::from(refinery.position.rx) * 256 + refinery.position.sub_x.to_num::<i64>();
    let refinery_y =
        i64::from(refinery.position.ry) * 256 + refinery.position.sub_y.to_num::<i64>();
    let terrain = sim.resolved_terrain.as_ref()?;
    let miner_cell = terrain.cell(miner.position.rx, miner.position.ry)?;
    let refinery_cell = terrain.cell(refinery.position.rx, refinery.position.ry)?;
    let miner_z = i64::from(
        ground_height_leptons(
            miner_cell.level,
            miner_cell.slope_type,
            miner_x as i32,
            miner_y as i32,
        )
        .ok()?,
    ) + if miner.on_bridge {
        i64::from(crate::sim::map::bridge_topology::BRIDGE_DECK_HEIGHT_LEPTONS)
    } else {
        0
    } + miner
        .locomotor
        .as_ref()
        .map(|locomotor| locomotor.altitude.to_num::<i64>())
        .unwrap_or(0);
    let refinery_z = i64::from(
        ground_height_leptons(
            refinery_cell.level,
            refinery_cell.slope_type,
            refinery_x as i32,
            refinery_y as i32,
        )
        .ok()?,
    ) + if refinery.on_bridge {
        i64::from(crate::sim::map::bridge_topology::BRIDGE_DECK_HEIGHT_LEPTONS)
    } else {
        0
    } + refinery
        .locomotor
        .as_ref()
        .map(|locomotor| locomotor.altitude.to_num::<i64>())
        .unwrap_or(0);

    let dx = miner_x - refinery_x;
    let dy = miner_y - refinery_y;
    let dz = miner_z - refinery_z;
    let distance_sq = dx * dx + dy * dy + dz * dz;
    let threshold = i64::from(threshold_cells.max(1)) * 256;
    Some(distance_sq > threshold * threshold)
}

#[cfg(test)]
mod gsi_04_03b_tests {
    use super::*;
    use crate::map::resolved_terrain::{ResolvedTerrainCell, ResolvedTerrainGrid};
    use crate::rules::locomotor_type::LocomotorKind;
    use crate::sim::game_entity::GameEntity;
    use crate::sim::movement::locomotor::LocomotorState;

    fn sloped_cell() -> ResolvedTerrainCell {
        ResolvedTerrainCell {
            rx: 0,
            ry: 0,
            source_tile_index: 0,
            source_sub_tile: 0,
            final_tile_index: 0,
            final_sub_tile: 0,
            is_wood_bridge_repair_tile: false,
            level: 0,
            filled_clear: true,
            tileset_index: Some(0),
            land_type: 0,
            yr_cell_land_type: 0,
            slope_type: 1,
            template_height: 0,
            render_offset_x: 0,
            render_offset_y: 0,
            terrain_class: Default::default(),
            speed_costs: Default::default(),
            is_water: false,
            is_cliff_like: false,
            is_rough: false,
            is_road: false,
            is_cliff_redraw: false,
            variant: 0,
            has_ramp: true,
            canonical_ramp: None,
            ground_walk_blocked: false,
            terrain_object_blocks: false,
            terrain_object_occupation: None,
            overlay_blocks: false,
            overlay_zone_type: None,
            outside_playfield: false,
            zone_type: 0,
            base_ground_walk_blocked: false,
            base_build_blocked: false,
            base_land_type: 0,
            base_yr_cell_land_type: 0,
            base_terrain_class: Default::default(),
            base_speed_costs: Default::default(),
            build_blocked: false,
            has_bridge_deck: false,
            bridge_walkable: false,
            bridge_transition: false,
            bridge_deck_level: 0,
            bridge_layer: None,
            bridge_facts: Default::default(),
            tube_index: None,
            radar_left: [0; 3],
            radar_right: [0; 3],
            accepts_smudge: true,
            allows_tiberium: false,
            has_damaged_data: false,
            bridgehead_anchor_class_at_load: None,
        }
    }

    #[test]
    fn gsi_04_03b_miner_return_distance_uses_terrain_bridge_and_altitude_z() {
        let mut sim = Simulation::new();
        let mut miner = GameEntity::test_default(1, "CMIN", "Allies", 0, 0);
        miner.position.sub_x = SimFixed::from_num(0);
        let mut refinery = GameEntity::test_default(2, "GAOREP", "Allies", 0, 0);
        refinery.position.sub_x = SimFixed::from_num(255);
        sim.substrate.entities.insert(miner);
        sim.substrate.entities.insert(refinery);
        sim.resolved_terrain = Some(ResolvedTerrainGrid::from_cells(1, 1, vec![sloped_cell()]));

        assert_eq!(
            return_exceeds_too_far_threshold(&sim, 1, 2, 1),
            Some(true),
            "255 horizontal leptons plus the slope Z delta exceeds one cell"
        );

        sim.substrate.entities.get_mut(1).unwrap().position.sub_x = SimFixed::from_num(0);
        sim.substrate.entities.get_mut(2).unwrap().position.sub_x = SimFixed::from_num(0);
        assert_eq!(return_exceeds_too_far_threshold(&sim, 1, 2, 1), Some(false));

        sim.substrate.entities.get_mut(1).unwrap().on_bridge = true;
        assert_eq!(
            return_exceeds_too_far_threshold(&sim, 1, 2, 1),
            Some(true),
            "OnBridge coordinate Z contributes the full deck offset"
        );

        let miner = sim.substrate.entities.get_mut(1).unwrap();
        miner.on_bridge = false;
        let mut locomotor = LocomotorState::for_test_kind(LocomotorKind::Fly);
        locomotor.altitude = SimFixed::from_num(300);
        miner.locomotor = Some(locomotor);
        assert_eq!(
            return_exceeds_too_far_threshold(&sim, 1, 2, 1),
            Some(true),
            "locomotor altitude contributes to raw object-coordinate Z"
        );
    }

    #[test]
    fn gsi_04_05_sequential_miner_helper_reserves_head_without_reconcile() {
        let mut sim = Simulation::new();
        let owner = sim.interner.intern("AMERICANS");
        let type_ref = sim.interner.intern("HARV");
        for (entity_id, rx, facing) in [(1, 1, 0x40), (2, 3, 0xC0)] {
            let mut miner = GameEntity::test_default(entity_id, "HARV", "AMERICANS", rx, 2);
            miner.owner = owner;
            miner.type_ref = type_ref;
            miner.facing = facing;
            miner.locomotor = Some(LocomotorState::for_test_kind(LocomotorKind::Drive));
            miner.drive_locomotion = Some(Default::default());
            sim.substrate.entities.insert(miner);
            sim.add_entity_occupancy(entity_id);
        }

        let grid = PathGrid::new(5, 5);
        let shared_head = (2, 2);
        issue_move_if_idle(
            &mut sim,
            None,
            &grid,
            1,
            shared_head,
            SimFixed::from_num(128),
        );

        assert_eq!(
            sim.substrate
                .entities
                .get(1)
                .and_then(|entity| entity.drive_locomotion.as_ref())
                .and_then(|drive| drive.occupation_head_to)
                .map(|head| (head.rx, head.ry)),
            Some(shared_head)
        );
        assert!(sim.substrate.cell_occupation.occupied_by_other(
            shared_head.0,
            shared_head.1,
            MovementLayer::Ground,
            2,
        ));

        issue_move_if_idle(
            &mut sim,
            None,
            &grid,
            2,
            shared_head,
            SimFixed::from_num(128),
        );

        let second = sim.substrate.entities.get(2).expect("second miner");
        assert_ne!(
            second
                .drive_locomotion
                .as_ref()
                .and_then(|drive| drive.occupation_head_to)
                .map(|head| (head.rx, head.ry)),
            Some(shared_head),
            "the second production miner helper must observe the first head mark immediately"
        );
        assert_ne!(
            second
                .movement_target
                .as_ref()
                .and_then(|movement| movement.final_goal),
            Some(shared_head),
            "contention must be resolved before any movement-tick reconciliation"
        );
    }
}

/// The per-frame handler return. Native Mission_Harvest returns it from the
/// harvesting state (all paths), the enter/dock state, and the productive
/// search paths (ore found and moved toward); every other exit goes through
/// the default `[Harvest] Rate` epilogue or the fixed no-ore wait below.
pub(super) const DISPATCH_NEXT_FRAME: i32 = 1;

/// Jitter ceiling of the default handler epilogue: `RandomRanged(0, 2)`.
const RATE_EPILOGUE_JITTER_MAX_FRAMES: u32 = 2;

/// Keyless-`[Harvest]` fallback for the epilogue base; the stock `Rate=.016`
/// resolves to `ftol(.016 × 900) = 14` from the mission-control table, so this
/// is only reached when a mod strips the section. The gamemd MissionControl
/// ctor default for that case is UNCHECKED.
const HARVEST_RATE_FALLBACK_FRAMES: u8 = 14;

/// Install the native default handler epilogue as the dispatch delay:
/// `ftol([Harvest] Rate × 900)` plus one `RandomRanged(0, 2)` drawn on the
/// scenario stream. Paths that take it: the return/finding-home state on
/// every dispatch, the idle state on every dispatch, the search state's
/// archive-consume and still-driving returns, and any cursor outside the
/// native handler's switch. The base lookup consumes no RNG, so the single
/// draw here keeps the scenario stream position aligned with the native
/// epilogue.
pub(super) fn arm_rate_epilogue(sim: &mut Simulation, rules: &RuleSet, snap: &mut MinerSnapshot) {
    let base = super::miner_dock_sequence::mission_base_frames(
        rules,
        crate::sim::mission::MissionType::Harvest,
        HARVEST_RATE_FALLBACK_FRAMES,
    );
    let jitter = sim
        .miner_jitter_rng()
        .next_range_u32_inclusive(0, RATE_EPILOGUE_JITTER_MAX_FRAMES);
    snap.dispatch_delay = i32::from(base) + jitter as i32;
}

/// Snapshot of one miner entity for one Harvest dispatch.
pub(super) struct MinerSnapshot {
    pub(super) entity_id: u64,
    pub(super) owner: InternedId,
    pub(super) type_id: InternedId,
    pub(super) rx: u16,
    pub(super) ry: u16,
    pub(super) speed: SimFixed,
    pub(super) miner: Miner,
    /// FSM cursor working copy — decoded from `MissionCom::handler_state` at
    /// dispatch entry, committed back through it at dispatch commit.
    pub(super) state: MinerState,
    /// Handler return value: frames until the next dispatch, written into the
    /// mission dispatch timer by the commit (the native post-handler epilogue).
    pub(super) dispatch_delay: i32,
    /// Buffered miner state change events — flushed to entity at commit.
    pub(super) debug_events: Vec<(String, String)>,
    /// Buffered dock phase change events — flushed to entity at commit.
    pub(super) debug_dock_events: Vec<(String, String)>,
}

/// Release dock reservations held by/on dying objects before the Harvest
/// dispatches run, so queued miners promote without waiting through the death
/// anim. Gated on a live dispatchable miner existing — matching the legacy
/// global tick, whose sweep only ran when its snapshot list was non-empty
/// (hash-identical when no miners are present).
pub(crate) fn sweep_dead_dock_reservations(sim: &mut Simulation) {
    let order = sim.live_object_order_snapshot();
    sweep_dead_dock_reservations_for_keys(sim, &order);
}

fn sweep_dead_dock_reservations_for_keys(sim: &mut Simulation, order: &[u64]) {
    let any_miner = order.iter().any(|&id| {
        sim.substrate.entities.get(id).is_some_and(|e| {
            !e.dying
                && e.miner
                    .as_ref()
                    .is_some_and(|miner| miner.kind != MinerKind::Slave)
        })
    });
    if !any_miner {
        return;
    }
    let alive_sids: BTreeSet<u64> = sim
        .substrate
        .entities
        .values()
        .filter(|e| !e.dying)
        .map(|e| e.stable_id)
        .collect();
    sim.production.dock_reservations.cleanup_dead(&alive_sids);
}

/// Build the dispatch snapshot for one live, non-dying, non-slave miner.
/// Returns `None` when the object is not a dispatchable miner.
pub(super) fn build_miner_snapshot(
    sim: &Simulation,
    rules: &RuleSet,
    id: u64,
) -> Option<MinerSnapshot> {
    let entity = sim.substrate.entities.get(id)?;
    // A Dying miner corpse (sold/captured this tick, awaiting the end-of-tick
    // drain) must not move, harvest, or deposit.
    if entity.dying {
        return None;
    }
    let miner = entity.miner.as_ref()?;
    // Slave Miners use their own system (slave_miner.rs) — never dispatched here.
    if miner.kind == MinerKind::Slave {
        return None;
    }
    // Use the authentic RA2 speed formula: Speed=4 → ~0.586 cells/sec.
    let raw_speed: i32 = sim
        .object_type(entity.type_ref, rules)
        .map(|obj| obj.speed.max(1))
        .unwrap_or(4);
    let speed: SimFixed = ra2_speed_to_leptons_per_second(raw_speed);
    let cursor = MinerState::from_cursor(entity.mission.handler_state());
    debug_assert!(
        cursor.is_some(),
        "miner {} carries an out-of-vocabulary Harvest cursor {:#x}",
        id,
        entity.mission.handler_state(),
    );
    Some(MinerSnapshot {
        entity_id: id,
        owner: entity.owner,
        type_id: entity.type_ref,
        rx: entity.position.rx,
        ry: entity.position.ry,
        speed,
        miner: miner.clone(),
        state: cursor.unwrap_or(MinerState::SearchOre),
        dispatch_delay: DISPATCH_NEXT_FRAME,
        debug_events: Vec::new(),
        debug_dock_events: Vec::new(),
    })
}

/// Commit one dispatched snapshot back to the entity: miner mutations, the
/// FSM cursor of record (`MissionCom::handler_state`), the post-handler
/// dispatch-timer epilogue (verified host shape: start = current frame,
/// delay = handler return), buffered debug events, and the render-side
/// harvest-visual flags (the former global-tick Phases 3/4/4b for one object).
pub(super) fn commit_miner_snapshot(sim: &mut Simulation, snap: &MinerSnapshot, now: u32) {
    let Some(entity) = sim.substrate.entities.get_mut(snap.entity_id) else {
        return;
    };
    entity.miner = Some(snap.miner.clone());
    entity.mission.set_handler_state(snap.state.cursor());
    entity
        .mission
        .write_dispatch_epilogue(now as i32, snap.dispatch_delay);
    for (from, to) in &snap.debug_events {
        entity.push_debug_event(
            sim.session.tick as u32,
            DebugEventKind::MinerStateChange {
                from: from.clone(),
                to: to.clone(),
            },
        );
    }
    for (from, to) in &snap.debug_dock_events {
        entity.push_debug_event(
            sim.session.tick as u32,
            DebugEventKind::DockPhaseChange {
                from: from.clone(),
                to: to.clone(),
            },
        );
    }
    // Drive VoxelAnimation + HarvestOverlay (oregath.shp) from the Harvest
    // cursor — render-side flags, never hashed.
    let is_harvesting: bool = snap.state == MinerState::Harvest;
    if let Some(ref mut va) = entity.voxel_animation {
        va.playing = is_harvesting;
        if !is_harvesting {
            va.frame = 0;
            va.elapsed_frames = 0;
        }
    }
    if let Some(ref mut ho) = entity.harvest_overlay {
        if is_harvesting && !ho.visible {
            ho.visible = true;
            ho.frame = 0;
            ho.elapsed_frames = 0;
        } else if !is_harvesting && ho.visible {
            ho.visible = false;
            ho.frame = 0;
            ho.elapsed_frames = 0;
        }
    }
}

/// Test-only mirror of the production Harvest dispatch walk: the same
/// per-entity dispatch (timer gate + epilogue) the host Unit arm performs, in
/// live-object order, with the legacy stable-id fallback for direct-insert
/// fixtures that never build a LogicVector.
#[cfg(test)]
pub(crate) fn tick_miners(
    sim: &mut Simulation,
    rules: &RuleSet,
    config: &MinerConfig,
    path_grid: Option<&PathGrid>,
) {
    tick_miners_with_overlay_registry(sim, rules, config, path_grid, None);
}

#[cfg(test)]
pub(crate) fn tick_miners_with_overlay_registry(
    sim: &mut Simulation,
    rules: &RuleSet,
    config: &MinerConfig,
    path_grid: Option<&PathGrid>,
    overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
) {
    let live_order = sim.live_object_order_snapshot();
    let keys: Vec<u64> = if live_order.is_empty() {
        sim.substrate.entities.keys_sorted()
    } else {
        live_order
    };
    sweep_dead_dock_reservations_for_keys(sim, &keys);
    for id in keys {
        super::harvest_mission::dispatch_harvest_for_object(
            sim,
            rules,
            config,
            path_grid,
            overlay_registry,
            id,
        );
    }
}

/// Process one miner through one tick of its state machine.
///
/// `pub(super)` so the Harvest mission handler seam (`harvest_mission.rs`) can
/// dispatch to it; the visibility widening is behavior-neutral.
pub(super) fn process_miner(
    sim: &mut Simulation,
    rules: &RuleSet,
    config: &MinerConfig,
    path_grid: Option<&PathGrid>,
    overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
    snap: &mut MinerSnapshot,
) {
    if sim
        .substrate
        .entities
        .get(snap.entity_id)
        .is_some_and(|entity| entity.forced_drive_track.is_some())
    {
        return;
    }

    let state_before = format!("{:?}", snap.state);
    match snap.state {
        MinerState::SearchOre => {
            handle_search_ore(sim, rules, config, path_grid, overlay_registry, snap)
        }
        MinerState::MoveToOre => {
            handle_move_to_ore(sim, rules, config, path_grid, overlay_registry, snap)
        }
        MinerState::Harvest => {
            handle_harvest(sim, rules, config, path_grid, overlay_registry, snap)
        }
        MinerState::ReturnToRefinery => {
            handle_return(sim, rules, config, path_grid, snap);
            // Native return/finding-home state has no per-frame exit: every
            // dispatch leaves through the default Rate epilogue.
            arm_rate_epilogue(sim, rules, snap);
        }
        MinerState::Dock => {
            super::miner_dock_sequence::handle_dock_sequence(sim, rules, config, path_grid, snap)
        }
        MinerState::Unload => {
            // Legacy state — production code never enters this path. If we
            // encounter it (e.g., a save from before the FSM rewrite), fall
            // through to SearchOre. Outside the native handler's switch, so
            // it exits through the default epilogue.
            snap.state = MinerState::SearchOre;
            arm_rate_epilogue(sim, rules, snap);
        }
        MinerState::WaitNoOre => {
            handle_wait_no_ore(snap, sim.session.binary_frame);
            // Native idle state falls straight into the default epilogue.
            arm_rate_epilogue(sim, rules, snap);
        }
        MinerState::ForcedReturn => {
            handle_forced_return(sim, rules, config, path_grid, snap);
            // VERA-internal cursor; outside the native handler's switch, so
            // it exits through the default epilogue like any high cursor.
            arm_rate_epilogue(sim, rules, snap);
        }
    }
    let state_after = format!("{:?}", snap.state);
    if state_before != state_after {
        log::info!(
            "MINER {} state: {} → {} pos=({},{}) target_ore={:?} cargo={} timer={:?}",
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
    let entity = sim.substrate.entities.get(snap.entity_id);
    let mz = entity
        .and_then(|e| e.locomotor.as_ref())
        .map(|loc| loc.movement_zone)
        .unwrap_or(MovementZone::Normal);
    let layer = entity
        .map(|e| e.movement_layer_or_ground())
        .unwrap_or(MovementLayer::Ground);
    let zone_grid = sim.zone_grid.as_ref()?;
    let anchor = effective_zone_cell(zone_grid, mz, snap.rx, snap.ry)?;
    let occupancy = &sim.substrate.occupancy;
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
    sim: &mut Simulation,
    rules: &RuleSet,
    config: &MinerConfig,
    path_grid: Option<&PathGrid>,
    overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
    snap: &mut MinerSnapshot,
) {
    // gamemd's Mission_Harvest state 0 checks full storage before scanning
    // ore, so a full miner that lost its refinery keeps trying to return.
    if snap.miner.is_full() {
        snap.miner.target_ore_cell = None;
        snap.state = MinerState::ReturnToRefinery;
        return;
    }

    // L10: the post-unload ore search is paced by the Mission_Harvest epilogue's
    // RandomRanged(0,2) jitter, armed at the state-4 dock exit. Wait it out so the
    // search resumes at exit_frame + jitter, not immediately. For every other
    // entry the harvest timer is long-elapsed (always due), so this is a no-op.
    if !snap.miner.harvest_timer.due(sim.session.binary_frame) {
        return;
    }

    /// Scan decision, computed under the scan filter's immutable `sim` borrow
    /// and committed after it drops (the epilogue draw needs `&mut sim`).
    enum ScanOutcome {
        /// Ghost-cell archive consumed — the native archive-target return.
        Archive((u16, u16)),
        /// Fresh ore target from the bounded or global scan.
        Found((u16, u16)),
        /// No reachable ore anywhere.
        NoOre,
    }

    let outcome = {
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
        let mut archive_hit = None;
        if let Some(archive) = snap.miner.last_harvest_cell {
            let archive_has_ore = resource_cell_present(sim, rules, overlay_registry, archive);
            let archive_reachable = filter_ref.is_none_or(|f| f(archive));
            if archive_has_ore && archive_reachable {
                archive_hit = Some(ScanOutcome::Archive(archive));
            } else {
                // Stale archive (depleted or unreachable) — drop it so we
                // don't keep retrying.
                snap.miner.last_harvest_cell = None;
            }
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
        archive_hit.unwrap_or_else(|| {
            search_local_resource(
                sim,
                rules,
                overlay_registry,
                (snap.rx, snap.ry),
                config.long_scan_radius,
                filter_ref,
                config,
            )
            // Global search — find nearest reachable ore anywhere on the map.
            .or_else(|| {
                pick_best_resource_cell(
                    sim,
                    rules,
                    overlay_registry,
                    (snap.rx, snap.ry),
                    filter_ref,
                )
            })
            .map_or(ScanOutcome::NoOre, ScanOutcome::Found)
        })
    };

    match outcome {
        ScanOutcome::Archive(cell) => {
            snap.miner.target_ore_cell = Some(cell);
            snap.state = MinerState::MoveToOre;
            snap.miner.last_harvest_cell = None;
            // The native archive-consume exit goes through the default Rate
            // epilogue, not the per-frame return.
            arm_rate_epilogue(sim, rules, snap);
        }
        ScanOutcome::Found(cell) => {
            snap.miner.target_ore_cell = Some(cell);
            snap.state = MinerState::MoveToOre;
            // Productive search: per-frame dispatch (native return 1).
        }
        ScanOutcome::NoOre => {
            // Native: no ore, no owner destination, no archive parks the
            // handler in the idle state and returns the fixed 105-frame wait
            // directly — bypassing the Rate epilogue, so no RNG draw. The
            // dispatch delay carries the wait; the internal rescan gate is
            // armed to the same expiry (the gate fires inclusively at
            // start + duration), so the two never double-count.
            snap.state = MinerState::WaitNoOre;
            snap.miner.rescan_cooldown.arm(
                sim.session.binary_frame,
                u32::from(config.rescan_cooldown_ticks),
            );
            snap.dispatch_delay = i32::from(config.rescan_cooldown_ticks);
        }
    }
}

fn handle_move_to_ore(
    sim: &mut Simulation,
    rules: &RuleSet,
    config: &MinerConfig,
    path_grid: Option<&PathGrid>,
    overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
    snap: &mut MinerSnapshot,
) {
    let has_destination_or_movement =
        sim.substrate
            .entities
            .get(snap.entity_id)
            .is_some_and(|entity| {
                entity.navigation.nav_com.is_some() || entity.movement_target.is_some()
            });

    // Native Search_For_Tiberium_And_Move returns immediately for a non-null
    // owner NavCom before target validation, arrival, or scan; the handler
    // then exits through the default Rate epilogue (the still-driving
    // return). MovementTarget remains Rust's transitional second owner until
    // the broader Drive host is migrated.
    if has_destination_or_movement {
        arm_rate_epilogue(sim, rules, snap);
        return;
    }

    let Some(current_target) = snap.miner.target_ore_cell else {
        snap.state = MinerState::SearchOre;
        return;
    };

    // Check if current target has been depleted.
    let still_has_ore = resource_cell_present(sim, rules, overlay_registry, current_target);
    if !still_has_ore {
        snap.miner.target_ore_cell = None;
        snap.state = MinerState::SearchOre;
        return;
    }

    // Wait for any in-progress teleport to complete (chrono delay).
    // Must be checked BEFORE the arrival check — during ChronoDelay the
    // entity is already at the target position but still materializing
    // (50% translucent). Transitioning to Harvest during delay would skip
    // the warp-in visual.
    let has_teleport = sim
        .substrate
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
        search_local_resource(
            sim,
            rules,
            overlay_registry,
            (snap.rx, snap.ry),
            config.long_scan_radius,
            filter_ref,
            config,
        )
    };
    let target = new_target.unwrap_or(current_target);
    if target != current_target {
        snap.miner.target_ore_cell = Some(target);
    }

    // Arrived?
    if (snap.rx, snap.ry) == target {
        snap.state = MinerState::Harvest;
        // This physical-arrival anchor is legacy Rust behavior; native initializes
        // the timer when search/move succeeds, a separately tracked acquisition-
        // timing drift. Retain +1 for the verified mission-before-timer observation.
        snap.miner.harvest_timer.arm(
            sim.session.binary_frame,
            u32::from(config.harvest_tick_interval) + 1,
        );
        return;
    }

    if let Some(grid) = path_grid {
        let _ = issue_stock_miner_drive_move(sim, rules, grid, snap.entity_id, target);
    }
}

fn handle_harvest(
    sim: &mut Simulation,
    rules: &RuleSet,
    config: &MinerConfig,
    path_grid: Option<&PathGrid>,
    overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
    snap: &mut MinerSnapshot,
) {
    // Frame-anchored gate (was a per-tick countdown).
    if !snap.miner.harvest_timer.due(sim.session.binary_frame) {
        return;
    }

    if snap.miner.is_full() {
        // Harvest_Ore_Tick checks full storage before Reduce_Tiberium, resets its
        // timer, and returns failure. Mission_Harvest then writes return state
        // before choosing the ghost/archive cell; state-2 work waits for the next
        // mission dispatch.
        snap.miner.harvest_timer.reset(sim.session.binary_frame);
        snap.state = MinerState::ReturnToRefinery;
        save_archive_via_short_scan(sim, rules, config, path_grid, overlay_registry, snap);
        return;
    }

    let cell = (snap.rx, snap.ry);
    let empty: u16 = snap
        .miner
        .capacity_bales
        .saturating_sub(snap.miner.cargo.len() as u16);

    // Shared CellClass::Reduce_Tiberium boundary: caller owns cargo insertion,
    // while the helper owns overlay/resource/dirty/queue side effects.
    let reduction =
        sim.reduce_tiberium_at_with_native_context(cell, empty, Some(rules), overlay_registry);

    if reduction.removed_amount > 0 {
        let Some(resource_type) = reduction.resource_type else {
            return;
        };
        let value = match resource_type {
            ResourceType::Ore => config.ore_bale_value,
            ResourceType::Gem => config.gem_bale_value,
        };
        snap.miner
            .cargo
            .extend((0..reduction.removed_amount).map(|_| CargoBale {
                resource_type,
                value,
            }));

        // A positive extraction is success even when it fills storage. Native
        // Mission_Harvest remains in state 1 and observes fullness only at the
        // next helper gate: 9 * HarvesterLoadRate + 1 frame numbers under the
        // verified mission-before-timer order.
        snap.miner.harvest_timer.arm(
            sim.session.binary_frame,
            u32::from(config.harvest_tick_interval) + 1,
        );
        return;
    }

    // No bales extracted while not full. Run the caller-owned short continuation
    // scan; a hit moves toward the next patch, while a miss begins the existing
    // no-resource return path.

    // Short scan. The filter's closure captures `&sim`; scope it so the
    // immutable borrow drops before `begin_return` needs `&mut sim` below.
    let continuation_target = {
        let scan_filter = build_scan_filter(sim, path_grid, snap);
        let filter_ref: Option<&dyn Fn((u16, u16)) -> bool> = scan_filter.as_deref();
        search_local_resource(
            sim,
            rules,
            overlay_registry,
            (snap.rx, snap.ry),
            config.local_continuation_radius,
            filter_ref,
            config,
        )
    };
    if let Some(next_cell) = continuation_target {
        snap.miner.target_ore_cell = Some(next_cell);
        snap.state = MinerState::MoveToOre;
        return;
    }

    // Scan miss while not full → return to refinery, clear archive.
    snap.miner.last_harvest_cell = None;
    begin_return(sim, rules, config, path_grid, snap);
}

/// Save a fresh ghost-cell archive by running a short-radius scan from
/// the miner's current position. The due full-failure caller invokes this only
/// after selecting Return, so the next `SearchOre` cycle can return directly to
/// a nearby still-productive patch. On scan miss, clears the archive.
fn save_archive_via_short_scan(
    sim: &Simulation,
    rules: &RuleSet,
    config: &MinerConfig,
    path_grid: Option<&PathGrid>,
    overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
    snap: &mut MinerSnapshot,
) {
    let scan_filter = build_scan_filter(sim, path_grid, snap);
    let filter_ref: Option<&dyn Fn((u16, u16)) -> bool> = scan_filter.as_deref();
    snap.miner.last_harvest_cell = search_local_resource(
        sim,
        rules,
        overlay_registry,
        (snap.rx, snap.ry),
        config.local_continuation_radius,
        filter_ref,
        config,
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
        .substrate
        .entities
        .get(snap.entity_id)
        .is_some_and(|e| e.teleport_state.is_some());
    if has_teleport {
        return;
    }

    // Active-YR HARV state 2 checks its NavCom before refinery selection.
    // MovementTarget remains Rust's transitional duplicate movement owner.
    let has_destination_or_movement = snap.miner.kind == MinerKind::War
        && sim
            .substrate
            .entities
            .get(snap.entity_id)
            .is_some_and(|entity| {
                entity.navigation.nav_com.is_some() || entity.movement_target.is_some()
            });
    if has_destination_or_movement {
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
            if try_begin_close_return_radio(sim, rules, config, path_grid, snap, rsid) {
                return;
            }
            if try_issue_standard_far_return_drive(sim, rules, config, path_grid, snap, rsid) {
                return;
            }
        } else {
            snap.state = MinerState::WaitNoOre;
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
        snap.miner.dock_enter_retry.clear();
        snap.miner.exit_cell = None;
        if snap.miner.is_full() {
            snap.miner.target_ore_cell = None;
            snap.state = MinerState::ReturnToRefinery;
        } else {
            snap.state = MinerState::SearchOre;
        }
        return;
    };

    let moving = sim
        .substrate
        .entities
        .get(snap.entity_id)
        .is_some_and(|entity| entity.movement_target.is_some());
    if !moving && try_issue_chrono_far_return_teleport(sim, rules, config, path_grid, snap, ref_sid)
    {
        return;
    }
    if try_begin_close_return_radio(sim, rules, config, path_grid, snap, ref_sid) {
        return;
    }
    if !moving && try_issue_standard_far_return_drive(sim, rules, config, path_grid, snap, ref_sid)
    {
        return;
    }

    let at_dock = (snap.rx, snap.ry) == dock;
    let contact = if snap.miner.kind == MinerKind::Chrono {
        at_dock
    } else {
        let stopped_close_enough =
            sim.substrate
                .entities
                .get(snap.entity_id)
                .is_some_and(|entity| {
                    entity.movement_target.is_none()
                        && is_within_close_enough(
                            (snap.rx, snap.ry),
                            dock,
                            rules.general.close_enough,
                        )
                });
        is_adjacent_or_at((snap.rx, snap.ry), dock) || stopped_close_enough
    };

    if contact {
        snap.state = MinerState::Dock;
        snap.miner.dock_phase = RefineryDockPhase::Approach;
        snap.miner.dock_enter_retry.clear();
        return;
    }

    if let Some(grid) = path_grid {
        issue_move_if_idle(sim, Some(rules), grid, snap.entity_id, dock, snap.speed);
    }
}

fn handle_wait_no_ore(snap: &mut MinerSnapshot, now: u32) {
    if !snap.miner.rescan_cooldown.due(now) {
        return;
    }
    snap.state = MinerState::SearchOre;
}

fn handle_forced_return(
    sim: &mut Simulation,
    rules: &RuleSet,
    config: &MinerConfig,
    path_grid: Option<&PathGrid>,
    snap: &mut MinerSnapshot,
) {
    let has_teleport = sim
        .substrate
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
            snap.state = MinerState::WaitNoOre;
            snap.miner.rescan_cooldown.arm(
                sim.session.binary_frame,
                u32::from(config.rescan_cooldown_ticks),
            );
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
    rules: &RuleSet,
    overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
    cell: (u16, u16),
    config: &MinerConfig,
) -> Option<CargoBale> {
    let outcome =
        sim.reduce_tiberium_at_with_native_context(cell, 1, Some(rules), overlay_registry);
    if outcome.removed_amount == 0 {
        return None;
    }
    let resource_type = outcome.resource_type?;
    let value = match resource_type {
        ResourceType::Ore => config.ore_bale_value,
        ResourceType::Gem => config.gem_bale_value,
    };
    Some(CargoBale {
        resource_type,
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
    let outcome = sim.reduce_tiberium_at(cell, empty_capacity_bales);
    let Some(resource_type) = outcome.resource_type else {
        return Vec::new();
    };
    let value = match resource_type {
        ResourceType::Ore => config.ore_bale_value,
        ResourceType::Gem => config.gem_bale_value,
    };
    (0..outcome.removed_amount)
        .map(|_| CargoBale {
            resource_type,
            value,
        })
        .collect()
}

/// Begin the return-to-refinery sequence.
///
/// Miners inside their kind's "too far" threshold (CMIN:
/// `ChronoHarvTooFarDistance=50`, HARV: `HarvesterTooFarDistance=5`) keep the
/// normal refinery radio/contact path to the accepted dock cell. Miners beyond
/// that threshold use the far-return destination: the `QueueingCell` passable-cell
/// search result, not the pad/contact cell. CMIN warps to the staging cell;
/// HARV drives to it.
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
        if try_begin_close_return_radio(sim, rules, config, path_grid, snap, rsid) {
            return;
        }
        if try_issue_standard_far_return_drive(sim, rules, config, path_grid, snap, rsid) {
            return;
        }
        snap.state = MinerState::ReturnToRefinery;
    } else {
        snap.state = MinerState::WaitNoOre;
    }
}

fn try_begin_close_return_radio(
    sim: &mut Simulation,
    rules: &RuleSet,
    config: &MinerConfig,
    path_grid: Option<&PathGrid>,
    snap: &mut MinerSnapshot,
    ref_sid: u64,
) -> bool {
    // Close-radio HELLO + state=Dock fast-path is currently chrono-only.
    // HARV close path uses the existing adjacency-based contact check in
    // `handle_return`. The binary sends HELLO at HarvesterTooFarDistance
    // (5 cells) for HARV too, but generalizing the close-radio path here
    // surfaced a phase_mission_enter direct-move limitation that needs a
    // deeper fix; see 2026-05-24 audit follow-up.
    if snap.miner.kind != MinerKind::Chrono {
        return false;
    }

    match return_exceeds_too_far_threshold(
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
    super::miner_dock_sequence::bus_hello(
        sim,
        snap.entity_id,
        ref_sid,
        dock_capacity,
        admission == ContactAdmission::Accepted,
    );

    if let Some(entity) = sim.substrate.entities.get_mut(snap.entity_id) {
        entity.movement_target = None;
    }

    snap.state = MinerState::Dock;
    snap.miner.dock_queued = admission != ContactAdmission::Accepted;
    if admission == ContactAdmission::Accepted {
        // G5: the accepted close-return HELLO queues Mission_Enter via the
        // Harvest epilogue; arm the retry so the first CAN_DOCK waits the
        // ~14-16f cadence (and draws the RandomRanged(0,2) the dispatch
        // consumes), instead of an always-due next-tick collapse.
        super::miner_dock_sequence::schedule_enter_retry(sim, rules, snap);
        snap.miner.dock_phase = RefineryDockPhase::MissionEnter;
    } else {
        snap.miner.dock_enter_retry.clear();
        snap.miner.dock_phase = RefineryDockPhase::Approach;
    }

    if admission != ContactAdmission::Accepted {
        if let Some(staging) = chrono_return_staging_cell_for_sid(sim, rules, ref_sid, path_grid)
            && !is_adjacent_or_at((snap.rx, snap.ry), staging)
            && let Some(grid) = path_grid
        {
            issue_move_if_idle(sim, Some(rules), grid, snap.entity_id, staging, snap.speed);
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

    if !return_exceeds_too_far_threshold(
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

    let issued = movement::set_destination_for_teleporter_entity(
        &mut sim.substrate.entities,
        path_grid,
        snap.entity_id,
        staging,
        snap.speed,
        false,
        None,
        None,
        None,
        sim.zone_grid.as_ref(),
        None,
        false,
        &rules.general,
        true,
        true,
        false,
    );
    if issued {
        emit_chrono_warp_sounds(sim, rules, snap.type_id, (snap.rx, snap.ry), staging);
    }
    issued
}

/// HARV far-return: mirror of the chrono teleport but drives to the staging cell.
/// Triggered when a standard (War) miner is beyond `HarvesterTooFarDistance` from
/// its reserved refinery. Same QueueingCell + `Find_Nearby_Passable_Cell` staging
/// the chrono path uses; CMIN warps, HARV drives. Transitions the miner to
/// `ReturnToRefinery` so the outer state machine treats the next tick as
/// delivering (matches the binary's Mission_Harvest case-2 fallback path, which
/// stays in case-2 after issuing the destination).
fn try_issue_standard_far_return_drive(
    sim: &mut Simulation,
    rules: &RuleSet,
    config: &MinerConfig,
    path_grid: Option<&PathGrid>,
    snap: &mut MinerSnapshot,
    ref_sid: u64,
) -> bool {
    if snap.miner.kind != MinerKind::War {
        return false;
    }

    if !return_exceeds_too_far_threshold(
        sim,
        snap.entity_id,
        ref_sid,
        config.too_far_threshold_standard,
    )
    .unwrap_or(false)
    {
        return false;
    }

    let Some(staging) = chrono_return_staging_cell_for_sid(sim, rules, ref_sid, path_grid) else {
        return false;
    };
    let Some(grid) = path_grid else {
        return false;
    };

    let _ = issue_stock_miner_drive_move(sim, rules, grid, snap.entity_id, staging);
    snap.state = MinerState::ReturnToRefinery;
    true
}

fn emit_chrono_warp_sounds(
    sim: &mut Simulation,
    rules: &RuleSet,
    type_id: InternedId,
    depart: (u16, u16),
    arrive: (u16, u16),
) {
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
    for entity in sim.substrate.entities.values() {
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
    let entity = sim.substrate.entities.get(ref_sid)?;
    if entity.dying || entity.health.current == 0 {
        return None;
    }
    let obj = sim.object_type(entity.type_ref, rules);
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
    let entity = sim.substrate.entities.get(ref_sid)?;
    if entity.dying || entity.health.current == 0 {
        return None;
    }
    sim.object_type(entity.type_ref, rules)
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
    let entity = sim.substrate.entities.get(ref_sid)?;
    let obj = sim.object_type(entity.type_ref, rules);
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
            u64::from(sim.session.binary_frame),
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

fn native_tiberium_context<'a>(
    sim: &'a Simulation,
    rules: &'a RuleSet,
    overlay_registry: Option<&'a crate::map::overlay_types::OverlayTypeRegistry>,
) -> Option<(
    &'a crate::sim::overlay_grid::OverlayGrid,
    &'a crate::map::overlay_types::OverlayTypeRegistry,
    &'a crate::rules::tiberium_type::TiberiumTypeRegistry,
)> {
    let grid = sim.overlay_grid.as_ref()?;
    let registry = overlay_registry?;
    (!rules.tiberium_types.is_empty()).then_some((grid, registry, &rules.tiberium_types))
}

pub(crate) fn resource_cell_present(
    sim: &Simulation,
    rules: &RuleSet,
    overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
    cell: (u16, u16),
) -> bool {
    if let Some((grid, registry, types)) = native_tiberium_context(sim, rules, overlay_registry) {
        return crate::sim::tiberium::tiberium_cell_view(grid, registry, types, cell).is_some();
    }
    sim.production
        .resource_nodes
        .get(&cell)
        .is_some_and(|node| node.remaining > 0)
}

pub(crate) fn search_local_resource(
    sim: &Simulation,
    rules: &RuleSet,
    overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
    center: (u16, u16),
    radius: u16,
    filter: Option<&dyn Fn((u16, u16)) -> bool>,
    config: &MinerConfig,
) -> Option<(u16, u16)> {
    if let Some((grid, registry, types)) = native_tiberium_context(sim, rules, overlay_registry) {
        return search_local_tiberium(grid, registry, types, center, radius, filter);
    }
    search_local_ore(
        &sim.production.resource_nodes,
        center,
        radius,
        filter,
        config.ore_bale_value,
        config.gem_bale_value,
    )
}

fn pick_best_resource_cell(
    sim: &Simulation,
    rules: &RuleSet,
    overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
    from: (u16, u16),
    filter: Option<&dyn Fn((u16, u16)) -> bool>,
) -> Option<(u16, u16)> {
    let Some((grid, registry, types)) = native_tiberium_context(sim, rules, overlay_registry)
    else {
        return pick_best_resource_node(&sim.production.resource_nodes, from, filter);
    };
    let mut best: Option<(i32, u32, u16, u16)> = None;
    for (rx, ry, _) in grid.iter_occupied() {
        let cell = (rx, ry);
        if filter.is_some_and(|candidate_filter| !candidate_filter(cell)) {
            continue;
        }
        let Some(view) = crate::sim::tiberium::tiberium_cell_view(grid, registry, types, cell)
        else {
            continue;
        };
        let dx = i64::from(rx) - i64::from(from.0);
        let dy = i64::from(ry) - i64::from(from.1);
        let distance = (dx * dx + dy * dy) as u32;
        let candidate = (view.nominal_value, distance, ry, rx);
        let replace = match best {
            None => true,
            Some(current) => {
                candidate.0 > current.0
                    || (candidate.0 == current.0 && candidate.1 < current.1)
                    || (candidate.0 == current.0
                        && candidate.1 == current.1
                        && (candidate.2, candidate.3) < (current.2, current.3))
            }
        };
        if replace {
            best = Some(candidate);
        }
    }
    best.map(|(_, _, ry, rx)| (rx, ry))
}

fn search_local_tiberium(
    grid: &crate::sim::overlay_grid::OverlayGrid,
    registry: &crate::map::overlay_types::OverlayTypeRegistry,
    types: &crate::rules::tiberium_type::TiberiumTypeRegistry,
    center: (u16, u16),
    radius: u16,
    filter: Option<&dyn Fn((u16, u16)) -> bool>,
) -> Option<(u16, u16)> {
    if crate::sim::tiberium::tiberium_cell_view(grid, registry, types, center).is_some() {
        return Some(center);
    }
    let cx = i32::from(center.0);
    let cy = i32::from(center.1);
    for ring in 1..i32::from(radius) {
        let mut best_in_ring: Option<(i32, (u16, u16))> = None;
        for col in -ring..=ring {
            for (nx, ny) in [
                (cx + col, cy - ring),
                (cx + col, cy + ring),
                (cx - ring, cy + col),
                (cx + ring, cy + col),
            ] {
                if nx < 0 || ny < 0 || nx > i32::from(u16::MAX) || ny > i32::from(u16::MAX) {
                    continue;
                }
                let cell = (nx as u16, ny as u16);
                if filter.is_some_and(|candidate_filter| !candidate_filter(cell)) {
                    continue;
                }
                let Some(view) =
                    crate::sim::tiberium::tiberium_cell_view(grid, registry, types, cell)
                else {
                    continue;
                };
                if best_in_ring.is_none_or(|(value, _)| view.nominal_value > value) {
                    best_in_ring = Some((view.nominal_value, cell));
                }
            }
        }
        if let Some((_, cell)) = best_in_ring {
            return Some(cell);
        }
    }
    None
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

/// Hand a selected stock-miner destination to the normal Drive command authority.
pub(crate) fn issue_stock_miner_drive_move(
    sim: &mut Simulation,
    rules: &RuleSet,
    grid: &PathGrid,
    entity_id: u64,
    target: (u16, u16),
) -> bool {
    if target.0 >= grid.width() || target.1 >= grid.height() {
        return false;
    }
    let Some(info) = sim.resolve_move_info(entity_id, Some(rules)) else {
        return false;
    };

    let activation_snapshot = if info.is_teleporter && info.is_harvester {
        sim.substrate
            .entities
            .get_mut(entity_id)
            .and_then(|entity| entity.locomotor.as_mut())
            .map(|locomotor| {
                let snapshot = (
                    locomotor.kind,
                    locomotor.slot,
                    locomotor.piggyback,
                    locomotor.layer,
                    locomotor.phase,
                );
                let _ = locomotor.begin_drive_piggyback_for_teleporter();
                snapshot
            })
    } else {
        None
    };

    let terrain_costs = sim.terrain_costs.get(&info.speed_type);
    let blocker_neighbor_counts = movement::bump_crush::build_blocker_neighbor_counts(
        &sim.substrate.entities,
        grid.width(),
        grid.height(),
        sim.resolved_terrain.as_ref(),
        &sim.interner,
        Some(rules),
    );
    let issued = movement::issue_move_command_with_layered(
        &mut sim.substrate.entities,
        grid,
        entity_id,
        target,
        info.speed,
        false,
        terrain_costs,
        None,
        sim.resolved_terrain.as_ref(),
        sim.zone_grid.as_ref(),
        None,
        info.mover_is_crusher,
        Some(&blocker_neighbor_counts),
        Some(&mut sim.substrate.cell_occupation),
    );
    if !issued {
        if let Some((kind, slot, piggyback, layer, phase)) = activation_snapshot
            && let Some(locomotor) = sim
                .substrate
                .entities
                .get_mut(entity_id)
                .and_then(|entity| entity.locomotor.as_mut())
        {
            locomotor.kind = kind;
            locomotor.slot = slot;
            locomotor.piggyback = piggyback;
            locomotor.layer = layer;
            locomotor.phase = phase;
        }
        return false;
    }

    if let Some(movement) = sim
        .substrate
        .entities
        .get_mut(entity_id)
        .and_then(|entity| entity.movement_target.as_mut())
    {
        movement.accel_factor = info.accel_factor;
        movement.decel_factor = info.decel_factor;
        movement.slowdown_distance = info.slowdown_distance;
    }
    true
}

/// Issue a move command only if the entity isn't already pathing to this target.
pub(crate) fn issue_move_if_idle(
    sim: &mut Simulation,
    rules: Option<&RuleSet>,
    grid: &PathGrid,
    entity_id: u64,
    target: (u16, u16),
    speed: SimFixed,
) {
    if target.0 >= grid.width() || target.1 >= grid.height() {
        return;
    }
    let already = sim
        .substrate
        .entities
        .get(entity_id)
        .and_then(|e| e.movement_target.as_ref())
        .and_then(|mt| mt.path.last().copied())
        .is_some_and(|goal| goal == target);
    if !already {
        let blocker_neighbor_counts = movement::bump_crush::build_blocker_neighbor_counts(
            &sim.substrate.entities,
            grid.width(),
            grid.height(),
            sim.resolved_terrain.as_ref(),
            &sim.interner,
            rules,
        );
        let _ = movement::issue_move_command_with_layered(
            &mut sim.substrate.entities,
            grid,
            entity_id,
            target,
            speed,
            false,
            None,
            None,
            sim.resolved_terrain.as_ref(),
            sim.zone_grid.as_ref(),
            None,
            false,
            Some(&blocker_neighbor_counts),
            Some(&mut sim.substrate.cell_occupation),
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
    sim.substrate
        .entities
        .values()
        .filter(|e| {
            // A Dying purifier corpse (sold/destroyed this tick) must not keep
            // paying its deposit bonus until the end-of-tick drain.
            !e.dying
                && e.category == EntityCategory::Structure
                && sim.interner.resolve(e.owner).eq_ignore_ascii_case(owner)
                && sim
                    .object_type(e.type_ref, rules)
                    .is_some_and(|obj| obj.ore_purifier)
        })
        .count() as i32
}

/// Effective purifier count used in the deposit bonus formula.
///
/// Returns `real_purifiers + AI_virtual_purifiers`, where the AI term is
/// `general.ai_virtual_purifiers[refinery_owner.difficulty]` for non-human
/// houses. Both terms are sourced from the refinery's owner — credit
/// destination is a separate concern.
///
/// Native also gates the virtual term on raw `g_GameMode != 0`. The simulation
/// does not yet carry that verified raw mode authority, so adding that gate is
/// an explicit parity follow-up; this function must not infer it from unrelated
/// session fields.
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
    let Some(house) =
        crate::sim::house_state::house_state_for_owner(&sim.houses, refinery_owner, &sim.interner)
    else {
        return real;
    };
    if house.is_human {
        return real;
    }
    let table = rules.general.ai_virtual_purifiers;
    let virtual_count = table[house.difficulty.table_index()];
    real + virtual_count
}
