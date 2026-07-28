//! Refinery detection, completion-owned FreeUnit spawning, and fallback cell finding.
//!
//! Extracted from production_placement.rs for file-size limits.

use std::collections::BTreeMap;

use crate::map::entities::EntityCategory;
use crate::rules::ruleset::RuleSet;
use crate::sim::pathfinding::PathGrid;
use crate::sim::world::Simulation;

use super::production_tech::foundation_dimensions;

/// Native primary FreeUnit facing byte. Under the project facing convention,
/// 0xC0 is west.
const FREE_UNIT_FACING_PRIMARY: u8 = 0xC0;
/// Native fallback FreeUnit facing byte. Under the project facing convention,
/// 0xA0 is southwest.
const FREE_UNIT_FACING_FALLBACK: u8 = 0xA0;

/// Spawn configured refinery FreeUnits for buildings whose build-up completed
/// this tick. The input order is the deterministic completion order.
pub(crate) fn spawn_completed_refinery_free_units(
    sim: &mut Simulation,
    completed_building_ids: &[u64],
    rules: &RuleSet,
    path_grid: Option<&PathGrid>,
    height_map: &BTreeMap<(u16, u16), u8>,
) -> bool {
    let mut any_spawned = false;

    for &stable_id in completed_building_ids {
        let Some((owner_id, type_ref, rx, ry, width, height)) = ({
            let entity = sim.substrate.entities.get(stable_id);
            entity.and_then(|entity| {
                if entity.category != EntityCategory::Structure || entity.dying {
                    return None;
                }
                let (width, height) = foundation_dimensions(&entity.foundation);
                Some((
                    entity.owner,
                    entity.type_ref,
                    entity.position.rx,
                    entity.position.ry,
                    width,
                    height,
                ))
            })
        }) else {
            continue;
        };

        // These allocations occur only for completed buildings, not every tick.
        // They end immutable interner borrows before spawn_object mutates sim.
        let owner = sim.interner.resolve(owner_id).to_owned();
        let building_type_id = sim.interner.resolve(type_ref).to_owned();
        any_spawned |= try_spawn_refinery_free_unit(
            sim,
            rules,
            &owner,
            &building_type_id,
            rx,
            ry,
            width,
            height,
            path_grid,
            height_map,
        );
    }

    any_spawned
}

fn try_spawn_refinery_free_unit(
    sim: &mut Simulation,
    rules: &RuleSet,
    owner: &str,
    building_type_id: &str,
    building_rx: u16,
    building_ry: u16,
    width: u16,
    height: u16,
    path_grid: Option<&PathGrid>,
    height_map: &BTreeMap<(u16, u16), u8>,
) -> bool {
    if !rules.is_refinery_type(building_type_id) {
        return false;
    }

    let Some(free_unit_type) = rules.refinery_free_unit(building_type_id) else {
        return false;
    };

    let (rx, ry, facing) = if let Some((primary_rx, primary_ry)) =
        primary_free_unit_cell(building_rx, building_ry, width, height)
    {
        // By completion PathGrid contains the source refinery's own static
        // blocker over this native internal bay, so it cannot reject primary.
        (primary_rx, primary_ry, FREE_UNIT_FACING_PRIMARY)
    } else {
        let Some((fallback_rx, fallback_ry)) =
            find_adjacent_spawn_cell(building_rx, building_ry, width, height, path_grid)
        else {
            log::warn!(
                "No representable cell near completed refinery ({},{}) to spawn {}",
                building_rx,
                building_ry,
                free_unit_type
            );
            return false;
        };
        (fallback_rx, fallback_ry, FREE_UNIT_FACING_FALLBACK)
    };

    let spawned = sim
        .spawn_object(free_unit_type, owner, rx, ry, facing, rules, height_map)
        .is_some();

    if spawned {
        log::info!(
            "Completed refinery {} spawned free {} at ({},{}) for {}",
            building_type_id,
            free_unit_type,
            rx,
            ry,
            owner
        );
    } else {
        log::warn!(
            "Completed refinery {} resolved free unit {} but spawn_object failed at ({},{}) for {}",
            building_type_id,
            free_unit_type,
            rx,
            ry,
            owner
        );
    }

    spawned
}

fn primary_free_unit_cell(
    building_rx: u16,
    building_ry: u16,
    width: u16,
    height: u16,
) -> Option<(u16, u16)> {
    let center_x = u32::from(building_rx).checked_add(u32::from(width) / 2)?;
    let center_y = u32::from(building_ry).checked_add(u32::from(height) / 2)?;
    let primary_y = center_y.checked_add(1)?;
    Some((
        u16::try_from(center_x).ok()?,
        u16::try_from(primary_y).ok()?,
    ))
}

/// Compatibility fallback retained as known drift until the native two-pass
/// nearby-cell option and candidate order are decoded.
fn find_adjacent_spawn_cell(
    cx: u16,
    cy: u16,
    width: u16,
    height: u16,
    path_grid: Option<&PathGrid>,
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

#[cfg(test)]
mod tests {
    use super::primary_free_unit_cell;

    #[test]
    fn stock_4x3_primary_cell_is_center_plus_south() {
        assert_eq!(primary_free_unit_cell(20, 20, 4, 3), Some((22, 22)));
    }

    #[test]
    fn primary_cell_rejects_u16_overflow() {
        assert_eq!(primary_free_unit_cell(u16::MAX, u16::MAX, 4, 3), None);
    }
}
