//! Bridge layer transitions — resolves ground-to-bridge and bridge-to-ground layer changes
//! during cell boundary crossings, and applies bridge render state for smooth visual transitions.
//!
//! Uses **reactive height-based detection**:
//! - `abs(unit_z - cell.ground_level) >= 2` → unit is at bridge level → stay on bridge
//! - `abs(unit_z - cell.ground_level) < 2` → unit is at ground level → pass under
//! - Ramp entry: `src_z == dst_ground + 4` with bridge flag → going UP onto bridge
//! Path layers are NOT used for bridge state decisions; the unit's Z relative to the
//! cell's ground height determines everything at runtime.
//!
//! TODO(RE): The stock game keeps explicit bridge-layer state on the unit
//! (`FootClass+0x79`) and feeds that into bridge-aware zone lookups. This module still
//! infers bridge state from reactive height heuristics and ignores the pathfinder's
//! `_next_layer` hints. Keep this conservative until the runtime bridge-layer update
//! rules are fully wired in.

use crate::rules::locomotor_type::MovementZone;
use crate::sim::components::{BridgeOccupancy, Position};
use crate::sim::movement::locomotor::{LocomotorState, MovementLayer};
use crate::sim::pathfinding::PathGrid;
use crate::util::fixed_math::SimFixed;

/// Result of the gamemd on_bridge transition predicate at a cell boundary.
///
/// Two independent conditions:
///   Enter: dst.height_level == src.height_level - 4 AND dst has bridge structural flag
///   Exit:  !(dst has bridge structural flag) AND src has bridge structural flag
/// Both conditions are mutually exclusive on retail data but evaluated
/// independently to match the original behavior exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BridgeTransition {
    /// Unit just entered the bridge body deck. Set on_bridge=true, position.z=deck_level.
    Enter { deck_level: u8 },
    /// Unit just exited the bridge structure. Set on_bridge=false, position.z=dst.ground_level.
    Exit,
    /// No layer-state change at this transition.
    NoChange,
}

/// Bridge state update produced by `resolve_cell_transition_bridge_state`.
/// Drives `on_bridge` and `BridgeOccupancy` independently from `loco.layer`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BridgeStateUpdate {
    /// on_bridge = true; bridge_occupancy = Some(BridgeOccupancy { deck_level })
    Set(u8),
    /// on_bridge = false; bridge_occupancy = None
    Clear,
    /// Leave on_bridge and bridge_occupancy unchanged
    Unchanged,
}

/// Threshold for ground vs bridge level detection.
/// If `abs(unit_z - cell.ground_level) >= HEIGHT_THRESHOLD`, unit is at bridge level.
const HEIGHT_THRESHOLD: u8 = 2;

/// Height of one ship Z-step in leptons.
/// Computed as `ftol(sin(30 deg) * 256*sqrt(2) * 0.5) = 90`.
#[allow(dead_code)]
pub(super) const SHIP_HEIGHT_STEP: SimFixed = SimFixed::lit("90");

/// Bridge vertical clearance in leptons.
/// Equals `SHIP_HEIGHT_STEP * 4 = 360` -- the Z distance from water surface to bridge deck.
/// Added to braking distance when a ship passes under a bridge cell.
pub(super) const BRIDGE_Z_OFFSET: SimFixed = SimFixed::lit("360");

/// The on_bridge cell-flag predicate at a cell-boundary crossing.
///
/// Both conditions evaluate independently — they are mutually exclusive on retail data
/// but the audit caught a doc pseudocode bug that incorrectly structured them as if/else.
///
/// Height arithmetic uses signed i8 via wrapping_sub. u8 subtraction would underflow on
/// malformed maps with height ≥ 128; retail maps use 0-15 only, so wrapping is functionally
/// identical in practice.
pub(super) fn compute_bridge_transition(
    src: &crate::sim::pathfinding::PathCell,
    dst: &crate::sim::pathfinding::PathCell,
) -> BridgeTransition {
    let src_h = src.ground_level as i8;
    let dst_h = dst.ground_level as i8;

    let entry = dst_h == src_h.wrapping_sub(4) && dst.bridge_walkable;
    let exit = !dst.bridge_walkable && src.bridge_walkable;

    if entry {
        return BridgeTransition::Enter {
            deck_level: dst.bridge_deck_level_if_any().unwrap_or(dst.ground_level),
        };
    }
    if exit {
        return BridgeTransition::Exit;
    }
    BridgeTransition::NoChange
}

/// Resolve bridge layer state at a cell boundary crossing using reactive height
/// comparison.
///
/// Compares the unit's current Z to the destination cell's ground height to decide
/// ground vs bridge level. The `_next_layer` parameter from path_layers is available
/// but currently unused — see module-level TODO(RE).
pub(super) fn resolve_cell_transition_bridge_state(
    position: &mut Position,
    path_grid: Option<&PathGrid>,
    _next_layer: MovementLayer,
    nx: u16,
    ny: u16,
    _diag_entity_id: u64,
    _diag_source: &str,
) -> (MovementLayer, Option<Option<u8>>) {
    let mut pending_bridge_update: Option<Option<u8>> = None;

    if let Some(grid) = path_grid {
        if let Some(cell) = grid.cell(nx, ny) {
            if let Some(deck_level) = cell.bridge_deck_level_if_any() {
                // Cell has a bridge deck. Use height comparison to decide layer.
                //   abs(height_param - cell.height_level) < 2 -> ground level
                //   else -> bridge level
                let height_diff =
                    (position.z as i16 - cell.ground_level as i16).unsigned_abs() as u8;
                if height_diff >= HEIGHT_THRESHOLD {
                    // Unit is at bridge level → stay on bridge deck.
                    position.z = deck_level;
                    pending_bridge_update = Some(Some(deck_level));
                    return (MovementLayer::Bridge, pending_bridge_update);
                }
            }
            // No bridge deck, or unit is at ground level → ground layer.
            position.z = cell.ground_level;
            pending_bridge_update = Some(None);
            return (MovementLayer::Ground, pending_bridge_update);
        }
    }

    (_next_layer, pending_bridge_update)
}

pub(super) fn apply_pending_bridge_render_state(
    locomotor: &mut Option<LocomotorState>,
    bridge_occupancy: &mut Option<BridgeOccupancy>,
    on_bridge: &mut bool,
    active_layer: MovementLayer,
    pending_bridge_update: Option<Option<u8>>,
    _diag_entity_id: u64,
) {
    if let Some(loco) = locomotor {
        loco.layer = active_layer;
    }
    *on_bridge = active_layer == MovementLayer::Bridge;
    if let Some(bridge_level) = pending_bridge_update {
        match bridge_level {
            Some(level) => {
                *bridge_occupancy = Some(BridgeOccupancy { deck_level: level });
            }
            None => {
                *bridge_occupancy = None;
            }
        }
    }
}

/// Preemptive bridge detection for units approaching a bridge cell.
///
/// Uses height comparison to decide if the unit should be elevated to bridge
/// deck level. Only fires when bridge_occupancy is not already set and the
/// unit's Z indicates it's at bridge level relative to the next cell.
///
/// The planned next-step layer is only used as a conservative gate here: we
/// never pre-claim bridge occupancy unless the path already says the next step
/// is on the bridge layer. Full bridge-state parity is still pending the
/// runtime layer-state RE noted in the module-level TODO(RE).
pub(super) fn apply_bridge_lookahead_if_needed(
    position: &mut Position,
    bridge_occupancy: &mut Option<BridgeOccupancy>,
    on_bridge: &mut bool,
    mover_zone: MovementZone,
    next_step: Option<(u16, u16)>,
    next_step_layer: MovementLayer,
    path_grid: Option<&PathGrid>,
) {
    if mover_zone.is_water_mover()
        || bridge_occupancy.is_some()
        || next_step_layer != MovementLayer::Bridge
    {
        return;
    }

    let Some((nx, ny)) = next_step else {
        return;
    };
    if let Some(pg) = path_grid {
        if let Some(cell) = pg.cell(nx, ny) {
            if let Some(deck) = cell.bridge_deck_level_if_any() {
                // Same height check as resolve: if unit Z is far from ground,
                // it's approaching at bridge level (e.g., coming from a ramp).
                let height_diff =
                    (position.z as i16 - cell.ground_level as i16).unsigned_abs() as u8;
                if height_diff >= HEIGHT_THRESHOLD {
                    *bridge_occupancy = Some(BridgeOccupancy { deck_level: deck });
                    *on_bridge = true;
                    position.z = deck;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::pathfinding::PathCell;

    /// Construct a synthetic PathCell with the bridge fields we care about.
    fn cell(ground_level: u8, bridge_walkable: bool, transition: bool) -> PathCell {
        let bridge_deck_level = if bridge_walkable {
            ground_level.saturating_add(4)
        } else {
            0
        };
        PathCell {
            ground_walkable: true,
            bridge_walkable,
            transition,
            ground_level,
            bridge_deck_level,
        }
    }

    #[test]
    fn entry_from_ramp_to_body() {
        // Ramp at height 4 (bridge_walkable, transition=true) → Body at height 0
        // (bridge_walkable, no transition).
        let src = cell(4, true, true);
        let dst = cell(0, true, false);
        match compute_bridge_transition(&src, &dst) {
            BridgeTransition::Enter { deck_level } => assert_eq!(deck_level, 4),
            other => panic!("expected Enter, got {:?}", other),
        }
    }

    #[test]
    fn exit_from_body_to_ground() {
        // Body at height 0 (bridge_walkable) → Ground at height 0 (NOT bridge_walkable).
        let src = cell(0, true, false);
        let dst = cell(0, false, false);
        assert_eq!(
            compute_bridge_transition(&src, &dst),
            BridgeTransition::Exit
        );
    }

    #[test]
    fn body_to_body_no_change() {
        let src = cell(0, true, false);
        let dst = cell(0, true, false);
        assert_eq!(
            compute_bridge_transition(&src, &dst),
            BridgeTransition::NoChange
        );
    }

    #[test]
    fn ground_to_ground_no_change() {
        let src = cell(0, false, false);
        let dst = cell(0, false, false);
        assert_eq!(
            compute_bridge_transition(&src, &dst),
            BridgeTransition::NoChange
        );
    }

    #[test]
    fn ground_to_bridgehead_no_change() {
        // Going UP onto a ramp is NOT an on_bridge transition: dst is HIGHER than src.
        // src=ground 0, dst=ramp 4. dst_h(4) == src_h(0) - 4 (=-4)? No. Entry doesn't fire.
        // src has no bridge_walkable; exit doesn't fire. NoChange.
        let src = cell(0, false, false);
        let dst = cell(4, true, true);
        assert_eq!(
            compute_bridge_transition(&src, &dst),
            BridgeTransition::NoChange
        );
    }

    #[test]
    fn cliff_drop_off_bridge_ramp() {
        // Edge case: src=ramp (h=4, bridge_walkable, transition),
        // dst=ground at lower elevation (h=0, NOT bridge_walkable). Height-diff matches 4
        // AND exit condition fires (!dst.bridge_walkable && src.bridge_walkable).
        // Exit precedence: predicate produces Exit (since entry needs dst.bridge_walkable=true).
        let src = cell(4, true, true);
        let dst = cell(0, false, false);
        assert_eq!(
            compute_bridge_transition(&src, &dst),
            BridgeTransition::Exit
        );
    }

    #[test]
    fn signed_height_arithmetic() {
        // Verify wrapping_sub handles the i8 boundary. src.ground_level = 4 (as i8 = 4),
        // dst.ground_level = 0 (as i8 = 0). 4.wrapping_sub(4) == 0. Entry should fire.
        let src = cell(4, true, true);
        let dst = cell(0, true, false);
        assert!(matches!(
            compute_bridge_transition(&src, &dst),
            BridgeTransition::Enter { deck_level: 4 }
        ));
    }

    #[test]
    fn entry_without_bridge_walkable_no_change() {
        // Height-diff matches 4 but dst is NOT bridge_walkable. Entry must NOT fire.
        let src = cell(4, false, false);
        let dst = cell(0, false, false);
        assert_eq!(
            compute_bridge_transition(&src, &dst),
            BridgeTransition::NoChange
        );
    }
}
