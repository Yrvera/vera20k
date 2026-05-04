//! Refinery detection, harvester spawning, and ore dropoff cell finding.
//!
//! Extracted from production_placement.rs for file-size limits.

use std::collections::BTreeMap;

use crate::rules::ruleset::RuleSet;
use crate::sim::world::Simulation;

use super::production_tech::foundation_dimensions;

/// Facing for the free harvester when the south-of-foundation spawn cell
/// is walkable. 0xC0 = 192 = south in the 0..255 facing system.
const FREE_UNIT_FACING_PRIMARY: u8 = 0xC0;
/// Fallback facing when the primary cell is blocked and a perimeter cell is
/// chosen instead. 0xA0 = 160 = south-southwest.
const FREE_UNIT_FACING_FALLBACK: u8 = 0xA0;

/// Spawn a free harvester when a refinery is placed (RA2 standard behavior).
pub(super) fn maybe_spawn_refinery_harvester(
    sim: &mut Simulation,
    rules: &RuleSet,
    owner: &str,
    building_type_id: &str,
    building_rx: u16,
    building_ry: u16,
    path_grid: Option<&crate::sim::pathfinding::PathGrid>,
    height_map: &BTreeMap<(u16, u16), u8>,
) {
    if !rules.is_refinery_type(building_type_id) {
        return;
    }

    let Some(harvester_type) = rules.refinery_free_unit(building_type_id) else {
        return;
    };

    let obj = rules.object_case_insensitive(building_type_id);
    let (width, height) = obj
        .map(|o| foundation_dimensions(&o.foundation))
        .unwrap_or((1, 1));

    // Primary spawn cell: one row south of the foundation, on the building's
    // middle column. For a 4x3 refinery at (rx, ry) this lands at (rx+2, ry+3)
    // — directly below the south face of the structure.
    let primary: (u16, u16) = (
        building_rx.saturating_add(width / 2),
        building_ry.saturating_add(height),
    );
    let (rx, ry, facing) = if path_grid.is_none_or(|g| g.is_walkable(primary.0, primary.1)) {
        (primary.0, primary.1, FREE_UNIT_FACING_PRIMARY)
    } else {
        match find_adjacent_spawn_cell(building_rx, building_ry, width, height, path_grid) {
            Some((fx, fy)) => (fx, fy, FREE_UNIT_FACING_FALLBACK),
            None => {
                log::warn!(
                    "No walkable cell near refinery ({},{}) to spawn {}",
                    building_rx,
                    building_ry,
                    harvester_type
                );
                return;
            }
        }
    };
    if sim
        .spawn_object(harvester_type, owner, rx, ry, facing, rules, height_map)
        .is_some()
    {
        log::info!(
            "Refinery {} spawned free {} at ({},{}) for {}",
            building_type_id,
            harvester_type,
            rx,
            ry,
            owner
        );
    } else {
        log::warn!(
            "Refinery {} resolved free unit {} but spawn_object failed at ({},{}) for {}",
            building_type_id,
            harvester_type,
            rx,
            ry,
            owner
        );
    }
}

fn find_adjacent_spawn_cell(
    cx: u16,
    cy: u16,
    width: u16,
    height: u16,
    path_grid: Option<&crate::sim::pathfinding::PathGrid>,
) -> Option<(u16, u16)> {
    let Some(grid) = path_grid else {
        return Some((cx.saturating_add(width), cy.saturating_add(height / 2)));
    };

    let building_max_x = i32::from(cx) + i32::from(width) - 1;
    let building_max_y = i32::from(cy) + i32::from(height) - 1;
    for radius in 1..=5_i32 {
        let min_x = i32::from(cx) - radius;
        let max_x = building_max_x + radius;
        let min_y = i32::from(cy) - radius;
        let max_y = building_max_y + radius;
        for ry in min_y..=max_y {
            for rx in min_x..=max_x {
                let on_perimeter = rx == min_x || rx == max_x || ry == min_y || ry == max_y;
                if !on_perimeter || rx < 0 || ry < 0 {
                    continue;
                }
                let (rx_u16, ry_u16) = (rx as u16, ry as u16);
                if grid.is_walkable(rx_u16, ry_u16) {
                    return Some((rx_u16, ry_u16));
                }
            }
        }
    }
    None
}
