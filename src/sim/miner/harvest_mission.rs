//! Harvest mission handler seam (AI-shell migration, Slice L5).
//!
//! Re-expresses the existing miner `match miner.state` FSM as the Harvest
//! mission handler the per-object Techno-AI shell dispatches to. THIS SLICE is
//! a pure routing seam: `harvest_mission_step` calls the unchanged
//! `process_miner`, so the dock handshake, deposit cadence, slot order, credit
//! identity, RNG consumption, live-object iteration order, and the lockstep
//! state hash are all bit-identical to calling `process_miner` directly. The
//! seam only relocates the *entry point* — it is the named handler the shell
//! reaches for `MissionType::Harvest`; the FSM bodies and their order are
//! untouched.
//!
//! What this slice deliberately does NOT do (each is its own later, gated flip):
//! - It does NOT make `MissionCom.substate` the FSM cursor — `miner.state`
//!   stays the authoritative cursor. The substate-authority flip is shell S5
//!   (depends on mission/radio Slice 6).
//! - It does NOT change the dock admission source — the refinery registry
//!   (`production.dock_reservations`, type `RefineryDockContacts`) stays the
//!   admission decision; the `RadioBus` (`radio_contacts` / `dock_entered_with`)
//!   stays the lockstep shadow, gated on the registry decision. Un-gating the
//!   bus so `refinery_hello` admits independently, then retiring the registry
//!   mirror and its hash folds, is the registry-retire slice the dock code
//!   labels "Slice 8".
//! - It does NOT add or remove any hashed field and does NOT bump
//!   `SNAPSHOT_VERSION` — L5 flips nothing authoritative.
//!
//! Depends on: `world::Simulation`, `miner::miner_system::process_miner`.
//! Must NOT depend on render/ui/sidebar/audio/net (sim invariant #1).
//! Dispatch stays the existing `match miner.state` — no trait / dyn / vtable
//! (invariant #2).

use crate::map::overlay_types::OverlayTypeRegistry;
use crate::rules::ruleset::RuleSet;
use crate::sim::pathfinding::PathGrid;
use crate::sim::world::Simulation;

use super::MinerConfig;
use super::miner_system::{MinerSnapshot, process_miner};

/// Dispatch one miner through the Harvest mission handler for this tick.
///
/// Called per snapshot, in live-object order, from the miner tick (and, once
/// the full shell owns dispatch, from the `Unit` arm of the per-category
/// shell). It runs the existing `process_miner` FSM unchanged — a
/// function-move-by-reference, not a rewrite.
pub(super) fn harvest_mission_step(
    sim: &mut Simulation,
    rules: &RuleSet,
    config: &MinerConfig,
    path_grid: Option<&PathGrid>,
    overlay_registry: Option<&OverlayTypeRegistry>,
    snap: &mut MinerSnapshot,
) {
    // Shadow-agreement check (debug-only, never hashed): at dispatch entry the
    // entity's derived MissionCom must already classify this object as Harvest
    // with the FSM cursor as its sub-phase. This pins the derived-mission ↔
    // FSM-cursor mapping so the later substate-authority flip (shell S5) has a
    // proven invariant. `snap.miner` is the pre-process snapshot, equal to the
    // live entity's miner here (write-back is the miner tick's Phase 3), so the
    // two agree at entry. A divergence is surfaced, never silently equalized.
    #[cfg(debug_assertions)]
    if let Some(entity) = sim.substrate.entities.get(snap.entity_id) {
        debug_assert_eq!(
            entity.derived_mission(),
            (
                crate::sim::mission::MissionType::Harvest,
                snap.miner.state as u8
            ),
            "L5: entity {} derived mission must be Harvest with the FSM cursor \
             ({:?}) as its sub-phase",
            snap.entity_id,
            snap.miner.state,
        );
    }

    // Routing seam only: run the existing FSM unchanged. No reorder, no new
    // authority, no hash movement, no RNG-position change.
    process_miner(sim, rules, config, path_grid, overlay_registry, snap);
}
