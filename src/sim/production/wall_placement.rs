//! Authoritative regular-wall autofill scanning and stamping.
//!
//! Native provenance: `HouseClass::Place_Production @ 0x004FB0E0` admits and
//! consumes the ready product, `FUN_00588750 @ 0x00588750` scans and commits
//! ordinary fillers, and `OverlayClass::Mark @ 0x005FC570` stamps each overlay.

use crate::map::overlay_types::OverlayTypeRegistry;
use crate::rules::object_type::ObjectType;
use crate::rules::ruleset::RuleSet;
use crate::sim::intern::InternedId;
use crate::sim::overlay_grid::{
    recalc_overlay_passability, refresh_wall_connectivity_after_placement,
};
use crate::sim::pathfinding::PathGrid;
use crate::sim::world::Simulation;

use super::production_placement::can_this_exist_here;

/// Native regular-wall visit order: north, east, south, west.
pub(super) const CARDINAL_DIRECTIONS: [(i32, i32); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];

/// Resolve a BuildingType through its merged ART `ToOverlay=` identity.
pub(super) fn linked_overlay_id(
    object_type: &ObjectType,
    registry: &OverlayTypeRegistry,
) -> Option<u8> {
    let overlay_id = registry.id_for_name(object_type.to_overlay.as_deref()?)?;
    registry
        .flags(overlay_id)
        .is_some_and(|flags| flags.wall)
        .then_some(overlay_id)
}

/// Scan one regular-wall direction.
///
/// `FUN_00588750 @ 0x00588750` checks a same-ToOverlay, same-owner endpoint
/// before asking whether the visited cell can accept a filler. Any blocker
/// discards the whole direction; a found endpoint returns the gap in
/// nearest-to-click order. Rust stores GuardRange in I16F16 cells, so integer
/// conversion is the equivalent of native's signed fixed-point shift by 8.
#[allow(clippy::too_many_arguments)]
pub(super) fn scan_autofill_direction(
    sim: &Simulation,
    rules: &RuleSet,
    object_type: &ObjectType,
    path_grid: Option<&PathGrid>,
    origin: (u16, u16),
    owner: InternedId,
    overlay_id: u8,
    direction: (i32, i32),
) -> Vec<(u16, u16)> {
    let limit = object_type
        .guard_range
        .map(|range| range.to_num::<i32>())
        .unwrap_or(0)
        .max(0) as usize;
    if limit == 0 {
        return Vec::new();
    }
    let Some(grid) = sim.overlay_grid.as_ref() else {
        return Vec::new();
    };
    let (width, height) = (i32::from(grid.width()), i32::from(grid.height()));
    let (mut cx, mut cy) = (
        i32::from(origin.0) + direction.0,
        i32::from(origin.1) + direction.1,
    );
    let mut gap = Vec::with_capacity(limit.saturating_sub(1));

    while gap.len() < limit {
        if cx < 0 || cy < 0 || cx >= width || cy >= height {
            return Vec::new();
        }
        let cell_coord = (cx as u16, cy as u16);
        let cell = *grid.cell(cell_coord.0, cell_coord.1);
        if cell.overlay_id == Some(overlay_id) && cell.wall_owner == Some(owner) {
            return gap;
        }
        if !can_this_exist_here(
            sim,
            &sim.substrate.entities,
            rules,
            object_type,
            path_grid,
            cell_coord.0,
            cell_coord.1,
        ) {
            return Vec::new();
        }
        gap.push(cell_coord);
        cx += direction.0;
        cy += direction.1;
    }
    Vec::new()
}

#[allow(clippy::too_many_arguments)]
pub(super) fn autofill_cells(
    sim: &Simulation,
    rules: &RuleSet,
    object_type: &ObjectType,
    path_grid: Option<&PathGrid>,
    origin: (u16, u16),
    owner: InternedId,
    overlay_id: u8,
) -> Vec<(u16, u16)> {
    let mut cells = Vec::new();
    for direction in CARDINAL_DIRECTIONS {
        cells.extend(scan_autofill_direction(
            sim,
            rules,
            object_type,
            path_grid,
            origin,
            owner,
            overlay_id,
            direction,
        ));
    }
    cells
}

/// Stamp one wall cell and synchronously publish its passability projection.
pub(super) fn stamp_wall(
    sim: &mut Simulation,
    registry: &OverlayTypeRegistry,
    rx: u16,
    ry: u16,
    overlay_id: u8,
    owner: InternedId,
) -> bool {
    let Some(grid) = sim.overlay_grid.as_mut() else {
        return false;
    };
    if rx >= grid.width() || ry >= grid.height() {
        return false;
    }
    grid.place_owned_wall(rx, ry, overlay_id, 0, owner);
    refresh_wall_connectivity_after_placement(grid, registry, rx, ry);
    if let Some(terrain) = sim.resolved_terrain.as_mut() {
        const PLACEMENT_CROSS: [(i32, i32); 5] =
            [(0, 0), (0, -1), (1, 0), (0, 1), (-1, 0)];
        for (dx, dy) in PLACEMENT_CROSS {
            let nx = i32::from(rx) + dx;
            let ny = i32::from(ry) + dy;
            if nx < 0
                || ny < 0
                || nx >= i32::from(grid.width())
                || ny >= i32::from(grid.height())
            {
                continue;
            }
            let (nx, ny) = (nx as u16, ny as u16);
            let changed = recalc_overlay_passability(grid, terrain, registry, nx, ny);
            grid.record_synchronous_passability_change_at(nx, ny, changed);
        }
    }
    true
}
