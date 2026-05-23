//! Spawn cell selection for newly produced units.
//!
//! Determines where to place a unit after production completes, based on
//! factory location, exit offsets, and walkability. Extracted from
//! production_tech.rs for file-size limits.

use crate::map::resolved_terrain::ResolvedTerrainGrid;
use crate::rules::locomotor_type::MovementZone;
use crate::rules::object_type::{FactoryType, ObjectCategory};
use crate::rules::ruleset::RuleSet;
use crate::sim::world::Simulation;

use super::production_tech::producer_candidates_for_owner_category;
use super::production_types::ProductionCategory;
use crate::sim::movement::bump_crush;
use crate::sim::movement::locomotor::MovementLayer;
use crate::sim::occupancy::OccupancyGrid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProductionSpawnSelection {
    pub producer_id: u64,
    pub cell: (u16, u16),
}

pub fn find_spawn_cell_for_owner(
    sim: &mut Simulation,
    rules: &RuleSet,
    owner: &str,
    produced_category: ObjectCategory,
    path_grid: Option<&crate::sim::pathfinding::PathGrid>,
    require_water: bool,
) -> Option<(u16, u16)> {
    find_spawn_selection_for_owner(
        sim,
        rules,
        owner,
        produced_category,
        path_grid,
        require_water,
    )
    .map(|selection| selection.cell)
}

pub fn find_spawn_selection_for_owner(
    sim: &mut Simulation,
    rules: &RuleSet,
    owner: &str,
    produced_category: ObjectCategory,
    path_grid: Option<&crate::sim::pathfinding::PathGrid>,
    require_water: bool,
) -> Option<ProductionSpawnSelection> {
    let Some(queue_category) = producer_queue_category_for_object(produced_category) else {
        return None;
    };
    let preferred_factories = producer_candidates_for_owner_category(
        &sim.entities,
        rules,
        owner,
        queue_category,
        true,
        &sim.interner,
    );
    let fallback_structures = producer_candidates_for_owner_category(
        &sim.entities,
        rules,
        owner,
        queue_category,
        false,
        &sim.interner,
    );
    let mut ordered_bases = preferred_factories.clone();
    let owner_id = sim.interner.intern(owner);
    if let Some(active_sid) = sim
        .production
        .active_producer_by_owner
        .get(&owner_id)
        .and_then(|categories| categories.get(&queue_category))
        .copied()
    {
        if let Some(index) = ordered_bases
            .iter()
            .position(|candidate| candidate.0 == active_sid)
        {
            ordered_bases.rotate_left(index);
        }
    } else if let Some(first) = preferred_factories.first() {
        sim.production
            .active_producer_by_owner
            .entry(owner_id)
            .or_default()
            .insert(queue_category, first.0);
    }

    let bases: &[(u64, u16, u16, String)] = if ordered_bases.is_empty() {
        &fallback_structures
    } else {
        &ordered_bases
    };
    let resolved_terrain = sim.resolved_terrain.as_ref();

    if produced_category == ObjectCategory::Vehicle && !require_water {
        if let Some((producer_id, bx, by, structure_id)) = bases.first() {
            if exact_land_vehicle_exit_factory(rules, structure_id) {
                return find_exact_exitcoord_spawn_cell(
                    *bx,
                    *by,
                    structure_id,
                    produced_category,
                    rules,
                    path_grid,
                    &sim.occupancy,
                    resolved_terrain,
                    require_water,
                )
                .map(|cell| ProductionSpawnSelection {
                    producer_id: *producer_id,
                    cell,
                });
            }
        }
    }

    for (producer_id, bx, by, structure_id) in bases {
        let cell = match produced_category {
            ObjectCategory::Infantry => {
                find_infantry_spawn_cell_near_structure(rules, *bx, *by, structure_id)
            }
            _ => find_spawn_cell_near_structure(
                *bx,
                *by,
                structure_id,
                produced_category,
                rules,
                path_grid,
                &sim.occupancy,
                resolved_terrain,
                require_water,
            ),
        };
        if let Some(cell) = cell {
            return Some(ProductionSpawnSelection {
                producer_id: *producer_id,
                cell,
            });
        }
    }
    None
}

/// Mark the produced unit as having the reciprocal RadioClass contact created
/// by successful stock land war-factory unlimbo.
///
/// The caller must invoke this immediately after `spawn_object` returns the
/// produced unit stable ID. `find_spawn_selection_for_owner` supplies the
/// `producer_id` without changing the older cell-only API.
pub fn mark_war_factory_spawn_contact(
    sim: &mut Simulation,
    rules: &RuleSet,
    producer_id: u64,
    produced_id: u64,
) -> bool {
    let Some((producer_type, produced_is_vehicle)) = sim.entities.get(producer_id).and_then(|p| {
        let producer_type = sim.interner.resolve(p.type_ref).to_string();
        let produced = sim.entities.get(produced_id)?;
        Some((
            producer_type,
            produced.category == crate::map::entities::EntityCategory::Unit,
        ))
    }) else {
        return false;
    };

    if !produced_is_vehicle || !exact_land_vehicle_exit_factory(rules, &producer_type) {
        return false;
    }

    let Some(produced) = sim.entities.get_mut(produced_id) else {
        return false;
    };
    produced.mark_live_contact_with(producer_id);
    true
}

fn exact_land_vehicle_exit_factory(rules: &RuleSet, structure_id: &str) -> bool {
    rules.object(structure_id).is_some_and(|obj| {
        obj.factory == Some(FactoryType::UnitType) && !obj.naval && obj.exit_coord.is_some()
    })
}

fn producer_queue_category_for_object(
    produced_category: ObjectCategory,
) -> Option<ProductionCategory> {
    match produced_category {
        ObjectCategory::Infantry => Some(ProductionCategory::Infantry),
        ObjectCategory::Vehicle => Some(ProductionCategory::Vehicle),
        ObjectCategory::Aircraft => Some(ProductionCategory::Aircraft),
        ObjectCategory::Building => None,
    }
}

fn find_spawn_cell_near_structure(
    base_rx: u16,
    base_ry: u16,
    structure_id: &str,
    produced_category: ObjectCategory,
    rules: &RuleSet,
    path_grid: Option<&crate::sim::pathfinding::PathGrid>,
    occupancy: &OccupancyGrid,
    resolved_terrain: Option<&ResolvedTerrainGrid>,
    require_water: bool,
) -> Option<(u16, u16)> {
    let offsets: Vec<(i16, i16)> = preferred_exit_offsets(rules, structure_id);
    for (ox, oy) in offsets {
        let Some(cand) = add_cell_offset(base_rx, base_ry, ox, oy) else {
            continue;
        };
        match path_grid {
            Some(grid) => {
                if cand.0 < grid.width()
                    && cand.1 < grid.height()
                    && spawn_cell_passable(grid, cand, resolved_terrain, require_water)
                    && cell_available_for_spawn(
                        cand,
                        produced_category,
                        occupancy,
                        resolved_terrain,
                        require_water,
                    )
                {
                    return Some(cand);
                }
            }
            None => {
                if cell_available_for_spawn(
                    cand,
                    produced_category,
                    occupancy,
                    resolved_terrain,
                    require_water,
                ) {
                    return Some(cand);
                }
            }
        }
    }

    let Some(grid) = path_grid else {
        return Some((base_rx.saturating_add(2), base_ry.saturating_add(2)));
    };
    nearest_walkable_around(
        grid,
        (base_rx, base_ry),
        12,
        produced_category,
        occupancy,
        resolved_terrain,
        require_water,
    )
}

fn find_exact_exitcoord_spawn_cell(
    base_rx: u16,
    base_ry: u16,
    structure_id: &str,
    produced_category: ObjectCategory,
    rules: &RuleSet,
    path_grid: Option<&crate::sim::pathfinding::PathGrid>,
    occupancy: &OccupancyGrid,
    resolved_terrain: Option<&ResolvedTerrainGrid>,
    require_water: bool,
) -> Option<(u16, u16)> {
    let (lx, ly, _lz) = rules.object(structure_id)?.exit_coord?;
    let cand = add_cell_offset(base_rx, base_ry, lepton_to_cell(lx), lepton_to_cell(ly))?;
    if let Some(grid) = path_grid {
        if cand.0 >= grid.width()
            || cand.1 >= grid.height()
            || !spawn_cell_passable(grid, cand, resolved_terrain, require_water)
        {
            return None;
        }
    }
    cell_available_for_spawn(
        cand,
        produced_category,
        occupancy,
        resolved_terrain,
        require_water,
    )
    .then_some(cand)
}

/// Infantry-specific spawn cell: the foundation-center cell of the producing
/// barracks. Matches the original engine's alt-path Unlimbo at the building's
/// center lepton coord; `ExitCoord` is intentionally ignored, no passability
/// check is performed, and there is no fallback to a nearby cell.
///
/// The infantry then walks out of the foundation via the existing pathfinder
/// once the rally MoveTo is issued; the foundation cells are passable to
/// infantry (only vehicles are hard-blocked).
fn find_infantry_spawn_cell_near_structure(
    rules: &RuleSet,
    base_rx: u16,
    base_ry: u16,
    structure_id: &str,
) -> Option<(u16, u16)> {
    let obj = rules.object(structure_id)?;
    let (w, h) = super::production_tech::foundation_dimensions(&obj.foundation);
    Some((base_rx.saturating_add(w / 2), base_ry.saturating_add(h / 2)))
}

fn nearest_walkable_around(
    grid: &crate::sim::pathfinding::PathGrid,
    center: (u16, u16),
    max_radius: u16,
    produced_category: ObjectCategory,
    occupancy: &OccupancyGrid,
    resolved_terrain: Option<&ResolvedTerrainGrid>,
    require_water: bool,
) -> Option<(u16, u16)> {
    let cx = center.0 as i32;
    let cy = center.1 as i32;
    let w = grid.width() as i32;
    let h = grid.height() as i32;
    for r in 1..=max_radius as i32 {
        let min_x = (cx - r).max(0);
        let max_x = (cx + r).min(w - 1);
        let min_y = (cy - r).max(0);
        let max_y = (cy + r).min(h - 1);
        for x in min_x..=max_x {
            let top = (x as u16, min_y as u16);
            if spawn_cell_passable(grid, top, resolved_terrain, require_water)
                && cell_available_for_spawn(
                    top,
                    produced_category,
                    occupancy,
                    resolved_terrain,
                    require_water,
                )
            {
                return Some(top);
            }
            let bot = (x as u16, max_y as u16);
            if spawn_cell_passable(grid, bot, resolved_terrain, require_water)
                && cell_available_for_spawn(
                    bot,
                    produced_category,
                    occupancy,
                    resolved_terrain,
                    require_water,
                )
            {
                return Some(bot);
            }
        }
        for y in (min_y + 1)..=(max_y - 1) {
            let left = (min_x as u16, y as u16);
            if spawn_cell_passable(grid, left, resolved_terrain, require_water)
                && cell_available_for_spawn(
                    left,
                    produced_category,
                    occupancy,
                    resolved_terrain,
                    require_water,
                )
            {
                return Some(left);
            }
            let right = (max_x as u16, y as u16);
            if spawn_cell_passable(grid, right, resolved_terrain, require_water)
                && cell_available_for_spawn(
                    right,
                    produced_category,
                    occupancy,
                    resolved_terrain,
                    require_water,
                )
            {
                return Some(right);
            }
        }
    }
    None
}

/// Check whether a cell can accept a newly spawned unit. Infantry require a free
/// sub-cell (max 3 per cell). Vehicles/aircraft require no existing blockers.
/// When `require_water` is true, only water cells are accepted (naval units).
/// When false, water cells are rejected (land units shouldn't spawn on water).
fn cell_available_for_spawn(
    cell: (u16, u16),
    produced_category: ObjectCategory,
    occupancy: &OccupancyGrid,
    resolved_terrain: Option<&ResolvedTerrainGrid>,
    require_water: bool,
) -> bool {
    // Terrain type filter: naval units need water, land units avoid water.
    if let Some(terrain) = resolved_terrain {
        let is_water = terrain.cell(cell.0, cell.1).map_or(false, |c| c.is_water);
        if require_water && !is_water {
            return false;
        }
        if !require_water && is_water {
            return false;
        }
    }
    let occ = occupancy.get(cell.0, cell.1);
    match produced_category {
        ObjectCategory::Infantry => {
            bump_crush::cell_passable_for_infantry(occ, MovementLayer::Ground)
        }
        _ => {
            // Vehicles/aircraft need no vehicle or structure already in the cell.
            match occ {
                Some(o) => !o.has_blockers_on(MovementLayer::Ground),
                None => true,
            }
        }
    }
}

fn spawn_cell_passable(
    grid: &crate::sim::pathfinding::PathGrid,
    cell: (u16, u16),
    resolved_terrain: Option<&ResolvedTerrainGrid>,
    require_water: bool,
) -> bool {
    if require_water {
        crate::sim::pathfinding::is_cell_passable_for_mover(
            grid,
            cell.0,
            cell.1,
            Some(MovementZone::Water),
            resolved_terrain,
        )
    } else {
        grid.is_walkable(cell.0, cell.1)
    }
}

/// Determine exit cell offsets for a factory building, data-driven from rules.ini.
///
/// If the building has `ExitCoord=X,Y,Z` in rules.ini, converts leptons to a cell
/// offset (256 leptons = 1 cell) and generates candidates around it. Otherwise,
/// falls back to foundation-perimeter offsets derived from the building's Foundation=.
fn preferred_exit_offsets(rules: &RuleSet, structure_id: &str) -> Vec<(i16, i16)> {
    if let Some(obj) = rules.object(structure_id) {
        // Data-driven: use ExitCoord from rules.ini if available.
        if let Some((lx, ly, _lz)) = obj.exit_coord {
            let primary_x: i16 = lepton_to_cell(lx);
            let primary_y: i16 = lepton_to_cell(ly);
            return exit_candidates_around(primary_x, primary_y);
        }
        // No ExitCoord: generate offsets from foundation perimeter.
        let (w, h) = super::production_tech::foundation_dimensions(&obj.foundation);
        return foundation_perimeter_offsets(w as i16, h as i16);
    }
    // Unknown structure: simple default.
    foundation_perimeter_offsets(2, 2)
}

/// Convert a lepton value to the nearest cell offset (256 leptons = 1 cell).
fn lepton_to_cell(leptons: i32) -> i16 {
    // Round toward the nearest cell center. +128 for positive, -128 for negative.
    let rounded: i32 = if leptons >= 0 {
        (leptons + 128) / 256
    } else {
        (leptons - 128) / 256
    };
    rounded as i16
}

/// Generate exit candidate offsets around a primary exit cell.
/// Returns the primary cell first, then its 8 neighbors, providing
/// fallback positions if the primary cell is blocked.
fn exit_candidates_around(cx: i16, cy: i16) -> Vec<(i16, i16)> {
    vec![
        (cx, cy),
        (cx + 1, cy),
        (cx - 1, cy),
        (cx, cy + 1),
        (cx, cy - 1),
        (cx + 1, cy + 1),
        (cx - 1, cy + 1),
        (cx + 1, cy - 1),
        (cx - 1, cy - 1),
    ]
}

/// Generate exit offsets around the perimeter of a foundation.
/// Tries bottom edge first, then right edge, then remaining sides.
fn foundation_perimeter_offsets(w: i16, h: i16) -> Vec<(i16, i16)> {
    let mut offsets: Vec<(i16, i16)> = Vec::with_capacity(((w + h) * 2 + 8) as usize);
    // Bottom edge (y = h).
    for x in 0..w {
        offsets.push((x, h));
    }
    // Right edge (x = w).
    for y in 0..h {
        offsets.push((w, y));
    }
    // Top edge (y = -1).
    for x in 0..w {
        offsets.push((x, -1));
    }
    // Left edge (x = -1).
    for y in 0..h {
        offsets.push((-1, y));
    }
    // Corners just outside the foundation.
    offsets.push((w, h));
    offsets.push((-1, -1));
    offsets.push((w, -1));
    offsets.push((-1, h));
    offsets
}

fn add_cell_offset(base_rx: u16, base_ry: u16, ox: i16, oy: i16) -> Option<(u16, u16)> {
    let rx = base_rx as i32 + ox as i32;
    let ry = base_ry as i32 + oy as i32;
    if rx < 0 || ry < 0 {
        return None;
    }
    Some((rx as u16, ry as u16))
}

/// Find an airfield with a free dock slot for a newly produced aircraft.
///
/// Returns `(airfield_stable_id, spawn_rx, spawn_ry)` — the airfield's
/// foundation center cell where the aircraft entity will be placed.
/// Returns `None` if no airfield has a free dock slot.
pub fn find_helipad_for_aircraft(
    sim: &Simulation,
    rules: &RuleSet,
    owner: &str,
) -> Option<(u64, u16, u16)> {
    let owner_id = sim.interner.get(owner)?;

    for entity in sim.entities.values() {
        if entity.category != crate::map::entities::EntityCategory::Structure {
            continue;
        }
        if entity.health.current == 0 || entity.dying {
            continue;
        }
        if entity.owner != owner_id {
            continue;
        }
        let type_str = sim.interner.resolve(entity.type_ref);
        let Some(obj) = rules.object(type_str) else {
            continue;
        };
        if !obj.helipad && !obj.unit_reload {
            continue;
        }
        let max_slots = obj.number_of_docks.max(1);
        if !sim
            .production
            .airfield_docks
            .has_free_slot(entity.stable_id, max_slots)
        {
            continue;
        }
        let (fw, fh) = crate::sim::production::foundation_dimensions(&obj.foundation);
        let cx = entity.position.rx + fw / 2;
        let cy = entity.position.ry + fh / 2;
        return Some((entity.stable_id, cx, cy));
    }

    None
}
