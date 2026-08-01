//! Unit movement system — moves entities along A* paths each tick.
//!
//! The movement system reads MovementTarget fields and advances entities
//! toward their destination using lepton-based sub-cell movement.
//! Each tick, `sub_x`/`sub_y` advance along the direction vector at
//! `speed` leptons per second. Cell transitions occur when sub_x/sub_y
//! cross the cell boundary (0 or 256 leptons).
//!
//! ## Coordinate update
//! Every tick, screen position is recomputed from lepton coordinates via
//! `lepton_to_screen()`, giving smooth sub-cell movement without render
//! interpolation.
//!
//! ## Facing
//! RA2 uses a 0-255 screen-relative DirStruct byte: 0=north on screen (iso -x,-y),
//! 64=east on screen (iso +x,-y), 128=south on screen (iso +x,+y),
//! 192=west on screen (iso -x,+y). Facing is updated whenever the entity starts
//! moving toward a new cell.
//!
//! ## Sub-modules
//! - `movement_commands` — A* pathfinding and MovementTarget attachment
//! - `movement_tick` — per-tick ground movement state machine (the main loop)
//!
//! ## Dependency rules
//! - Part of sim/ — depends on sim/entity_store, sim/game_entity, sim/pathfinding.
//! - Uses map/terrain::iso_to_screen for coordinate conversion.
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

use std::collections::BTreeMap;

use crate::map::entities::EntityCategory;
use crate::map::houses::HouseAllianceMap;
use crate::map::resolved_terrain::ResolvedTerrainGrid;
use crate::rules::locomotor_type::{MovementZone, SpeedType};
use crate::sim::entity_store::EntityStore;
use crate::sim::intern::InternedId;
use crate::sim::lifecycle_request::LifecycleRequest;
use crate::sim::pathfinding::PathGrid;
use crate::sim::pathfinding::terrain_cost::TerrainCostGrid;
use crate::sim::pathfinding::terrain_speed::TerrainSpeedConfig;
use crate::sim::pathfinding::zone_map::ZoneGrid;
use crate::sim::rng::SimRng;
use crate::util::fixed_math::{SIM_ONE, SIM_ZERO, SimFixed, facing_from_delta_int};

// --- Internal submodules ---
mod drive_locomotion;
pub(crate) mod locomotor_ready;
mod movement_blocked;
mod movement_bridge;
mod movement_commands;
mod movement_occupancy;
mod movement_path;
mod movement_reservation;
mod movement_step;
mod movement_tick;
mod navcom;
mod path_markers;
pub(crate) mod ready_producer;

// --- Movement-related modules (public API) ---
pub mod air_movement;
pub mod bump_crush;
pub mod drive_track;
pub mod facing_class;
pub mod group_destination;
pub mod homing_movement;
pub mod hover;
pub mod jumpjet_movement;
pub mod locomotion;
pub mod locomotor;
pub mod parachute_descent;
pub mod rocket_movement;
pub mod scatter;
pub mod teleport_movement;
pub mod tube_movement;
pub mod turret;

pub use facing_class::FacingClass;

// The drive-locomotor "Process" presence marker, consumed read-only by the
// per-object AI shell (sim/world/techno_ai.rs, Slice S1) to observe that the
// locomotor would process AFTER mission dispatch. Behavior-neutral re-export.
// Gated to match its only consumer (the debug/test S1 shadow) so release builds
// carry no unused re-export.
#[cfg(any(test, debug_assertions))]
pub(crate) use drive_locomotion::{DriveProcessOutcome, process_drive_locomotion_shell};

// Re-export command functions so callers can use `movement::issue_move_command` etc.
pub use movement_commands::{
    clear_navigation_for_entity, issue_direct_move, issue_move_command,
    issue_move_command_with_layered, set_destination_for_teleporter_entity,
};
#[cfg(test)]
pub(crate) use movement_path::{
    path_search_used_zone_grid_marker, reset_path_search_used_zone_grid_marker,
};
// Re-export the tick function so callers can use `movement::tick_movement_with_grids`.
pub(crate) use movement_tick::{
    sync_formation_speeds_after_live_pass, tick_movement_object_with_grids,
    tick_movement_with_grids,
};

/// Install the active-YR `DriveLocomotion::Force_Track` state for a flat-ground
/// unit. The caller supplies head offsets from the unit's current cell origin;
/// the stored head itself is an exact absolute lepton coordinate.
pub(crate) fn install_forced_drive_track(
    entity: &mut crate::sim::game_entity::GameEntity,
    cell_occupation: &mut crate::sim::occupancy::CellOccupationGrid,
    mut forced: drive_track::ForcedDriveTrackState,
) -> bool {
    if entity.occupancy_list_layer() != Some(locomotor::MovementLayer::Ground) {
        return false;
    }

    let absolute_x = i32::from(entity.position.rx) * 256 + forced.track.head_offset_x;
    let absolute_y = i32::from(entity.position.ry) * 256 + forced.track.head_offset_y;
    let target_rx = absolute_x.div_euclid(256);
    let target_ry = absolute_y.div_euclid(256);
    let (Ok(target_rx), Ok(target_ry)) = (u16::try_from(target_rx), u16::try_from(target_ry))
    else {
        return false;
    };

    let head = crate::sim::components::DriveCoord {
        x: absolute_x,
        y: absolute_y,
        z: i32::from(entity.position.z),
    };
    let footprint = crate::sim::components::DriveOccupationFootprint {
        rx: target_rx,
        ry: target_ry,
        layer: locomotor::MovementLayer::Ground,
    };

    let drive = entity
        .drive_locomotion
        .get_or_insert_with(crate::sim::components::DriveLocomotionRuntime::default);
    // Force_Track preserves DriveLocomotion's integer movement residual. The
    // detached forced cursor mirrors that canonical owner field for snapshots.
    forced.track.residual = drive.residual_budget;
    drive.destination = Some(head);
    drive.head_to = Some(head);
    drive.track_index = i16::from(forced.turn_track_index);
    drive.point_index = forced.track.point_index;
    drive.track_valid = true;
    drive.target_speed_fraction = SIM_ONE;
    drive.current_speed_fraction = SIM_ONE;
    // Force_Track directly installs the new head mark. Its active retail
    // callers enter with no old head, so ordinary replacement/old-mark clear
    // semantics do not apply here.
    cell_occupation.mark_vehicle_on_layer(
        footprint.rx,
        footprint.ry,
        entity.stable_id,
        footprint.layer,
    );
    drive.occupation_head_to = Some(footprint);

    entity.drive_track = None;
    entity.forced_drive_track = Some(forced);
    entity.facing_target = None;
    true
}

// ---------------------------------------------------------------------------
// Constants — shared across movement submodules via `super::`
// ---------------------------------------------------------------------------

/// Initial path retry counter before giving up (original engine: FootClass+0x64C, init=10).
/// Decremented on each failed Find_Path. At 0 the unit abandons the move order.
const PATH_STUCK_INIT: u8 = 10;
/// Minimum height level difference to trigger Rust's defensive cliff detection.
/// Original engine: abs(current_z / HeightStep - cell.height) >= 3 levels.
const CLIFF_HEIGHT_THRESHOLD: u16 = 3;
/// Infantry wobble phase increment per second (radians/sec).
/// One full cycle (2π) per ~2.5 seconds ≈ 2.5 rad/s. Matches slow
/// infantry walk cadence in the original game.
const INFANTRY_WOBBLE_RATE: f32 = 2.5;
/// Minimum speed as a fraction of max speed during normal braking.
/// Original engine: 0.3 (30% of max speed).
const MIN_BRAKE_FRACTION: SimFixed = SimFixed::lit("0.3");

// ---------------------------------------------------------------------------
// Types — shared across movement submodules
// ---------------------------------------------------------------------------

/// Read-only grid/terrain environment for pathfinding and movement decisions.
#[derive(Clone, Copy)]
pub(super) struct PathfindingContext<'a> {
    pub path_grid: Option<&'a PathGrid>,
    pub zone_grid: Option<&'a ZoneGrid>,
    pub resolved_terrain: Option<&'a ResolvedTerrainGrid>,
    pub blocker_neighbor_counts: Option<&'a crate::sim::pathfinding::BlockerNeighborCounts>,
}

/// Movement timing/threshold config derived from rules.ini [General] section.
/// Separate from `PathfindingContext` because `find_move_path` doesn't need these.
#[derive(Clone, Copy)]
pub(super) struct MovementConfig {
    pub close_enough: SimFixed,
    pub path_delay_ticks: u16,
    pub blockage_path_delay_ticks: u16,
}

/// Snapshot of mover properties taken before the inner movement loop.
/// Avoids repeated `entities.get()` calls and survives across the mutable/immutable
/// borrow boundary (lines ~211–920 hold `&mut GameEntity`, lines ~920–1230 release
/// the borrow for `&EntityStore` lookups).
pub(super) struct MoverSnapshot {
    pub category: EntityCategory,
    pub speed_type: Option<SpeedType>,
    pub movement_zone: MovementZone,
    pub omni_crusher: bool,
    pub regular_crusher: bool,
    pub drive_accelerates: bool,
    pub owner: InternedId,
    pub too_big_to_fit_under_bridge: bool,
    pub on_bridge: bool,
    pub locomotor: Option<locomotor::LocomotorState>,
    pub rot: i32,
    /// Mover's `MovementTarget.bypass_grid` flag — when true, structure
    /// occupants are skipped during the foundation-cross occupancy check
    /// (harvester dock drive: buildings are not scatter targets).
    pub bypass_grid: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct PendingCrushKill {
    pub victim_id: u64,
    pub crusher_id: u64,
    pub crush_coord: (i32, i32),
}

/// Per-tick movement diagnostics — returned by `tick_movement_with_grids`.
#[derive(Debug, Default, Clone, Copy)]
pub struct MovementTickStats {
    pub movers_total: u32,
    pub moved_steps: u32,
    pub blocked_attempts: u32,
    pub repath_attempts: u32,
    pub repath_successes: u32,
    pub scatter_successes: u32,
    pub crush_kills: u32,
    pub stuck_aborts: u32,
    /// Scatter attempts triggered when infantry are blocked.
    pub scatter_attempts: u32,
    /// Track selections triggered for vehicle turns.
    pub track_selections: u32,
    /// Stuck entities that recovered via repath or scatter.
    pub stuck_recoveries: u32,
    /// Elapsed microseconds for the entire tick.
    pub elapsed_us: u64,
}

impl MovementTickStats {
    pub(crate) fn merge(&mut self, other: Self) {
        self.movers_total = self.movers_total.saturating_add(other.movers_total);
        self.moved_steps = self.moved_steps.saturating_add(other.moved_steps);
        self.blocked_attempts = self.blocked_attempts.saturating_add(other.blocked_attempts);
        self.repath_attempts = self.repath_attempts.saturating_add(other.repath_attempts);
        self.repath_successes = self.repath_successes.saturating_add(other.repath_successes);
        self.scatter_successes = self
            .scatter_successes
            .saturating_add(other.scatter_successes);
        self.crush_kills = self.crush_kills.saturating_add(other.crush_kills);
        self.stuck_aborts = self.stuck_aborts.saturating_add(other.stuck_aborts);
        self.scatter_attempts = self.scatter_attempts.saturating_add(other.scatter_attempts);
        self.track_selections = self.track_selections.saturating_add(other.track_selections);
        self.stuck_recoveries = self.stuck_recoveries.saturating_add(other.stuck_recoveries);
        self.elapsed_us = self.elapsed_us.saturating_add(other.elapsed_us);
    }
}

/// Command to move an entity to a target cell (queued for next tick).
#[derive(Debug, Clone)]
pub struct MoveCommand {
    pub entity_id: u64,
    pub target_rx: u16,
    pub target_ry: u16,
    pub queue: bool,
}

// ---------------------------------------------------------------------------
// Public utilities
// ---------------------------------------------------------------------------

/// Compute the active-retail screen-relative facing byte from a coordinate delta.
///
/// Computed directions use the high byte of the native 65,534-scale word;
/// authored quarter-turn values remain distinct.
pub fn facing_from_delta(dx: i32, dy: i32) -> u8 {
    facing_from_delta_int(dx, dy)
}

/// Restore active piggyback locomotors whose owner is no longer moving,
/// teleporting, or deploying.
///
/// This bridge mirrors FootClass::AI's per-tick "ok to end piggyback" check
/// without changing existing movement ownership for non-migrated special flows.
pub fn tick_locomotor_piggyback_restore(entities: &mut EntityStore) -> usize {
    let mut restored = 0usize;
    let keys = entities.keys_sorted();
    for id in keys {
        restored = restored.saturating_add(usize::from(tick_locomotor_piggyback_restore_one(
            entities, id,
        )));
    }
    restored
}

pub(crate) fn tick_locomotor_piggyback_restore_one(entities: &mut EntityStore, id: u64) -> bool {
    let Some(entity) = entities.get_mut(id) else {
        return false;
    };
    let owner_moving = entity.movement_target.is_some() || entity.forced_drive_track.is_some();
    let owner_teleporting = entity.teleport_state.is_some();
    let owner_deploying = entity.building_up.is_some()
        || entity.building_down.is_some()
        || entity.deploy_state.is_some();
    let mut retired_drive = false;
    let restored_now = if let Some(ref mut loco) = entity.locomotor {
        retired_drive = loco.active_kind() == crate::rules::locomotor_type::LocomotorKind::Drive;
        loco.can_restore_primary_from_piggyback(owner_moving, owner_teleporting, owner_deploying)
            && loco.restore_primary_from_piggyback()
    } else {
        false
    };
    if restored_now && retired_drive {
        // Native FootClass::AI releases the old active locomotor before
        // installing the stored primary. Do not retain hashed Drive
        // state after primary Teleport is active again.
        entity.drive_locomotion = None;
        entity.drive_track = None;
    }
    restored_now
}

// ---------------------------------------------------------------------------
// Tick entry points (thin wrappers)
// ---------------------------------------------------------------------------

/// Advance all entities with MovementTarget along their paths.
///
/// Called once per admitted native gameplay frame.
/// Entities that reach their destination have MovementTarget removed automatically.
pub(crate) fn tick_movement(
    entities: &mut EntityStore,
    interner: &mut crate::sim::intern::StringInterner,
    lifecycle_requests: &mut Vec<LifecycleRequest>,
) {
    let empty_costs: BTreeMap<SpeedType, TerrainCostGrid> = BTreeMap::new();
    let empty_alliances: HouseAllianceMap = HouseAllianceMap::new();
    let mut rng: SimRng = SimRng::new(0);
    let mut empty_occupancy = crate::sim::occupancy::OccupancyGrid::new();
    let _ = tick_movement_with_grid(
        entities,
        None,
        &empty_costs,
        &empty_alliances,
        &mut empty_occupancy,
        &mut rng,
        0, // sim_tick not available in test-only wrapper
        interner,
        lifecycle_requests,
    );
}

/// Advance movement and perform deterministic blocked-cell recovery.
///
/// `terrain_costs` is the per-SpeedType cost map for cost-aware repath.
/// When provided, repath attempts use `find_path_with_costs` to prefer
/// roads and avoid rough terrain.
pub(crate) fn tick_movement_with_grid(
    entities: &mut EntityStore,
    path_grid: Option<&PathGrid>,
    terrain_costs: &BTreeMap<SpeedType, TerrainCostGrid>,
    alliances: &HouseAllianceMap,
    occupancy: &mut crate::sim::occupancy::OccupancyGrid,
    rng: &mut SimRng,
    sim_tick: u64,
    interner: &mut crate::sim::intern::StringInterner,
    lifecycle_requests: &mut Vec<LifecycleRequest>,
) -> MovementTickStats {
    let mut sound_events: Vec<crate::sim::world::SimSoundEvent> = Vec::new();
    let mut next_occupancy_enter_order = crate::sim::world::EnterOrderCounter::new();
    let mut cell_occupation = crate::sim::occupancy::CellOccupationGrid::rebuild(entities);
    tick_movement_with_grids(
        entities,
        None,
        path_grid,
        terrain_costs,
        alliances,
        occupancy,
        &mut cell_occupation,
        &mut next_occupancy_enter_order,
        rng,
        sim_tick,
        sim_tick as u32, // native-frame proxy (test-only wrapper: 1 frame/tick)
        None,            // No zone grid in legacy wrapper
        None,            // No resolved terrain in legacy wrapper
        &TerrainSpeedConfig::default(),
        SIM_ZERO, // No CloseEnough in legacy wrapper
        9,        // Default PathDelay
        60,       // Default BlockagePathDelay
        interner,
        None, // No RuleSet in legacy wrapper — crush sounds suppressed
        &mut sound_events,
        lifecycle_requests,
    )
}

// ---------------------------------------------------------------------------
// Internal helpers — shared across movement submodules
// ---------------------------------------------------------------------------

/// Returns true if the entity has a within-cell destination it hasn't reached yet.
/// Used for both infantry (sub-cell corners) and vehicles (cell center).
/// The locomotor's `subcell_dest` field stores the target lepton coordinates.
///
/// Takes individual fields to avoid borrow conflicts with `entity.movement_target`.
fn walking_to_subcell_dest(
    locomotor: &Option<crate::sim::movement::locomotor::LocomotorState>,
    sub_x: SimFixed,
    sub_y: SimFixed,
) -> bool {
    let Some(loco) = locomotor else {
        return false;
    };
    let Some((dest_x, dest_y)) = loco.subcell_dest else {
        return false;
    };
    let threshold: SimFixed = SimFixed::from_num(4);
    (dest_x - sub_x).abs() > threshold || (dest_y - sub_y).abs() > threshold
}

#[cfg(test)]
mod movement_tests;
#[cfg(test)]
mod prone_speed_tests;
