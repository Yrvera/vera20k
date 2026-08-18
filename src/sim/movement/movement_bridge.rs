//! Bridge layer transitions — applies the on_bridge cell-flag predicate at each
//! cell boundary crossing, and decouples `on_bridge` from `loco.layer` so that the
//! A* path layer (walkability-driving) and the runtime bridge state (predicate-driven)
//! can disagree at ramp cells — which they must, to match the reference behavior.
//!
//! Predicate:
//!   Enter:  dst.height_level == src.height_level - 4 AND dst has bridge structural flag
//!   Exit:   !(dst has bridge structural flag) AND src has bridge structural flag
//! Both conditions independent; signed i8 height arithmetic.
//!
//! See docs/plans/2026-05-11-bridge-locomotor-layer-correctness-design.md.

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

/// Persisted analogue of the runtime Foot bridge-state mismatch byte.
///
/// The owner integration stores this alongside locomotor runtime state. It is
/// intentionally independent from A* bridge-layer selection and from the
/// `on_bridge` occupancy byte.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct RuntimeBridgeTransitionState {
    /// Set when candidate bridge bit and current bridge state differ.
    pub pending_mismatch: bool,
}

/// Result supplied by the runtime-only bridge policy seam.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RuntimeBridgePolicyResult {
    Continue,
    Reject,
}

/// Evaluate the runtime bridge mismatch branch without borrowing A* policy.
///
/// Original: each locomotor's `Process_Movement` compares
/// candidate `CellClass+0x140 & 0x100` with Foot `+0x8C`, sets `+0x68B` on a
/// mismatch, then calls virtual `+0x29C` to select the local return path.
///
/// The comparison lives in each locomotor's own `Process_Movement`, not in a
/// shared helper: `DriveLocomotionClass` @ `0x004B3376`-`0x004B339D`
/// (`EDX = (cell->Flags >> 8) & 1`; `AL = [foot+0x8C]`; `XOR EDX,EAX; JZ`;
/// `MOV byte [ECX+0x68B],1`; `CALL [vtable+0x29C]`), with twins at
/// `0x0075B662` (Walk), `0x00515513` (Hover), `0x006A29E0` and `0x006A3C19`
/// (Ship) and `0x00736038` (Tube). There is no
/// `FootClass::EvaluateCellEnterabilityOrCost` in this program.
///
/// **VERA-internal, gamemd equivalent UNCHECKED:** `pending_mismatch` is
/// written on every call, so a matching tick clears it. Native `[foot+0x68B]`
/// is write-1-only — initialised once in `FootClass::Constructor` @
/// `0x004D33BA`, set at eleven locomotor sites, never cleared, and read by
/// `FootClass::ComputeChecksum` @ `0x004DBD0C`, so it is part of the sync
/// checksum. Trigger: any tick where the candidate's bridge bit matches the
/// mover's. Player effect: none today — the field has no reader outside tests
/// and the only production caller passes a policy that never rejects.
/// Frequency: zero while that holds. Downstream risk: a flip to authoritative
/// would start from the wrong latch semantics and a wrong checksum input.
pub(crate) fn evaluate_runtime_bridge_transition(
    state: &mut RuntimeBridgeTransitionState,
    candidate_bridge_bit: bool,
    current_bridge_state: bool,
    policy: impl FnOnce() -> RuntimeBridgePolicyResult,
) -> RuntimeBridgePolicyResult {
    let mismatch = candidate_bridge_bit != current_bridge_state;
    state.pending_mismatch = mismatch;
    if mismatch {
        policy()
    } else {
        RuntimeBridgePolicyResult::Continue
    }
}

/// Read-only runtime bridge row for oracle diagnostics.
#[cfg(test)]
#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub struct BridgeRuntimeOracleTick {
    pub tick: u64,
    pub current_cell: (u16, u16),
    pub next_cell: (u16, u16),
    pub loco_layer_before: MovementLayer,
    pub next_path_layer: MovementLayer,
    pub on_bridge_before: bool,
    pub on_bridge_after: bool,
    pub bridge_update: String,
    pub bridge_occupancy_before: Option<u8>,
    pub bridge_occupancy_after: Option<u8>,
    pub visible_z_after: u8,
}

pub(super) fn projected_on_bridge(current: bool, update: BridgeStateUpdate) -> bool {
    match update {
        BridgeStateUpdate::Set(_) => true,
        BridgeStateUpdate::Clear => false,
        BridgeStateUpdate::Unchanged => current,
    }
}

/// Bridge vertical clearance in leptons.
/// 416 == 104 * 4 — the verified Foot-role Z distance from water surface to bridge deck.
/// Added to braking distance when a ship passes under a bridge cell.
/// Same physical fact as `sim::map::bridge_topology::BRIDGE_DECK_HEIGHT_LEPTONS`
/// (a `SimFixed` literal cannot be const-derived from it; the equality is
/// pinned by `bridge_z_offset_matches_deck_height` below).
pub(super) const BRIDGE_Z_OFFSET: SimFixed = SimFixed::lit("416");

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

    let entry = dst_h == src_h.wrapping_sub(4) && dst.has_structural_bridge();
    let exit = !dst.has_structural_bridge() && src.has_structural_bridge();

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

/// Apply the on_bridge cell-flag predicate at a cell-boundary crossing.
///
/// Reads src and dst PathCells and computes the BridgeStateUpdate using
/// `compute_bridge_transition`. Writes `position.z` to the post-transition height
/// (deck_level on Enter, dst.ground_level on Exit, or next_layer's effective height
/// on NoChange).
///
/// Does NOT return a layer — the caller continues to use `next_layer` from A*'s
/// `path_layers` for `loco.layer`. The predicate's role is independent: it drives
/// `on_bridge` and `BridgeOccupancy` via the returned `BridgeStateUpdate`.
///
/// Fallback: returns `Unchanged` (no position.z modification) when `path_grid` is
/// `None` or either cell lookup is out-of-bounds. Out-of-bounds at the boundary
/// crossing indicates a path-data bug elsewhere; the resolver is not the recovery point.
pub(super) fn resolve_cell_transition_bridge_state(
    position: &mut Position,
    path_grid: Option<&PathGrid>,
    src: (u16, u16),
    dst: (u16, u16),
    next_layer: MovementLayer,
) -> BridgeStateUpdate {
    let Some(grid) = path_grid else {
        return BridgeStateUpdate::Unchanged;
    };
    let (Some(src_cell), Some(dst_cell)) = (grid.cell(src.0, src.1), grid.cell(dst.0, dst.1))
    else {
        return BridgeStateUpdate::Unchanged;
    };

    match compute_bridge_transition(src_cell, dst_cell) {
        BridgeTransition::Enter { deck_level } => {
            position.z = deck_level;
            position.exact_z_leptons = None;
            BridgeStateUpdate::Set(deck_level)
        }
        BridgeTransition::Exit => {
            position.z = dst_cell.ground_level;
            position.exact_z_leptons = None;
            BridgeStateUpdate::Clear
        }
        BridgeTransition::NoChange => {
            position.z = dst_cell.effective_cell_z_for_layer(next_layer);
            position.exact_z_leptons = None;
            BridgeStateUpdate::Unchanged
        }
    }
}

/// Diagnostic wrapper for a single boundary crossing. It computes the same
/// bridge update as `resolve_cell_transition_bridge_state` and returns an
/// oracle row; callers opt in explicitly from tests/tools.
#[cfg(test)]
pub(super) fn resolve_cell_transition_bridge_state_oracle(
    position: &mut Position,
    path_grid: Option<&PathGrid>,
    src: (u16, u16),
    dst: (u16, u16),
    current_layer: MovementLayer,
    next_layer: MovementLayer,
    on_bridge_before: bool,
    bridge_occupancy_before: Option<BridgeOccupancy>,
    tick: u64,
) -> (BridgeStateUpdate, BridgeRuntimeOracleTick) {
    let update = resolve_cell_transition_bridge_state(position, path_grid, src, dst, next_layer);
    let on_bridge_after = projected_on_bridge(on_bridge_before, update);
    let bridge_occupancy_after = match update {
        BridgeStateUpdate::Set(deck_level) => Some(BridgeOccupancy { deck_level }),
        BridgeStateUpdate::Clear => None,
        BridgeStateUpdate::Unchanged => bridge_occupancy_before,
    };
    let row = BridgeRuntimeOracleTick {
        tick,
        current_cell: src,
        next_cell: dst,
        loco_layer_before: current_layer,
        next_path_layer: next_layer,
        on_bridge_before,
        on_bridge_after,
        bridge_update: format!("{:?}", update),
        bridge_occupancy_before: bridge_occupancy_before.map(|occ| occ.deck_level),
        bridge_occupancy_after: bridge_occupancy_after.map(|occ| occ.deck_level),
        visible_z_after: position.z,
    };
    (update, row)
}

/// Apply the post-resolver bridge state to entity components.
///
/// `loco.layer` follows `active_layer` (= A*'s path_layer for this step), which drives
/// walkability and cell_entry occupancy lookup.
///
/// `on_bridge` and `bridge_occupancy` are driven INDEPENDENTLY by `bridge_update` from
/// the cell-flag predicate. This is the load-bearing G2 parity fix: the runtime
/// on_bridge state is NOT derivable from the A* layer, because on a ramp going up
/// loco.layer=Bridge but on_bridge=false (predicate hasn't fired Enter yet), and on a
/// ramp going down loco.layer=Ground but on_bridge=true.
pub(super) fn apply_pending_bridge_render_state(
    locomotor: &mut Option<LocomotorState>,
    bridge_occupancy: &mut Option<BridgeOccupancy>,
    on_bridge: &mut bool,
    active_layer: MovementLayer,
    bridge_update: BridgeStateUpdate,
    _diag_entity_id: u64,
) {
    if let Some(loco) = locomotor {
        loco.layer = active_layer;
    }
    match bridge_update {
        BridgeStateUpdate::Set(deck_level) => {
            *on_bridge = true;
            *bridge_occupancy = Some(BridgeOccupancy { deck_level });
        }
        BridgeStateUpdate::Clear => {
            *on_bridge = false;
            *bridge_occupancy = None;
        }
        BridgeStateUpdate::Unchanged => {
            // on_bridge and bridge_occupancy retain their previous values
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::movement::locomotion::LocomotorSlot;
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
            bridge_structural: bridge_walkable && !transition,
            bridge_marker_0x80: false,
            transition,
            ground_level,
            bridge_deck_level,
            slope_type: 0,
            tube_index: None,
            low_bridge_tube_cell: false,
        }
    }

    #[test]
    fn gsi_04_03b_water_mover_bridge_clearance_crosses_braking_boundary() {
        let planar_distance = SimFixed::from_num(100);
        let slowdown_distance = SimFixed::from_num(500);
        let distance_under_bridge = planar_distance + BRIDGE_Z_OFFSET;

        assert_eq!(distance_under_bridge, SimFixed::from_num(516));
        assert!(
            !(distance_under_bridge < slowdown_distance),
            "the verified 416-lepton deck offset must keep this mover outside braking range"
        );
        assert!(
            planar_distance + SimFixed::from_num(360) < slowdown_distance,
            "the stale 360-lepton offset would incorrectly begin braking at this boundary"
        );
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
        // dst=ground at lower elevation (h=0, NOT bridge_walkable). Ramps are bridge-layer
        // pathing cells but are not structural bridge body cells, so the on_bridge predicate
        // does not fire Exit here.
        let src = cell(4, true, true);
        let dst = cell(0, false, false);
        assert_eq!(
            compute_bridge_transition(&src, &dst),
            BridgeTransition::NoChange
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
    fn projected_on_bridge_applies_pending_update_without_mutation() {
        assert!(projected_on_bridge(false, BridgeStateUpdate::Set(4)));
        assert!(!projected_on_bridge(true, BridgeStateUpdate::Clear));
        assert!(projected_on_bridge(true, BridgeStateUpdate::Unchanged));
        assert!(!projected_on_bridge(false, BridgeStateUpdate::Unchanged));
    }

    #[test]
    fn runtime_bridge_mismatch_sets_pending_state_and_uses_policy_seam() {
        let mut state = RuntimeBridgeTransitionState::default();
        let outcome = evaluate_runtime_bridge_transition(&mut state, true, false, || {
            RuntimeBridgePolicyResult::Reject
        });

        assert!(state.pending_mismatch);
        assert_eq!(outcome, RuntimeBridgePolicyResult::Reject);
    }

    #[test]
    fn matching_runtime_bridge_state_skips_policy_seam() {
        let mut state = RuntimeBridgeTransitionState::default();
        let mut policy_called = false;
        let outcome = evaluate_runtime_bridge_transition(&mut state, true, true, || {
            policy_called = true;
            RuntimeBridgePolicyResult::Reject
        });

        assert!(!state.pending_mismatch);
        assert!(!policy_called);
        assert_eq!(outcome, RuntimeBridgePolicyResult::Continue);
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

    // ------------------------------------------------------------------------
    // Resolver tests (resolve_cell_transition_bridge_state)
    // ------------------------------------------------------------------------

    use crate::sim::components::Position;
    use crate::sim::pathfinding::PathGrid;
    use crate::util::fixed_math::SimFixed;

    fn make_grid_with_cells(cells: &[(u16, u16, u8, bool, bool)]) -> PathGrid {
        let mut g = PathGrid::new(16, 16);
        for &(x, y, ground_level, bridge_walkable, transition) in cells {
            g.set_cell_for_test(x, y, ground_level, bridge_walkable, transition);
        }
        g
    }

    fn pos_at(rx: u16, ry: u16, z: u8) -> Position {
        Position {
            rx,
            ry,
            z,
            exact_z_leptons: None,
            sub_x: SimFixed::ZERO,
            sub_y: SimFixed::ZERO,
            // screen_x/screen_y are #[serde(skip, default)] but Position has no
            // Default impl, so we must initialize them explicitly in struct literals.
        }
    }

    #[test]
    fn resolver_fallback_when_path_grid_none() {
        let mut p = pos_at(5, 5, 10);
        let update = resolve_cell_transition_bridge_state(
            &mut p,
            None,
            (5, 5),
            (6, 5),
            MovementLayer::Ground,
        );
        assert_eq!(update, BridgeStateUpdate::Unchanged);
        assert_eq!(p.z, 10, "position.z must be untouched on Unchanged");
    }

    #[test]
    fn resolver_fallback_when_cell_out_of_bounds() {
        let g = make_grid_with_cells(&[]);
        let mut p = pos_at(0, 0, 10);
        // src in bounds, dst out of bounds:
        let update = resolve_cell_transition_bridge_state(
            &mut p,
            Some(&g),
            (0, 0),
            (999, 999),
            MovementLayer::Ground,
        );
        assert_eq!(update, BridgeStateUpdate::Unchanged);
        assert_eq!(p.z, 10);
    }

    #[test]
    fn resolver_enter_writes_deck_level_and_set() {
        // src=ramp at h=4, dst=body at h=0 with bridge_walkable
        let g = make_grid_with_cells(&[
            (5, 5, 4, true, true),  // ramp
            (6, 5, 0, true, false), // body
        ]);
        let mut p = pos_at(6, 5, 4);
        let update = resolve_cell_transition_bridge_state(
            &mut p,
            Some(&g),
            (5, 5),
            (6, 5),
            MovementLayer::Bridge,
        );
        assert_eq!(update, BridgeStateUpdate::Set(4));
        assert_eq!(p.z, 4, "position.z must equal deck_level on Enter");
    }

    #[test]
    fn resolver_oracle_reports_runtime_bridge_split_fields() {
        let g = make_grid_with_cells(&[(5, 5, 4, true, true), (6, 5, 0, true, false)]);
        let mut p = pos_at(6, 5, 0);
        let (update, row) = resolve_cell_transition_bridge_state_oracle(
            &mut p,
            Some(&g),
            (5, 5),
            (6, 5),
            MovementLayer::Ground,
            MovementLayer::Bridge,
            false,
            None,
            123,
        );

        assert_eq!(update, BridgeStateUpdate::Set(4));
        assert_eq!(row.tick, 123);
        assert_eq!(row.next_path_layer, MovementLayer::Bridge);
        assert!(row.on_bridge_after);
        assert_eq!(row.bridge_occupancy_after, Some(4));
        assert_eq!(row.visible_z_after, 4);
    }

    #[test]
    fn resolver_exit_writes_ground_level_and_clear() {
        // src=body at h=0 bridge_walkable, dst=ground at h=0 NOT bridge_walkable
        let g = make_grid_with_cells(&[(5, 5, 0, true, false), (6, 5, 0, false, false)]);
        let mut p = pos_at(6, 5, 4);
        let update = resolve_cell_transition_bridge_state(
            &mut p,
            Some(&g),
            (5, 5),
            (6, 5),
            MovementLayer::Ground,
        );
        assert_eq!(update, BridgeStateUpdate::Clear);
        assert_eq!(p.z, 0, "position.z must equal dst.ground_level on Exit");
    }

    #[test]
    fn resolver_no_change_with_next_layer_bridge_uses_deck() {
        // Body-to-body. NoChange. next_layer=Bridge → position.z = dst.bridge_deck_level (4).
        let g = make_grid_with_cells(&[(5, 5, 0, true, false), (6, 5, 0, true, false)]);
        let mut p = pos_at(6, 5, 0);
        let update = resolve_cell_transition_bridge_state(
            &mut p,
            Some(&g),
            (5, 5),
            (6, 5),
            MovementLayer::Bridge,
        );
        assert_eq!(update, BridgeStateUpdate::Unchanged);
        assert_eq!(
            p.z, 4,
            "NoChange with next_layer=Bridge must use bridge_deck_level"
        );
    }

    #[test]
    fn resolver_no_change_with_next_layer_ground_uses_ground() {
        // Ground-to-ground. NoChange. next_layer=Ground → position.z = dst.ground_level (0).
        let g = make_grid_with_cells(&[(5, 5, 0, false, false), (6, 5, 0, false, false)]);
        let mut p = pos_at(6, 5, 0);
        let update = resolve_cell_transition_bridge_state(
            &mut p,
            Some(&g),
            (5, 5),
            (6, 5),
            MovementLayer::Ground,
        );
        assert_eq!(update, BridgeStateUpdate::Unchanged);
        assert_eq!(p.z, 0);
    }

    // ------------------------------------------------------------------------
    // Render-state apply tests (apply_pending_bridge_render_state)
    // ------------------------------------------------------------------------

    use crate::rules::locomotor_type::{LocomotorKind, MovementZone, SpeedType};
    use crate::sim::movement::locomotor::{AirMovePhase, GroundMovePhase};
    use crate::util::fixed_math::{SIM_ONE, SIM_ZERO};

    /// Build a minimal `LocomotorState` for tests. Lists all fields explicitly
    /// with sensible defaults — LocomotorState has no `Default` impl.
    fn make_loco(layer: MovementLayer) -> Option<LocomotorState> {
        Some(LocomotorState {
            kind: LocomotorKind::Drive,
            slot: LocomotorSlot::from_kind(LocomotorKind::Drive),
            powered: true,
            piggyback: None,
            runtime_payload: crate::sim::movement::locomotion::LocomotorRuntimePayload::for_kind(
                LocomotorKind::Drive,
            ),
            layer,
            phase: GroundMovePhase::Idle,
            air_phase: AirMovePhase::Landed,
            speed_multiplier: SIM_ONE,
            speed_fraction: SIM_ONE,
            fly_current_speed: SIM_ZERO,
            altitude: SIM_ZERO,
            target_altitude: SIM_ZERO,
            climb_rate: SIM_ZERO,
            jumpjet_speed: SIM_ZERO,
            jumpjet_accel: SIM_ZERO,
            jumpjet_current_speed: SIM_ZERO,
            jumpjet_deviation: 0,
            jumpjet_crash_speed: SIM_ZERO,
            jumpjet_turn_rate: 4,
            balloon_hover: false,
            hover_attack: false,
            speed_type: SpeedType::Track,
            movement_zone: MovementZone::Normal,
            rot: 0,
            air_progress: SIM_ZERO,
            infantry_wobble_phase: 0.0,
            subcell_dest: None,
            hover_throttle: crate::util::fixed_math::SIM_ZERO,
            hover_speed_request: crate::util::fixed_math::SIM_ZERO,
            hover_bob_offset: crate::util::fixed_math::SIM_ZERO,
        })
    }

    #[test]
    fn render_state_on_bridge_decoupled_from_loco_layer() {
        // active_layer=Bridge but bridge_update=Unchanged.
        // on_bridge must retain its prior value (does NOT become true just because layer is Bridge).
        let mut loco = make_loco(MovementLayer::Ground);
        let mut occ: Option<BridgeOccupancy> = None;
        let mut on_b = false;
        apply_pending_bridge_render_state(
            &mut loco,
            &mut occ,
            &mut on_b,
            MovementLayer::Bridge,
            BridgeStateUpdate::Unchanged,
            42,
        );
        assert_eq!(loco.as_ref().unwrap().layer, MovementLayer::Bridge);
        assert!(!on_b, "on_bridge must NOT be derived from active_layer");
        assert!(occ.is_none(), "bridge_occupancy must be unchanged");
    }

    #[test]
    fn render_state_ramp_going_up_keeps_on_bridge_false() {
        // Going up onto a ramp: A*'s path puts the ramp on Bridge layer, but the
        // predicate doesn't fire Enter until Ramp→Body next tick. So this tick:
        //   active_layer = Bridge, bridge_update = Unchanged, on_bridge = false (prior).
        let mut loco = make_loco(MovementLayer::Ground);
        let mut occ: Option<BridgeOccupancy> = None;
        let mut on_b = false;
        apply_pending_bridge_render_state(
            &mut loco,
            &mut occ,
            &mut on_b,
            MovementLayer::Bridge,
            BridgeStateUpdate::Unchanged,
            42,
        );
        assert_eq!(loco.as_ref().unwrap().layer, MovementLayer::Bridge);
        assert!(!on_b, "on_bridge must stay false on the ramp tick going up");
    }

    #[test]
    fn render_state_ramp_going_down_keeps_on_bridge_true() {
        // Coming off a bridge: A*'s path puts the ramp on Ground layer (is_at_bridge_level
        // returns false), but the predicate hasn't fired Exit yet. on_bridge stays true.
        let mut loco = make_loco(MovementLayer::Bridge);
        let mut occ = Some(BridgeOccupancy { deck_level: 4 });
        let mut on_b = true;
        apply_pending_bridge_render_state(
            &mut loco,
            &mut occ,
            &mut on_b,
            MovementLayer::Ground,
            BridgeStateUpdate::Unchanged,
            42,
        );
        assert_eq!(loco.as_ref().unwrap().layer, MovementLayer::Ground);
        assert!(on_b, "on_bridge must stay true on the ramp tick going down");
        assert!(
            occ.is_some(),
            "bridge_occupancy must be unchanged on Unchanged"
        );
    }

    #[test]
    fn render_state_set_writes_occupancy() {
        let mut loco = make_loco(MovementLayer::Bridge);
        let mut occ: Option<BridgeOccupancy> = None;
        let mut on_b = false;
        apply_pending_bridge_render_state(
            &mut loco,
            &mut occ,
            &mut on_b,
            MovementLayer::Bridge,
            BridgeStateUpdate::Set(4),
            42,
        );
        assert!(on_b);
        assert_eq!(occ.unwrap().deck_level, 4);
    }

    #[test]
    fn render_state_clear_drops_occupancy() {
        let mut loco = make_loco(MovementLayer::Ground);
        let mut occ = Some(BridgeOccupancy { deck_level: 4 });
        let mut on_b = true;
        apply_pending_bridge_render_state(
            &mut loco,
            &mut occ,
            &mut on_b,
            MovementLayer::Ground,
            BridgeStateUpdate::Clear,
            42,
        );
        assert!(!on_b);
        assert!(occ.is_none());
    }
}

#[cfg(test)]
mod bridge_constant_tests {
    use super::*;

    /// Ship braking clearance and the deck snap height are the same physical
    /// fact; if `LEPTONS_PER_LEVEL` is ever corrected, both must move.
    #[test]
    fn bridge_z_offset_matches_deck_height() {
        assert_eq!(
            BRIDGE_Z_OFFSET,
            SimFixed::from_num(crate::sim::map::bridge_topology::BRIDGE_DECK_HEIGHT_LEPTONS)
        );
    }
}
