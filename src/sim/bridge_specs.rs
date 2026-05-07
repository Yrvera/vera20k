//! RE-backed bridge helper algorithms not yet fully wired into the live runtime.
//!
//! These helpers mirror closed behavior from the RE repo:
//! - low-bridge overlay damage step (RA2)
//! - low-bridge connected-section selector (YR)
//! - ZoneConnection record decode + proximity matching
//! - bridge-layer zone-id policy gate (RA2/YR)
//!
//! They are kept as pure functions so the runtime can adopt them incrementally
//! once mutable overlay state and ZoneConnection records are available.

use crate::sim::bridge_state::{AnchorSpan, Axis, BridgeRuntimeState, DamageState, Direction, Phase};

const BRIDGE_GATE_BIT: u32 = 0x0100;
const NO_ZONE_CONNECTION: i16 = -1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BridgeOverlayTriple {
    pub a: i32,
    pub center: i32,
    pub b: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LowBridgeOverlayDamageReason {
    NotBridgeOverlay,
    GateFailed,
    NoTransition,
    Changed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LowBridgeOverlayDamageStepResult {
    pub ok: bool,
    pub reason: LowBridgeOverlayDamageReason,
    pub changed: bool,
    pub triple_out: BridgeOverlayTriple,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LowBridgeConnectedBand {
    WoodBand1,
    WoodBand2,
    ConcreteBand1,
    ConcreteBand2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LowBridgeConnectedAnchor {
    OppositeAdjacent,
    Center,
    PrimaryAdjacent,
    ConnectedChainHelper,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LowBridgeConnectedPattern {
    A,
    B,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LowBridgeConnectedSectionSelectorResult {
    pub handled: bool,
    pub reason_not_bridge_overlay: bool,
    pub pattern: Option<LowBridgeConnectedPattern>,
    pub band: Option<LowBridgeConnectedBand>,
    pub anchor: Option<LowBridgeConnectedAnchor>,
    pub neighbor_range_lo: Option<i32>,
    pub neighbor_range_hi: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZoneConnectionRecord {
    pub cell_a: (i16, i16),
    pub cell_b: (i16, i16),
    pub flags: u32,
    pub flags_byte8: u8,
    pub skip_if_nonzero: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeZoneIdPolicyTarget {
    Ra21006,
    Yr1001,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BridgeZoneIdPolicyDecision {
    pub use_bridge_path: bool,
    pub call_bridge_remap_fallback: bool,
    pub return_no_zone: bool,
}

pub fn low_bridge_overlay_damage_step_ra2(
    triple: BridgeOverlayTriple,
    damage: i32,
    bridge_strength: i32,
    atom_damage: i32,
    random_ranged_1_bridge_strength: i32,
) -> LowBridgeOverlayDamageStepResult {
    let center = triple.center;
    let in_a = in_range_inclusive(center, 0x4a, 0x63);
    let in_b = in_range_inclusive(center, 0xcd, 0xe6);

    if !in_a && !in_b {
        return LowBridgeOverlayDamageStepResult {
            ok: true,
            reason: LowBridgeOverlayDamageReason::NotBridgeOverlay,
            changed: false,
            triple_out: triple,
        };
    }

    if damage != atom_damage {
        if bridge_strength <= 0 || random_ranged_1_bridge_strength >= damage {
            return LowBridgeOverlayDamageStepResult {
                ok: true,
                reason: LowBridgeOverlayDamageReason::GateFailed,
                changed: false,
                triple_out: triple,
            };
        }
    }

    let new_index = if in_a {
        pattern_a_new_index(center)
    } else {
        pattern_b_new_index(center)
    };

    match new_index {
        Some(new_index) => LowBridgeOverlayDamageStepResult {
            ok: true,
            reason: LowBridgeOverlayDamageReason::Changed,
            changed: true,
            triple_out: BridgeOverlayTriple {
                a: new_index,
                center: new_index,
                b: new_index,
            },
        },
        None => LowBridgeOverlayDamageStepResult {
            ok: true,
            reason: LowBridgeOverlayDamageReason::NoTransition,
            changed: false,
            triple_out: triple,
        },
    }
}

pub fn low_bridge_connected_section_selector_yr(
    center_overlay_type_index: i32,
    primary_probe_in_family_range: bool,
    secondary_probe_in_family_range: bool,
) -> LowBridgeConnectedSectionSelectorResult {
    let Some(band) = classify_low_bridge_band(center_overlay_type_index) else {
        return LowBridgeConnectedSectionSelectorResult {
            handled: false,
            reason_not_bridge_overlay: true,
            pattern: None,
            band: None,
            anchor: None,
            neighbor_range_lo: None,
            neighbor_range_hi: None,
        };
    };

    let (pattern, neighbor_range_lo, neighbor_range_hi) = match band {
        LowBridgeConnectedBand::WoodBand1 | LowBridgeConnectedBand::WoodBand2 => {
            (LowBridgeConnectedPattern::A, 0x4a, 0x65)
        }
        LowBridgeConnectedBand::ConcreteBand1 | LowBridgeConnectedBand::ConcreteBand2 => {
            (LowBridgeConnectedPattern::B, 0xcd, 0xe8)
        }
    };

    let anchor = if !primary_probe_in_family_range {
        LowBridgeConnectedAnchor::OppositeAdjacent
    } else if !secondary_probe_in_family_range {
        LowBridgeConnectedAnchor::Center
    } else if matches!(
        band,
        LowBridgeConnectedBand::WoodBand1 | LowBridgeConnectedBand::ConcreteBand1
    ) {
        LowBridgeConnectedAnchor::PrimaryAdjacent
    } else {
        LowBridgeConnectedAnchor::ConnectedChainHelper
    };

    LowBridgeConnectedSectionSelectorResult {
        handled: true,
        reason_not_bridge_overlay: false,
        pattern: Some(pattern),
        band: Some(band),
        anchor: Some(anchor),
        neighbor_range_lo: Some(neighbor_range_lo),
        neighbor_range_hi: Some(neighbor_range_hi),
    }
}

pub fn decode_zone_connection_record(record: &[u8]) -> ZoneConnectionRecord {
    assert_eq!(record.len(), 16, "expected 16-byte ZoneConnection record");

    let flags = read_u32_le(record, 0x08);
    ZoneConnectionRecord {
        cell_a: (read_i16_le(record, 0x00), read_i16_le(record, 0x02)),
        cell_b: (read_i16_le(record, 0x04), read_i16_le(record, 0x06)),
        flags,
        flags_byte8: (flags & 0xff) as u8,
        skip_if_nonzero: read_u32_le(record, 0x0c),
    }
}

pub fn zone_connection_matches_cell(record: &[u8], cell: (i16, i16), dist: i16) -> bool {
    let decoded = decode_zone_connection_record(record);
    if decoded.skip_if_nonzero != 0 {
        return false;
    }

    let dist = dist.max(0);
    let ((ax, ay), (bx, by)) = (decoded.cell_a, decoded.cell_b);

    if ax == bx {
        let y_min = ay.min(by);
        let y_max = ay.max(by);
        cell.1 >= y_min && cell.1 <= y_max && (cell.0 - ax).abs() <= dist
    } else {
        let x_min = ax.min(bx);
        let x_max = ax.max(bx);
        cell.0 >= x_min && cell.0 <= x_max && (cell.1 - ay).abs() <= dist
    }
}

pub fn get_cell_zone_id_bridge_policy_decision(
    target: BridgeZoneIdPolicyTarget,
    on_bridge: bool,
    cell_flags_dword: u32,
    zone_connection_index: i16,
) -> BridgeZoneIdPolicyDecision {
    let use_bridge_path = on_bridge && (cell_flags_dword & BRIDGE_GATE_BIT) != 0;
    if !use_bridge_path {
        return BridgeZoneIdPolicyDecision {
            use_bridge_path: false,
            call_bridge_remap_fallback: false,
            return_no_zone: false,
        };
    }

    if zone_connection_index != NO_ZONE_CONNECTION {
        return BridgeZoneIdPolicyDecision {
            use_bridge_path: true,
            call_bridge_remap_fallback: false,
            return_no_zone: false,
        };
    }

    match target {
        BridgeZoneIdPolicyTarget::Yr1001 => BridgeZoneIdPolicyDecision {
            use_bridge_path: true,
            call_bridge_remap_fallback: true,
            return_no_zone: false,
        },
        BridgeZoneIdPolicyTarget::Ra21006 => BridgeZoneIdPolicyDecision {
            use_bridge_path: true,
            call_bridge_remap_fallback: false,
            return_no_zone: true,
        },
    }
}

fn in_range_inclusive(x: i32, lo: i32, hi: i32) -> bool {
    x >= lo && x <= hi
}

fn pattern_a_new_index(center_overlay_type_index: i32) -> Option<i32> {
    match center_overlay_type_index {
        0x60 => Some(0x61),
        0x62 => Some(0x63),
        x if x < 0x59 => Some(0x59),
        x if x < 0x5c => Some(0x65),
        _ => None,
    }
}

fn pattern_b_new_index(center_overlay_type_index: i32) -> Option<i32> {
    match center_overlay_type_index {
        0xe3 => Some(0xe4),
        0xe5 => Some(0xe6),
        x if x < 0xdc => Some(0xdc),
        x if x < 0xdf => Some(0xe8),
        _ => None,
    }
}

fn classify_low_bridge_band(center_overlay_type_index: i32) -> Option<LowBridgeConnectedBand> {
    let x = center_overlay_type_index;

    if in_range_inclusive(x, 0x4a, 0x52) || in_range_inclusive(x, 0x5c, 0x5f) || x == 0x64 {
        return Some(LowBridgeConnectedBand::WoodBand1);
    }
    if in_range_inclusive(x, 0x53, 0x5b) || in_range_inclusive(x, 0x60, 0x63) || x == 0x65 {
        return Some(LowBridgeConnectedBand::WoodBand2);
    }
    if in_range_inclusive(x, 0xcd, 0xd5) || in_range_inclusive(x, 0xdf, 0xe2) || x == 0xe7 {
        return Some(LowBridgeConnectedBand::ConcreteBand1);
    }
    if in_range_inclusive(x, 0xd6, 0xde) || in_range_inclusive(x, 0xe3, 0xe6) || x == 0xe8 {
        return Some(LowBridgeConnectedBand::ConcreteBand2);
    }

    None
}

fn read_u16_le(bytes: &[u8], off: usize) -> u16 {
    u16::from_le_bytes([bytes[off], bytes[off + 1]])
}

fn read_i16_le(bytes: &[u8], off: usize) -> i16 {
    read_u16_le(bytes, off) as i16
}

fn read_u32_le(bytes: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([bytes[off], bytes[off + 1], bytes[off + 2], bytes[off + 3]])
}

/// Apply a single ramp state transition. Mirrors one of the binary's 16
/// `UpdateRamp_*_High/_Low` helpers (HIGH §11.1).
///
/// State byte semantics (CellClass+0x11E):
/// - NS-axis range: 0..=8 (0..=3 healthy, 4 = DamageA-set, 5 = DamageB-set,
///   6 = both halves damaged, 7 = PartialCollapseA, 8 = PartialCollapseB)
/// - EW-axis range: 9..=17 (9..=12 healthy, 0x0D = DamageB-set, 0x0E =
///   DamageA-set, 0x0F = both halves damaged, 0x10 = PartialCollapseB,
///   0x11 = PartialCollapseA)
///
/// Returns `Some(next_state)` on a defined transition, `None` if the
/// `(axis, phase, current_state)` combination has no transition (cell
/// unchanged).
///
/// **Collapse-final special case:** when input matches the "opposite-already-
/// collapsed" partial state (NS Collapse{A,B}: state 8/7; EW Collapse{A,B}:
/// state 0x10/0x11), the function returns `Some(0)` — but the caller MUST
/// also clear the bridge-direction flag, set `IsoTileTypeIndex = -1`, fire
/// `UpdateAdjacentBridges`, and zone-refresh. Body-cell driver detects this
/// via `(prev_state.is_partial_collapse() && phase.is_collapse() && next == 0)`.
///
/// `_Low` variants are intentionally not a parameter: state transitions are
/// identical, so the same function serves both. Overlay propagation (§11.2 +
/// `pick_destruction_overlay`) is what distinguishes HIGH from LOW.
pub fn apply_ramp_transition(
    current_state: u8,
    axis: Axis,
    phase: Phase,
) -> Option<u8> {
    match (axis, phase, current_state) {
        // --- NS axis (state 0..=8) ---
        // NS_DamageA: 0..=3 → 4, 5 → 6
        (Axis::NS, Phase::DamageA, 0..=3) => Some(4),
        (Axis::NS, Phase::DamageA, 5) => Some(6),
        // NS_DamageB: 0..=3 → 5, 4 → 6
        (Axis::NS, Phase::DamageB, 0..=3) => Some(5),
        (Axis::NS, Phase::DamageB, 4) => Some(6),
        // NS_CollapseA: 0..=6 → 7, 8 → 0 (collapse-final)
        (Axis::NS, Phase::CollapseA, 0..=6) => Some(7),
        (Axis::NS, Phase::CollapseA, 8) => Some(0),
        // NS_CollapseB: 0..=6 → 8, 7 → 0 (collapse-final)
        (Axis::NS, Phase::CollapseB, 0..=6) => Some(8),
        (Axis::NS, Phase::CollapseB, 7) => Some(0),

        // --- EW axis (state 9..=17 / 0x09..=0x11) ---
        // EW_DamageA: 9..=12 → 0x0E, 0x0D → 0x0F
        (Axis::EW, Phase::DamageA, 9..=12) => Some(0x0E),
        (Axis::EW, Phase::DamageA, 0x0D) => Some(0x0F),
        // EW_DamageB: 9..=12 → 0x0D, 0x0E → 0x0F
        (Axis::EW, Phase::DamageB, 9..=12) => Some(0x0D),
        (Axis::EW, Phase::DamageB, 0x0E) => Some(0x0F),
        // EW_CollapseA: 9..=15 → 0x11, 0x10 → 0 (collapse-final)
        (Axis::EW, Phase::CollapseA, 9..=15) => Some(0x11),
        (Axis::EW, Phase::CollapseA, 0x10) => Some(0),
        // EW_CollapseB: 9..=15 → 0x10, 0x11 → 0 (collapse-final)
        (Axis::EW, Phase::CollapseB, 9..=15) => Some(0x10),
        (Axis::EW, Phase::CollapseB, 0x11) => Some(0),

        // No defined transition.
        _ => None,
    }
}

/// Pick the next overlay byte for a destroying bridge cell. Mirrors
/// `ApplyBridgeDestruction_NS_High @ 0x57E7A0` and `_EW_High @ 0x57ED00`
/// (HIGH §11.2). Indexed by the result of `CheckBridgeNeighbors_*` —
/// i.e., a small integer encoding which adjacent cells still hold bridge
/// overlay. Distinct from `apply_ramp_transition` which handles state
/// (CellClass+0x11E); this one writes the visible overlay byte (+0x44).
///
/// `0xFF` in the table represents the binary's `-1` sentinel ("no
/// transition for this neighbor pattern" — leave overlay alone).
pub fn pick_destruction_overlay(
    neighbor_check: u8,
    axis: Axis,
    is_high_bridge: bool,
) -> Option<u8> {
    if neighbor_check >= 16 {
        return None;
    }
    let table: &[u8; 16] = match (axis, is_high_bridge) {
        (Axis::NS, true) => &DESTRUCTION_OVERLAY_HIGH_NS,
        (Axis::EW, true) => &DESTRUCTION_OVERLAY_HIGH_EW,
        (Axis::NS, false) => &DESTRUCTION_OVERLAY_LOW_NS,
        (Axis::EW, false) => &DESTRUCTION_OVERLAY_LOW_EW,
    };
    let val = table[neighbor_check as usize];
    if val == 0xFF { None } else { Some(val) }
}

/// HIGH NS destruction overlay table per HIGH §11.2 (`ApplyBridgeDestruction_NS_High`
/// @ `0x57E7A0`). Indexed by `CheckBridgeNeighbors_EW_High` result.
/// All 16 entries verified live byte-for-byte (indices 11..=15 explicitly
/// initialized to `0xffffffff` in the function prologue — no fall-through).
static DESTRUCTION_OVERLAY_HIGH_NS: [u8; 16] = [
    0xFF, 0xD2, 0xD5, 0xFF, 0xD1, 0xD3, 0xD5, 0xFF,
    0xD4, 0xD4, 0xE7, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
];

/// HIGH EW destruction overlay table per HIGH §11.2 (`ApplyBridgeDestruction_EW_High`
/// @ `0x57ED00`). Indexed by `CheckBridgeNeighbors_NS_High` result.
static DESTRUCTION_OVERLAY_HIGH_EW: [u8; 16] = [
    0xFF, 0xDB, 0xDE, 0xFF, 0xDA, 0xDC, 0xDE, 0xFF,
    0xDD, 0xDD, 0xE8, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
];

/// LOW NS destruction overlay table per `ApplyBridgeDestruction_NS_Low`
/// @ `0x0057DD50` (verified live, see HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md
/// §11.2-LOW). Indexed by `CheckBridgeNeighbors_EW_Low` result. Final
/// destroyed byte = `0x64`. Outer overlay gate: `0x4A..=0x65`.
/// Progressive intermediates (handled by caller, not table):
/// `0x5C → 0x5D`, `0x5E → 0x5F`.
static DESTRUCTION_OVERLAY_LOW_NS: [u8; 16] = [
    0xFF, 0x4F, 0x52, 0xFF, 0x4E, 0x50, 0x52, 0xFF,
    0x51, 0x51, 0x64, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
];

/// LOW EW destruction overlay table per `ApplyBridgeDestruction_EW_Low`
/// @ `0x0057E2A0` (verified live). Indexed by `CheckBridgeNeighbors_NS_Low`
/// result. Final destroyed byte = `0x65`. Outer overlay gate: `0x4A..=0x65`.
/// Progressive intermediates: `0x60 → 0x61`, `0x62 → 0x63`.
static DESTRUCTION_OVERLAY_LOW_EW: [u8; 16] = [
    0xFF, 0x58, 0x5B, 0xFF, 0x57, 0x59, 0x5B, 0xFF,
    0x5A, 0x5A, 0x65, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
];

/// Per-cell action emitted by `set_bridge_direction` walker. The orchestrator
/// in `world::resolve_bridge_state_changes` consumes these and dispatches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellAction {
    /// Cell receives `BlowUpBridge` (kill ground, Limbo bridge-deck, debris).
    /// Destruction path slots 0, 1, 2, 4 (binary cells 1, 2, 3, 5).
    BlowUpBridge,
    /// Cell receives flag-only update — no BlowUpBridge. Slot 3 (binary
    /// cell 4) and slot 5 (binary cell 6) on destruction path.
    FlagOnly,
}

/// Result from `set_bridge_direction` walker. Each entry is one cell + its
/// action.
#[derive(Debug, Clone)]
pub struct SetBridgeDirectionResult {
    pub actions: Vec<((u16, u16), usize, CellAction)>,
}

/// Emit the per-cell action list for an anchor span. Mirrors binary's
/// `SetBridgeDirection_NESW @ 0x47E040`.
///
/// `set == false` is the destruction path (4 BlowUpBridge calls + 1–2
/// flag-only). `set == true` is the build/intact path (no BlowUpBridge —
/// flag writes only). Tier 2 only consumes destruction path; build path
/// is exercised by map-load anchor walker construction (Task 7).
pub fn set_bridge_direction(span: &AnchorSpan, set: bool) -> SetBridgeDirectionResult {
    let mut actions = Vec::with_capacity(6);
    for (slot, cell) in span.iter_cells() {
        let action = if !set {
            // Destruction path: slots 0, 1, 2, 4 = BlowUpBridge; 3, 5 = FlagOnly.
            if AnchorSpan::BLOW_UP_SLOTS.contains(&slot) {
                CellAction::BlowUpBridge
            } else {
                CellAction::FlagOnly
            }
        } else {
            // Build path: every cell is FlagOnly (no BlowUpBridge). Used by
            // map-load construction.
            CellAction::FlagOnly
        };
        actions.push((cell, slot, action));
    }
    SetBridgeDirectionResult { actions }
}

/// Outcome of one perpendicular `UpdateRamp_*_High`-style call. Mirrors the
/// inner side effects of binary `UpdateRamp_NS_DamageA_High @ 0x00572230` and
/// peers (HIGH §11.1).
///
/// Currently models only the **anchor-flag-gated state-byte transition**.
/// The pavement/bridgehead-overlay-write branch fires off-screen
/// (`SetOverlayAndPropagate` / `ToggleBridgePavement`) and is deferred until
/// the runtime-initialized tile-class constants are observed live —
/// see plan §"Deferred to Implementation".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RampOutcome {
    /// True if the target cell's `damage_state` was mutated (target was an
    /// anchor and the `apply_ramp_transition` returned `Some`).
    pub state_changed: bool,
}

/// Compute the perpendicular-walk direction for a body-driver UpdateRamp call.
/// A-side and B-side perpendiculars per `[GHIDRA 0x576BA0]` body branch:
/// NS axis: A → E (dir 2), B → W (dir 6).
/// EW axis: A → S (dir 4), B → N (dir 0).
fn perpendicular_direction(axis: Axis, phase: Phase) -> Direction {
    let is_a_side = matches!(phase, Phase::DamageA | Phase::CollapseA);
    match (axis, is_a_side) {
        (Axis::NS, true) => Direction::E,
        (Axis::NS, false) => Direction::W,
        (Axis::EW, true) => Direction::S,
        (Axis::EW, false) => Direction::N,
    }
}

/// Walk one perpendicular cell from `anchor_pos` and apply the `UpdateRamp_*`
/// state-byte transition if the target is an anchor cell.
///
/// **State-byte branch only** — overlay-write branch deferred (see plan).
/// Mirrors the anchor-flag-gated `+0x11E` write of binary `UpdateRamp_*_High`.
///
/// `is_high_bridge` is currently unused (state transitions are identical for
/// HIGH and LOW per HIGH §11.1) but kept for API symmetry with the deferred
/// overlay-write branch and future Task 14 (bridgehead driver).
pub fn update_ramp_perpendicular(
    state: &mut BridgeRuntimeState,
    anchor_pos: (u16, u16),
    axis: Axis,
    phase: Phase,
    _is_high_bridge: bool,
) -> RampOutcome {
    let dir = perpendicular_direction(axis, phase);
    let (dx, dy) = dir.offset();
    let target_x = anchor_pos.0 as i32 + dx;
    let target_y = anchor_pos.1 as i32 + dy;
    if target_x < 0 || target_y < 0 {
        return RampOutcome { state_changed: false };
    }
    let target_pos = (target_x as u16, target_y as u16);

    // Snapshot target read (avoids borrow conflict with subsequent mut access).
    let Some(target_cell) = state.cell(target_pos.0, target_pos.1).copied() else {
        return RampOutcome { state_changed: false };
    };
    // Anchor-flag gate. In binary: `target.flags & 0x80`. In our model:
    // role == Anchor.
    if !matches!(target_cell.role, crate::sim::bridge_state::BridgeCellRole::Anchor) {
        return RampOutcome { state_changed: false };
    }
    let Some(target_axis) = target_cell.axis else {
        return RampOutcome { state_changed: false };
    };

    let current_byte = target_cell.damage_state.to_state_byte(target_axis);
    let Some(next_byte) = apply_ramp_transition(current_byte, axis, phase) else {
        return RampOutcome { state_changed: false };
    };

    // Decode next byte. Per `apply_ramp_transition` docstring, next_byte == 0
    // only fires for the collapse-final case (state 7/8/0x10/0x11 + matching
    // CollapseA/B phase). When the perpendicular target hits its recurse-to-0
    // branch the binary sets `state = 0; IsoTileTypeIndex = -1`, which in our
    // model is Destroyed. So decode 0 → Destroyed for this path.
    let next_state = if next_byte == 0 {
        DamageState::Destroyed
    } else {
        match DamageState::from_state_byte(next_byte) {
            Some(s) => s,
            None => return RampOutcome { state_changed: false },
        }
    };

    // Mut access to write the new state.
    if let Some(cell_mut) = state.cell_mut(target_pos.0, target_pos.1) {
        cell_mut.damage_state = next_state;
        RampOutcome { state_changed: true }
    } else {
        RampOutcome { state_changed: false }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::bridge_state::{
        AnchorSpan, Axis, BridgeCellRole, BridgeRuntimeCell, BridgeRuntimeState,
        DamageState, Direction, Phase,
    };

    #[test]
    fn low_bridge_damage_step_ignores_non_bridge_overlay() {
        let out = low_bridge_overlay_damage_step_ra2(
            BridgeOverlayTriple {
                a: 1,
                center: 1234,
                b: 2,
            },
            50,
            150,
            999,
            1,
        );
        assert_eq!(out.reason, LowBridgeOverlayDamageReason::NotBridgeOverlay);
        assert!(!out.changed);
        assert_eq!(out.triple_out.center, 1234);
    }

    #[test]
    fn low_bridge_damage_step_applies_rng_gate() {
        let out = low_bridge_overlay_damage_step_ra2(
            BridgeOverlayTriple {
                a: 96,
                center: 96,
                b: 96,
            },
            10,
            150,
            999,
            10,
        );
        assert_eq!(out.reason, LowBridgeOverlayDamageReason::GateFailed);
        assert!(!out.changed);
    }

    #[test]
    fn low_bridge_damage_step_atom_damage_bypasses_gate() {
        let out = low_bridge_overlay_damage_step_ra2(
            BridgeOverlayTriple {
                a: 96,
                center: 96,
                b: 96,
            },
            999,
            150,
            999,
            150,
        );
        assert_eq!(out.reason, LowBridgeOverlayDamageReason::Changed);
        assert!(out.changed);
        assert_eq!(
            out.triple_out,
            BridgeOverlayTriple {
                a: 97,
                center: 97,
                b: 97,
            }
        );
    }

    #[test]
    fn low_bridge_damage_step_maps_wood_family() {
        let out = low_bridge_overlay_damage_step_ra2(
            BridgeOverlayTriple {
                a: 74,
                center: 74,
                b: 74,
            },
            2,
            150,
            999,
            1,
        );
        assert_eq!(out.triple_out.center, 89);

        let out = low_bridge_overlay_damage_step_ra2(
            BridgeOverlayTriple {
                a: 89,
                center: 89,
                b: 90,
            },
            2,
            150,
            999,
            1,
        );
        assert_eq!(out.triple_out.center, 101);
    }

    #[test]
    fn low_bridge_damage_step_maps_concrete_family_and_no_transition() {
        let out = low_bridge_overlay_damage_step_ra2(
            BridgeOverlayTriple {
                a: 227,
                center: 227,
                b: 227,
            },
            2,
            150,
            999,
            1,
        );
        assert_eq!(out.triple_out.center, 228);

        let no_change = low_bridge_overlay_damage_step_ra2(
            BridgeOverlayTriple {
                a: 223,
                center: 223,
                b: 223,
            },
            2,
            150,
            999,
            1,
        );
        assert_eq!(no_change.reason, LowBridgeOverlayDamageReason::NoTransition);
        assert!(!no_change.changed);
    }

    #[test]
    fn low_bridge_selector_rejects_non_bridge_overlay() {
        let out = low_bridge_connected_section_selector_yr(1, false, false);
        assert!(!out.handled);
        assert!(out.reason_not_bridge_overlay);
    }

    #[test]
    fn low_bridge_selector_uses_exact_anchor_policy() {
        let out = low_bridge_connected_section_selector_yr(74, false, false);
        assert_eq!(out.pattern, Some(LowBridgeConnectedPattern::A));
        assert_eq!(out.band, Some(LowBridgeConnectedBand::WoodBand1));
        assert_eq!(out.anchor, Some(LowBridgeConnectedAnchor::OppositeAdjacent));
        assert_eq!(out.neighbor_range_lo, Some(74));
        assert_eq!(out.neighbor_range_hi, Some(101));

        let out = low_bridge_connected_section_selector_yr(74, true, false);
        assert_eq!(out.anchor, Some(LowBridgeConnectedAnchor::Center));

        let out = low_bridge_connected_section_selector_yr(74, true, true);
        assert_eq!(out.anchor, Some(LowBridgeConnectedAnchor::PrimaryAdjacent));

        let out = low_bridge_connected_section_selector_yr(83, true, true);
        assert_eq!(out.band, Some(LowBridgeConnectedBand::WoodBand2));
        assert_eq!(
            out.anchor,
            Some(LowBridgeConnectedAnchor::ConnectedChainHelper)
        );

        let out = low_bridge_connected_section_selector_yr(205, false, false);
        assert_eq!(out.pattern, Some(LowBridgeConnectedPattern::B));
        assert_eq!(out.band, Some(LowBridgeConnectedBand::ConcreteBand1));
        assert_eq!(out.anchor, Some(LowBridgeConnectedAnchor::OppositeAdjacent));
        assert_eq!(out.neighbor_range_lo, Some(205));
        assert_eq!(out.neighbor_range_hi, Some(232));

        let out = low_bridge_connected_section_selector_yr(214, true, true);
        assert_eq!(out.band, Some(LowBridgeConnectedBand::ConcreteBand2));
        assert_eq!(
            out.anchor,
            Some(LowBridgeConnectedAnchor::ConnectedChainHelper)
        );
    }

    #[test]
    fn zone_connection_record_decodes_layout() {
        let record = [10, 0, 254, 255, 10, 0, 5, 0, 1, 0, 0, 0, 0, 0, 0, 0];
        let decoded = decode_zone_connection_record(&record);
        assert_eq!(decoded.cell_a, (10, -2));
        assert_eq!(decoded.cell_b, (10, 5));
        assert_eq!(decoded.flags, 1);
        assert_eq!(decoded.flags_byte8, 1);
        assert_eq!(decoded.skip_if_nonzero, 0);
    }

    #[test]
    fn zone_connection_match_uses_axis_aligned_segment_proximity() {
        let record = [10, 0, 254, 255, 10, 0, 5, 0, 1, 0, 0, 0, 0, 0, 0, 0];
        assert!(zone_connection_matches_cell(&record, (9, 0), 1));
        assert!(!zone_connection_matches_cell(&record, (8, 0), 1));
        assert!(!zone_connection_matches_cell(&record, (10, 6), 1));
    }

    #[test]
    fn zone_connection_match_respects_skip_flag() {
        let record = [10, 0, 254, 255, 10, 0, 5, 0, 1, 0, 0, 0, 1, 0, 0, 0];
        assert!(!zone_connection_matches_cell(&record, (10, 0), 1));
    }

    #[test]
    fn bridge_zone_policy_turns_off_when_on_bridge_false() {
        let out = get_cell_zone_id_bridge_policy_decision(
            BridgeZoneIdPolicyTarget::Yr1001,
            false,
            0x0100,
            -1,
        );
        assert_eq!(
            out,
            BridgeZoneIdPolicyDecision {
                use_bridge_path: false,
                call_bridge_remap_fallback: false,
                return_no_zone: false,
            }
        );
    }

    #[test]
    fn bridge_zone_policy_turns_off_when_bridge_bit_clear() {
        let out =
            get_cell_zone_id_bridge_policy_decision(BridgeZoneIdPolicyTarget::Ra21006, true, 0, -1);
        assert!(!out.use_bridge_path);
        assert!(!out.call_bridge_remap_fallback);
        assert!(!out.return_no_zone);
    }

    #[test]
    fn bridge_zone_policy_matches_ra2_and_yr_fallback_split() {
        let hit = get_cell_zone_id_bridge_policy_decision(
            BridgeZoneIdPolicyTarget::Ra21006,
            true,
            0x0100,
            3,
        );
        assert_eq!(
            hit,
            BridgeZoneIdPolicyDecision {
                use_bridge_path: true,
                call_bridge_remap_fallback: false,
                return_no_zone: false,
            }
        );

        let ra2_miss = get_cell_zone_id_bridge_policy_decision(
            BridgeZoneIdPolicyTarget::Ra21006,
            true,
            0x0100,
            -1,
        );
        assert_eq!(
            ra2_miss,
            BridgeZoneIdPolicyDecision {
                use_bridge_path: true,
                call_bridge_remap_fallback: false,
                return_no_zone: true,
            }
        );

        let yr_miss = get_cell_zone_id_bridge_policy_decision(
            BridgeZoneIdPolicyTarget::Yr1001,
            true,
            0x0100,
            -1,
        );
        assert_eq!(
            yr_miss,
            BridgeZoneIdPolicyDecision {
                use_bridge_path: true,
                call_bridge_remap_fallback: true,
                return_no_zone: false,
            }
        );
    }

    #[test]
    fn ramp_ns_damage_a_healthy_to_4() {
        for s in 0..=3 {
            assert_eq!(
                apply_ramp_transition(s, Axis::NS, Phase::DamageA),
                Some(4),
                "state {s}"
            );
        }
    }

    #[test]
    fn ramp_ns_damage_a_5_to_6() {
        assert_eq!(apply_ramp_transition(5, Axis::NS, Phase::DamageA), Some(6));
    }

    #[test]
    fn ramp_ns_damage_b_healthy_to_5() {
        for s in 0..=3 {
            assert_eq!(apply_ramp_transition(s, Axis::NS, Phase::DamageB), Some(5));
        }
    }

    #[test]
    fn ramp_ns_damage_b_4_to_6() {
        assert_eq!(apply_ramp_transition(4, Axis::NS, Phase::DamageB), Some(6));
    }

    #[test]
    fn ramp_ns_collapse_a_to_7() {
        for s in 0..=6 {
            assert_eq!(apply_ramp_transition(s, Axis::NS, Phase::CollapseA), Some(7));
        }
    }

    #[test]
    fn ramp_ns_collapse_a_final_state_8_to_0() {
        // Collapse-final: caller must also clear bridge dir + IsoTileTypeIndex.
        assert_eq!(apply_ramp_transition(8, Axis::NS, Phase::CollapseA), Some(0));
    }

    #[test]
    fn ramp_ns_collapse_b_to_8() {
        for s in 0..=6 {
            assert_eq!(apply_ramp_transition(s, Axis::NS, Phase::CollapseB), Some(8));
        }
    }

    #[test]
    fn ramp_ns_collapse_b_final_state_7_to_0() {
        assert_eq!(apply_ramp_transition(7, Axis::NS, Phase::CollapseB), Some(0));
    }

    #[test]
    fn ramp_ew_damage_a_healthy_to_e() {
        for s in 9..=12 {
            assert_eq!(apply_ramp_transition(s, Axis::EW, Phase::DamageA), Some(0x0E));
        }
    }

    #[test]
    fn ramp_ew_damage_a_d_to_f() {
        assert_eq!(apply_ramp_transition(0x0D, Axis::EW, Phase::DamageA), Some(0x0F));
    }

    #[test]
    fn ramp_ew_damage_b_healthy_to_d() {
        for s in 9..=12 {
            assert_eq!(apply_ramp_transition(s, Axis::EW, Phase::DamageB), Some(0x0D));
        }
    }

    #[test]
    fn ramp_ew_damage_b_e_to_f() {
        assert_eq!(apply_ramp_transition(0x0E, Axis::EW, Phase::DamageB), Some(0x0F));
    }

    #[test]
    fn ramp_ew_collapse_a_to_11() {
        for s in 9..=15 {
            assert_eq!(apply_ramp_transition(s, Axis::EW, Phase::CollapseA), Some(0x11));
        }
    }

    #[test]
    fn ramp_ew_collapse_a_final_state_10_to_0() {
        assert_eq!(apply_ramp_transition(0x10, Axis::EW, Phase::CollapseA), Some(0));
    }

    #[test]
    fn ramp_ew_collapse_b_to_10() {
        for s in 9..=15 {
            assert_eq!(apply_ramp_transition(s, Axis::EW, Phase::CollapseB), Some(0x10));
        }
    }

    #[test]
    fn ramp_ew_collapse_b_final_state_11_to_0() {
        assert_eq!(apply_ramp_transition(0x11, Axis::EW, Phase::CollapseB), Some(0));
    }

    #[test]
    fn ramp_undefined_combination_returns_none() {
        // EW phase on NS-range state, etc.
        assert_eq!(apply_ramp_transition(0, Axis::EW, Phase::DamageA), None);
        assert_eq!(apply_ramp_transition(15, Axis::NS, Phase::DamageA), None);
        // State outside both ranges.
        assert_eq!(apply_ramp_transition(0xFF, Axis::NS, Phase::DamageA), None);
    }

    #[test]
    fn destruction_overlay_high_ns_known_entries() {
        // Spot-check verified entries from HIGH §11.2.
        assert_eq!(pick_destruction_overlay(1, Axis::NS, true), Some(0xD2));
        assert_eq!(pick_destruction_overlay(2, Axis::NS, true), Some(0xD5));
        assert_eq!(pick_destruction_overlay(4, Axis::NS, true), Some(0xD1));
        assert_eq!(pick_destruction_overlay(10, Axis::NS, true), Some(0xE7)); // final destroyed
    }

    #[test]
    fn destruction_overlay_high_ew_known_entries() {
        assert_eq!(pick_destruction_overlay(1, Axis::EW, true), Some(0xDB));
        assert_eq!(pick_destruction_overlay(2, Axis::EW, true), Some(0xDE));
        assert_eq!(pick_destruction_overlay(10, Axis::EW, true), Some(0xE8)); // final destroyed
    }

    #[test]
    fn destruction_overlay_unused_indices_return_none() {
        assert_eq!(pick_destruction_overlay(0, Axis::NS, true), None);
        assert_eq!(pick_destruction_overlay(3, Axis::NS, true), None);
        assert_eq!(pick_destruction_overlay(11, Axis::NS, true), None);
    }

    #[test]
    fn destruction_overlay_out_of_range_returns_none() {
        assert_eq!(pick_destruction_overlay(16, Axis::NS, true), None);
        assert_eq!(pick_destruction_overlay(0xFF, Axis::EW, true), None);
    }

    #[test]
    fn destruction_overlay_low_ns_known_entries() {
        // Verified from ApplyBridgeDestruction_NS_Low @ 0x0057DD50.
        assert_eq!(pick_destruction_overlay(1, Axis::NS, false), Some(0x4F));
        assert_eq!(pick_destruction_overlay(2, Axis::NS, false), Some(0x52));
        assert_eq!(pick_destruction_overlay(4, Axis::NS, false), Some(0x4E));
        assert_eq!(pick_destruction_overlay(10, Axis::NS, false), Some(0x64)); // final destroyed
    }

    #[test]
    fn destruction_overlay_low_ew_known_entries() {
        // Verified from ApplyBridgeDestruction_EW_Low @ 0x0057E2A0.
        assert_eq!(pick_destruction_overlay(1, Axis::EW, false), Some(0x58));
        assert_eq!(pick_destruction_overlay(2, Axis::EW, false), Some(0x5B));
        assert_eq!(pick_destruction_overlay(4, Axis::EW, false), Some(0x57));
        assert_eq!(pick_destruction_overlay(10, Axis::EW, false), Some(0x65)); // final destroyed
    }

    #[test]
    fn destruction_overlay_low_unused_indices_return_none() {
        // Slots 0/3/7/11..=15 unused in both NS and EW LOW tables.
        for i in [0, 3, 7, 11, 12, 13, 14, 15] {
            assert_eq!(pick_destruction_overlay(i, Axis::NS, false), None, "NS slot {i}");
            assert_eq!(pick_destruction_overlay(i, Axis::EW, false), None, "EW slot {i}");
        }
    }

    #[test]
    fn set_bridge_direction_destruction_emits_4_blow_up_actions() {
        let span = AnchorSpan {
            id: 1,
            anchor: (5, 5),
            cells: [
                Some((5, 5)), Some((6, 5)), Some((7, 5)),
                Some((8, 5)), Some((4, 5)), None,
            ],
            axis: Axis::NS,
            direction: Direction::E,
            damage_state: DamageState::Damaged,
            bridge_group_id: 1,
        };
        let result = set_bridge_direction(&span, false);
        let blow_ups = result.actions.iter()
            .filter(|(_, _, a)| matches!(a, CellAction::BlowUpBridge))
            .count();
        assert_eq!(blow_ups, 4);
        let flag_only = result.actions.iter()
            .filter(|(_, _, a)| matches!(a, CellAction::FlagOnly))
            .count();
        assert_eq!(flag_only, 1); // slot 3 (cell 4)
    }

    #[test]
    fn set_bridge_direction_build_emits_no_blow_up_actions() {
        let span = AnchorSpan {
            id: 1,
            anchor: (0, 0),
            cells: [Some((0, 0)), None, None, None, None, None],
            axis: Axis::NS,
            direction: Direction::E,
            damage_state: DamageState::Healthy { variant: 0 },
            bridge_group_id: 1,
        };
        let result = set_bridge_direction(&span, true);
        assert!(result.actions.iter().all(|(_, _, a)| matches!(a, CellAction::FlagOnly)));
    }

    #[test]
    fn set_bridge_direction_includes_slot_5_only_when_present() {
        let span = AnchorSpan {
            id: 1,
            anchor: (5, 5),
            cells: [
                Some((5, 5)), Some((6, 5)), Some((7, 5)),
                Some((8, 5)), Some((4, 5)),
                Some((6, 5)), // hypothetical slot 5
            ],
            axis: Axis::NS,
            direction: Direction::W,
            damage_state: DamageState::Damaged,
            bridge_group_id: 1,
        };
        let result = set_bridge_direction(&span, false);
        let slot_5_action = result.actions.iter()
            .find(|(_, slot, _)| *slot == 5)
            .map(|(_, _, a)| *a);
        assert_eq!(slot_5_action, Some(CellAction::FlagOnly));
    }

    /// Build a minimal BridgeRuntimeState for update_ramp tests:
    /// anchors at (4,5), (5,5), (6,5), all NS axis, Healthy{variant: 0}.
    /// Uses `test_seed_cell` (Task 1 Step 5).
    fn make_perpendicular_test_state() -> BridgeRuntimeState {
        let mut state = BridgeRuntimeState::default();
        let template = BridgeRuntimeCell {
            deck_present: true,
            destroyable: true,
            deck_level: 0,
            bridge_group_id: Some(1),
            damage_state: DamageState::Healthy { variant: 0 },
            axis: Some(Axis::NS),
            role: BridgeCellRole::Anchor,
            anchor_span_id: Some(1),
            bridgehead_step: 0,
            overlay_byte: 0x18, // HIGH bridge anchor overlay
        };
        for (rx, ry) in [(4u16, 5u16), (5, 5), (6, 5)] {
            state.test_seed_cell(rx, ry, template);
        }
        state
    }

    #[test]
    fn update_ramp_perpendicular_ns_damage_a_anchor_target_transitions_to_4() {
        let mut state = make_perpendicular_test_state();
        let outcome = update_ramp_perpendicular(
            &mut state, (5, 5), Axis::NS, Phase::DamageA, true,
        );
        assert!(outcome.state_changed);
        let target = state.cell(6, 5).expect("E target");
        assert_eq!(target.damage_state, DamageState::Healthy { variant: 4 });
    }

    #[test]
    fn update_ramp_perpendicular_ns_damage_b_anchor_target_walks_west() {
        let mut state = make_perpendicular_test_state();
        let outcome = update_ramp_perpendicular(
            &mut state, (5, 5), Axis::NS, Phase::DamageB, true,
        );
        assert!(outcome.state_changed);
        let target = state.cell(4, 5).expect("W target");
        assert_eq!(target.damage_state, DamageState::Healthy { variant: 5 });
    }

    #[test]
    fn update_ramp_perpendicular_non_anchor_target_no_change() {
        let mut state = make_perpendicular_test_state();
        // Patch (6,5) to Body role (not Anchor).
        state.cell_mut(6, 5).unwrap().role = BridgeCellRole::Body;
        let outcome = update_ramp_perpendicular(
            &mut state, (5, 5), Axis::NS, Phase::DamageA, true,
        );
        assert!(!outcome.state_changed);
        assert_eq!(
            state.cell(6, 5).unwrap().damage_state,
            DamageState::Healthy { variant: 0 }
        );
    }

    #[test]
    fn update_ramp_perpendicular_target_off_map_no_change() {
        let mut state = make_perpendicular_test_state();
        // Anchor at (0, 0) calling NS DamageB → walks W → target x = -1 → out of bounds.
        let outcome = update_ramp_perpendicular(
            &mut state, (0, 0), Axis::NS, Phase::DamageB, true,
        );
        assert!(!outcome.state_changed);
    }

    #[test]
    fn update_ramp_perpendicular_collapse_final_target_to_destroyed() {
        let mut state = make_perpendicular_test_state();
        state.cell_mut(6, 5).unwrap().damage_state = DamageState::PartialCollapseB;
        let outcome = update_ramp_perpendicular(
            &mut state, (5, 5), Axis::NS, Phase::CollapseA, true,
        );
        assert!(outcome.state_changed);
        let target = state.cell(6, 5).expect("E target");
        assert_eq!(target.damage_state, DamageState::Destroyed);
    }

    #[test]
    fn update_ramp_perpendicular_ew_collapse_walks_south() {
        // Build a separate fixture with EW-axis anchors at (5,4), (5,5), (5,6).
        // (The default fixture is NS-axis at (4,5)/(5,5)/(6,5); using it for an
        // EW test would require re-seeding cells AND axes, so we just build
        // fresh.)
        let mut state = BridgeRuntimeState::default();
        let template = BridgeRuntimeCell {
            deck_present: true,
            destroyable: true,
            deck_level: 0,
            bridge_group_id: Some(1),
            damage_state: DamageState::Healthy { variant: 0 },
            axis: Some(Axis::EW),
            role: BridgeCellRole::Anchor,
            anchor_span_id: Some(1),
            bridgehead_step: 0,
            overlay_byte: 0x18,
        };
        for (rx, ry) in [(5u16, 4u16), (5, 5), (5, 6)] {
            state.test_seed_cell(rx, ry, template);
        }
        // EW CollapseA → walks S → target (5, 6).
        // Target state byte 9 (Healthy{0} EW) → apply_ramp_transition EW
        // CollapseA: 9..=15 → 0x11 = PartialCollapseA.
        let outcome = update_ramp_perpendicular(
            &mut state, (5, 5), Axis::EW, Phase::CollapseA, true,
        );
        assert!(outcome.state_changed);
        let target = state.cell(5, 6).expect("S target");
        assert_eq!(target.damage_state, DamageState::PartialCollapseA);
    }
}
