//! Refinery detection, completion-owned FreeUnit spawning, and fallback cell finding.
//!
//! Extracted from production_placement.rs for file-size limits.

use std::collections::BTreeMap;

use crate::map::entities::EntityCategory;
use crate::rules::ruleset::RuleSet;
use crate::sim::movement::locomotor::MovementLayer;
use crate::sim::pathfinding::PathGrid;
use crate::sim::world::{
    PlacementEvidence, RevealOutcome, RevealPosition, RevealRequest, Simulation,
};

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
            stable_id,
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
    source_refinery_id: u64,
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
    let free_unit_type = free_unit_type.to_owned();
    let refund = rules
        .object(&free_unit_type)
        .map_or(0, |object| object.cost.max(0));

    let primary = primary_free_unit_cell(building_rx, building_ry, width, height);
    let fallbacks =
        find_compatibility_fallback_cells(building_rx, building_ry, width, height, path_grid);
    let initial = primary
        .or_else(|| fallbacks.first().copied())
        .map(|(rx, ry)| {
            let facing = if primary == Some((rx, ry)) {
                FREE_UNIT_FACING_PRIMARY
            } else {
                FREE_UNIT_FACING_FALLBACK
            };
            (rx, ry, facing)
        });
    let Some((initial_rx, initial_ry, initial_facing)) = initial else {
        log::warn!(
            "No representable cell near completed refinery ({},{}) to construct {}",
            building_rx,
            building_ry,
            free_unit_type
        );
        return false;
    };

    // Native constructs one UnitClass, then retries Unlimbo on that same object.
    // Keep that one stable ID in limbo until a placement commits.
    let initial_z = height_map
        .get(&(initial_rx, initial_ry))
        .copied()
        .unwrap_or(0);
    let Some(free_unit_id) = sim.spawn_object_limbo_at_height(
        &free_unit_type,
        owner,
        initial_rx,
        initial_ry,
        initial_facing,
        initial_z,
        rules,
    ) else {
        refund_failed_free_unit(sim, owner, refund);
        log::warn!(
            "Completed refinery {} could not construct free unit {}; refunded {} to {}",
            building_type_id,
            free_unit_type,
            refund,
            owner,
        );
        return false;
    };

    if let Some((primary_rx, primary_ry)) = primary
        && try_place_free_unit(
            sim,
            free_unit_id,
            primary_rx,
            primary_ry,
            FREE_UNIT_FACING_PRIMARY,
            Some(source_refinery_id),
            height_map,
        )
    {
        log::info!(
            "Completed refinery {} spawned free {} at ({},{}) for {}",
            building_type_id,
            free_unit_type,
            primary_rx,
            primary_ry,
            owner
        );
        return true;
    }

    // The native function performs exactly two ordered nearby searches. The
    // exact differing FNPC option remains undecoded, so these candidates retain
    // the existing deterministic compatibility order without claiming exact
    // cell-selection parity.
    for (fallback_rx, fallback_ry) in fallbacks.into_iter().take(2) {
        if try_place_free_unit(
            sim,
            free_unit_id,
            fallback_rx,
            fallback_ry,
            FREE_UNIT_FACING_FALLBACK,
            None,
            height_map,
        ) {
            log::info!(
                "Completed refinery {} spawned free {} at fallback ({},{}) for {}",
                building_type_id,
                free_unit_type,
                fallback_rx,
                fallback_ry,
                owner
            );
            return true;
        }
    }

    // gamemd refunds before uninitializing the constructed UnitClass.
    refund_failed_free_unit(sim, owner, refund);
    sim.uninit(free_unit_id);
    log::warn!(
        "Completed refinery {} could not place free unit {}; refunded {} to {}",
        building_type_id,
        free_unit_type,
        refund,
        owner,
    );
    false
}

fn try_place_free_unit(
    sim: &mut Simulation,
    free_unit_id: u64,
    rx: u16,
    ry: u16,
    facing: u8,
    allowed_ground_occupant: Option<u64>,
    height_map: &BTreeMap<(u16, u16), u8>,
) -> bool {
    let admitted = sim.substrate.occupancy.get(rx, ry).is_none_or(|occupancy| {
        occupancy
            .blockers(MovementLayer::Ground)
            .all(|occupant_id| Some(occupant_id) == allowed_ground_occupant)
    });
    let Some((sub_x, sub_y)) = sim.substrate.entities.get_mut(free_unit_id).map(|entity| {
        entity.facing = facing;
        (entity.position.sub_x, entity.position.sub_y)
    }) else {
        return false;
    };
    let outcome = sim.try_reveal_entity(
        free_unit_id,
        RevealRequest {
            position: RevealPosition {
                rx,
                ry,
                z: height_map.get(&(rx, ry)).copied().unwrap_or(0),
                sub_x,
                sub_y,
            },
            placement: if admitted {
                PlacementEvidence::MarkSucceeded
            } else {
                PlacementEvidence::MarkFailed
            },
            logic_eligible: true,
        },
    );
    matches!(outcome, RevealOutcome::Revealed { .. })
}

fn refund_failed_free_unit(sim: &mut Simulation, owner: &str, refund: i32) {
    let credits = super::credits_entry_for_owner(sim, owner);
    *credits = credits.saturating_add(refund.max(0));
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

/// Return at most two distinct compatibility candidates in the existing
/// deterministic perimeter order. Native also performs two attempts, but its
/// differing FNPC option and exact returned cells remain an explicit residual.
fn find_compatibility_fallback_cells(
    cx: u16,
    cy: u16,
    width: u16,
    height: u16,
    path_grid: Option<&PathGrid>,
) -> Vec<(u16, u16)> {
    let Some(grid) = path_grid else {
        let Some(rx) = cx.checked_add(width) else {
            return Vec::new();
        };
        let Some(first_ry) = cy.checked_add(height / 2) else {
            return Vec::new();
        };
        let mut cells = vec![(rx, first_ry)];
        if let Some(second_ry) = first_ry.checked_add(1) {
            cells.push((rx, second_ry));
        }
        return cells;
    };

    let mut candidates = Vec::with_capacity(2);
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
                if !on_perimeter {
                    continue;
                }
                let (Ok(rx_u16), Ok(ry_u16)) = (u16::try_from(rx), u16::try_from(ry)) else {
                    continue;
                };
                if grid.is_walkable(rx_u16, ry_u16) {
                    candidates.push((rx_u16, ry_u16));
                    if candidates.len() == 2 {
                        return candidates;
                    }
                }
            }
        }
    }
    candidates
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
