//! Building placement validation, sell/repair, and producer focus management.
//!
//! Handles placement preview, build area checks, sell refunds with crew ejection,
//! repair tick, and producer cycling.

use std::collections::BTreeMap;

use crate::map::bridge_facts::BRIDGE_FLAG_DESTROYED_OR_RAMP;
use crate::map::entities::EntityCategory;
use crate::map::houses::are_houses_friendly;
use crate::rules::locomotor_type::MovementZone;
use crate::rules::object_type::ObjectCategory;
use crate::rules::ruleset::RuleSet;
use crate::sim::components::BuildingUp;
use crate::sim::entity_store::EntityStore;
use crate::sim::pathfinding;
use crate::sim::world::Simulation;

use super::production_refinery::maybe_spawn_refinery_harvester;
use super::production_tech::{foundation_dimensions, producer_candidates_for_owner_category};
use super::production_types::*;

pub fn placement_preview_for_owner(
    sim: &Simulation,
    rules: &RuleSet,
    owner: &str,
    type_id: &str,
    rx: u16,
    ry: u16,
    path_grid: Option<&crate::sim::pathfinding::PathGrid>,
    height_map: &BTreeMap<(u16, u16), u8>,
) -> Option<BuildingPlacementPreview> {
    let obj = rules.object(type_id)?;
    let (width, height) = foundation_dimensions(&obj.foundation);
    let reason =
        evaluate_building_placement(sim, rules, owner, type_id, rx, ry, path_grid, height_map)
            .err();
    let in_build_area = reason.as_ref().map_or(true, |r| {
        !matches!(r, BuildingPlacementError::OutOfBuildArea)
    });
    let mut cell_valid: Vec<bool> = Vec::with_capacity((width as usize) * (height as usize));
    for dy in 0..height {
        for dx in 0..width {
            let cx: u16 = rx.saturating_add(dx);
            let cy: u16 = ry.saturating_add(dy);
            let ok = cell_placeable(
                sim,
                &sim.substrate.entities,
                rules,
                path_grid,
                cx,
                cy,
                obj.water_bound,
            );
            cell_valid.push(in_build_area && ok);
        }
    }
    let type_interned = sim.interner.get(type_id).unwrap_or_default();
    Some(BuildingPlacementPreview {
        type_id: type_interned,
        rx,
        ry,
        width,
        height,
        valid: reason.is_none(),
        reason,
        cell_valid,
    })
}

pub fn active_producer_for_owner_category(
    sim: &Simulation,
    rules: &RuleSet,
    owner: &str,
    category: ProductionCategory,
) -> Option<ProducerFocusView> {
    let candidates = producer_candidates_for_owner_category(
        &sim.substrate.entities,
        rules,
        owner,
        category,
        true,
        &sim.interner,
    );
    let owner_id = sim.interner.get(owner);
    let active_sid = owner_id
        .and_then(|id| sim.production.active_producer_by_owner.get(&id))
        .and_then(|categories| categories.get(&category))
        .copied();
    let selected = active_sid
        .and_then(|sid| {
            candidates
                .iter()
                .find(|candidate| candidate.0 == sid)
                .cloned()
        })
        .or_else(|| candidates.into_iter().next())?;
    let display_name = rules
        .object(&selected.3)
        .and_then(|obj| obj.name.clone())
        .unwrap_or_else(|| selected.3.clone());
    Some(ProducerFocusView {
        stable_id: selected.0,
        display_name,
        category,
        rx: selected.1,
        ry: selected.2,
    })
}

pub fn toggle_pause_for_owner_category(
    sim: &mut Simulation,
    owner: &str,
    category: ProductionCategory,
) -> bool {
    let owner_id = sim.interner.intern(owner);
    // P5d: pause is a registry flag on the active build (the retired `front.state` Paused
    // bridge). `step_all` skips a `manual` factory without losing progress; unpausing
    // auto-resumes via `set_rate`.
    sim.production.factory_shadow.toggle_pause(owner_id, category)
}

pub fn cycle_active_producer_for_owner_category(
    sim: &mut Simulation,
    rules: &RuleSet,
    owner: &str,
    category: ProductionCategory,
) -> bool {
    let candidates = producer_candidates_for_owner_category(
        &sim.substrate.entities,
        rules,
        owner,
        category,
        true,
        &sim.interner,
    );
    if candidates.is_empty() {
        return false;
    }

    let owner_id = sim.interner.intern(owner);
    let current = sim
        .production
        .active_producer_by_owner
        .get(&owner_id)
        .and_then(|categories| categories.get(&category))
        .copied();
    let next_sid =
        match current.and_then(|sid| candidates.iter().position(|candidate| candidate.0 == sid)) {
            Some(index) => candidates[(index + 1) % candidates.len()].0,
            None => candidates[0].0,
        };
    sim.production
        .active_producer_by_owner
        .entry(owner_id)
        .or_default()
        .insert(category, next_sid);
    true
}

pub fn place_ready_building(
    sim: &mut Simulation,
    rules: &RuleSet,
    owner: &str,
    type_id: &str,
    rx: u16,
    ry: u16,
    path_grid: Option<&crate::sim::pathfinding::PathGrid>,
    height_map: &BTreeMap<(u16, u16), u8>,
) -> bool {
    let Some(obj) = rules.object(type_id) else {
        return false;
    };
    if obj.category != ObjectCategory::Building {
        return false;
    }

    let owner_id = sim.interner.intern(owner);
    let type_interned = sim.interner.intern(type_id);
    let Some(ready_queue) = sim.production.ready_by_owner.get(&owner_id) else {
        return false;
    };
    if !ready_queue.iter().any(|&queued| queued == type_interned) {
        return false;
    }
    if evaluate_building_placement(sim, rules, owner, type_id, rx, ry, path_grid, height_map)
        .is_err()
    {
        return false;
    }
    let foundation_str: String = rules
        .object(type_id)
        .map(|o| o.foundation.clone())
        .unwrap_or_else(|| "?".to_string());
    let z: u8 = height_map.get(&(rx, ry)).copied().unwrap_or(0);
    log::info!(
        "Placing building {} at ({},{}) z={} foundation={}",
        type_id,
        rx,
        ry,
        z,
        foundation_str,
    );
    let new_sid = match sim.spawn_object(type_id, owner, rx, ry, 0, rules, height_map) {
        Some(sid) => sid,
        None => return false,
    };
    // Log screen position for debugging placement alignment.
    if let Some(ge) = sim.substrate.entities.get(new_sid) {
        let (fw, fh) = foundation_dimensions(&foundation_str);
        log::info!(
            "  → spawned sid={} screen=({:.0},{:.0}) foundation_cells: ({},{})..({},{})",
            new_sid,
            ge.position.screen_x,
            ge.position.screen_y,
            rx,
            ry,
            rx + fw - 1,
            ry + fh - 1,
        );
    }
    // Tag newly placed buildings with build-up animation (~1 second at 30Hz).
    if let Some(ge) = sim.substrate.entities.get_mut(new_sid) {
        ge.building_up = Some(BuildingUp {
            elapsed_ticks: 0,
            total_ticks: 30,
        });
    }
    maybe_spawn_refinery_harvester(sim, rules, owner, type_id, rx, ry, path_grid, height_map);

    // Refresh superweapon grants — newly placed building may provide a SW.
    if sim.game_options.super_weapons {
        crate::sim::superweapon::refresh_super_weapons_for_owner(sim, rules, owner_id);
    }

    let Some(ready_queue) = sim.production.ready_by_owner.get_mut(&owner_id) else {
        return false;
    };
    let Some(index) = ready_queue
        .iter()
        .position(|&queued| queued == type_interned)
    else {
        return false;
    };
    ready_queue.remove(index);
    if ready_queue.is_empty() {
        sim.production.ready_by_owner.remove(&owner_id);
    }
    true
}

fn evaluate_building_placement(
    sim: &Simulation,
    rules: &RuleSet,
    owner: &str,
    type_id: &str,
    rx: u16,
    ry: u16,
    path_grid: Option<&crate::sim::pathfinding::PathGrid>,
    _height_map: &BTreeMap<(u16, u16), u8>,
) -> Result<(), BuildingPlacementError> {
    let Some(obj) = rules.object(type_id) else {
        return Err(BuildingPlacementError::NotBuilding);
    };
    let (width, height) = foundation_dimensions(&obj.foundation);
    if obj.category != ObjectCategory::Building {
        return Err(BuildingPlacementError::NotBuilding);
    }
    let owner_id = sim.interner.get(owner);
    let type_interned = sim.interner.get(type_id);
    let ready_for_owner = owner_id.and_then(|id| sim.production.ready_by_owner.get(&id));
    let Some(ready_for_owner) = ready_for_owner else {
        return Err(BuildingPlacementError::NotReady);
    };
    let has_type = type_interned.map_or(false, |tid| {
        ready_for_owner.iter().any(|&queued| queued == tid)
    });
    if !has_type {
        return Err(BuildingPlacementError::NotReady);
    }
    for dy in 0..height {
        for dx in 0..width {
            let cell_x = rx.saturating_add(dx);
            let cell_y = ry.saturating_add(dy);
            if !cell_placeable(
                sim,
                &sim.substrate.entities,
                rules,
                path_grid,
                cell_x,
                cell_y,
                obj.water_bound,
            ) {
                // Distinguish overlap from terrain for the error variant.
                if structure_occupies_cell(&sim.substrate.entities, rules, cell_x, cell_y, &sim.interner) {
                    return Err(BuildingPlacementError::OverlapsStructure);
                }
                return Err(BuildingPlacementError::BlockedTerrain);
            }
        }
    }
    if is_within_build_area(sim, rules, owner, obj, rx, ry, width, height) {
        Ok(())
    } else {
        let providers: Vec<String> = sim
            .substrate.entities
            .values()
            .filter(|e| {
                e.category == EntityCategory::Structure
                    && sim.interner.resolve(e.owner).eq_ignore_ascii_case(owner)
            })
            .map(|e| {
                let type_str = sim.interner.resolve(e.type_ref);
                let bn = rules.object(type_str).map_or(false, |o| o.base_normal);
                format!(
                    "{}@({},{}) bn={}",
                    type_str, e.position.rx, e.position.ry, bn
                )
            })
            .collect();
        log::warn!(
            "Placement rejected: ({},{}) {}x{} outside build area for {} adj={} providers=[{}]",
            rx,
            ry,
            width,
            height,
            owner,
            obj.adjacent,
            providers.join(", "),
        );
        Err(BuildingPlacementError::OutOfBuildArea)
    }
}

/// Per-cell placement check shared by preview and validation.
///
/// When `water_bound` is true (naval yards), the cell MUST be ship-passable
/// water terrain, not merely `is_water=true`. Shore/beach cells can look watery
/// but are not valid for `MovementZone::Water` ships, which would trap produced
/// destroyers/cruisers while still allowing amphibious craft to move.
///
/// Normal walkability/build_blocked checks are skipped for WaterBound buildings
/// because water cells are intentionally blocked in those generic land-building
/// paths. Instead, we validate against the ship passability matrix plus static
/// overlay/terrain blockers.
fn cell_placeable(
    sim: &Simulation,
    entities: &EntityStore,
    rules: &RuleSet,
    path_grid: Option<&crate::sim::pathfinding::PathGrid>,
    cx: u16,
    cy: u16,
    water_bound: bool,
) -> bool {
    let no_overlap = !structure_occupies_cell(entities, rules, cx, cy, &sim.interner);

    if water_bound {
        let cell_ok = if let Some(terrain) = sim.resolved_terrain.as_ref() {
            terrain.cell(cx, cy).is_some_and(|cell| {
                let ship_passable = pathfinding::passability::is_passable_for_zone(
                    cell.land_type,
                    MovementZone::Water,
                );
                ship_passable
                    && !cell.overlay_blocks
                    && !cell.terrain_object_blocks
                    && !cell.has_bridge_deck
                    && !cell.bridge_walkable
                    && !cell.bridge_facts.has_flag(BRIDGE_FLAG_DESTROYED_OR_RAMP)
            })
        } else {
            path_grid.is_some_and(|grid| {
                pathfinding::is_cell_passable_for_mover(
                    grid,
                    cx,
                    cy,
                    Some(MovementZone::Water),
                    None,
                )
            })
        };
        cell_ok && no_overlap
    } else {
        let cell_ok = if let Some(terrain) = sim.resolved_terrain.as_ref() {
            terrain.cell(cx, cy).is_some_and(|cell| {
                !cell.build_blocked
                    && !cell.overlay_blocks
                    && !cell.terrain_object_blocks
                    && !cell.has_bridge_deck
                    && !cell.bridge_walkable
                    && !cell.bridge_facts.has_flag(BRIDGE_FLAG_DESTROYED_OR_RAMP)
                    && cell.slope_type == 0
            })
        } else {
            let walkable = path_grid.map_or(true, |g| g.is_walkable(cx, cy));
            let not_blocked = !sim.effective_build_blocked(cx, cy).unwrap_or(false);
            walkable && not_blocked
        };
        cell_ok && no_overlap
    }
}

pub(crate) fn structure_occupies_cell(
    entities: &EntityStore,
    rules: &RuleSet,
    rx: u16,
    ry: u16,
    interner: &crate::sim::intern::StringInterner,
) -> bool {
    entities.values().any(|e| {
        // A dying structure is unmarked from cell lists synchronously in uninit;
        // it must not block placement during its deferred-delete window.
        if e.dying {
            return false;
        }
        if e.category != EntityCategory::Structure {
            return false;
        }
        let Some(existing) = rules.object(interner.resolve(e.type_ref)) else {
            return false;
        };
        // Wall entities render and behave as overlays — they don't block building
        // placement of other structures. A wall cell is only blocked to another wall
        // of the same type, which is handled by the overlay list, not the entity store.
        if existing.wall {
            return false;
        }
        let (width, height) = foundation_dimensions(&existing.foundation);
        rx >= e.position.rx
            && rx < e.position.rx.saturating_add(width)
            && ry >= e.position.ry
            && ry < e.position.ry.saturating_add(height)
    })
}

fn is_within_build_area(
    sim: &Simulation,
    rules: &RuleSet,
    owner: &str,
    obj: &crate::rules::object_type::ObjectType,
    rx: u16,
    ry: u16,
    width: u16,
    height: u16,
) -> bool {
    let placed_adjacent = obj.adjacent;
    if placed_adjacent < 0 {
        return false;
    }
    for e in sim.substrate.entities.values() {
        // A dying structure provides no build-area adjacency during its window.
        if e.dying {
            continue;
        }
        if e.category != EntityCategory::Structure {
            continue;
        }
        let provider_owner = sim.interner.resolve(e.owner);
        let Some(existing) = sim.object_type(e.type_ref, rules) else {
            continue;
        };
        if provider_owner.eq_ignore_ascii_case(owner) {
            if !existing.base_normal {
                continue;
            }
        } else if !(sim.game_options.build_off_ally
            && existing.eligibile_for_ally_building
            && are_houses_friendly(&sim.house_alliances, provider_owner, owner))
        {
            continue;
        }
        let (provider_width, provider_height) = foundation_dimensions(&existing.foundation);
        if provider_intersects_build_area_ring(
            (rx, ry, width, height),
            (
                e.position.rx,
                e.position.ry,
                provider_width,
                provider_height,
            ),
            placed_adjacent,
        ) {
            return true;
        }
    }
    false
}

fn provider_intersects_build_area_ring(
    placed: (u16, u16, u16, u16),
    provider: (u16, u16, u16, u16),
    adjacent: i32,
) -> bool {
    let (placed_rx, placed_ry, placed_width, placed_height) = placed;
    let (provider_rx, provider_ry, provider_width, provider_height) = provider;
    let expansion = adjacent.saturating_add(1);
    let placed_min_x = i32::from(placed_rx);
    let placed_min_y = i32::from(placed_ry);
    let placed_max_x = i32::from(placed_rx) + i32::from(placed_width) - 1;
    let placed_max_y = i32::from(placed_ry) + i32::from(placed_height) - 1;
    let min_x = placed_min_x - expansion;
    let min_y = placed_min_y - expansion;
    let max_x = placed_max_x + expansion;
    let max_y = placed_max_y + expansion;
    let provider_min_x = i32::from(provider_rx);
    let provider_min_y = i32::from(provider_ry);
    let provider_max_x = i32::from(provider_rx) + i32::from(provider_width) - 1;
    let provider_max_y = i32::from(provider_ry) + i32::from(provider_height) - 1;

    let intersects_expanded = provider_min_x <= max_x
        && provider_max_x >= min_x
        && provider_min_y <= max_y
        && provider_max_y >= min_y;
    let intersects_foundation = provider_min_x <= placed_max_x
        && provider_max_x >= placed_min_x
        && provider_min_y <= placed_max_y
        && provider_max_y >= placed_min_y;

    intersects_expanded && !intersects_foundation
}
