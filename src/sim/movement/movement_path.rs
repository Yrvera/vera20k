//! Movement path management — path computation, repath-after-block, and bridge pathing support.
//!
//! Wraps the A* pathfinder for use by the movement tick: computes initial paths,
//! retries after blockages with zone-aware corridor search, and determines whether
//! an entity's locomotor supports layered bridge pathing.

use std::collections::BTreeSet;

use crate::map::resolved_terrain::ResolvedTerrainGrid;
use crate::rules::locomotor_type::{LocomotorKind, MovementZone, SpeedType};
use crate::sim::components::MovementTarget;
use crate::sim::find_nearby_cell::{
    NearbyQuery, PassabilityArgs, RADIUS_HARD_CAP, find_nearby_passable_cell,
};
use crate::sim::movement::locomotor::{LocomotorState, MovementLayer};
use crate::sim::pathfinding::LayeredEntityBlockMap;
use crate::sim::pathfinding::path_smooth;
use crate::sim::pathfinding::terrain_cost::TerrainCostGrid;
use crate::sim::pathfinding::zone_map::{ZONE_INVALID, ZoneGrid};
use crate::sim::pathfinding::zone_search;
use crate::sim::pathfinding::{
    MAX_PATH_SEGMENT_STEPS, PathGrid, SearchMarkerOverlay, truncate_layered_path,
};
use crate::sim::rng::SimRng;
use crate::util::fixed_math::facing_from_delta_int as facing_from_delta;

use super::{MovementConfig, PathfindingContext};

#[cfg(test)]
pub(crate) fn reset_path_search_used_zone_grid_marker() {
    PATH_SEARCH_USED_ZONE_GRID.with(|used| used.set(false));
}

#[cfg(test)]
pub(crate) fn path_search_used_zone_grid_marker() -> bool {
    PATH_SEARCH_USED_ZONE_GRID.with(|used| used.get())
}

#[cfg(test)]
thread_local! {
    static PATH_SEARCH_USED_ZONE_GRID: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

pub(super) fn merge_path_blocks(
    entity_blocks: Option<&BTreeSet<(u16, u16)>>,
    _resolved_terrain: Option<&ResolvedTerrainGrid>,
    _movement_zone: Option<MovementZone>,
    _too_big_to_fit_under_bridge: bool,
) -> BTreeSet<(u16, u16)> {
    // gamemd does not gate movement on TooBigToFitUnderBridge — the flag at
    // TechnoTypeClass+0xE16 is read only in the draw pipeline (sprite Z fudge
    // for units on bridge edge cells). UnitClass::Can_Enter_Cell never touches
    // it. See TOO_BIG_TO_FIT_UNDER_BRIDGE_GHIDRA_REPORT.md.
    entity_blocks.cloned().unwrap_or_default()
}

pub(super) fn supports_layered_bridge_pathing(
    loco: &LocomotorState,
    grid: &PathGrid,
    on_bridge: bool,
) -> bool {
    if grid.width() == 0 || grid.height() == 0 {
        return false;
    }
    matches!(
        loco.kind,
        LocomotorKind::Drive | LocomotorKind::Walk | LocomotorKind::Mech
    ) || on_bridge
}

fn is_bridge_layer_walkable(grid: Option<&PathGrid>, cell: (u16, u16)) -> bool {
    grid.is_some_and(|g| g.is_walkable_on_layer(cell.0, cell.1, MovementLayer::Bridge))
}

fn is_bridge_only_goal(grid: &PathGrid, goal: (u16, u16)) -> bool {
    !grid.is_walkable(goal.0, goal.1) && is_bridge_layer_walkable(Some(grid), goal)
}

pub(super) fn is_move_goal_walkable(
    grid: &PathGrid,
    goal: (u16, u16),
    movement_zone: Option<MovementZone>,
    resolved_terrain: Option<&ResolvedTerrainGrid>,
) -> bool {
    if movement_zone.is_some_and(|mz| mz.is_water_mover()) {
        return crate::sim::pathfinding::is_cell_passable_for_mover(
            grid,
            goal.0,
            goal.1,
            movement_zone,
            resolved_terrain,
        );
    }
    grid.is_any_layer_walkable(goal.0, goal.1)
}

fn nearest_move_goal(
    grid: &PathGrid,
    goal: (u16, u16),
    max_radius: u16,
    blocked_cells: Option<&BTreeSet<(u16, u16)>>,
    movement_zone: Option<MovementZone>,
    resolved_terrain: Option<&ResolvedTerrainGrid>,
) -> Option<(u16, u16)> {
    if !movement_zone.is_some_and(|mz| mz.is_water_mover()) {
        return grid.nearest_walkable_any_layer(goal.0, goal.1, max_radius, blocked_cells, None);
    }

    let check = |x: u16, y: u16| {
        is_move_goal_walkable(grid, (x, y), movement_zone, resolved_terrain)
            && blocked_cells.map_or(true, |blocks| !blocks.contains(&(x, y)))
    };
    if check(goal.0, goal.1) {
        return Some(goal);
    }
    for radius in 1..=max_radius {
        let r = radius as i32;
        for d in -r..=r {
            let candidates = [
                (goal.0 as i32 + d, goal.1 as i32 - r),
                (goal.0 as i32 + d, goal.1 as i32 + r),
                (goal.0 as i32 - r, goal.1 as i32 + d),
                (goal.0 as i32 + r, goal.1 as i32 + d),
            ];
            for (x, y) in candidates {
                if x < 0 || y < 0 || x >= grid.width() as i32 || y >= grid.height() as i32 {
                    continue;
                }
                let candidate = (x as u16, y as u16);
                if check(candidate.0, candidate.1) {
                    return Some(candidate);
                }
            }
        }
    }
    None
}

pub(super) fn resolve_requested_move_goal(
    grid: &PathGrid,
    goal: (u16, u16),
    blocked_cells: Option<&BTreeSet<(u16, u16)>>,
    movement_zone: Option<MovementZone>,
    resolved_terrain: Option<&ResolvedTerrainGrid>,
    max_radius: u16,
) -> Option<(u16, u16)> {
    if is_move_goal_walkable(grid, goal, movement_zone, resolved_terrain)
        && blocked_cells.map_or(true, |blocks| !blocks.contains(&goal))
    {
        return Some(goal);
    }

    nearest_move_goal(
        grid,
        goal,
        max_radius,
        blocked_cells,
        movement_zone,
        resolved_terrain,
    )
}

/// Which zone layer a destination cell is resolved on.
///
/// Gamemd passes the destination cell's own bridge bit into `Can_Reach_Zone` and
/// into the nearby-passable-cell search from the ground-click destination
/// resolver, so a click on a bridge deck is answered on the bridge layer.
fn goal_zone_layer(
    resolved_terrain: Option<&ResolvedTerrainGrid>,
    goal: (u16, u16),
) -> MovementLayer {
    let on_bridge = resolved_terrain
        .and_then(|terrain| terrain.cell(goal.0, goal.1))
        .is_some_and(|cell| cell.bridge_facts.has_structural_bridge());
    if on_bridge {
        MovementLayer::Bridge
    } else {
        MovementLayer::Ground
    }
}

/// Gamemd's order-time reachability gate and zone-constrained destination
/// substitution.
///
/// The ground-click destination resolver runs `Can_Reach_Zone(myCell, clicked,
/// mzRow, myLayer, clickedBridgeBit, 0)` before the mission is installed:
///
/// * success — the clicked cell is used verbatim;
/// * failure — **the order is still accepted**. The resolver takes the mover's
///   own zone id and runs the nearby-passable-cell search seeded at the clicked
///   cell, requiring that zone, so the unit drives to the near bank of the river
///   / the near side of the cliff instead of standing still. Only a search that
///   finds no candidate at all drops the order (the engine's null-cell branch).
///
/// Returns `Some(goal)` unchanged when there is no zone data — that is the
/// engine's `mzRow == -1` short-circuit, which returns "reachable".
///
/// Selection mode: the search is given the clicked cell as its distance
/// reference, so the substitute is the nearest surviving candidate to the click.
/// The engine's alternative frame-counter selection is reached only when the
/// reference coord is the null cell; which of the two this call site takes is
/// UNCHECKED, and nearest-to-click is the deterministic choice that needs no
/// frame input threaded through every move-order caller.
pub(super) fn resolve_reachable_move_goal(
    grid: &PathGrid,
    zone_grid: Option<&ZoneGrid>,
    resolved_terrain: Option<&ResolvedTerrainGrid>,
    start: (u16, u16),
    start_layer: MovementLayer,
    goal: (u16, u16),
    zone_mz: MovementZone,
    speed_type: SpeedType,
) -> Option<(u16, u16)> {
    let Some(zone_grid) = zone_grid else {
        return Some(goal);
    };
    let goal_layer = goal_zone_layer(resolved_terrain, goal);
    if start == goal || zone_grid.can_reach(zone_mz, start, start_layer, goal, goal_layer) {
        return Some(goal);
    }

    let Some(zone_map) = zone_grid.map_for(zone_mz) else {
        return Some(goal);
    };
    let required_zone = zone_map.zone_at(start.0, start.1, start_layer);
    if required_zone == ZONE_INVALID {
        // The mover itself has no zone — a unit standing on a footprint cell, a
        // factory exit or an unmapped bridge cell. There is no zone to require,
        // so the order passes through unchanged rather than being refused.
        // VERA-internal; the engine labels such a node rather than leaving it
        // undefined, so the gamemd equivalent here is UNCHECKED.
        return Some(goal);
    }

    // Engine radius cap: min(map width + map height, 32).
    let radius_cap = grid
        .width()
        .saturating_add(grid.height())
        .min(RADIUS_HARD_CAP);
    let query = NearbyQuery {
        passability: PassabilityArgs {
            speed_type,
            required_zone_id: Some(required_zone),
            movement_zone: zone_mz,
            bridge_aware_zone: goal_layer == MovementLayer::Bridge,
        },
        allow_bridge_cells: true,
        check_height: false,
        check_occupancy: false,
        radius_cap,
        target_cell: Some((goal.0 as i32, goal.1 as i32)),
        path_grid: Some(grid),
        resolved_terrain,
        overlay_grid: None,
        occupancy: None,
        entities: None,
        zone_grid: Some(zone_grid),
        map_size: Some((grid.width(), grid.height())),
        playfield_bounds: None,
    };
    // `target_cell` is set, so the frame counter is not consulted for selection.
    find_nearby_passable_cell((goal.0 as i32, goal.1 as i32), &query, 0)
}

pub(super) fn find_move_path(
    ctx: PathfindingContext<'_>,
    layered_pathing: bool,
    start: (u16, u16),
    start_layer: MovementLayer,
    goal: (u16, u16),
    terrain_costs: Option<&TerrainCostGrid>,
    entity_blocks: Option<&BTreeSet<(u16, u16)>>,
    ground_blocks: Option<&BTreeSet<(u16, u16)>>,
    bridge_blocks: Option<&BTreeSet<(u16, u16)>>,
    zone_mz: MovementZone,
    movement_zone: Option<MovementZone>,
    too_big_to_fit_under_bridge: bool,
    entity_block_map: Option<&LayeredEntityBlockMap>,
    urgency: u8,
    mover_is_crusher: bool,
    is_infantry: bool,
) -> Option<(Vec<(u16, u16)>, Vec<MovementLayer>)> {
    find_move_path_with_marker(
        ctx,
        layered_pathing,
        start,
        start_layer,
        goal,
        terrain_costs,
        entity_blocks,
        ground_blocks,
        bridge_blocks,
        zone_mz,
        movement_zone,
        too_big_to_fit_under_bridge,
        entity_block_map,
        None,
        urgency,
        mover_is_crusher,
        is_infantry,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn find_move_path_with_marker(
    ctx: PathfindingContext<'_>,
    layered_pathing: bool,
    start: (u16, u16),
    start_layer: MovementLayer,
    goal: (u16, u16),
    terrain_costs: Option<&TerrainCostGrid>,
    entity_blocks: Option<&BTreeSet<(u16, u16)>>,
    ground_blocks: Option<&BTreeSet<(u16, u16)>>,
    bridge_blocks: Option<&BTreeSet<(u16, u16)>>,
    zone_mz: MovementZone,
    movement_zone: Option<MovementZone>,
    too_big_to_fit_under_bridge: bool,
    entity_block_map: Option<&LayeredEntityBlockMap>,
    marker_overlay: Option<&SearchMarkerOverlay>,
    urgency: u8,
    mover_is_crusher: bool,
    is_infantry: bool,
) -> Option<(Vec<(u16, u16)>, Vec<MovementLayer>)> {
    let grid = ctx.path_grid?;
    let zone_grid = ctx.zone_grid;
    #[cfg(test)]
    if zone_grid.is_some() {
        PATH_SEARCH_USED_ZONE_GRID.with(|used| used.set(true));
    }
    let resolved_terrain = ctx.resolved_terrain;
    let merged_entity_blocks = merge_path_blocks(
        entity_blocks,
        resolved_terrain,
        movement_zone,
        too_big_to_fit_under_bridge,
    );
    let entity_blocks = (!merged_entity_blocks.is_empty()).then_some(&merged_entity_blocks);
    if layered_pathing {
        let layered_result = zone_search::find_layered_path_zoned_marker(
            grid,
            ground_blocks,
            bridge_blocks,
            start,
            start_layer,
            goal,
            zone_grid,
            zone_mz,
            terrain_costs,
            movement_zone,
            resolved_terrain,
            entity_block_map,
            marker_overlay,
            ctx.blocker_neighbor_counts,
            urgency,
            mover_is_crusher,
            is_infantry,
        );
        let Some(path) = layered_result else {
            log::trace!(
                "find_move_path: layered A* failed ({:?} layer={:?} → {:?})",
                start,
                start_layer,
                goal,
            );
            return None;
        };
        log::trace!(
            "find_move_path: layered A* succeeded ({:?}→{:?}), {} steps",
            start,
            goal,
            path.len(),
        );
        let coords: Vec<(u16, u16)> = path.iter().map(|step| (step.rx, step.ry)).collect();
        let layers: Vec<MovementLayer> = path.iter().map(|step| step.layer).collect();
        if contains_non_adjacent_step(&coords) {
            let (coords, layers) = truncate_layered_path(coords, layers, MAX_PATH_SEGMENT_STEPS);
            return Some((coords, layers));
        }
        let layered_smooth_walkable = |x: u16, y: u16, layer: MovementLayer| -> bool {
            if !grid.is_walkable_on_layer(x, y, layer) {
                return false;
            }
            // Soft-blocked cells (code 2/5/6) must not be used as zigzag
            // shortcuts: A* deliberately routed around them, smoothing
            // through them would undo the detour.
            if entity_block_map.is_some_and(|m| m.contains_key(layer, &(x, y))) {
                return false;
            }
            if marker_overlay.is_some_and(|m| m.contains((x, y)) && (x, y) != goal) {
                return false;
            }
            match layer {
                MovementLayer::Ground => !ground_blocks.is_some_and(|gb| gb.contains(&(x, y))),
                MovementLayer::Bridge => !bridge_blocks.is_some_and(|bb| bb.contains(&(x, y))),
                _ => true,
            }
        };
        let (coords, layers) =
            path_smooth::smooth_layered_path(coords, layers, &layered_smooth_walkable);
        let (coords, layers) =
            path_smooth::optimize_layered_path(coords, layers, &layered_smooth_walkable);
        let (coords, layers) = truncate_layered_path(coords, layers, MAX_PATH_SEGMENT_STEPS);
        return Some((coords, layers));
    }

    if is_bridge_only_goal(grid, goal) {
        return None;
    }

    let path = zone_search::find_path_zoned_marker(
        grid,
        start,
        goal,
        terrain_costs,
        entity_blocks,
        zone_grid,
        zone_mz,
        movement_zone,
        resolved_terrain,
        entity_block_map,
        marker_overlay,
        ctx.blocker_neighbor_counts,
        urgency,
        mover_is_crusher,
        is_infantry,
    )?;

    if contains_non_adjacent_step(&path) {
        let path_layers = build_flat_fallback_layers(&path, start_layer, grid);
        let (path, path_layers) = truncate_layered_path(path, path_layers, MAX_PATH_SEGMENT_STEPS);
        return Some((path, path_layers));
    }

    let smooth_walkable = |x: u16, y: u16| -> bool {
        let terrain_ok = if movement_zone.is_some_and(|mz| mz.is_water_mover()) {
            crate::sim::pathfinding::is_cell_passable_for_mover(
                grid,
                x,
                y,
                movement_zone,
                resolved_terrain,
            )
        } else {
            grid.is_walkable(x, y)
        };
        // Soft-blocked cells (code 2/5/6) must not be used as zigzag shortcuts:
        // A* deliberately routed around them; smoothing through them would
        // undo the detour and walk the unit straight through the blocker.
        terrain_ok
            && !entity_blocks.is_some_and(|eb| eb.contains(&(x, y)))
            && !entity_block_map.is_some_and(|m| m.contains_any(&(x, y)))
            && !marker_overlay.is_some_and(|m| m.contains((x, y)) && (x, y) != goal)
    };
    let path = path_smooth::smooth_path(path, &smooth_walkable);
    let path = path_smooth::optimize_path(path, &smooth_walkable);
    let path_layers = build_flat_fallback_layers(&path, start_layer, grid);
    let (path, path_layers) = truncate_layered_path(path, path_layers, MAX_PATH_SEGMENT_STEPS);
    Some((path, path_layers))
}

fn contains_non_adjacent_step(path: &[(u16, u16)]) -> bool {
    path.windows(2).any(|pair| {
        let dx = pair[1].0.abs_diff(pair[0].0);
        let dy = pair[1].1.abs_diff(pair[0].1);
        dx > 1 || dy > 1
    })
}

/// Build per-cell movement layers for a flat A* fallback path.
///
/// If the entity starts on a bridge, preserve `MovementLayer::Bridge` for
/// contiguous bridge-walkable cells from the path start. Once the path
/// leaves the bridge deck, all remaining cells are `Ground`.
fn build_flat_fallback_layers(
    path: &[(u16, u16)],
    start_layer: MovementLayer,
    grid: &PathGrid,
) -> Vec<MovementLayer> {
    if start_layer != MovementLayer::Bridge {
        return vec![MovementLayer::Ground; path.len()];
    }
    let mut layers = Vec::with_capacity(path.len());
    let mut on_bridge = true;
    for &(x, y) in path {
        if on_bridge && grid.is_walkable_on_layer(x, y, MovementLayer::Bridge) {
            layers.push(MovementLayer::Bridge);
        } else {
            on_bridge = false;
            layers.push(MovementLayer::Ground);
        }
    }
    layers
}

#[allow(clippy::too_many_arguments)]
pub(super) fn try_repath_after_block(
    target: &mut MovementTarget,
    facing: &mut u8,
    current: (u16, u16),
    current_layer: MovementLayer,
    layered_pathing: bool,
    ctx: PathfindingContext<'_>,
    terrain_costs: Option<&TerrainCostGrid>,
    entity_blocks: Option<&BTreeSet<(u16, u16)>>,
    _rng: &mut SimRng,
    movement_zone: Option<MovementZone>,
    too_big_to_fit_under_bridge: bool,
    mcfg: MovementConfig,
    entity_block_map: Option<&LayeredEntityBlockMap>,
    urgency: u8,
    mover_is_crusher: bool,
    is_infantry: bool,
    marker_search: Option<&super::path_markers::BridgeMarkerSearch>,
) -> bool {
    let goal = target
        .final_goal
        .unwrap_or_else(|| target.path.last().copied().unwrap_or(current));
    if goal == current {
        return false;
    }
    let Some(grid) = ctx.path_grid else {
        target.movement_delay = mcfg.path_delay_ticks;
        return false;
    };

    let combined_blocks: BTreeSet<(u16, u16)> = merge_path_blocks(
        entity_blocks,
        ctx.resolved_terrain,
        movement_zone,
        too_big_to_fit_under_bridge,
    );
    let Some(effective_goal) = resolve_requested_move_goal(
        grid,
        goal,
        Some(&combined_blocks),
        movement_zone,
        ctx.resolved_terrain,
        10,
    ) else {
        target.movement_delay = mcfg.path_delay_ticks;
        return false;
    };
    if effective_goal != goal {
        log::info!(
            "Repath: goal ({},{}) blocked, redirecting to ({},{})",
            goal.0,
            goal.1,
            effective_goal.0,
            effective_goal.1,
        );
        target.final_goal = Some(effective_goal);
    }

    let zone_mz = movement_zone.unwrap_or(MovementZone::Normal);
    // The layered A* path consults ground_blocks/bridge_blocks (not entity_blocks)
    // for per-layer hard blocking. Pass the merged set as both ground_blocks and
    // bridge_blocks so the layered search sees structure footprints / stationary
    // obstacles on either layer the same way the flat search does.
    let marker_overlay = marker_search
        .map(|search| &search.overlay)
        .filter(|overlay| !overlay.is_empty());
    let effective_urgency = marker_search.map_or(urgency, |search| search.effective_urgency);
    let path_result = find_move_path_with_marker(
        ctx,
        layered_pathing,
        current,
        current_layer,
        effective_goal,
        terrain_costs,
        Some(&combined_blocks),
        Some(&combined_blocks),
        Some(&combined_blocks),
        zone_mz,
        movement_zone,
        too_big_to_fit_under_bridge,
        entity_block_map,
        marker_overlay,
        effective_urgency,
        mover_is_crusher,
        is_infantry,
    );
    let Some((new_path, new_layers)) = path_result else {
        target.movement_delay = mcfg.path_delay_ticks;
        return false;
    };
    if new_path.len() < 2 {
        target.movement_delay = mcfg.path_delay_ticks;
        return false;
    }

    target.path = new_path;
    target.path_layers = new_layers;
    debug_assert_eq!(
        target.path.len(),
        target.path_layers.len(),
        "path/path_layers desync after blocked repath"
    );
    target.next_index = 1;
    // Infantry: clear blocking state on repath success (fresh grace period).
    // Vehicles: keep both flags — permanent impatience after first blockage.
    if is_infantry {
        target.blocked_delay = 0;
        target.path_blocked = false;
    }
    // Do NOT set movement_delay on successful repath. gamemd chains
    // Process_Drive_Track(is_retry=1) in the same tick, producing a 0-tick
    // gap. The new path starts consuming on the next tick.
    let next = target.path[target.next_index];
    let dx = next.0 as i32 - current.0 as i32;
    let dy = next.1 as i32 - current.1 as i32;
    let (d_x, d_y, d_len) = crate::util::lepton::cell_delta_to_lepton_dir(dx, dy);
    target.move_dir_x = d_x;
    target.move_dir_y = d_y;
    target.move_dir_len = d_len;
    *facing = facing_from_delta(dx, dy);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::resolved_terrain::{
        ResolvedTerrainCell, ResolvedTerrainGrid, YR_CELL_LAND_TUNNEL,
    };
    use crate::map::tube_facts::{TubeFact, TubeId};
    use crate::rules::terrain_rules::{SpeedCostProfile, TerrainClass};
    use crate::sim::pathfinding::passability::LandType;
    use crate::sim::pathfinding::zone_map::ZoneGrid;
    use std::collections::BTreeMap;

    fn make_resolved_cell(rx: u16, ry: u16) -> ResolvedTerrainCell {
        ResolvedTerrainCell {
            rx,
            ry,
            source_tile_index: 0,
            source_sub_tile: 0,
            final_tile_index: 0,
            final_sub_tile: 0,
            is_wood_bridge_repair_tile: false,
            level: 0,
            filled_clear: false,
            tileset_index: Some(0),
            land_type: 0,
            yr_cell_land_type: 0,
            slope_type: 0,
            template_height: 0,
            render_offset_x: 0,
            render_offset_y: 0,
            terrain_class: TerrainClass::Clear,
            speed_costs: SpeedCostProfile::default(),
            is_water: false,
            is_cliff_like: false,
            is_rough: false,
            is_road: false,
            accepts_smudge: false,
            allows_tiberium: false,
            height_in_pixels: 0,
            variant: 0,
            has_ramp: false,
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
            bridge_facts: crate::map::bridge_facts::BridgeCellFacts::default(),
            tube_index: None,
            radar_left: [0, 0, 0],
            radar_right: [0, 0, 0],
            has_damaged_data: false,
            bridgehead_anchor_class_at_load: None,
        }
    }

    #[test]
    fn water_mover_goal_redirect_stays_on_water_cells() {
        let mut cells = Vec::new();
        for ry in 0..3 {
            for rx in 0..3 {
                let is_water = matches!((rx, ry), (0, 1) | (1, 1) | (2, 1));
                cells.push(ResolvedTerrainCell {
                    land_type: if is_water {
                        LandType::Water.as_index()
                    } else {
                        LandType::Clear.as_index()
                    },
                    is_water,
                    ..make_resolved_cell(rx, ry)
                });
            }
        }
        let terrain = ResolvedTerrainGrid::from_cells(3, 3, cells);
        let grid = PathGrid::from_resolved_terrain(&terrain);
        let mut blocked = BTreeSet::new();
        blocked.insert((1, 1));

        let redirected = resolve_requested_move_goal(
            &grid,
            (1, 1),
            Some(&blocked),
            Some(MovementZone::Water),
            Some(&terrain),
            2,
        )
        .expect("water mover should find an alternate water goal");

        assert!(
            matches!(redirected, (0, 1) | (2, 1)),
            "water mover redirect should stay on water, got {:?}",
            redirected
        );
    }

    #[test]
    fn explicit_tube_path_survives_zone_precheck_and_smoothing() {
        let mut cells: Vec<_> = (0..5).map(|x| make_resolved_cell(x, 0)).collect();
        cells[0].yr_cell_land_type = YR_CELL_LAND_TUNNEL;
        cells[0].tube_index = Some(TubeId(0));
        for cell in &mut cells[1..4] {
            cell.ground_walk_blocked = true;
            cell.base_ground_walk_blocked = true;
        }
        let terrain = ResolvedTerrainGrid::from_cells_with_tubes(
            5,
            1,
            cells,
            vec![TubeFact::explicit((0, 0), (4, 0), 2, vec![2, 2, 2, 2])],
        );
        let grid = PathGrid::from_resolved_terrain(&terrain);
        let zone_grid = ZoneGrid::build(&grid, &BTreeMap::new(), 5, 1);

        let (path, layers) = find_move_path(
            PathfindingContext {
                path_grid: Some(&grid),
                zone_grid: Some(&zone_grid),
                resolved_terrain: Some(&terrain),
                blocker_neighbor_counts: None,
            },
            false,
            (0, 0),
            MovementLayer::Ground,
            (4, 0),
            None,
            None,
            None,
            None,
            MovementZone::Normal,
            Some(MovementZone::Normal),
            false,
            None,
            0,
            false,
            false,
        )
        .expect("movement path should use explicit tube despite disconnected zones");

        assert_eq!(path, vec![(0, 0), (4, 0)]);
        assert_eq!(layers, vec![MovementLayer::Ground, MovementLayer::Ground]);
    }

    #[test]
    fn move_path_marker_overlay_survives_path_smoothing() {
        let grid = PathGrid::test_all_passable(5, 3);
        let mut marker_overlay = SearchMarkerOverlay::new();
        marker_overlay.toggle((1, 1));
        marker_overlay.toggle((2, 1));
        marker_overlay.toggle((3, 1));

        let (path, layers) = find_move_path_with_marker(
            PathfindingContext {
                path_grid: Some(&grid),
                zone_grid: None,
                resolved_terrain: None,
                blocker_neighbor_counts: None,
            },
            false,
            (0, 1),
            MovementLayer::Ground,
            (4, 1),
            None,
            None,
            None,
            None,
            MovementZone::Normal,
            Some(MovementZone::Normal),
            false,
            None,
            Some(&marker_overlay),
            0,
            false,
            false,
        )
        .expect("marker overlay should still allow a path");

        assert_eq!(path.first().copied(), Some((0, 1)));
        assert_eq!(path.last().copied(), Some((4, 1)));
        assert!(
            !path
                .iter()
                .any(|cell| matches!(cell, (1, 1) | (2, 1) | (3, 1))),
            "path smoothing must not collapse the marker-avoiding route back through {:?}",
            path
        );
        assert_eq!(path.len(), layers.len());
    }

    #[test]
    fn gsi_04_12_marker_blocked_repath_consumes_overlay() {
        let grid = PathGrid::test_all_passable(5, 3);
        let mut marker_search = super::super::path_markers::BridgeMarkerSearch::default();
        marker_search.effective_urgency = 1;
        for x in 1..=3 {
            marker_search.overlay.toggle((x, 1));
        }
        let mut target = MovementTarget {
            path: vec![(0, 1), (1, 1), (2, 1), (3, 1), (4, 1)],
            path_layers: vec![MovementLayer::Ground; 5],
            next_index: 1,
            final_goal: Some((4, 1)),
            ..MovementTarget::default()
        };
        let mut facing = 0;
        let mut rng = crate::sim::rng::SimRng::new(0);

        assert!(try_repath_after_block(
            &mut target,
            &mut facing,
            (0, 1),
            MovementLayer::Ground,
            false,
            PathfindingContext {
                path_grid: Some(&grid),
                zone_grid: None,
                resolved_terrain: None,
                blocker_neighbor_counts: None,
            },
            None,
            None,
            &mut rng,
            Some(MovementZone::Normal),
            false,
            MovementConfig {
                close_enough: crate::util::fixed_math::SIM_ZERO,
                path_delay_ticks: 9,
                blockage_path_delay_ticks: 60,
            },
            None,
            1,
            false,
            false,
            Some(&marker_search),
        ));
        assert!(
            !target
                .path
                .iter()
                .any(|cell| matches!(cell, (1, 1) | (2, 1) | (3, 1))),
            "blocked repath must route around the search-scoped marker: {:?}",
            target.path
        );
    }

    #[test]
    fn gsi_04_12_layered_failure_does_not_retry_flat_ground_path() {
        let mut grid = PathGrid::test_all_passable(3, 1);
        // Start on a non-transition bridge deck. Native layered A* cannot
        // descend directly to the ground-only neighbor, while a separate flat
        // search from the same coordinate would incorrectly find (0..=2, 0).
        grid.set_cell_for_test(0, 0, 0, true, false);
        assert!(grid.is_walkable(2, 0), "goal must remain ground-walkable");

        let flat = find_move_path(
            PathfindingContext {
                path_grid: Some(&grid),
                zone_grid: None,
                resolved_terrain: None,
                blocker_neighbor_counts: None,
            },
            false,
            (0, 0),
            MovementLayer::Ground,
            (2, 0),
            None,
            None,
            None,
            None,
            MovementZone::Normal,
            Some(MovementZone::Normal),
            false,
            None,
            0,
            false,
            false,
        )
        .expect("fixture must prove the removed ground-only retry could succeed");
        assert_eq!(flat.0.last().copied(), Some((2, 0)));

        let layered = find_move_path(
            PathfindingContext {
                path_grid: Some(&grid),
                zone_grid: None,
                resolved_terrain: None,
                blocker_neighbor_counts: None,
            },
            true,
            (0, 0),
            MovementLayer::Bridge,
            (2, 0),
            None,
            None,
            None,
            None,
            MovementZone::Normal,
            Some(MovementZone::Normal),
            false,
            None,
            0,
            false,
            false,
        );

        assert!(
            layered.is_none(),
            "a failed layered search must remain failed instead of launching a second flat A*"
        );
    }
}
