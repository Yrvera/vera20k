//! Refinery detection, completion-owned FreeUnit spawning, and fallback cell finding.
//!
//! Extracted from production_placement.rs for file-size limits.

use std::collections::BTreeMap;

use crate::map::entities::EntityCategory;
use crate::rules::ruleset::RuleSet;
use crate::sim::find_nearby_cell::{
    NearbyAnchorGate, NearbyFootprint, NearbyQuery, NearbySearchOptions, PassabilityArgs,
    RADIUS_HARD_CAP, find_nearby_passable_cell_with_options,
};
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

/// The two ordered nearby-cell searches gamemd runs once the primary placement is
/// refused. Both calls are argument-for-argument identical except for overlay
/// rejection: the first pass refuses to drop the free unit on a cell carrying an
/// overlay (so it walks out to bare ground instead of standing on the ore field),
/// and only the second pass accepts one.
const FREE_UNIT_FALLBACK_ATTEMPTS: [NearbySearchOptions; 2] = [
    NearbySearchOptions {
        reject_any_overlay: true,
    },
    NearbySearchOptions {
        reject_any_overlay: false,
    },
];

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
    let free_unit_type = free_unit_type.to_owned();
    let refund = rules
        .object(&free_unit_type)
        .map_or(0, |object| object.cost.max(0));

    let primary = primary_free_unit_cell(building_rx, building_ry, width, height);
    // Both nearby searches are seeded from the building's NORTH-WEST footprint cell —
    // not the foundation centre the primary cell is derived from, and not the
    // footprint rectangle.
    let search_seed = (building_rx, building_ry);

    // Native constructs one UnitClass, then retries Unlimbo on that same object.
    // Keep that one stable ID in limbo until a placement commits. The limbo cell is
    // the primary target; it is overwritten by whichever attempt commits, and the
    // object is not on the map until one does.
    let (initial_rx, initial_ry) = primary.unwrap_or(search_seed);
    let initial_z = height_map
        .get(&(initial_rx, initial_ry))
        .copied()
        .unwrap_or(0);
    let Some(free_unit_id) = sim.spawn_object_limbo_at_height(
        &free_unit_type,
        owner,
        initial_rx,
        initial_ry,
        FREE_UNIT_FACING_PRIMARY,
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

    // Exactly two ordered nearby searches, each followed by one placement try. The
    // second attempt runs only when the first produced no cell or its placement was
    // refused, and each search runs here — after the primary has failed — rather
    // than being precomputed, so it sees the occupancy the placement will meet.
    for options in FREE_UNIT_FALLBACK_ATTEMPTS {
        let Some((fallback_rx, fallback_ry)) = find_free_unit_nearby_cell(
            sim,
            rules,
            &free_unit_type,
            search_seed,
            path_grid,
            options,
        ) else {
            continue;
        };
        if try_place_free_unit(
            sim,
            free_unit_id,
            fallback_rx,
            fallback_ry,
            FREE_UNIT_FACING_FALLBACK,
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
    sim.uninit_with_rules(free_unit_id, rules);
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
    height_map: &BTreeMap<(u16, u16), u8>,
) -> bool {
    // gamemd admits the cell only when its cell-entry test returns "clear", and that
    // test grants no exemption to the building that is placing the unit: a refinery
    // sitting on its own bay cell refuses the free unit exactly like any other
    // blocker, which is why the two-attempt nearby search below is the ordinary path
    // rather than the exception.
    //
    // Only the ground-blocker half of that test is modelled here. The rest of the
    // native cell-entry family — the per-cell bib / impassable-row escapes that let a
    // unit stand on a refinery's unload lane, infantry sub-cell occupants, and the
    // terrain/zone clauses — is NOT modelled and is left as a stated residual rather
    // than replaced by a substitute rule.
    let admitted = sim
        .substrate
        .occupancy
        .get(rx, ry)
        .is_none_or(|occupancy| !occupancy.has_blockers_on(MovementLayer::Ground));
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

/// One of the two ordered nearby-cell searches for a refused FreeUnit placement,
/// routed through the shared search port.
///
/// Mirrors the engine's two calls: 1x1 candidates seeded from the building's NW
/// footprint cell, the free unit's own movement zone, bridge cells refused, the
/// terrain-level gate on, no target cell — so selection is the frame-counter modulo
/// over the candidate pool, consuming no RNG. `options` carries the one argument the
/// two calls differ in.
fn find_free_unit_nearby_cell(
    sim: &Simulation,
    rules: &RuleSet,
    free_unit_type: &str,
    seed: (u16, u16),
    path_grid: Option<&PathGrid>,
    options: NearbySearchOptions,
) -> Option<(u16, u16)> {
    let free_unit = rules.object(free_unit_type)?;
    let query = NearbyQuery {
        passability: PassabilityArgs {
            // SUBSTITUTION, not a match. The engine hardcodes one fixed speed-type
            // index (2) at both callsites; VERA passes the free unit's own
            // `SpeedType=` instead. The index-to-`SpeedType=`-string mapping is
            // UNCHECKED, and the tree's own evidence points the other way: index 2 is
            // Wheel under `rules::locomotor_type::SpeedType`'s ordering, while
            // `[CMIN]` and `[HARV]` declare no `SpeedType=` and therefore default to
            // Track. The substitution is invisible to THIS search under the stock
            // terrain tables — Track and Wheel are zero together and non-zero together
            // in all twelve stock land-type sections, and the search reads only the
            // resulting accept/reject verdict, never the cost percentage — but it is
            // not a verified equivalence for a modded table or another caller.
            speed_type: free_unit.speed_type,
            // The engine requires each candidate to share the seed cell's movement
            // zone. The seed is always a building-footprint cell, to which the Rust
            // zone map assigns no zone, so requiring it here would reject every
            // candidate. Left unrequired — a stated residual, not a substitute rule.
            required_zone_id: None,
            movement_zone: free_unit.movement_zone,
            bridge_aware_zone: false,
        },
        footprint: NearbyFootprint::SINGLE,
        anchor_gate: NearbyAnchorGate::UnverifiedCompatibilityBypass,
        // Both callsites reject bridge cells.
        allow_bridge_cells: false,
        // Both callsites enable the terrain-level gate.
        check_height: true,
        // DROPPED native constraint, recorded. The per-candidate occupancy rejection
        // itself is not lost — it lives inside the passability check, which reads the
        // same occupancy grid. What is lost is the rest of that column: the isometric
        // playfield-diamond bound (the terrain rectangle still rejects off-grid
        // cells), the terrain-object block, the slope byte and the ground-building
        // test. Whether the engine sets its separate rect-occupancy argument at these
        // two callsites was not re-derived — UNCHECKED. It stays off because the
        // ported check rejects ANY overlay cell unconditionally, which would cancel
        // the second attempt's overlay-allowed contract above and leave the two calls
        // indistinguishable.
        check_occupancy: false,
        // The engine's cap is `min(map-rect width + height, RADIUS_HARD_CAP)`, which
        // is the hard cap itself on any playable map.
        radius_cap: RADIUS_HARD_CAP,
        target_cell: None,
        path_grid,
        resolved_terrain: sim.resolved_terrain.as_ref(),
        overlay_grid: sim.overlay_grid.as_ref(),
        occupancy: Some(&sim.substrate.occupancy),
        entities: Some(&sim.substrate.entities),
        zone_grid: sim.zone_grid.as_ref(),
        playfield_bounds: sim.playfield_bounds,
    };
    find_nearby_passable_cell_with_options(
        (i32::from(seed.0), i32::from(seed.1)),
        &query,
        options,
        // The frame counter is committed late, so during this advance it still holds
        // the current frame — the value the selection modulo must alias.
        sim.session.binary_frame,
    )
}

#[cfg(test)]
mod tests {
    use super::{FREE_UNIT_FALLBACK_ATTEMPTS, primary_free_unit_cell};

    #[test]
    fn stock_4x3_primary_cell_is_center_plus_south() {
        assert_eq!(primary_free_unit_cell(20, 20, 4, 3), Some((22, 22)));
    }

    #[test]
    fn primary_cell_rejects_u16_overflow() {
        assert_eq!(primary_free_unit_cell(u16::MAX, u16::MAX, 4, 3), None);
    }

    #[test]
    fn fallback_attempts_reject_overlays_first_then_allow_them() {
        // Exactly two attempts, and the only argument that differs between them is
        // overlay rejection — set on the first, clear on the second. Reversing the
        // order would drop the free unit onto the ore field it should walk off.
        assert_eq!(FREE_UNIT_FALLBACK_ATTEMPTS.len(), 2);
        assert!(FREE_UNIT_FALLBACK_ATTEMPTS[0].reject_any_overlay);
        assert!(!FREE_UNIT_FALLBACK_ATTEMPTS[1].reject_any_overlay);
    }
}
