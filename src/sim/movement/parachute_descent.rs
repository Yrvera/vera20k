//! Parachute descent — per-tick altitude integrator for paradropped infantry.
//!
//! Mirrors the gamemd descent block byte-exact:
//! - rate accumulates by `-1` per tick (integer DEC, not float)
//! - clamps to `Rules.ParachuteMaxFallRate` (default `-3`)
//! - Z integrates as `altitude += rate` per tick (integer leptons)
//! - first tick has `rate == 0` → no movement (3-tick ramp: 0,-1,-2,-3,-3,...)
//! - landing on `altitude <= 0` (inclusive bound)
//! - body sequence set to `Paradrop` on attach, reset to `Stand` on landing
//!
//! Sibling of `droppod_movement` and follows the same shape: an `Option<State>`
//! field on `GameEntity`, a `begin_*` entry, a `tick_*` per-tick driver, and
//! cleanup via the locomotor override stack.
//!
//! ## Dependency rules
//! - Part of sim/ — depends on sim/game_entity, sim/entity_store, sim/locomotor.
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

use crate::sim::animation::SequenceKind;
use crate::sim::debug_event_log::DebugEventKind;
use crate::sim::entity_store::EntityStore;
use crate::sim::movement::locomotor::OverrideKind;
use crate::util::fixed_math::{SIM_ZERO, SimFixed, sim_to_f32};

/// Visual height offset per lepton of altitude (matches DropPod). Render-only f32.
const ALTITUDE_VISUAL_SCALE: f32 = 0.06;

/// Per-entity parachute descent state. Set by [`begin_parachute_descent`],
/// cleared on landing. While `Some`, the entity's locomotor is overridden
/// (`OverrideKind::Parachute`) so it does not occupy ground cells.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ParachuteDescentState {
    /// Descent rate in leptons/tick. Negative = falling.
    /// Starts at 0; decrements by 1 per tick; clamps to `Rules.ParachuteMaxFallRate`.
    pub rate: i32,
    /// Current altitude in leptons. Decreases by `rate` each tick.
    pub altitude: SimFixed,
}
